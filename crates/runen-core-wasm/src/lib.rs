#![forbid(unsafe_code)]
//! First production physical realization for a deliberately bounded Runen Core subset.
//!
//! The WebAssembly representation and Wasmtime protocol in this crate are private
//! implementation details. `runen-core-ir` remains the semantic program boundary.

mod coverage;
mod encoding;
mod scalar;

use std::error::Error;
use std::fmt;

use runen_core_ir::{Fault, FunctionId, ValidatedProgram, Value};
use wasmtime::{Engine, Instance, Module, Store};

pub use coverage::{
    CoverageError, CoverageErrorKind, CoverageLocation, UnsupportedOperandKind,
    UnsupportedStatementKind, UnsupportedTerminatorKind, UnsupportedTypeCategory,
};
use encoding::{EncodedProgram, EntryInfo, entry_export_name};

const STATUS_RETURNED: i32 = 0;
const STATUS_FAULTED: i32 = 1;
const INVALID_BACKEND_RESULT: &str = "private backend produced an invalid result";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Returned(Option<Value>),
    Faulted(Fault),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendPhase {
    Compile,
    Instantiate,
    LookupEntry,
    Execute,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RealizationError {
    Coverage(CoverageError),
    InvalidEntry(FunctionId),
    EntryHasParameters(FunctionId),
    Backend {
        phase: BackendPhase,
        message: String,
    },
    BackendInvariant(String),
}

impl fmt::Display for RealizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coverage(error) => write!(
                formatter,
                "unsupported Core realization coverage: {error:?}"
            ),
            Self::InvalidEntry(function) => {
                write!(formatter, "invalid Core entry function: {function:?}")
            }
            Self::EntryHasParameters(function) => {
                write!(
                    formatter,
                    "Core entry function has parameters: {function:?}"
                )
            }
            Self::Backend { phase, message } => {
                write!(
                    formatter,
                    "Wasmtime backend failure during {phase:?}: {message}"
                )
            }
            Self::BackendInvariant(message) => {
                write!(
                    formatter,
                    "Core Wasm realization invariant failed: {message}"
                )
            }
        }
    }
}

impl Error for RealizationError {}

impl From<CoverageError> for RealizationError {
    fn from(error: CoverageError) -> Self {
        Self::Coverage(error)
    }
}

pub struct RealizedProgram {
    engine: Engine,
    module: Module,
    faults: Vec<Fault>,
    entries: Vec<EntryInfo>,
}

impl RealizedProgram {
    pub fn new(program: &ValidatedProgram) -> Result<Self, RealizationError> {
        coverage::validate(program)?;
        let EncodedProgram {
            bytes,
            faults,
            entries,
        } = encoding::encode(program)?;
        let engine = Engine::default();
        let module = Module::new(&engine, &bytes).map_err(|error| RealizationError::Backend {
            phase: BackendPhase::Compile,
            message: error.to_string(),
        })?;
        Ok(Self {
            engine,
            module,
            faults,
            entries,
        })
    }

    pub fn execute(&self, entry: FunctionId) -> Result<ExecutionOutcome, RealizationError> {
        let entry_info = self
            .entries
            .get(entry.0 as usize)
            .copied()
            .ok_or(RealizationError::InvalidEntry(entry))?;
        if entry_info.parameter_count != 0 {
            return Err(RealizationError::EntryHasParameters(entry));
        }

        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &self.module, &[]).map_err(|error| {
            RealizationError::Backend {
                phase: BackendPhase::Instantiate,
                message: error.to_string(),
            }
        })?;
        let function = instance
            .get_typed_func::<(), (i32, i64)>(&mut store, &entry_export_name(entry))
            .map_err(|error| RealizationError::Backend {
                phase: BackendPhase::LookupEntry,
                message: error.to_string(),
            })?;
        let (status, payload) =
            function
                .call(&mut store, ())
                .map_err(|error| RealizationError::Backend {
                    phase: BackendPhase::Execute,
                    message: error.to_string(),
                })?;

        match status {
            STATUS_RETURNED => {
                let result = entry_info
                    .result
                    .map(|kind| kind.decode(payload))
                    .transpose()?;
                Ok(ExecutionOutcome::Returned(result))
            }
            STATUS_FAULTED => {
                let index = usize::try_from(payload).map_err(|_| invalid_backend_result())?;
                let fault = self
                    .faults
                    .get(index)
                    .cloned()
                    .ok_or_else(invalid_backend_result)?;
                Ok(ExecutionOutcome::Faulted(fault))
            }
            _ => Err(invalid_backend_result()),
        }
    }
}

pub(crate) fn invalid_backend_result() -> RealizationError {
    RealizationError::BackendInvariant(INVALID_BACKEND_RESULT.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use runen_core_ir::{
        BasicBlock, BasicBlockId, Body, Function, PersistentDecl, Program,
        SafeReferenceResultContract, ScalarType, Terminator, TypeDef, TypeTable, Value,
        validate_program,
    };

    fn empty_entry_program(types: TypeTable, persistent: Vec<PersistentDecl>) -> ValidatedProgram {
        validate_program(Program {
            types,
            persistent,
            external_callables: Vec::new(),
            functions: vec![Function {
                name: "entry".into(),
                parameters: Vec::new(),
                result: None,
                safe_reference_result_contract: SafeReferenceResultContract::None,
                body: Body {
                    locals: Vec::new(),
                    loans: Vec::new(),
                    entry: BasicBlockId(0),
                    blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
                },
            }],
        })
        .expect("module-shape fixture must be valid Core")
    }

    #[test]
    fn generated_modules_without_persistents_keep_the_original_reviewed_shape() {
        let program = empty_entry_program(TypeTable::new(), Vec::new());
        let encoded = encoding::encode(&program).expect("supported fixture must encode");

        assert!(
            encoded.bytes.starts_with(b"\0asm\x01\0\0\0"),
            "generated artifact must use the core WebAssembly v1 header"
        );
        let non_custom_sections: Vec<_> = section_ids(&encoded.bytes[8..])
            .into_iter()
            .filter(|id| *id != 0)
            .collect();
        assert_eq!(
            non_custom_sections,
            vec![1, 3, 7, 10],
            "persistent-free modules may contain only type, function, export, and code sections"
        );
    }

    #[test]
    fn persistent_modules_add_only_private_immutable_i64_globals() {
        let mut types = TypeTable::new();
        let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
        let program = empty_entry_program(
            types,
            vec![
                PersistentDecl::new(i64_ty, Value::I64(7)),
                PersistentDecl::new(i64_ty, Value::I64(7)),
            ],
        );
        let encoded = encoding::encode(&program).expect("supported persistent fixture must encode");
        let module = &encoded.bytes[8..];

        let non_custom_sections: Vec<_> = section_ids(module)
            .into_iter()
            .filter(|id| *id != 0)
            .collect();
        assert_eq!(
            non_custom_sections,
            vec![1, 3, 6, 7, 10],
            "persistents may add only the core Wasm global section"
        );

        let globals = section_payload(module, 6);
        let mut global_cursor = 0_usize;
        assert_eq!(
            read_u32_leb(globals, &mut global_cursor),
            2,
            "equal-valued Core declarations must remain two private globals"
        );
        assert_eq!(globals.get(global_cursor), Some(&0x7e), "global must be i64");
        assert_eq!(
            globals.get(global_cursor + 1),
            Some(&0x00),
            "global must be immutable"
        );

        assert_eq!(
            export_kinds(section_payload(module, 7)),
            vec![0],
            "only the entry function may be exported; persistent globals stay private"
        );
    }

    fn section_ids(bytes: &[u8]) -> Vec<u8> {
        let mut cursor = 0_usize;
        let mut ids = Vec::new();
        while cursor < bytes.len() {
            let section_id = bytes[cursor];
            cursor += 1;
            let payload_len = read_u32_leb(bytes, &mut cursor);
            let end = cursor
                .checked_add(payload_len)
                .expect("section payload length must fit usize");
            assert!(end <= bytes.len(), "section payload must fit module bytes");
            ids.push(section_id);
            cursor = end;
        }
        ids
    }

    fn section_payload(bytes: &[u8], wanted: u8) -> &[u8] {
        let mut cursor = 0_usize;
        while cursor < bytes.len() {
            let section_id = bytes[cursor];
            cursor += 1;
            let payload_len = read_u32_leb(bytes, &mut cursor);
            let start = cursor;
            let end = start
                .checked_add(payload_len)
                .expect("section payload length must fit usize");
            assert!(end <= bytes.len(), "section payload must fit module bytes");
            if section_id == wanted {
                return &bytes[start..end];
            }
            cursor = end;
        }
        panic!("missing expected Wasm section {wanted}");
    }

    fn export_kinds(bytes: &[u8]) -> Vec<u8> {
        let mut cursor = 0_usize;
        let count = read_u32_leb(bytes, &mut cursor);
        let mut kinds = Vec::with_capacity(count);
        for _ in 0..count {
            let name_len = read_u32_leb(bytes, &mut cursor);
            cursor = cursor
                .checked_add(name_len)
                .expect("export name length must fit usize");
            assert!(cursor < bytes.len(), "export kind must be present");
            kinds.push(bytes[cursor]);
            cursor += 1;
            let _index = read_u32_leb(bytes, &mut cursor);
        }
        assert_eq!(cursor, bytes.len(), "export section must be consumed exactly");
        kinds
    }

    fn read_u32_leb(bytes: &[u8], cursor: &mut usize) -> usize {
        let mut result = 0_usize;
        let mut shift = 0_u32;
        loop {
            let byte = *bytes
                .get(*cursor)
                .expect("section size LEB must be present in generated module");
            *cursor += 1;
            result |= usize::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return result;
            }
            shift += 7;
            assert!(shift < 35, "section size LEB must fit u32");
        }
    }
}
