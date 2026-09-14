#![forbid(unsafe_code)]
//! First production physical realization for a deliberately bounded Runen Core subset.
//!
//! The WebAssembly representation and Wasmtime protocol in this crate are private
//! implementation details. `runen-core-ir` remains the semantic program boundary.

mod coverage;
mod encoding;
mod layout;
mod scalar;

use std::error::Error;
use std::fmt;

use runen_core_ir::{Fault, FunctionId, TypeTable, ValidatedProgram, Value};
use wasmtime::{Engine, Instance, Module, Store, Val};

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
    types: TypeTable,
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
            types: program.as_program().types.clone(),
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
            .get_func(&mut store, &entry_export_name(entry))
            .ok_or_else(|| RealizationError::Backend {
                phase: BackendPhase::LookupEntry,
                message: "entry export is missing".into(),
            })?;
        let payload_count = entry_info.result_carrier_count.max(1);
        let mut results = Vec::with_capacity(payload_count + 1);
        results.push(Val::I32(0));
        results.extend(std::iter::repeat_n(Val::I64(0), payload_count));
        function
            .call(&mut store, &[], &mut results)
            .map_err(|error| RealizationError::Backend {
                phase: BackendPhase::Execute,
                message: error.to_string(),
            })?;

        let status = match results.first() {
            Some(Val::I32(status)) => *status,
            _ => return Err(invalid_backend_result()),
        };
        let payloads = results[1..]
            .iter()
            .map(|value| match value {
                Val::I64(value) => Ok(*value),
                _ => Err(invalid_backend_result()),
            })
            .collect::<Result<Vec<_>, _>>()?;

        match status {
            STATUS_RETURNED => {
                let result = entry_info
                    .result
                    .map(|ty| {
                        layout::decode_value(
                            &self.types,
                            ty,
                            &payloads[..entry_info.result_carrier_count],
                        )
                    })
                    .transpose()?;
                Ok(ExecutionOutcome::Returned(result))
            }
            STATUS_FAULTED => {
                let payload = payloads
                    .first()
                    .copied()
                    .ok_or_else(invalid_backend_result)?;
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
        BasicBlock, BasicBlockId, Body, CallableInterface, Field, Function, LocalDecl, LocalId,
        Operand, PersistentDecl, Place, Program, SafeReferenceResultContract, ScalarType,
        Statement, Terminator, TypeDef, TypeId, TypeTable, Value, validate_program,
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

    fn aggregate_program() -> ValidatedProgram {
        let mut types = TypeTable::new();
        let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
        let u8_ty = types.push(TypeDef::scalar("U8", ScalarType::U8));
        let pair_ty = types.push(TypeDef::structure(
            "Pair",
            vec![Field::new("left", i64_ty), Field::new("right", u8_ty)],
        ));
        validate_program(Program {
            types,
            persistent: Vec::new(),
            external_callables: Vec::new(),
            functions: vec![Function {
                name: "entry".into(),
                parameters: Vec::new(),
                result: Some(pair_ty),
                safe_reference_result_contract: SafeReferenceResultContract::None,
                body: Body {
                    locals: Vec::new(),
                    loans: Vec::new(),
                    entry: BasicBlockId(0),
                    blocks: vec![BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Constant(Value::Struct(vec![
                            Value::I64(42),
                            Value::U8(7),
                        ])))),
                    )],
                },
            }],
        })
        .expect("aggregate module-shape fixture must be valid Core")
    }

    fn callable_program(
        persistent: Vec<PersistentDecl>,
        types: TypeTable,
        callable: TypeId,
    ) -> ValidatedProgram {
        validate_program(Program {
            types,
            persistent,
            external_callables: Vec::new(),
            functions: vec![
                Function {
                    name: "entry".into(),
                    parameters: Vec::new(),
                    result: None,
                    safe_reference_result_contract: SafeReferenceResultContract::None,
                    body: Body {
                        locals: vec![LocalDecl::new("callee", callable, false)],
                        loans: Vec::new(),
                        entry: BasicBlockId(0),
                        blocks: vec![
                            BasicBlock::new(
                                vec![Statement::Init {
                                    dst: Place::local(LocalId(0)),
                                    src: Operand::FunctionValue(FunctionId(1)),
                                }],
                                Terminator::IndirectCall {
                                    callable,
                                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                                    arguments: Vec::new(),
                                    destination: None,
                                    target: BasicBlockId(1),
                                },
                            ),
                            BasicBlock::new(Vec::new(), Terminator::Return(None)),
                        ],
                    },
                },
                Function {
                    name: "target".into(),
                    parameters: Vec::new(),
                    result: None,
                    safe_reference_result_contract: SafeReferenceResultContract::None,
                    body: Body {
                        locals: Vec::new(),
                        loans: Vec::new(),
                        entry: BasicBlockId(0),
                        blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
                    },
                },
            ],
        })
        .expect("callable module-shape fixture must be valid Core")
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
            "persistent-free callable-free modules retain the reviewed section shape"
        );
    }

    #[test]
    fn aggregate_modules_add_no_storage_or_import_sections() {
        let encoded = encoding::encode(&aggregate_program())
            .expect("supported aggregate fixture must encode");
        let module = &encoded.bytes[8..];
        assert_eq!(
            section_ids(module),
            vec![1, 3, 7, 10],
            "structural carriers must not add imports, tables, memory, globals, elements, or data"
        );
        assert_eq!(
            export_kinds(section_payload(module, 7)),
            vec![0],
            "aggregate realization exports only the existing entry function"
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
        assert_eq!(
            globals.get(global_cursor),
            Some(&0x7e),
            "global must be i64"
        );
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

    #[test]
    fn callable_modules_add_only_private_table_and_element_sections() {
        let mut types = TypeTable::new();
        let callable = types.push(TypeDef::callable(
            "Thunk",
            CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
        ));
        let program = callable_program(Vec::new(), types, callable);
        let encoded = encoding::encode(&program).expect("supported callable fixture must encode");
        let module = &encoded.bytes[8..];

        assert_eq!(
            section_ids(module),
            vec![1, 3, 4, 7, 9, 10],
            "callable dispatch may add only table and element sections"
        );
        assert_private_two_function_table(section_payload(module, 4));
        assert_two_function_element_population(section_payload(module, 9));
        assert_eq!(
            export_kinds(section_payload(module, 7)),
            vec![0, 0],
            "existing zero-parameter function exports remain function-only; the table stays private"
        );
    }

    #[test]
    fn callable_and_persistent_private_sections_compose_without_new_exports() {
        let mut types = TypeTable::new();
        let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
        let callable = types.push(TypeDef::callable(
            "Thunk",
            CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
        ));
        let program = callable_program(
            vec![PersistentDecl::new(i64_ty, Value::I64(7))],
            types,
            callable,
        );
        let encoded =
            encoding::encode(&program).expect("supported callable/persistent fixture must encode");
        let module = &encoded.bytes[8..];

        assert_eq!(section_ids(module), vec![1, 3, 4, 6, 7, 9, 10]);
        assert_private_two_function_table(section_payload(module, 4));
        assert_two_function_element_population(section_payload(module, 9));
        assert!(
            export_kinds(section_payload(module, 7))
                .into_iter()
                .all(|kind| kind == 0),
            "neither callable tables nor persistent globals may be exported"
        );
    }

    fn assert_private_two_function_table(bytes: &[u8]) {
        let mut cursor = 0_usize;
        assert_eq!(read_u32_leb(bytes, &mut cursor), 1, "exactly one table");
        assert_eq!(bytes.get(cursor), Some(&0x70), "table must be funcref");
        cursor += 1;
        assert_eq!(
            read_u32_leb(bytes, &mut cursor),
            1,
            "table limits must include an exact maximum"
        );
        assert_eq!(read_u32_leb(bytes, &mut cursor), 2, "table minimum");
        assert_eq!(read_u32_leb(bytes, &mut cursor), 2, "table maximum");
        assert_eq!(
            cursor,
            bytes.len(),
            "table section must be consumed exactly"
        );
    }

    fn assert_two_function_element_population(bytes: &[u8]) {
        let mut cursor = 0_usize;
        assert_eq!(read_u32_leb(bytes, &mut cursor), 1, "one element segment");
        assert_eq!(
            read_u32_leb(bytes, &mut cursor),
            0,
            "active table-zero segment"
        );
        assert_eq!(bytes.get(cursor), Some(&0x41), "offset uses i32.const");
        cursor += 1;
        assert_eq!(bytes.get(cursor), Some(&0x00), "offset is zero");
        cursor += 1;
        assert_eq!(bytes.get(cursor), Some(&0x0b), "offset expression ends");
        cursor += 1;
        assert_eq!(read_u32_leb(bytes, &mut cursor), 2, "two function elements");
        assert_eq!(read_u32_leb(bytes, &mut cursor), 0);
        assert_eq!(read_u32_leb(bytes, &mut cursor), 1);
        assert_eq!(
            cursor,
            bytes.len(),
            "element section must be consumed exactly"
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
        assert_eq!(
            cursor,
            bytes.len(),
            "export section must be consumed exactly"
        );
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
