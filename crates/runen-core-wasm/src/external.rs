use std::error::Error;
use std::fmt;
use std::sync::Arc;

use runen_core_ir::{CallableInterface, ExternalCallableId, TypeTable, ValidatedProgram, Value};
use wasmtime::{Engine, Extern, Func, FuncType, Store, Val, ValType};

use crate::RealizationError;
use crate::scalar::{ScalarKind, constant_residue};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalScalarValue {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
}

impl ExternalScalarValue {
    fn into_core_value(self) -> Value {
        match self {
            Self::Bool(value) => Value::Bool(value),
            Self::I8(value) => Value::I8(value),
            Self::I16(value) => Value::I16(value),
            Self::I32(value) => Value::I32(value),
            Self::I64(value) => Value::I64(value),
            Self::U8(value) => Value::U8(value),
            Self::U16(value) => Value::U16(value),
            Self::U32(value) => Value::U32(value),
            Self::U64(value) => Value::U64(value),
        }
    }

    fn from_core_value(value: Value) -> Option<Self> {
        match value {
            Value::Bool(value) => Some(Self::Bool(value)),
            Value::I8(value) => Some(Self::I8(value)),
            Value::I16(value) => Some(Self::I16(value)),
            Value::I32(value) => Some(Self::I32(value)),
            Value::I64(value) => Some(Self::I64(value)),
            Value::U8(value) => Some(Self::U8(value)),
            Value::U16(value) => Some(Self::U16(value)),
            Value::U32(value) => Some(Self::U32(value)),
            Value::U64(value) => Some(Self::U64(value)),
            Value::F16(_)
            | Value::F32(_)
            | Value::F64(_)
            | Value::TrackedFixture(_)
            | Value::Struct(_) => None,
        }
    }

    fn matches(self, kind: ScalarKind) -> bool {
        matches!(
            (self, kind),
            (Self::Bool(_), ScalarKind::Bool)
                | (Self::I8(_), ScalarKind::I8)
                | (Self::I16(_), ScalarKind::I16)
                | (Self::I32(_), ScalarKind::I32)
                | (Self::I64(_), ScalarKind::I64)
                | (Self::U8(_), ScalarKind::U8)
                | (Self::U16(_), ScalarKind::U16)
                | (Self::U32(_), ScalarKind::U32)
                | (Self::U64(_), ScalarKind::U64)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalProviderFailure {
    message: String,
}

impl ExternalProviderFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ExternalProviderFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ExternalProviderFailure {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalProviderAdmissionError {
    MissingProvider(ExternalCallableId),
    DuplicateProvider(ExternalCallableId),
    UnknownProvider(ExternalCallableId),
    InterfaceMismatch {
        external: ExternalCallableId,
        expected: CallableInterface,
        found: CallableInterface,
    },
}

impl fmt::Display for ExternalProviderAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "external provider admission failed: {self:?}")
    }
}

impl Error for ExternalProviderAdmissionError {}

type Provider = dyn Fn(&[ExternalScalarValue]) -> Result<Option<ExternalScalarValue>, ExternalProviderFailure>
    + Send
    + Sync
    + 'static;

#[derive(Clone)]
pub struct ExternalProviderBinding {
    external: ExternalCallableId,
    interface: CallableInterface,
    provider: Arc<Provider>,
}

impl fmt::Debug for ExternalProviderBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExternalProviderBinding")
            .field("external", &self.external)
            .field("interface", &self.interface)
            .finish_non_exhaustive()
    }
}

impl ExternalProviderBinding {
    pub fn no_result(
        external: ExternalCallableId,
        interface: CallableInterface,
        provider: impl Fn(&[ExternalScalarValue]) + Send + Sync + 'static,
    ) -> Self {
        Self::try_no_result(external, interface, move |arguments| {
            provider(arguments);
            Ok(())
        })
    }

    pub fn scalar_result(
        external: ExternalCallableId,
        interface: CallableInterface,
        provider: impl Fn(&[ExternalScalarValue]) -> ExternalScalarValue + Send + Sync + 'static,
    ) -> Self {
        Self::try_scalar_result(external, interface, move |arguments| {
            Ok(provider(arguments))
        })
    }

    pub fn try_no_result(
        external: ExternalCallableId,
        interface: CallableInterface,
        provider: impl Fn(&[ExternalScalarValue]) -> Result<(), ExternalProviderFailure>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            external,
            interface,
            provider: Arc::new(move |arguments| {
                provider(arguments)?;
                Ok(None)
            }),
        }
    }

    pub fn try_scalar_result(
        external: ExternalCallableId,
        interface: CallableInterface,
        provider: impl Fn(
            &[ExternalScalarValue],
        ) -> Result<ExternalScalarValue, ExternalProviderFailure>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            external,
            interface,
            provider: Arc::new(move |arguments| provider(arguments).map(Some)),
        }
    }

    pub fn external(&self) -> ExternalCallableId {
        self.external
    }

    pub fn interface(&self) -> &CallableInterface {
        &self.interface
    }
}

pub(crate) fn admit(
    program: &ValidatedProgram,
    bindings: Vec<ExternalProviderBinding>,
) -> Result<Vec<ExternalProviderBinding>, ExternalProviderAdmissionError> {
    let program = program.as_program();
    let mut admitted = (0..program.external_callables.len())
        .map(|_| None)
        .collect::<Vec<Option<ExternalProviderBinding>>>();

    for binding in bindings {
        let index = binding.external.0 as usize;
        let Some(declaration) = program.external_callables.get(index) else {
            return Err(ExternalProviderAdmissionError::UnknownProvider(
                binding.external,
            ));
        };
        if admitted[index].is_some() {
            return Err(ExternalProviderAdmissionError::DuplicateProvider(
                binding.external,
            ));
        }
        if binding.interface != declaration.interface {
            return Err(ExternalProviderAdmissionError::InterfaceMismatch {
                external: binding.external,
                expected: declaration.interface.clone(),
                found: binding.interface,
            });
        }
        admitted[index] = Some(binding);
    }

    admitted
        .into_iter()
        .enumerate()
        .map(|(index, binding)| {
            binding.ok_or_else(|| {
                ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(
                    u32::try_from(index).expect("validated external declaration index fits u32"),
                ))
            })
        })
        .collect()
}

pub(crate) fn instantiate_imports(
    engine: &Engine,
    store: &mut Store<()>,
    types: &TypeTable,
    providers: &[ExternalProviderBinding],
) -> Result<Vec<Extern>, RealizationError> {
    providers
        .iter()
        .map(|binding| {
            let parameter_kinds = binding
                .interface
                .parameters
                .iter()
                .copied()
                .map(|ty| {
                    ScalarKind::from_type(types, ty).ok_or_else(|| {
                        RealizationError::BackendInvariant(
                            "coverage admitted an unsupported external provider parameter".into(),
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let result_kind = binding
                .interface
                .result
                .map(|ty| {
                    ScalarKind::from_type(types, ty).ok_or_else(|| {
                        RealizationError::BackendInvariant(
                            "coverage admitted an unsupported external provider result".into(),
                        )
                    })
                })
                .transpose()?;
            let function_type = FuncType::new(
                engine,
                std::iter::repeat_n(ValType::I64, parameter_kinds.len()),
                result_kind.into_iter().map(|_| ValType::I64),
            );
            let external = binding.external;
            let provider = Arc::clone(&binding.provider);
            let function = Func::new(
                &mut *store,
                function_type,
                move |_caller, parameters, results| {
                    if parameters.len() != parameter_kinds.len() {
                        return Err(host_error(
                            external,
                            "private provider import received the wrong argument count",
                        ));
                    }
                    let mut arguments = Vec::with_capacity(parameter_kinds.len());
                    for (parameter, kind) in parameters.iter().zip(parameter_kinds.iter().copied()) {
                        let Val::I64(payload) = parameter else {
                            return Err(host_error(
                                external,
                                "private provider import received a non-i64 carrier",
                            ));
                        };
                        let value = kind.decode(*payload).map_err(|_| {
                            host_error(external, "private provider argument carrier was invalid")
                        })?;
                        arguments.push(
                            ExternalScalarValue::from_core_value(value).ok_or_else(|| {
                                host_error(external, "private provider argument was not scalar")
                            })?,
                        );
                    }

                    let returned = provider(&arguments).map_err(|error| {
                        host_error(
                            external,
                            format!("provider implementation failed: {}", error.message()),
                        )
                    })?;
                    match (result_kind, returned) {
                        (None, None) => {
                            if !results.is_empty() {
                                return Err(host_error(
                                    external,
                                    "private no-result provider import had result storage",
                                ));
                            }
                        }
                        (None, Some(_)) => {
                            return Err(host_error(
                                external,
                                "no-result provider returned a semantic value",
                            ));
                        }
                        (Some(_), None) => {
                            return Err(host_error(
                                external,
                                "result-bearing provider returned no semantic value",
                            ));
                        }
                        (Some(kind), Some(value)) => {
                            if !value.matches(kind) {
                                return Err(host_error(
                                    external,
                                    "provider returned a value of the wrong semantic scalar type",
                                ));
                            }
                            if results.len() != 1 {
                                return Err(host_error(
                                    external,
                                    "private provider import had the wrong result count",
                                ));
                            }
                            let residue = constant_residue(&value.into_core_value()).ok_or_else(|| {
                                host_error(external, "provider result had no scalar carrier")
                            })?;
                            results[0] = Val::I64(i64::from_ne_bytes(residue.to_ne_bytes()));
                        }
                    }
                    Ok(())
                },
            );
            Ok(Extern::Func(function))
        })
        .collect()
}

fn host_error(external: ExternalCallableId, message: impl Into<String>) -> wasmtime::Error {
    wasmtime::Error::msg(format!(
        "Runen external provider {:?}: {}",
        external,
        message.into()
    ))
}
