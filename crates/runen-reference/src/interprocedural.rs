use runen_core_ir::{
    BasicBlockId, BorrowKind, FunctionId, LoanId, LocalId, NumericContract, Operand, Place,
    PlaceAccess, Projection, ReferenceAccess, ReferencePermission, ScalarType, Statement,
    StorageInstanceId, StorageRegion, Terminator, TypeId, TypeKind, TypeTable, ValidatedProgram,
    Value,
};

use crate::floating::{
    RuntimeFloatValue, add_f16, add_f32, add_f64, mul_f16, mul_f32, mul_f64, sub_f16, sub_f32,
    sub_f64,
};
use crate::{
    ObservedValue, RawPointerValue, ReferenceAuthorityId, SafeReferenceValue, UndefinedBehaviorKind,
    VerificationWriteKind,
};

/// Verification-only identity of one dynamic function activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActivationId(pub u64);

/// One activation-scoped verification event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationEvent {
    pub activation: ActivationId,
    pub kind: VerificationEventKind,
}

/// Verification event payload for the interprocedural reference oracle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationEventKind {
    BorrowStart {
        loan: LoanId,
        kind: BorrowKind,
        place: Place,
    },
    BorrowEnd(LoanId),
    Read(Place),
    RawRead {
        pointer: RawPointerValue,
        target: Place,
    },
    RawMove {
        pointer: RawPointerValue,
        target: Place,
    },
    Move(Place),
    Copy(Place),
    Write {
        place: Place,
        kind: VerificationWriteKind,
    },
    AddressOf {
        place: Place,
        pointer: RawPointerValue,
    },
    RawPointerMove {
        place: Place,
        pointer: RawPointerValue,
    },
    RawPointerCopy {
        place: Place,
        pointer: RawPointerValue,
    },
    DropTrackedFixture {
        place: Place,
        id: u64,
    },
}

/// Terminal status of one defined interprocedural reference execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalStatus {
    Returned,
    Faulted(String),
}

/// Complete result of a terminating defined program execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReport {
    pub terminal: TerminalStatus,
    pub result: Option<ObservedValue>,
    pub verification_events: Vec<VerificationEvent>,
}

/// Verification-only report for detected undefined behavior.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndefinedBehavior {
    pub kind: UndefinedBehaviorKind,
    pub verification_events: Vec<VerificationEvent>,
}

/// Why a verification harness entry function cannot be invoked directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryError {
    InvalidFunction(FunctionId),
    EntryHasParameters(FunctionId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RuntimeValue {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F16(RuntimeFloatValue),
    F32(RuntimeFloatValue),
    F64(RuntimeFloatValue),
    RawPointer(RawPointerValue),
    SafeReference(SafeReferenceValue),
    TrackedFixture(u64),
    Struct(Vec<RuntimeValue>),
}

fn canonical_signed_residue(value: i128, width: u32) -> u128 {
    debug_assert!((1..=64).contains(&width));
    let modulus = 1_u128 << width;
    if value >= 0 {
        u128::try_from(value).expect("nonnegative represented integer fits canonical residue")
    } else {
        modulus
            - u128::try_from(-value)
                .expect("promoted represented signed magnitude fits canonical residue")
    }
}

fn signed_value_from_residue(residue: u128, width: u32) -> i128 {
    debug_assert!((1..=64).contains(&width));
    let modulus = 1_u128 << width;
    let signed_boundary = 1_u128 << (width - 1);
    let residue = i128::try_from(residue).expect("represented-width residue fits i128");
    if u128::try_from(residue).expect("residue is nonnegative") < signed_boundary {
        residue
    } else {
        residue - i128::try_from(modulus).expect("represented-width modulus fits i128")
    }
}

fn xor_signed_values(left: i128, right: i128, width: u32) -> i128 {
    let left = canonical_signed_residue(left, width);
    let right = canonical_signed_residue(right, width);
    signed_value_from_residue(left ^ right, width)
}

fn or_signed_values(left: i128, right: i128, width: u32) -> i128 {
    let left = canonical_signed_residue(left, width);
    let right = canonical_signed_residue(right, width);
    signed_value_from_residue(left | right, width)
}

impl RuntimeValue {
    fn from_constant(value: &Value) -> Self {
        match value {
            Value::Bool(value) => Self::Bool(*value),
            Value::I8(value) => Self::I8(*value),
            Value::I16(value) => Self::I16(*value),
            Value::I32(value) => Self::I32(*value),
            Value::I64(value) => Self::I64(*value),
            Value::U8(value) => Self::U8(*value),
            Value::U16(value) => Self::U16(*value),
            Value::U32(value) => Self::U32(*value),
            Value::U64(value) => Self::U64(*value),
            Value::F16(value) => Self::F16(RuntimeFloatValue::from_constant(*value)),
            Value::F32(value) => Self::F32(RuntimeFloatValue::from_constant(*value)),
            Value::F64(value) => Self::F64(RuntimeFloatValue::from_constant(*value)),
            Value::TrackedFixture(value) => Self::TrackedFixture(*value),
            Value::Struct(values) => Self::Struct(values.iter().map(Self::from_constant).collect()),
        }
    }

    fn wrapping_integer_add(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::I8(left), Self::I8(right)) => Self::I8(left.wrapping_add(right)),
            (Self::I16(left), Self::I16(right)) => Self::I16(left.wrapping_add(right)),
            (Self::I32(left), Self::I32(right)) => Self::I32(left.wrapping_add(right)),
            (Self::I64(left), Self::I64(right)) => Self::I64(left.wrapping_add(right)),
            (Self::U8(left), Self::U8(right)) => Self::U8(left.wrapping_add(right)),
            (Self::U16(left), Self::U16(right)) => Self::U16(left.wrapping_add(right)),
            (Self::U32(left), Self::U32(right)) => Self::U32(left.wrapping_add(right)),
            (Self::U64(left), Self::U64(right)) => Self::U64(left.wrapping_add(right)),
            _ => unreachable!("validated IntegerAdd has matching fixed-width integer operands"),
        }
    }

    fn wrapping_integer_sub(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::I8(left), Self::I8(right)) => Self::I8(left.wrapping_sub(right)),
            (Self::I16(left), Self::I16(right)) => Self::I16(left.wrapping_sub(right)),
            (Self::I32(left), Self::I32(right)) => Self::I32(left.wrapping_sub(right)),
            (Self::I64(left), Self::I64(right)) => Self::I64(left.wrapping_sub(right)),
            (Self::U8(left), Self::U8(right)) => Self::U8(left.wrapping_sub(right)),
            (Self::U16(left), Self::U16(right)) => Self::U16(left.wrapping_sub(right)),
            (Self::U32(left), Self::U32(right)) => Self::U32(left.wrapping_sub(right)),
            (Self::U64(left), Self::U64(right)) => Self::U64(left.wrapping_sub(right)),
            _ => unreachable!("validated IntegerSub has matching fixed-width integer operands"),
        }
    }

    fn wrapping_integer_mul(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::I8(left), Self::I8(right)) => Self::I8(left.wrapping_mul(right)),
            (Self::I16(left), Self::I16(right)) => Self::I16(left.wrapping_mul(right)),
            (Self::I32(left), Self::I32(right)) => Self::I32(left.wrapping_mul(right)),
            (Self::I64(left), Self::I64(right)) => Self::I64(left.wrapping_mul(right)),
            (Self::U8(left), Self::U8(right)) => Self::U8(left.wrapping_mul(right)),
            (Self::U16(left), Self::U16(right)) => Self::U16(left.wrapping_mul(right)),
            (Self::U32(left), Self::U32(right)) => Self::U32(left.wrapping_mul(right)),
            (Self::U64(left), Self::U64(right)) => Self::U64(left.wrapping_mul(right)),
            _ => unreachable!("validated IntegerMul has matching fixed-width integer operands"),
        }
    }

    fn integer_xor(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::I8(left), Self::I8(right)) => Self::I8(
                i8::try_from(xor_signed_values(i128::from(left), i128::from(right), 8))
                    .expect("I8 XOR residue maps to I8 domain"),
            ),
            (Self::I16(left), Self::I16(right)) => Self::I16(
                i16::try_from(xor_signed_values(i128::from(left), i128::from(right), 16))
                    .expect("I16 XOR residue maps to I16 domain"),
            ),
            (Self::I32(left), Self::I32(right)) => Self::I32(
                i32::try_from(xor_signed_values(i128::from(left), i128::from(right), 32))
                    .expect("I32 XOR residue maps to I32 domain"),
            ),
            (Self::I64(left), Self::I64(right)) => Self::I64(
                i64::try_from(xor_signed_values(i128::from(left), i128::from(right), 64))
                    .expect("I64 XOR residue maps to I64 domain"),
            ),
            (Self::U8(left), Self::U8(right)) => Self::U8(left ^ right),
            (Self::U16(left), Self::U16(right)) => Self::U16(left ^ right),
            (Self::U32(left), Self::U32(right)) => Self::U32(left ^ right),
            (Self::U64(left), Self::U64(right)) => Self::U64(left ^ right),
            _ => unreachable!("validated IntegerXor has matching fixed-width integer operands"),
        }
    }

    fn integer_or(left: Self, right: Self) -> Self {
        match (left, right) {
            (Self::I8(left), Self::I8(right)) => Self::I8(
                i8::try_from(or_signed_values(i128::from(left), i128::from(right), 8))
                    .expect("I8 OR residue maps to I8 domain"),
            ),
            (Self::I16(left), Self::I16(right)) => Self::I16(
                i16::try_from(or_signed_values(i128::from(left), i128::from(right), 16))
                    .expect("I16 OR residue maps to I16 domain"),
            ),
            (Self::I32(left), Self::I32(right)) => Self::I32(
                i32::try_from(or_signed_values(i128::from(left), i128::from(right), 32))
                    .expect("I32 OR residue maps to I32 domain"),
            ),
            (Self::I64(left), Self::I64(right)) => Self::I64(
                i64::try_from(or_signed_values(i128::from(left), i128::from(right), 64))
                    .expect("I64 OR residue maps to I64 domain"),
            ),
            (Self::U8(left), Self::U8(right)) => Self::U8(left | right),
            (Self::U16(left), Self::U16(right)) => Self::U16(left | right),
            (Self::U32(left), Self::U32(right)) => Self::U32(left | right),
            (Self::U64(left), Self::U64(right)) => Self::U64(left | right),
            _ => unreachable!("validated IntegerOr has matching fixed-width integer operands"),
        }
    }

    fn floating_add(contract: NumericContract, left: Self, right: Self) -> Self {
        match contract {
            NumericContract::Standard | NumericContract::Reproducible | NumericContract::Fast => {
                match (left, right) {
                    (Self::F16(left), Self::F16(right)) => Self::F16(add_f16(left, right)),
                    (Self::F32(left), Self::F32(right)) => Self::F32(add_f32(left, right)),
                    (Self::F64(left), Self::F64(right)) => Self::F64(add_f64(left, right)),
                    _ => unreachable!("validated FloatAdd has matching same-format operands"),
                }
            }
        }
    }

    fn floating_sub(contract: NumericContract, left: Self, right: Self) -> Self {
        match contract {
            NumericContract::Standard | NumericContract::Reproducible | NumericContract::Fast => {
                match (left, right) {
                    (Self::F16(left), Self::F16(right)) => Self::F16(sub_f16(left, right)),
                    (Self::F32(left), Self::F32(right)) => Self::F32(sub_f32(left, right)),
                    (Self::F64(left), Self::F64(right)) => Self::F64(sub_f64(left, right)),
                    _ => unreachable!("validated FloatSub has matching same-format operands"),
                }
            }
        }
    }

    fn floating_mul(contract: NumericContract, left: Self, right: Self) -> Self {
        match contract {
            NumericContract::Standard | NumericContract::Reproducible | NumericContract::Fast => {
                match (left, right) {
                    (Self::F16(left), Self::F16(right)) => Self::F16(mul_f16(left, right)),
                    (Self::F32(left), Self::F32(right)) => Self::F32(mul_f32(left, right)),
                    (Self::F64(left), Self::F64(right)) => Self::F64(mul_f64(left, right)),
                    _ => unreachable!("validated FloatMul has matching same-format operands"),
                }
            }
        }
    }

    fn floating_div(contract: NumericContract, left: Self, right: Self) -> Self {
        match contract {
            NumericContract::Standard | NumericContract::Reproducible | NumericContract::Fast => {
                match (left, right) {
                    (Self::F16(left), Self::F16(right)) => {
                        Self::F16(crate::floating::div_f16(left, right))
                    }
                    (Self::F32(left), Self::F32(right)) => {
                        Self::F32(crate::floating::div_f32(left, right))
                    }
                    (Self::F64(left), Self::F64(right)) => {
                        Self::F64(crate::floating::div_f64(left, right))
                    }
                    _ => unreachable!("validated FloatDiv has matching same-format operands"),
                }
            }
        }
    }

    fn into_observed_value(self) -> ObservedValue {
        match self {
            Self::Bool(value) => ObservedValue::Bool(value),
            Self::I8(value) => ObservedValue::I8(value),
            Self::I16(value) => ObservedValue::I16(value),
            Self::I32(value) => ObservedValue::I32(value),
            Self::I64(value) => ObservedValue::I64(value),
            Self::U8(value) => ObservedValue::U8(value),
            Self::U16(value) => ObservedValue::U16(value),
            Self::U32(value) => ObservedValue::U32(value),
            Self::U64(value) => ObservedValue::U64(value),
            Self::F16(value) => ObservedValue::F16(value.into_observed()),
            Self::F32(value) => ObservedValue::F32(value.into_observed()),
            Self::F64(value) => ObservedValue::F64(value.into_observed()),
            Self::TrackedFixture(value) => ObservedValue::TrackedFixture(value),
            Self::Struct(values) => {
                ObservedValue::Struct(values.into_iter().map(Self::into_observed_value).collect())
            }
            Self::RawPointer(_) | Self::SafeReference(_) => unreachable!(
                "validated entry result is result-transfer-safe and contains no pointer/reference"
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LeafState {
    NeverInitialized,
    Live(RuntimeValue),
    Dead,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ObjectState {
    Leaf(LeafState),
    Aggregate(Vec<ObjectState>),
}

impl ObjectState {
    fn uninitialized(types: &TypeTable, ty: TypeId) -> Self {
        let definition = types
            .get(ty)
            .expect("validated Core program references only known types");
        match &definition.kind {
            TypeKind::Scalar(_) => Self::Leaf(LeafState::NeverInitialized),
            TypeKind::Struct(fields) => Self::Aggregate(
                fields
                    .iter()
                    .map(|field| Self::uninitialized(types, field.ty))
                    .collect(),
            ),
        }
    }

    fn fully_live(&self) -> bool {
        match self {
            Self::Leaf(LeafState::Live(_)) => true,
            Self::Leaf(LeafState::NeverInitialized | LeafState::Dead) => false,
            Self::Aggregate(fields) => fields.iter().all(Self::fully_live),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LocalStorage {
    instance: StorageInstanceId,
    state: ObjectState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveLoan {
    kind: BorrowKind,
    place: Place,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveReferenceAuthority {
    target: StorageRegion,
    permission: ReferencePermission,
    parent: Option<ReferenceAuthorityId>,
    carriers: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Continuation {
    destination: Option<Place>,
    target: BasicBlockId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Frame {
    activation: ActivationId,
    function: FunctionId,
    current: BasicBlockId,
    locals: Vec<LocalStorage>,
    active_loans: Vec<Option<ActiveLoan>>,
    return_to: Option<Continuation>,
}

#[derive(Clone, Debug)]
struct ResolvedReferenceAccess {
    authority: ReferenceAuthorityId,
    permission: ReferencePermission,
    target: StorageRegion,
    selected_ty: TypeId,
}

/// Executable abstract machine for validated interprocedural Core programs.
pub struct Machine {
    program: ValidatedProgram,
    frames: Vec<Frame>,
    reference_authorities: Vec<Option<ActiveReferenceAuthority>>,
    next_activation: u64,
    next_storage_instance: u64,
    next_reference_authority: u64,
    verification_events: Vec<VerificationEvent>,
}

impl Machine {
    /// Construct a reference execution from a caller-selected zero-parameter entry.
    pub fn new(program: ValidatedProgram, entry: FunctionId) -> Result<Self, EntryError> {
        let function = program
            .as_program()
            .function(entry)
            .ok_or(EntryError::InvalidFunction(entry))?;
        if !function.parameters.is_empty() {
            return Err(EntryError::EntryHasParameters(entry));
        }

        let mut machine = Self {
            program,
            frames: Vec::new(),
            reference_authorities: Vec::new(),
            next_activation: 1,
            next_storage_instance: 1,
            next_reference_authority: 0,
            verification_events: Vec::new(),
        };
        let frame = machine.create_frame(entry, Vec::new(), None);
        machine.frames.push(frame);
        Ok(machine)
    }

    /// Execute until normal return, defined fault, or detected undefined behavior.
    ///
    /// A cyclic call/control-flow graph may diverge and therefore never return.
    pub fn execute(mut self) -> Result<ExecutionReport, UndefinedBehavior> {
        loop {
            let frame_index = self
                .frames
                .len()
                .checked_sub(1)
                .expect("machine always has an active frame while executing");
            let function_id = self.frames[frame_index].function;
            let current = self.frames[frame_index].current;
            let block = self
                .program
                .as_program()
                .function(function_id)
                .expect("validated frame references a known function")
                .body
                .block(current)
                .expect("validated frame reaches only known basic blocks")
                .clone();

            for statement in &block.statements {
                if let Err(kind) = self.execute_statement(frame_index, statement) {
                    return Err(UndefinedBehavior {
                        kind,
                        verification_events: self.verification_events,
                    });
                }
            }

            match block.terminator {
                Terminator::Goto(target) => self.frames[frame_index].current = target,
                Terminator::Branch {
                    condition,
                    true_target,
                    false_target,
                } => {
                    let value = match self.evaluate_operand(frame_index, &condition) {
                        Ok(value) => value,
                        Err(kind) => {
                            return Err(UndefinedBehavior {
                                kind,
                                verification_events: self.verification_events,
                            });
                        }
                    };
                    self.frames[frame_index].current = match value {
                        RuntimeValue::Bool(true) => true_target,
                        RuntimeValue::Bool(false) => false_target,
                        _ => unreachable!("validated Branch condition is Bool-valued"),
                    };
                }
                Terminator::Call {
                    function,
                    arguments,
                    destination,
                    target,
                } => {
                    let mut values = Vec::with_capacity(arguments.len());
                    for argument in &arguments {
                        match self.evaluate_operand(frame_index, argument) {
                            Ok(value) => values.push(value),
                            Err(kind) => {
                                return Err(UndefinedBehavior {
                                    kind,
                                    verification_events: self.verification_events,
                                });
                            }
                        }
                    }
                    let callee = self.create_frame(
                        function,
                        values,
                        Some(Continuation {
                            destination,
                            target,
                        }),
                    );
                    self.frames.push(callee);
                }
                Terminator::Return(result) => {
                    let result = if let Some(operand) = &result {
                        match self.evaluate_operand(frame_index, operand) {
                            Ok(value) => Some(value),
                            Err(kind) => {
                                return Err(UndefinedBehavior {
                                    kind,
                                    verification_events: self.verification_events,
                                });
                            }
                        }
                    } else {
                        None
                    };
                    let continuation = self.frames[frame_index].return_to.clone();
                    self.cleanup_frame(frame_index);
                    self.frames.pop();

                    if let Some(continuation) = continuation {
                        let caller_index = self
                            .frames
                            .len()
                            .checked_sub(1)
                            .expect("callee continuation has one suspended caller");
                        match (continuation.destination, result) {
                            (Some(destination), Some(value)) => {
                                let ty = self.place_type(caller_index, &destination);
                                {
                                    let types = &self.program.as_program().types;
                                    let frame = &mut self.frames[caller_index];
                                    write_value(
                                        types,
                                        ty,
                                        place_state_mut(&mut frame.locals, &destination),
                                        value,
                                    );
                                }
                                self.record(
                                    caller_index,
                                    VerificationEventKind::Write {
                                        place: destination,
                                        kind: VerificationWriteKind::Init,
                                    },
                                );
                            }
                            (None, None) => {}
                            _ => unreachable!(
                                "validated call destination and callee result structures agree"
                            ),
                        }
                        self.frames[caller_index].current = continuation.target;
                    } else {
                        return Ok(ExecutionReport {
                            terminal: TerminalStatus::Returned,
                            result: result.map(RuntimeValue::into_observed_value),
                            verification_events: self.verification_events,
                        });
                    }
                }
                Terminator::Fault(fault) => {
                    while !self.frames.is_empty() {
                        let index = self.frames.len() - 1;
                        self.cleanup_frame(index);
                        self.frames.pop();
                    }
                    return Ok(ExecutionReport {
                        terminal: TerminalStatus::Faulted(fault.code),
                        result: None,
                        verification_events: self.verification_events,
                    });
                }
            }
        }
    }

    fn create_frame(
        &mut self,
        function_id: FunctionId,
        arguments: Vec<RuntimeValue>,
        return_to: Option<Continuation>,
    ) -> Frame {
        let function = self
            .program
            .as_program()
            .function(function_id)
            .expect("validated call references a known function");
        let local_types = function
            .body
            .locals
            .iter()
            .map(|local| local.ty)
            .collect::<Vec<_>>();
        let parameters = function.parameters.clone();
        let entry = function.body.entry;
        let loan_count = function.body.loans.len();

        let activation = ActivationId(self.next_activation);
        self.next_activation = self
            .next_activation
            .checked_add(1)
            .expect("verification activation identity exhausted");

        let mut locals = Vec::with_capacity(local_types.len());
        for ty in local_types {
            let instance = StorageInstanceId(self.next_storage_instance);
            self.next_storage_instance = self
                .next_storage_instance
                .checked_add(1)
                .expect("reference storage-instance identity exhausted");
            locals.push(LocalStorage {
                instance,
                state: ObjectState::uninitialized(&self.program.as_program().types, ty),
            });
        }

        let mut frame = Frame {
            activation,
            function: function_id,
            current: entry,
            locals,
            active_loans: vec![None; loan_count],
            return_to,
        };

        for (parameter, value) in parameters.into_iter().zip(arguments) {
            let ty = self
                .program
                .as_program()
                .function(function_id)
                .expect("validated call references a known function")
                .body
                .local(parameter)
                .expect("validated parameter references a known local")
                .ty;
            write_value(
                &self.program.as_program().types,
                ty,
                place_state_mut(&mut frame.locals, &Place::local(parameter)),
                value,
            );
        }

        frame
    }

    fn execute_statement(
        &mut self,
        frame_index: usize,
        statement: &Statement,
    ) -> Result<(), UndefinedBehaviorKind> {
        match statement {
            Statement::Init { dst, src } => self.initialize(frame_index, dst, src),
            Statement::IntegerAdd { dst, left, right } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::IntegerAdd,
                RuntimeValue::wrapping_integer_add,
            ),
            Statement::IntegerSub { dst, left, right } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::IntegerSub,
                RuntimeValue::wrapping_integer_sub,
            ),
            Statement::IntegerMul { dst, left, right } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::IntegerMul,
                RuntimeValue::wrapping_integer_mul,
            ),
            Statement::IntegerXor { dst, left, right } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::IntegerXor,
                RuntimeValue::integer_xor,
            ),
            Statement::IntegerOr { dst, left, right } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::IntegerOr,
                RuntimeValue::integer_or,
            ),
            Statement::FloatAdd {
                contract,
                dst,
                left,
                right,
            } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::FloatAdd,
                |left, right| RuntimeValue::floating_add(*contract, left, right),
            ),
            Statement::FloatSub {
                contract,
                dst,
                left,
                right,
            } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::FloatSub,
                |left, right| RuntimeValue::floating_sub(*contract, left, right),
            ),
            Statement::FloatMul {
                contract,
                dst,
                left,
                right,
            } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::FloatMul,
                |left, right| RuntimeValue::floating_mul(*contract, left, right),
            ),
            Statement::FloatDiv {
                contract,
                dst,
                left,
                right,
            } => self.binary_write(
                frame_index,
                dst,
                left,
                right,
                VerificationWriteKind::FloatDiv,
                |left, right| RuntimeValue::floating_div(*contract, left, right),
            ),
            Statement::Borrow { loan, kind, src } => {
                self.begin_borrow(frame_index, *loan, *kind, src);
                Ok(())
            }
            Statement::EndBorrow { loan } => {
                self.end_borrow(frame_index, *loan);
                Ok(())
            }
            Statement::Read { src } => {
                self.read(frame_index, src);
                Ok(())
            }
            Statement::ReferenceRead { src } => {
                self.reference_read(frame_index, src);
                Ok(())
            }
            Statement::RawRead { pointer } => self.raw_read(frame_index, pointer),
            Statement::RawAssign { pointer, src } => self.raw_assign(frame_index, pointer, src),
            Statement::Assign { dst, src } => {
                self.replace(frame_index, dst, src, VerificationWriteKind::Assign)
            }
            Statement::InteriorAssign { dst, src } => {
                self.replace(frame_index, dst, src, VerificationWriteKind::InteriorAssign)
            }
            Statement::ReferenceAssign { dst, src } => {
                self.reference_replace(frame_index, dst, src)
            }
            Statement::ReferenceInteriorAssign { dst, src } => {
                self.reference_replace(frame_index, dst, src)
            }
            Statement::Drop { place } => {
                self.drop_explicit(frame_index, place);
                Ok(())
            }
            Statement::ReferenceDrop { place } => {
                self.reference_drop(frame_index, place);
                Ok(())
            }
        }
    }

    fn initialize(
        &mut self,
        frame_index: usize,
        dst: &Place,
        src: &Operand,
    ) -> Result<(), UndefinedBehaviorKind> {
        let dst_ty = self.place_type(frame_index, dst);
        let value = self.evaluate_operand(frame_index, src)?;
        {
            let types = &self.program.as_program().types;
            let frame = &mut self.frames[frame_index];
            write_value(
                types,
                dst_ty,
                place_state_mut(&mut frame.locals, dst),
                value,
            );
        }
        self.record(
            frame_index,
            VerificationEventKind::Write {
                place: dst.clone(),
                kind: VerificationWriteKind::Init,
            },
        );
        Ok(())
    }

    fn binary_write<F>(
        &mut self,
        frame_index: usize,
        dst: &Place,
        left: &Operand,
        right: &Operand,
        kind: VerificationWriteKind,
        operation: F,
    ) -> Result<(), UndefinedBehaviorKind>
    where
        F: FnOnce(RuntimeValue, RuntimeValue) -> RuntimeValue,
    {
        let dst_ty = self.place_type(frame_index, dst);
        let left = self.evaluate_operand(frame_index, left)?;
        let right = self.evaluate_operand(frame_index, right)?;
        let value = operation(left, right);
        {
            let types = &self.program.as_program().types;
            let frame = &mut self.frames[frame_index];
            write_value(
                types,
                dst_ty,
                place_state_mut(&mut frame.locals, dst),
                value,
            );
        }
        self.record(
            frame_index,
            VerificationEventKind::Write {
                place: dst.clone(),
                kind,
            },
        );
        Ok(())
    }

    fn begin_borrow(
        &mut self,
        frame_index: usize,
        loan: LoanId,
        kind: BorrowKind,
        src: &PlaceAccess,
    ) {
        let place = self.resolve_access(frame_index, src);
        let slot = self.frames[frame_index]
            .active_loans
            .get_mut(loan.0 as usize)
            .expect("validated Core MIR references only declared loans");
        assert!(
            slot.is_none(),
            "validated Core MIR cannot begin an already-active loan"
        );
        *slot = Some(ActiveLoan {
            kind,
            place: place.clone(),
        });
        self.record(
            frame_index,
            VerificationEventKind::BorrowStart { loan, kind, place },
        );
    }

    fn end_borrow(&mut self, frame_index: usize, loan: LoanId) {
        let slot = self.frames[frame_index]
            .active_loans
            .get_mut(loan.0 as usize)
            .expect("validated Core MIR references only declared loans");
        assert!(
            slot.take().is_some(),
            "validated Core MIR cannot end an inactive loan"
        );
        self.record(frame_index, VerificationEventKind::BorrowEnd(loan));
    }

    fn replace(
        &mut self,
        frame_index: usize,
        dst: &PlaceAccess,
        src: &Operand,
        kind: VerificationWriteKind,
    ) -> Result<(), UndefinedBehaviorKind> {
        let dst = self.resolve_access(frame_index, dst);
        let value = self.evaluate_operand(frame_index, src)?;
        self.replace_resolved(frame_index, dst, value, kind);
        Ok(())
    }

    fn replace_resolved(
        &mut self,
        frame_index: usize,
        dst: Place,
        value: RuntimeValue,
        kind: VerificationWriteKind,
    ) {
        let dst_ty = self.place_type(frame_index, &dst);
        self.drop_place_contents(frame_index, &dst);
        {
            let types = &self.program.as_program().types;
            let frame = &mut self.frames[frame_index];
            write_value(
                types,
                dst_ty,
                place_state_mut(&mut frame.locals, &dst),
                value,
            );
        }
        self.record(
            frame_index,
            VerificationEventKind::Write { place: dst, kind },
        );
    }

    fn reference_replace(
        &mut self,
        actor_frame: usize,
        dst: &ReferenceAccess,
        src: &Operand,
    ) -> Result<(), UndefinedBehaviorKind> {
        let resolved = self.resolve_reference_access(actor_frame, dst);
        let target = resolved.target;
        let selected_ty = resolved.selected_ty;
        let value = self.evaluate_operand(actor_frame, src)?;
        let (target_frame, target_place) = self.frame_place_for_storage_region(&target);
        self.drop_place_contents(target_frame, &target_place);
        let types = &self.program.as_program().types;
        write_value(
            types,
            selected_ty,
            place_state_mut(&mut self.frames[target_frame].locals, &target_place),
            value,
        );
        Ok(())
    }

    fn read(&mut self, frame_index: usize, src: &PlaceAccess) {
        let src = self.resolve_access(frame_index, src);
        self.record(frame_index, VerificationEventKind::Read(src));
    }

    fn reference_read(&self, actor_frame: usize, src: &ReferenceAccess) {
        let resolved = self.resolve_reference_access(actor_frame, src);
        let (target_frame, target_place) = self.frame_place_for_storage_region(&resolved.target);
        assert!(
            place_state(&self.frames[target_frame].locals, &target_place).fully_live(),
            "validated reference read reaches a fully-live referent"
        );
    }

    fn reference_drop(&mut self, actor_frame: usize, access: &ReferenceAccess) {
        let resolved = self.resolve_reference_access(actor_frame, access);
        let (target_frame, target_place) = self.frame_place_for_storage_region(&resolved.target);
        self.drop_place_contents(target_frame, &target_place);
    }

    fn raw_read(
        &mut self,
        frame_index: usize,
        pointer_access: &PlaceAccess,
    ) -> Result<(), UndefinedBehaviorKind> {
        let pointer_place = self.resolve_access(frame_index, pointer_access);
        let pointer = self.raw_pointer_at(frame_index, &pointer_place);
        let target = self.place_for_storage_region(frame_index, &pointer.target);

        if !place_state(&self.frames[frame_index].locals, &target).fully_live() {
            return Err(UndefinedBehaviorKind::RawReadTargetNotLive {
                target: pointer.target,
            });
        }

        if let Some((index, _)) = self.frames[frame_index]
            .active_loans
            .iter()
            .enumerate()
            .find(|(_, active)| {
                active.as_ref().is_some_and(|active| {
                    active.kind == BorrowKind::Exclusive && active.place.overlaps(&target)
                })
            })
        {
            return Err(UndefinedBehaviorKind::RawReadConflictsWithExclusiveLoan {
                target: pointer.target,
                loan: LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")),
            });
        }
        if let Some(authority) = self.raw_reference_conflict(&pointer.target, true) {
            return Err(UndefinedBehaviorKind::RawReadConflictsWithReferenceAuthority {
                target: pointer.target,
                authority,
            });
        }

        self.record(
            frame_index,
            VerificationEventKind::RawRead { pointer, target },
        );
        Ok(())
    }

    fn raw_assign(
        &mut self,
        frame_index: usize,
        pointer_access: &PlaceAccess,
        src: &Operand,
    ) -> Result<(), UndefinedBehaviorKind> {
        let pointer_place = self.resolve_access(frame_index, pointer_access);
        let pointer = self.raw_pointer_at(frame_index, &pointer_place);
        let target = self.place_for_storage_region(frame_index, &pointer.target);
        let value = self.evaluate_operand(frame_index, src)?;

        if let Some((index, _)) = self.frames[frame_index]
            .active_loans
            .iter()
            .enumerate()
            .find(|(_, active)| {
                active
                    .as_ref()
                    .is_some_and(|active| active.place.overlaps(&target))
            })
        {
            return Err(UndefinedBehaviorKind::RawAssignConflictsWithLoan {
                target: pointer.target,
                loan: LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")),
            });
        }
        if let Some(authority) = self.raw_reference_conflict(&pointer.target, false) {
            return Err(UndefinedBehaviorKind::RawAssignConflictsWithReferenceAuthority {
                target: pointer.target,
                authority,
            });
        }

        self.replace_resolved(frame_index, target, value, VerificationWriteKind::RawAssign);
        Ok(())
    }

    fn drop_explicit(&mut self, frame_index: usize, access: &PlaceAccess) {
        let place = self.resolve_access(frame_index, access);
        self.drop_place_contents(frame_index, &place);
    }

    fn evaluate_operand(
        &mut self,
        frame_index: usize,
        operand: &Operand,
    ) -> Result<RuntimeValue, UndefinedBehaviorKind> {
        match operand {
            Operand::Constant(value) => Ok(RuntimeValue::from_constant(value)),
            Operand::Move(src) => {
                let src = self.resolve_access(frame_index, src);
                let src_ty = self.place_type(frame_index, &src);
                let value = {
                    let types = &self.program.as_program().types;
                    let frame = &mut self.frames[frame_index];
                    take_value(types, src_ty, place_state_mut(&mut frame.locals, &src))
                };
                self.record(frame_index, VerificationEventKind::Move(src.clone()));
                if let RuntimeValue::RawPointer(pointer) = &value {
                    self.record(
                        frame_index,
                        VerificationEventKind::RawPointerMove {
                            place: src,
                            pointer: pointer.clone(),
                        },
                    );
                }
                Ok(value)
            }
            Operand::RawMove(pointer_access) => self.raw_move(frame_index, pointer_access),
            Operand::Copy(src) => {
                let src = self.resolve_access(frame_index, src);
                let src_ty = self.place_type(frame_index, &src);
                let value = {
                    let types = &self.program.as_program().types;
                    let state = place_state(&self.frames[frame_index].locals, &src);
                    clone_value(types, src_ty, state, &mut self.reference_authorities)
                };
                self.record(frame_index, VerificationEventKind::Copy(src.clone()));
                if let RuntimeValue::RawPointer(pointer) = &value {
                    self.record(
                        frame_index,
                        VerificationEventKind::RawPointerCopy {
                            place: src,
                            pointer: pointer.clone(),
                        },
                    );
                }
                Ok(value)
            }
            Operand::AddressOf(src) => {
                let place = self.resolve_access(frame_index, src);
                let pointer = RawPointerValue {
                    target: self.storage_region(frame_index, &place),
                };
                self.record(
                    frame_index,
                    VerificationEventKind::AddressOf {
                        place,
                        pointer: pointer.clone(),
                    },
                );
                Ok(RuntimeValue::RawPointer(pointer))
            }
            Operand::ReferenceRoot { permission, place } => {
                let target = self.storage_region(frame_index, place);
                let authority = self.allocate_reference_authority(target.clone(), *permission, None);
                Ok(RuntimeValue::SafeReference(SafeReferenceValue {
                    target,
                    authority,
                }))
            }
            Operand::ReferenceReborrow { permission, src } => {
                let parent = self.resolve_reference_access(frame_index, src);
                let authority = self.allocate_reference_authority(
                    parent.target.clone(),
                    *permission,
                    Some(parent.authority),
                );
                Ok(RuntimeValue::SafeReference(SafeReferenceValue {
                    target: parent.target,
                    authority,
                }))
            }
            Operand::ReferenceMove(src) => {
                let resolved = self.resolve_reference_access(frame_index, src);
                let (target_frame, target_place) =
                    self.frame_place_for_storage_region(&resolved.target);
                let value = {
                    let types = &self.program.as_program().types;
                    let frame = &mut self.frames[target_frame];
                    take_value(
                        types,
                        resolved.selected_ty,
                        place_state_mut(&mut frame.locals, &target_place),
                    )
                };
                Ok(value)
            }
            Operand::ReferenceCopy(src) => {
                let resolved = self.resolve_reference_access(frame_index, src);
                let (target_frame, target_place) =
                    self.frame_place_for_storage_region(&resolved.target);
                let types = &self.program.as_program().types;
                let state = place_state(&self.frames[target_frame].locals, &target_place);
                Ok(clone_value(
                    types,
                    resolved.selected_ty,
                    state,
                    &mut self.reference_authorities,
                ))
            }
        }
    }

    fn raw_move(
        &mut self,
        frame_index: usize,
        pointer_access: &PlaceAccess,
    ) -> Result<RuntimeValue, UndefinedBehaviorKind> {
        let pointer_place = self.resolve_access(frame_index, pointer_access);
        let pointer = self.raw_pointer_at(frame_index, &pointer_place);
        let target = self.place_for_storage_region(frame_index, &pointer.target);

        if !place_state(&self.frames[frame_index].locals, &target).fully_live() {
            return Err(UndefinedBehaviorKind::RawMoveTargetNotLive {
                target: pointer.target,
            });
        }
        if let Some((index, _)) = self.frames[frame_index]
            .active_loans
            .iter()
            .enumerate()
            .find(|(_, active)| {
                active
                    .as_ref()
                    .is_some_and(|active| active.place.overlaps(&target))
            })
        {
            return Err(UndefinedBehaviorKind::RawMoveConflictsWithLoan {
                target: pointer.target,
                loan: LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")),
            });
        }
        if let Some(authority) = self.raw_reference_conflict(&pointer.target, false) {
            return Err(UndefinedBehaviorKind::RawMoveConflictsWithReferenceAuthority {
                target: pointer.target,
                authority,
            });
        }

        let target_ty = self.place_type(frame_index, &target);
        let value = {
            let types = &self.program.as_program().types;
            let frame = &mut self.frames[frame_index];
            take_value(
                types,
                target_ty,
                place_state_mut(&mut frame.locals, &target),
            )
        };
        self.record(
            frame_index,
            VerificationEventKind::RawMove {
                pointer,
                target: target.clone(),
            },
        );
        if let RuntimeValue::RawPointer(pointer) = &value {
            self.record(
                frame_index,
                VerificationEventKind::RawPointerMove {
                    place: target,
                    pointer: pointer.clone(),
                },
            );
        }
        Ok(value)
    }

    fn raw_pointer_at(&self, frame_index: usize, place: &Place) -> RawPointerValue {
        match place_state(&self.frames[frame_index].locals, place) {
            ObjectState::Leaf(LeafState::Live(RuntimeValue::RawPointer(pointer))) => {
                pointer.clone()
            }
            _ => unreachable!("validated raw-pointer access reaches a fully-live pointer value"),
        }
    }

    fn resolve_reference_access(
        &self,
        frame_index: usize,
        access: &ReferenceAccess,
    ) -> ResolvedReferenceAccess {
        let carrier_place = self.resolve_access(frame_index, &access.reference);
        let carrier_ty = self.access_type(frame_index, &access.reference);
        let (referent, permission) = self
            .program
            .as_program()
            .types
            .reference(carrier_ty)
            .expect("validated reference access uses a safe-reference carrier");
        let reference = match place_state(&self.frames[frame_index].locals, &carrier_place) {
            ObjectState::Leaf(LeafState::Live(RuntimeValue::SafeReference(reference))) => {
                reference.clone()
            }
            _ => unreachable!("validated reference access reaches a live reference carrier"),
        };
        let active = self.reference_authority(reference.authority);
        debug_assert_eq!(active.target, reference.target);
        debug_assert_eq!(active.permission, permission);
        let selected_ty = self
            .program
            .as_program()
            .types
            .project_type(referent, &access.projections)
            .expect("validated reference access contains valid structural projections");
        let mut target = active.target.clone();
        target.projections.extend(access.projections.iter().copied());
        ResolvedReferenceAccess {
            authority: reference.authority,
            permission,
            target,
            selected_ty,
        }
    }

    fn resolve_access(&self, frame_index: usize, access: &PlaceAccess) -> Place {
        match access {
            PlaceAccess::Direct(place) => place.clone(),
            PlaceAccess::Loan { loan, projections } => {
                let active = self.frames[frame_index]
                    .active_loans
                    .get(loan.0 as usize)
                    .and_then(Option::as_ref)
                    .expect("validated Core MIR uses only active loans");
                let mut place = active.place.clone();
                place.projections.extend(projections.iter().copied());
                place
            }
        }
    }

    fn storage_region(&self, frame_index: usize, place: &Place) -> StorageRegion {
        let local = self.frames[frame_index]
            .locals
            .get(place.local.0 as usize)
            .expect("validated Core MIR references only known local storage");
        StorageRegion {
            instance: local.instance,
            projections: place.projections.clone(),
        }
    }

    /// Resolve a safe-reference target across every currently active frame.
    fn frame_place_for_storage_region(&self, region: &StorageRegion) -> (usize, Place) {
        self.frames
            .iter()
            .enumerate()
            .find_map(|(frame_index, frame)| {
                frame
                    .locals
                    .iter()
                    .position(|local| local.instance == region.instance)
                    .map(|local_index| {
                        (
                            frame_index,
                            Place {
                                local: LocalId(
                                    u32::try_from(local_index)
                                        .expect("local index exceeds u32::MAX"),
                                ),
                                projections: region.projections.clone(),
                            },
                        )
                    })
            })
            .expect("validated safe reference targets storage in an active frame")
    }

    /// Resolve raw-pointer storage within the actor frame only.
    fn place_for_storage_region(&self, frame_index: usize, region: &StorageRegion) -> Place {
        let local_index = self.frames[frame_index]
            .locals
            .iter()
            .position(|local| local.instance == region.instance)
            .expect("intra-activation raw pointer targets current frame storage");
        Place {
            local: LocalId(u32::try_from(local_index).expect("local index exceeds u32::MAX")),
            projections: region.projections.clone(),
        }
    }

    fn cleanup_frame(&mut self, frame_index: usize) {
        self.frames[frame_index].active_loans.fill(None);
        let local_count = self.frames[frame_index].locals.len();
        for local_index in (0..local_count).rev() {
            let local_id =
                LocalId(u32::try_from(local_index).expect("local index exceeds u32::MAX"));
            self.drop_place_contents(frame_index, &Place::local(local_id));
            let instance = self.frames[frame_index].locals[local_index].instance;
            assert!(
                !self.reference_authorities.iter().any(|active| active
                    .as_ref()
                    .is_some_and(|active| active.target.instance == instance)),
                "validated Core cleanup cannot end storage targeted by a surviving reference authority"
            );
        }
    }

    fn drop_place_contents(&mut self, frame_index: usize, place: &Place) {
        let ty = self.place_type(frame_index, place);
        let activation = self.frames[frame_index].activation;
        let types = &self.program.as_program().types;
        let (frames, authorities, trace) = (
            &mut self.frames,
            &mut self.reference_authorities,
            &mut self.verification_events,
        );
        let state = place_state_mut(&mut frames[frame_index].locals, place);
        drop_live_values(types, ty, state, place, activation, authorities, trace);
    }

    fn place_type(&self, frame_index: usize, place: &Place) -> TypeId {
        let frame = &self.frames[frame_index];
        let function = self
            .program
            .as_program()
            .function(frame.function)
            .expect("validated frame references a known function");
        let local = function
            .body
            .local(place.local)
            .expect("validated Core MIR references only known locals");
        self.program
            .as_program()
            .types
            .project_type(local.ty, &place.projections)
            .expect("validated Core MIR contains only valid projections")
    }

    fn access_type(&self, frame_index: usize, access: &PlaceAccess) -> TypeId {
        match access {
            PlaceAccess::Direct(place) => self.place_type(frame_index, place),
            PlaceAccess::Loan { loan, projections } => {
                let frame = &self.frames[frame_index];
                let function = self
                    .program
                    .as_program()
                    .function(frame.function)
                    .expect("validated frame references a known function");
                let declaration = function
                    .body
                    .loan(*loan)
                    .expect("validated Core MIR references only known loans");
                self.program
                    .as_program()
                    .types
                    .project_type(declaration.ty, projections)
                    .expect("validated loan access contains valid projections")
            }
        }
    }

    fn allocate_reference_authority(
        &mut self,
        target: StorageRegion,
        permission: ReferencePermission,
        parent: Option<ReferenceAuthorityId>,
    ) -> ReferenceAuthorityId {
        let id = ReferenceAuthorityId(self.next_reference_authority);
        self.next_reference_authority = self
            .next_reference_authority
            .checked_add(1)
            .expect("verification reference-authority identity exhausted");
        let index = authority_index(id);
        assert_eq!(
            index,
            self.reference_authorities.len(),
            "runtime authority identities are monotonic table indices"
        );
        self.reference_authorities.push(Some(ActiveReferenceAuthority {
            target,
            permission,
            parent,
            carriers: 1,
        }));
        id
    }

    fn reference_authority(&self, authority: ReferenceAuthorityId) -> &ActiveReferenceAuthority {
        self.reference_authorities
            .get(authority_index(authority))
            .and_then(Option::as_ref)
            .expect("live reference carrier names an active runtime authority")
    }

    fn raw_reference_conflict(
        &self,
        target: &StorageRegion,
        shared_read: bool,
    ) -> Option<ReferenceAuthorityId> {
        self.reference_authorities
            .iter()
            .enumerate()
            .find_map(|(index, active)| {
                let active = active.as_ref()?;
                let conflicts = active.target.overlaps(target)
                    && (!shared_read
                        || active.permission.alias_kind() == BorrowKind::Exclusive);
                conflicts.then(|| {
                    ReferenceAuthorityId(
                        u64::try_from(index).expect("reference authority index exceeds u64::MAX"),
                    )
                })
            })
    }

    fn record(&mut self, frame_index: usize, kind: VerificationEventKind) {
        self.verification_events.push(VerificationEvent {
            activation: self.frames[frame_index].activation,
            kind,
        });
    }
}

fn authority_index(authority: ReferenceAuthorityId) -> usize {
    usize::try_from(authority.0).expect("reference authority identity exceeds usize::MAX")
}

fn increment_reference_carrier(
    authorities: &mut [Option<ActiveReferenceAuthority>],
    authority: ReferenceAuthorityId,
) {
    let active = authorities[authority_index(authority)]
        .as_mut()
        .expect("copied live reference carrier names an active authority");
    active.carriers = active
        .carriers
        .checked_add(1)
        .expect("runtime reference carrier count overflow");
}

fn remove_reference_carrier(
    authorities: &mut [Option<ActiveReferenceAuthority>],
    authority: ReferenceAuthorityId,
) {
    let active = authorities[authority_index(authority)]
        .as_mut()
        .expect("destroyed live reference carrier names an active authority");
    active.carriers = active
        .carriers
        .checked_sub(1)
        .expect("runtime reference carrier count underflow");
    try_end_reference_authority(authorities, authority);
}

fn try_end_reference_authority(
    authorities: &mut [Option<ActiveReferenceAuthority>],
    authority: ReferenceAuthorityId,
) {
    let index = authority_index(authority);
    let Some(active) = authorities[index].as_ref() else {
        return;
    };
    if active.carriers != 0
        || authorities.iter().any(|candidate| {
            candidate
                .as_ref()
                .is_some_and(|candidate| candidate.parent == Some(authority))
        })
    {
        return;
    }
    let parent = authorities[index]
        .take()
        .expect("authority remains active until end")
        .parent;
    if let Some(parent) = parent {
        try_end_reference_authority(authorities, parent);
    }
}

fn place_state<'a>(locals: &'a [LocalStorage], place: &Place) -> &'a ObjectState {
    let mut state = &locals[place.local.0 as usize].state;
    for projection in &place.projections {
        match (state, projection) {
            (ObjectState::Aggregate(fields), Projection::Field(index)) => {
                state = &fields[*index as usize];
            }
            (ObjectState::Leaf(_), Projection::Field(_)) => {
                unreachable!("validated Core MIR cannot project a field from scalar state");
            }
        }
    }
    state
}

fn place_state_mut<'a>(locals: &'a mut [LocalStorage], place: &Place) -> &'a mut ObjectState {
    let mut state = &mut locals[place.local.0 as usize].state;
    for projection in &place.projections {
        match (state, projection) {
            (ObjectState::Aggregate(fields), Projection::Field(index)) => {
                state = &mut fields[*index as usize];
            }
            (ObjectState::Leaf(_), Projection::Field(_)) => {
                unreachable!("validated Core MIR cannot project a field from scalar state");
            }
        }
    }
    state
}

fn write_value(types: &TypeTable, ty: TypeId, state: &mut ObjectState, value: RuntimeValue) {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state, value) {
        (
            TypeKind::Scalar(ScalarType::Bool),
            ObjectState::Leaf(leaf),
            RuntimeValue::Bool(value),
        ) => *leaf = LeafState::Live(RuntimeValue::Bool(value)),
        (TypeKind::Scalar(ScalarType::I8), ObjectState::Leaf(leaf), RuntimeValue::I8(value)) => {
            *leaf = LeafState::Live(RuntimeValue::I8(value));
        }
        (TypeKind::Scalar(ScalarType::I16), ObjectState::Leaf(leaf), RuntimeValue::I16(value)) => {
            *leaf = LeafState::Live(RuntimeValue::I16(value));
        }
        (TypeKind::Scalar(ScalarType::I32), ObjectState::Leaf(leaf), RuntimeValue::I32(value)) => {
            *leaf = LeafState::Live(RuntimeValue::I32(value));
        }
        (TypeKind::Scalar(ScalarType::I64), ObjectState::Leaf(leaf), RuntimeValue::I64(value)) => {
            *leaf = LeafState::Live(RuntimeValue::I64(value));
        }
        (TypeKind::Scalar(ScalarType::U8), ObjectState::Leaf(leaf), RuntimeValue::U8(value)) => {
            *leaf = LeafState::Live(RuntimeValue::U8(value));
        }
        (TypeKind::Scalar(ScalarType::U16), ObjectState::Leaf(leaf), RuntimeValue::U16(value)) => {
            *leaf = LeafState::Live(RuntimeValue::U16(value));
        }
        (TypeKind::Scalar(ScalarType::U32), ObjectState::Leaf(leaf), RuntimeValue::U32(value)) => {
            *leaf = LeafState::Live(RuntimeValue::U32(value));
        }
        (TypeKind::Scalar(ScalarType::U64), ObjectState::Leaf(leaf), RuntimeValue::U64(value)) => {
            *leaf = LeafState::Live(RuntimeValue::U64(value));
        }
        (TypeKind::Scalar(ScalarType::F16), ObjectState::Leaf(leaf), RuntimeValue::F16(value)) => {
            *leaf = LeafState::Live(RuntimeValue::F16(value));
        }
        (TypeKind::Scalar(ScalarType::F32), ObjectState::Leaf(leaf), RuntimeValue::F32(value)) => {
            *leaf = LeafState::Live(RuntimeValue::F32(value));
        }
        (TypeKind::Scalar(ScalarType::F64), ObjectState::Leaf(leaf), RuntimeValue::F64(value)) => {
            *leaf = LeafState::Live(RuntimeValue::F64(value));
        }
        (
            TypeKind::Scalar(ScalarType::RawPointer(_)),
            ObjectState::Leaf(leaf),
            RuntimeValue::RawPointer(value),
        ) => *leaf = LeafState::Live(RuntimeValue::RawPointer(value)),
        (
            TypeKind::Scalar(ScalarType::Reference { .. }),
            ObjectState::Leaf(leaf),
            RuntimeValue::SafeReference(value),
        ) => *leaf = LeafState::Live(RuntimeValue::SafeReference(value)),
        (
            TypeKind::Scalar(ScalarType::TrackedFixture),
            ObjectState::Leaf(leaf),
            RuntimeValue::TrackedFixture(value),
        ) => *leaf = LeafState::Live(RuntimeValue::TrackedFixture(value)),
        (
            TypeKind::Struct(fields),
            ObjectState::Aggregate(states),
            RuntimeValue::Struct(values),
        ) if fields.len() == states.len() && fields.len() == values.len() => {
            for ((field, field_state), field_value) in fields.iter().zip(states).zip(values) {
                write_value(types, field.ty, field_state, field_value);
            }
        }
        _ => unreachable!("validated Core MIR guarantees value/type compatibility"),
    }
}

fn clone_value(
    types: &TypeTable,
    ty: TypeId,
    state: &ObjectState,
    authorities: &mut [Option<ActiveReferenceAuthority>],
) -> RuntimeValue {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state) {
        (
            TypeKind::Scalar(ScalarType::Reference {
                permission: ReferencePermission::Shared,
                ..
            }),
            ObjectState::Leaf(LeafState::Live(RuntimeValue::SafeReference(reference))),
        ) => {
            increment_reference_carrier(authorities, reference.authority);
            RuntimeValue::SafeReference(reference.clone())
        }
        (TypeKind::Scalar(ScalarType::Reference { .. }), _) => {
            unreachable!("validated Copy cannot duplicate an exclusive safe reference")
        }
        (TypeKind::Scalar(_), ObjectState::Leaf(LeafState::Live(value))) => value.clone(),
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            RuntimeValue::Struct(
                fields
                    .iter()
                    .zip(states)
                    .map(|(field, state)| clone_value(types, field.ty, state, authorities))
                    .collect(),
            )
        }
        _ => unreachable!("validated Core MIR guarantees copied state is fully live"),
    }
}

fn take_value(types: &TypeTable, ty: TypeId, state: &mut ObjectState) -> RuntimeValue {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(leaf)) => {
            match std::mem::replace(leaf, LeafState::Dead) {
                LeafState::Live(value) => value,
                LeafState::NeverInitialized | LeafState::Dead => {
                    unreachable!("validated Core MIR guarantees moved state is fully live")
                }
            }
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            RuntimeValue::Struct(
                fields
                    .iter()
                    .zip(states)
                    .map(|(field, state)| take_value(types, field.ty, state))
                    .collect(),
            )
        }
        _ => unreachable!("validated Core MIR guarantees moved state matches its type"),
    }
}

fn drop_live_values(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ObjectState,
    place: &Place,
    activation: ActivationId,
    authorities: &mut [Option<ActiveReferenceAuthority>],
    trace: &mut Vec<VerificationEvent>,
) {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(leaf)) => {
            let previous = std::mem::replace(leaf, LeafState::Dead);
            match previous {
                LeafState::Live(RuntimeValue::TrackedFixture(id)) => {
                    trace.push(VerificationEvent {
                        activation,
                        kind: VerificationEventKind::DropTrackedFixture {
                            place: place.clone(),
                            id,
                        },
                    });
                }
                LeafState::Live(RuntimeValue::SafeReference(reference)) => {
                    remove_reference_carrier(authorities, reference.authority);
                }
                LeafState::Live(_) => {}
                LeafState::NeverInitialized => *leaf = LeafState::NeverInitialized,
                LeafState::Dead => *leaf = LeafState::Dead,
            }
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            for index in (0..fields.len()).rev() {
                let field = &fields[index];
                let field_place = place.with_projection(Projection::Field(
                    u32::try_from(index).expect("field index exceeds u32::MAX"),
                ));
                drop_live_values(
                    types,
                    field.ty,
                    &mut states[index],
                    &field_place,
                    activation,
                    authorities,
                    trace,
                );
            }
        }
        _ => unreachable!("validated Core MIR guarantees state shape matches its type"),
    }
}
