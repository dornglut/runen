use std::error::Error;
use std::fmt;
use std::sync::Arc;

use runen_core_ir::{CallableInterface, ExternalCallableId, TypeTable, ValidatedProgram, Value};
use wasmtime::{Engine, Extern, Func, FuncType, Store, Val, ValType};

use crate::RealizationError;
use crate::invalid_backend_result;
use crate::scalar::{FloatingScalarValue, ScalarKind, constant_residue};

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
    F16(FloatingScalarValue),
    F32(FloatingScalarValue),
    F64(FloatingScalarValue),
}

impl ExternalScalarValue {
    fn from_carrier(kind: ScalarKind, payload: i64) -> Result<Self, RealizationError> {
        if kind.is_floating() {
            let value = kind.decode_floating(payload)?;
            return Ok(match kind {
                ScalarKind::F16 => Self::F16(value),
                ScalarKind::F32 => Self::F32(value),
                ScalarKind::F64 => Self::F64(value),
                ScalarKind::Bool
                | ScalarKind::I8
                | ScalarKind::I16
                | ScalarKind::I32
                | ScalarKind::I64
                | ScalarKind::U8
                | ScalarKind::U16
                | ScalarKind::U32
                | ScalarKind::U64 => return Err(invalid_backend_result()),
            });
        }
        Self::from_core_value(kind.decode(payload)?).ok_or_else(invalid_backend_result)
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
        match (self, kind) {
            (Self::Bool(_), ScalarKind::Bool)
            | (Self::I8(_), ScalarKind::I8)
            | (Self::I16(_), ScalarKind::I16)
            | (Self::I32(_), ScalarKind::I32)
            | (Self::I64(_), ScalarKind::I64)
            | (Self::U8(_), ScalarKind::U8)
            | (Self::U16(_), ScalarKind::U16)
            | (Self::U32(_), ScalarKind::U32)
            | (Self::U64(_), ScalarKind::U64) => true,
            (Self::F16(value), ScalarKind::F16)
            | (Self::F32(value), ScalarKind::F32)
            | (Self::F64(value), ScalarKind::F64) => match value {
                FloatingScalarValue::Represented(value) => kind.floating_value_matches(value),
                FloatingScalarValue::NaNClass => true,
            },
            _ => false,
        }
    }

    fn carrier_residue(self, kind: ScalarKind) -> Result<u64, RealizationError> {
        match (self, kind) {
            (Self::Bool(value), ScalarKind::Bool) => {
                constant_residue(&Value::Bool(value)).ok_or_else(invalid_backend_result)
            }
            (Self::I8(value), ScalarKind::I8) => {
                constant_residue(&Value::I8(value)).ok_or_else(invalid_backend_result)
            }
            (Self::I16(value), ScalarKind::I16) => {
                constant_residue(&Value::I16(value)).ok_or_else(invalid_backend_result)
            }
            (Self::I32(value), ScalarKind::I32) => {
                constant_residue(&Value::I32(value)).ok_or_else(invalid_backend_result)
            }
            (Self::I64(value), ScalarKind::I64) => {
                constant_residue(&Value::I64(value)).ok_or_else(invalid_backend_result)
            }
            (Self::U8(value), ScalarKind::U8) => {
                constant_residue(&Value::U8(value)).ok_or_else(invalid_backend_result)
            }
            (Self::U16(value), ScalarKind::U16) => {
                constant_residue(&Value::U16(value)).ok_or_else(invalid_backend_result)
            }
            (Self::U32(value), ScalarKind::U32) => {
                constant_residue(&Value::U32(value)).ok_or_else(invalid_backend_result)
            }
            (Self::U64(value), ScalarKind::U64) => {
                constant_residue(&Value::U64(value)).ok_or_else(invalid_backend_result)
            }
            (Self::F16(value), ScalarKind::F16)
            | (Self::F32(value), ScalarKind::F32)
            | (Self::F64(value), ScalarKind::F64) => kind.floating_residue(value),
            _ => Err(invalid_backend_result()),
        }
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
    ProviderResultShapeMismatch {
        external: ExternalCallableId,
        declaration_has_result: bool,
        provider_has_result: bool,
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

    pub fn no_result_for_program(
        program: &ValidatedProgram,
        external: ExternalCallableId,
        provider: impl Fn(&[ExternalScalarValue]) + Send + Sync + 'static,
    ) -> Result<Self, ExternalProviderAdmissionError> {
        Self::try_no_result_for_program(program, external, move |arguments| {
            provider(arguments);
            Ok(())
        })
    }

    pub fn scalar_result_for_program(
        program: &ValidatedProgram,
        external: ExternalCallableId,
        provider: impl Fn(&[ExternalScalarValue]) -> ExternalScalarValue + Send + Sync + 'static,
    ) -> Result<Self, ExternalProviderAdmissionError> {
        Self::try_scalar_result_for_program(program, external, move |arguments| {
            Ok(provider(arguments))
        })
    }

    pub fn try_no_result_for_program(
        program: &ValidatedProgram,
        external: ExternalCallableId,
        provider: impl Fn(&[ExternalScalarValue]) -> Result<(), ExternalProviderFailure>
        + Send
        + Sync
        + 'static,
    ) -> Result<Self, ExternalProviderAdmissionError> {
        let interface = interface_for_program(program, external, false)?;
        Ok(Self::try_no_result(external, interface, provider))
    }

    pub fn try_scalar_result_for_program(
        program: &ValidatedProgram,
        external: ExternalCallableId,
        provider: impl Fn(
            &[ExternalScalarValue],
        ) -> Result<ExternalScalarValue, ExternalProviderFailure>
        + Send
        + Sync
        + 'static,
    ) -> Result<Self, ExternalProviderAdmissionError> {
        let interface = interface_for_program(program, external, true)?;
        Ok(Self::try_scalar_result(external, interface, provider))
    }

    pub fn external(&self) -> ExternalCallableId {
        self.external
    }

    pub fn interface(&self) -> &CallableInterface {
        &self.interface
    }
}

fn interface_for_program(
    program: &ValidatedProgram,
    external: ExternalCallableId,
    provider_has_result: bool,
) -> Result<CallableInterface, ExternalProviderAdmissionError> {
    let declaration = program
        .as_program()
        .external_callables
        .get(external.0 as usize)
        .ok_or(ExternalProviderAdmissionError::UnknownProvider(external))?;
    let declaration_has_result = declaration.interface.result.is_some();
    if declaration_has_result != provider_has_result {
        return Err(
            ExternalProviderAdmissionError::ProviderResultShapeMismatch {
                external,
                declaration_has_result,
                provider_has_result,
            },
        );
    }
    Ok(declaration.interface.clone())
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
                    for (parameter, kind) in parameters.iter().zip(parameter_kinds.iter().copied())
                    {
                        let Val::I64(payload) = parameter else {
                            return Err(host_error(
                                external,
                                "private provider import received a non-i64 carrier",
                            ));
                        };
                        let value = ExternalScalarValue::from_carrier(kind, *payload).map_err(|_| {
                            host_error(external, "private provider argument carrier was invalid")
                        })?;
                        arguments.push(value);
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
                            let residue = value.carrier_residue(kind).map_err(|_| {
                                host_error(
                                    external,
                                    "provider result did not match the declared semantic scalar format",
                                )
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
