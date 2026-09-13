from pathlib import Path


def replace_exact(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:120]!r}")
    p.write_text(text.replace(old, new, count))

# Reference float carrier conversion stays semantic and representation-neutral.
replace_exact(
    "crates/runen-reference/src/floating.rs",
    "    pub(super) fn from_constant(value: BinaryFloatValue) -> Self {\n        Self::Represented(value)\n    }\n\n    pub(super) fn into_observed(self) -> ObservedBinaryFloatValue {",
    "    pub(super) fn from_constant(value: BinaryFloatValue) -> Self {\n        Self::Represented(value)\n    }\n\n    pub(super) fn from_observed(value: ObservedBinaryFloatValue) -> Self {\n        match value {\n            ObservedBinaryFloatValue::Represented(value) => Self::Represented(value),\n            ObservedBinaryFloatValue::NaNClass => Self::NaNClass,\n        }\n    }\n\n    pub(super) fn into_observed(self) -> ObservedBinaryFloatValue {",
)

p = Path("crates/runen-reference/src/interprocedural.rs")
text = p.read_text()
text = text.replace(
    "    BasicBlockId, BorrowKind, FunctionId, LoanId, LocalId, NumericContract, Operand, PersistentId,\n    Place, PlaceAccess, Projection, ReferenceAccess, ReferencePermission, ScalarType, Statement,\n    StorageInstanceId, StorageRegion, Terminator, TypeId, TypeKind, TypeTable, ValidatedProgram,\n    Value,\n",
    "    BasicBlockId, BorrowKind, CallableInterface, ExternalCallableId, FunctionId, LoanId, LocalId,\n    NumericContract, Operand, PersistentId, Place, PlaceAccess, Projection, ReferenceAccess,\n    ReferencePermission, ScalarType, Statement, StorageInstanceId, StorageRegion, Terminator, TypeId,\n    TypeKind, TypeTable, ValidatedProgram, Value,\n",
    1,
)
text = text.replace(
    "    ObservedValue, RawPointerValue, ReferenceAuthorityId, SafeReferenceValue,\n    UndefinedBehaviorKind, VerificationWriteKind,\n",
    "    ObservedBinaryFloatValue, ObservedValue, RawPointerValue, ReferenceAuthorityId,\n    SafeReferenceValue, UndefinedBehaviorKind, VerificationWriteKind,\n",
    1,
)
entry_anchor = '''/// Why a verification harness entry function cannot be invoked directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryError {
    InvalidFunction(FunctionId),
    EntryHasParameters(FunctionId),
}

'''
provider_types = '''/// Verification-only scalar value admitted at an external provider boundary.
///
/// This carrier is deliberately narrower than `ObservedValue`: it has no aggregate,
/// callable, pointer/reference, storage, or physical-representation variants.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    F16(ObservedBinaryFloatValue),
    F32(ObservedBinaryFloatValue),
    F64(ObservedBinaryFloatValue),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalProviderAdmissionError {
    InvalidExternalCallable(ExternalCallableId),
    DuplicateProvider(ExternalCallableId),
    MissingProvider(ExternalCallableId),
    InterfaceMismatch {
        external: ExternalCallableId,
        expected: CallableInterface,
        found: CallableInterface,
    },
    ResultShapeMismatch {
        external: ExternalCallableId,
        declaration_has_result: bool,
        provider_has_result: bool,
    },
}

/// Why a verification harness machine cannot be constructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryError {
    InvalidFunction(FunctionId),
    EntryHasParameters(FunctionId),
    ExternalProviderAdmission(ExternalProviderAdmissionError),
}

enum ExternalProvider {
    NoResult(Box<dyn FnMut(&[ExternalScalarValue])>),
    ScalarResult(Box<dyn FnMut(&[ExternalScalarValue]) -> ExternalScalarValue>),
}

impl ExternalProvider {
    fn has_result(&self) -> bool {
        matches!(self, Self::ScalarResult(_))
    }

    fn invoke(&mut self, arguments: &[ExternalScalarValue]) -> ExternalProviderReturn {
        match self {
            Self::NoResult(provider) => {
                provider(arguments);
                ExternalProviderReturn::NoResult
            }
            Self::ScalarResult(provider) => ExternalProviderReturn::Scalar(provider(arguments)),
        }
    }
}

enum ExternalProviderReturn {
    NoResult,
    Scalar(ExternalScalarValue),
}

/// One verification-only binding from a Core external requirement to a provider.
///
/// The declared interface participates in pre-execution environment admission. The
/// invocation closure has no represented Runen fault, exception, or UB return channel.
pub struct ExternalProviderBinding {
    external: ExternalCallableId,
    interface: CallableInterface,
    provider: ExternalProvider,
}

impl ExternalProviderBinding {
    #[must_use]
    pub fn no_result<F>(
        external: ExternalCallableId,
        interface: CallableInterface,
        provider: F,
    ) -> Self
    where
        F: FnMut(&[ExternalScalarValue]) + 'static,
    {
        Self {
            external,
            interface,
            provider: ExternalProvider::NoResult(Box::new(provider)),
        }
    }

    #[must_use]
    pub fn scalar_result<F>(
        external: ExternalCallableId,
        interface: CallableInterface,
        provider: F,
    ) -> Self
    where
        F: FnMut(&[ExternalScalarValue]) -> ExternalScalarValue + 'static,
    {
        Self {
            external,
            interface,
            provider: ExternalProvider::ScalarResult(Box::new(provider)),
        }
    }
}

'''
if text.count(entry_anchor) != 1:
    raise SystemExit("reference EntryError anchor mismatch")
text = text.replace(entry_anchor, provider_types, 1)

runtime_anchor = '''    fn into_observed_value(self) -> ObservedValue {
        match self {
'''
runtime_methods = '''    fn into_external_scalar(self) -> ExternalScalarValue {
        match self {
            Self::Bool(value) => ExternalScalarValue::Bool(value),
            Self::I8(value) => ExternalScalarValue::I8(value),
            Self::I16(value) => ExternalScalarValue::I16(value),
            Self::I32(value) => ExternalScalarValue::I32(value),
            Self::I64(value) => ExternalScalarValue::I64(value),
            Self::U8(value) => ExternalScalarValue::U8(value),
            Self::U16(value) => ExternalScalarValue::U16(value),
            Self::U32(value) => ExternalScalarValue::U32(value),
            Self::U64(value) => ExternalScalarValue::U64(value),
            Self::F16(value) => ExternalScalarValue::F16(value.into_observed()),
            Self::F32(value) => ExternalScalarValue::F32(value.into_observed()),
            Self::F64(value) => ExternalScalarValue::F64(value.into_observed()),
            Self::Function(_)
            | Self::RawPointer(_)
            | Self::SafeReference(_)
            | Self::TrackedFixture(_)
            | Self::Struct(_) => unreachable!(
                "validated external call operands contain only admitted scalar values"
            ),
        }
    }

    fn from_external_scalar(value: ExternalScalarValue) -> Self {
        match value {
            ExternalScalarValue::Bool(value) => Self::Bool(value),
            ExternalScalarValue::I8(value) => Self::I8(value),
            ExternalScalarValue::I16(value) => Self::I16(value),
            ExternalScalarValue::I32(value) => Self::I32(value),
            ExternalScalarValue::I64(value) => Self::I64(value),
            ExternalScalarValue::U8(value) => Self::U8(value),
            ExternalScalarValue::U16(value) => Self::U16(value),
            ExternalScalarValue::U32(value) => Self::U32(value),
            ExternalScalarValue::U64(value) => Self::U64(value),
            ExternalScalarValue::F16(value) => Self::F16(RuntimeFloatValue::from_observed(value)),
            ExternalScalarValue::F32(value) => Self::F32(RuntimeFloatValue::from_observed(value)),
            ExternalScalarValue::F64(value) => Self::F64(RuntimeFloatValue::from_observed(value)),
        }
    }

    fn into_observed_value(self) -> ObservedValue {
        match self {
'''
if text.count(runtime_anchor) != 1:
    raise SystemExit("RuntimeValue observation anchor mismatch")
text = text.replace(runtime_anchor, runtime_methods, 1)

machine_fields = '''pub struct Machine {
    program: ValidatedProgram,
    persistent: Vec<PersistentStorage>,
    frames: Vec<Frame>,
'''
new_machine_fields = '''pub struct Machine {
    program: ValidatedProgram,
    persistent: Vec<PersistentStorage>,
    external_providers: Vec<ExternalProvider>,
    frames: Vec<Frame>,
'''
if text.count(machine_fields) != 1:
    raise SystemExit("Machine field anchor mismatch")
text = text.replace(machine_fields, new_machine_fields, 1)

constructor_anchor = '''impl Machine {
    /// Construct a reference execution from a caller-selected zero-parameter entry.
    pub fn new(program: ValidatedProgram, entry: FunctionId) -> Result<Self, EntryError> {
        let function = program
            .as_program()
            .function(entry)
            .ok_or(EntryError::InvalidFunction(entry))?;
        if !function.parameters.is_empty() {
            return Err(EntryError::EntryHasParameters(entry));
        }

        let mut next_storage_instance = 1_u64;
'''
constructor_replacement = '''impl Machine {
    /// Construct a reference execution with no external provider bindings.
    ///
    /// This remains sufficient for programs with no external callable requirements.
    pub fn new(program: ValidatedProgram, entry: FunctionId) -> Result<Self, EntryError> {
        Self::new_with_external_providers(program, entry, Vec::new())
    }

    /// Construct a reference execution after admitting the complete external provider set.
    pub fn new_with_external_providers(
        program: ValidatedProgram,
        entry: FunctionId,
        providers: Vec<ExternalProviderBinding>,
    ) -> Result<Self, EntryError> {
        let external_providers = admit_external_providers(&program, providers)
            .map_err(EntryError::ExternalProviderAdmission)?;
        let function = program
            .as_program()
            .function(entry)
            .ok_or(EntryError::InvalidFunction(entry))?;
        if !function.parameters.is_empty() {
            return Err(EntryError::EntryHasParameters(entry));
        }

        let mut next_storage_instance = 1_u64;
'''
if text.count(constructor_anchor) != 1:
    raise SystemExit("Machine constructor anchor mismatch")
text = text.replace(constructor_anchor, constructor_replacement, 1)
text = text.replace(
    "        let mut machine = Self {\n            program,\n            persistent,\n            frames: Vec::new(),",
    "        let mut machine = Self {\n            program,\n            persistent,\n            external_providers,\n            frames: Vec::new(),",
    1,
)

call_arm = '''            Terminator::Call {
                function,
                arguments,
                destination,
                target,
            } => {
                self.start_call(frame_index, function, &arguments, destination, target)?;
            }
'''
external_call_arm = call_arm + '''            Terminator::ExternalCall {
                external,
                arguments,
                destination,
                target,
            } => {
                self.execute_external_call(
                    frame_index,
                    external,
                    &arguments,
                    destination,
                    target,
                )?;
            }
'''
if text.count(call_arm) != 1:
    raise SystemExit("reference Call arm mismatch")
text = text.replace(call_arm, external_call_arm, 1)

start_call_anchor = '''    fn start_call(
        &mut self,
        frame_index: usize,
        function: FunctionId,
'''
external_method = '''    fn execute_external_call(
        &mut self,
        frame_index: usize,
        external: ExternalCallableId,
        arguments: &[Operand],
        destination: Option<Place>,
        target: BasicBlockId,
    ) -> Result<(), UndefinedBehaviorKind> {
        let mut values = Vec::with_capacity(arguments.len());
        for argument in arguments {
            values.push(
                self.evaluate_operand(frame_index, argument)?
                    .into_external_scalar(),
            );
        }

        let result_type = self
            .program
            .as_program()
            .external_callable(external)
            .expect("validated external call references a known declaration")
            .interface
            .result;
        let returned = self.external_providers[external.0 as usize].invoke(&values);
        let result = match returned {
            ExternalProviderReturn::NoResult => {
                assert!(
                    result_type.is_none(),
                    "admitted no-result provider matches a no-result declaration"
                );
                None
            }
            ExternalProviderReturn::Scalar(value) => {
                let ty = result_type
                    .expect("admitted scalar-result provider matches a result declaration");
                assert!(
                    external_scalar_matches_type(&self.program.as_program().types, ty, &value),
                    "verification provider returned a scalar outside its admitted declared type"
                );
                Some(RuntimeValue::from_external_scalar(value))
            }
        };

        match (destination, result) {
            (Some(destination), Some(value)) => {
                let ty = self.place_type(frame_index, &destination);
                {
                    let types = &self.program.as_program().types;
                    let frame = &mut self.frames[frame_index];
                    write_value(
                        types,
                        ty,
                        place_state_mut(&mut frame.locals, &destination),
                        value,
                    );
                }
                self.record(
                    frame_index,
                    VerificationEventKind::Write {
                        place: destination,
                        kind: VerificationWriteKind::Init,
                    },
                );
            }
            (None, None) => {}
            _ => unreachable!(
                "validated external call destination and admitted provider result structures agree"
            ),
        }
        self.frames[frame_index].current = target;
        Ok(())
    }

'''
if text.count(start_call_anchor) != 1:
    raise SystemExit("start_call anchor mismatch")
text = text.replace(start_call_anchor, external_method + start_call_anchor, 1)

# Mechanical migration of the one in-file Program constructor reached by the compiler.
text = text.replace(
    "        let validated = validate_program(Program {\n            types,\n            persistent: vec![PersistentDecl::new(i64_ty, Value::I64(13))],\n            functions: vec![function],",
    "        let validated = validate_program(Program {\n            types,\n            persistent: vec![PersistentDecl::new(i64_ty, Value::I64(13))],\n            external_callables: Vec::new(),\n            functions: vec![function],",
    1,
)
p.write_text(text)

# Admission is deliberately outside Core language validation.
p = Path("crates/runen-reference/src/interprocedural.rs")
text = p.read_text()
anchor = "impl Machine {\n"
helpers = '''fn external_scalar_matches_type(
    types: &TypeTable,
    ty: TypeId,
    value: &ExternalScalarValue,
) -> bool {
    matches!(
        (types.get(ty).map(|definition| &definition.kind), value),
        (Some(TypeKind::Scalar(ScalarType::Bool)), ExternalScalarValue::Bool(_))
            | (Some(TypeKind::Scalar(ScalarType::I8)), ExternalScalarValue::I8(_))
            | (Some(TypeKind::Scalar(ScalarType::I16)), ExternalScalarValue::I16(_))
            | (Some(TypeKind::Scalar(ScalarType::I32)), ExternalScalarValue::I32(_))
            | (Some(TypeKind::Scalar(ScalarType::I64)), ExternalScalarValue::I64(_))
            | (Some(TypeKind::Scalar(ScalarType::U8)), ExternalScalarValue::U8(_))
            | (Some(TypeKind::Scalar(ScalarType::U16)), ExternalScalarValue::U16(_))
            | (Some(TypeKind::Scalar(ScalarType::U32)), ExternalScalarValue::U32(_))
            | (Some(TypeKind::Scalar(ScalarType::U64)), ExternalScalarValue::U64(_))
            | (Some(TypeKind::Scalar(ScalarType::F16)), ExternalScalarValue::F16(_))
            | (Some(TypeKind::Scalar(ScalarType::F32)), ExternalScalarValue::F32(_))
            | (Some(TypeKind::Scalar(ScalarType::F64)), ExternalScalarValue::F64(_))
    )
}

fn admit_external_providers(
    program: &ValidatedProgram,
    providers: Vec<ExternalProviderBinding>,
) -> Result<Vec<ExternalProvider>, ExternalProviderAdmissionError> {
    let declarations = &program.as_program().external_callables;
    let mut admitted = std::iter::repeat_with(|| None)
        .take(declarations.len())
        .collect::<Vec<Option<ExternalProvider>>>();

    for binding in providers {
        let ExternalProviderBinding {
            external,
            interface,
            provider,
        } = binding;
        let Some(declaration) = program.as_program().external_callable(external) else {
            return Err(ExternalProviderAdmissionError::InvalidExternalCallable(external));
        };
        let slot = &mut admitted[external.0 as usize];
        if slot.is_some() {
            return Err(ExternalProviderAdmissionError::DuplicateProvider(external));
        }
        if interface != declaration.interface {
            return Err(ExternalProviderAdmissionError::InterfaceMismatch {
                external,
                expected: declaration.interface.clone(),
                found: interface,
            });
        }
        let declaration_has_result = declaration.interface.result.is_some();
        let provider_has_result = provider.has_result();
        if declaration_has_result != provider_has_result {
            return Err(ExternalProviderAdmissionError::ResultShapeMismatch {
                external,
                declaration_has_result,
                provider_has_result,
            });
        }
        *slot = Some(provider);
    }

    admitted
        .into_iter()
        .enumerate()
        .map(|(index, provider)| {
            provider.ok_or_else(|| {
                ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(
                    u32::try_from(index).expect("external provider index exceeds u32::MAX"),
                ))
            })
        })
        .collect()
}

'''
if text.count(anchor) != 1:
    raise SystemExit("Machine impl helper anchor mismatch")
p.write_text(text.replace(anchor, helpers + anchor, 1))

# Public verification API exports.
replace_exact(
    "crates/runen-reference/src/lib.rs",
    "    ActivationId, EntryError, ExecutionReport, Machine, TerminalStatus, UndefinedBehavior,\n    VerificationEvent, VerificationEventKind,\n",
    "    ActivationId, EntryError, ExecutionReport, ExternalProviderAdmissionError,\n    ExternalProviderBinding, ExternalScalarValue, Machine, TerminalStatus, UndefinedBehavior,\n    VerificationEvent, VerificationEventKind,\n",
)

# Focused admission and execution coverage. Phase 1 has already introduced Core external declarations.
Path("crates/runen-reference/tests/external_callables.rs").write_text(r'''use std::cell::RefCell;
use std::rc::Rc;

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, SafeReferenceResultContract, ScalarType, Terminator, TypeDef, TypeId, TypeTable,
    Value, validate_program,
};
use runen_reference::{
    EntryError, ExternalProviderAdmissionError, ExternalProviderBinding, ExternalScalarValue,
    Machine, ObservedBinaryFloatValue, ObservedValue, TerminalStatus,
};

fn interface(parameters: Vec<TypeId>, result: Option<TypeId>) -> CallableInterface {
    CallableInterface {
        parameters,
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
    }
}

fn no_result_entry(types: TypeTable, external_callables: Vec<ExternalCallableDecl>) -> runen_core_ir::ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables,
        functions: vec![Function {
            name: "main".into(),
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
    .expect("test program is valid")
}

#[test]
fn admission_requires_exactly_one_matching_provider_for_every_declaration() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let first = interface(vec![i64_ty], None);
    let second = interface(Vec::new(), Some(i64_ty));
    let program = no_result_entry(
        types,
        vec![
            ExternalCallableDecl::new(first.clone()),
            ExternalCallableDecl::new(second.clone()),
        ],
    );

    assert_eq!(
        Machine::new(program.clone(), FunctionId(0)).err(),
        Some(EntryError::ExternalProviderAdmission(
            ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(0))
        ))
    );

    let missing_unused = Machine::new_with_external_providers(
        program.clone(),
        FunctionId(0),
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            first.clone(),
            |_| {},
        )],
    )
    .err();
    assert_eq!(
        missing_unused,
        Some(EntryError::ExternalProviderAdmission(
            ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(1))
        ))
    );

    let duplicate = Machine::new_with_external_providers(
        program.clone(),
        FunctionId(0),
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), first.clone(), |_| {}),
            ExternalProviderBinding::no_result(ExternalCallableId(0), first.clone(), |_| {}),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), second.clone(), |_| {
                ExternalScalarValue::I64(1)
            }),
        ],
    )
    .err();
    assert_eq!(
        duplicate,
        Some(EntryError::ExternalProviderAdmission(
            ExternalProviderAdmissionError::DuplicateProvider(ExternalCallableId(0))
        ))
    );

    let wrong_interface = interface(Vec::new(), None);
    let mismatch = Machine::new_with_external_providers(
        program.clone(),
        FunctionId(0),
        vec![
            ExternalProviderBinding::no_result(
                ExternalCallableId(0),
                wrong_interface.clone(),
                |_| {},
            ),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), second.clone(), |_| {
                ExternalScalarValue::I64(1)
            }),
        ],
    )
    .err();
    assert_eq!(
        mismatch,
        Some(EntryError::ExternalProviderAdmission(
            ExternalProviderAdmissionError::InterfaceMismatch {
                external: ExternalCallableId(0),
                expected: first.clone(),
                found: wrong_interface,
            }
        ))
    );

    Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), first, |_| {}),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), second, |_| {
                ExternalScalarValue::I64(1)
            }),
        ],
    )
    .expect("complete matching provider set is admitted before execution");
}

#[test]
fn external_calls_invoke_once_in_program_order_without_creating_a_callee_activation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let sink = interface(vec![i64_ty], None);
    let transform = interface(vec![i64_ty], Some(i64_ty));
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![
            ExternalCallableDecl::new(sink.clone()),
            ExternalCallableDecl::new(transform.clone()),
        ],
        functions: vec![Function {
            name: "main".into(),
            parameters: Vec::new(),
            result: Some(i64_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("result", i64_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Constant(Value::I64(2))],
                            destination: None,
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(1),
                            arguments: vec![Operand::Constant(Value::I64(7))],
                            destination: Some(Place::local(LocalId(0))),
                            target: BasicBlockId(2),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                    ),
                ],
            },
        }],
    })
    .expect("external-call execution fixture is valid");

    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink_seen = Rc::clone(&seen);
    let transform_seen = Rc::clone(&seen);
    let machine = Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), sink, move |arguments| {
                sink_seen.borrow_mut().push(arguments.to_vec());
            }),
            ExternalProviderBinding::scalar_result(
                ExternalCallableId(1),
                transform,
                move |arguments| {
                    transform_seen.borrow_mut().push(arguments.to_vec());
                    let [ExternalScalarValue::I64(value)] = arguments else {
                        panic!("exact admitted scalar arguments");
                    };
                    ExternalScalarValue::I64(value + 1)
                },
            ),
        ],
    )
    .expect("providers admitted");
    let report = machine.execute().expect("defined external calls return normally");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(8)));
    assert_eq!(
        *seen.borrow(),
        vec![
            vec![ExternalScalarValue::I64(2)],
            vec![ExternalScalarValue::I64(7)],
        ]
    );
    assert!(
        report
            .verification_events
            .iter()
            .all(|event| event.activation.0 == 1),
        "external provider invocation creates no Core callee activation"
    );
}

#[test]
fn floating_external_provider_observes_semantic_value_and_may_return_nan_class() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let float_interface = interface(vec![f32_ty], Some(f32_ty));
    let zero = BinaryFloatValue::Zero(BinaryFloatSign::Negative);
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(float_interface.clone())],
        functions: vec![Function {
            name: "main".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("result", f32_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Constant(Value::F32(zero))],
                            destination: Some(Place::local(LocalId(0))),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                    ),
                ],
            },
        }],
    })
    .expect("floating external-call fixture is valid");

    let machine = Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            float_interface,
            move |arguments| {
                assert_eq!(
                    arguments,
                    &[ExternalScalarValue::F32(ObservedBinaryFloatValue::Represented(zero))]
                );
                ExternalScalarValue::F32(ObservedBinaryFloatValue::NaNClass)
            },
        )],
    )
    .expect("floating provider admitted");
    let report = machine.execute().expect("provider returns normally");
    assert_eq!(
        report.result,
        Some(ObservedValue::F32(ObservedBinaryFloatValue::NaNClass))
    );
}
''')

print("staged #705 reference provider admission and execution")
