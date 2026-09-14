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
pub enum BackendProtocolError {
    InvalidStatus(i32),
    InvalidFaultIndex(i64),
    InvalidBooleanPayload(i64),
    NonCanonicalIntegerPayload { payload: u64, width: u32 },
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
    BackendProtocol(BackendProtocolError),
    BackendInvariant(String),
}

impl fmt::Display for RealizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coverage(error) => write!(formatter, "unsupported Core realization coverage: {error:?}"),
            Self::InvalidEntry(function) => write!(formatter, "invalid Core entry function: {function:?}"),
            Self::EntryHasParameters(function) => {
                write!(formatter, "Core entry function has parameters: {function:?}")
            }
            Self::Backend { phase, message } => {
                write!(formatter, "Wasmtime backend failure during {phase:?}: {message}")
            }
            Self::BackendProtocol(error) => {
                write!(formatter, "invalid private backend protocol result: {error:?}")
            }
            Self::BackendInvariant(message) => {
                write!(formatter, "Core Wasm realization invariant failed: {message}")
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
        let (status, payload) = function.call(&mut store, ()).map_err(|error| {
            RealizationError::Backend {
                phase: BackendPhase::Execute,
                message: error.to_string(),
            }
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
                let index = usize::try_from(payload).map_err(|_| {
                    RealizationError::BackendProtocol(BackendProtocolError::InvalidFaultIndex(
                        payload,
                    ))
                })?;
                let fault = self.faults.get(index).cloned().ok_or_else(|| {
                    RealizationError::BackendProtocol(BackendProtocolError::InvalidFaultIndex(
                        payload,
                    ))
                })?;
                Ok(ExecutionOutcome::Faulted(fault))
            }
            other => Err(RealizationError::BackendProtocol(
                BackendProtocolError::InvalidStatus(other),
            )),
        }
    }

    #[cfg(test)]
    fn module(&self) -> &Module {
        &self.module
    }
}
