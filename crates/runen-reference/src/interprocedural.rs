use runen_core_ir::interprocedural::Terminator;
use runen_core_ir::interprocedural_validation::ValidatedProgram;
use runen_core_ir::{
    BasicBlockId, BorrowKind, FunctionId, LoanId, LocalId, Operand, Place, PlaceAccess, Projection,
    ScalarType, Statement, StorageInstanceId, StorageRegion, TypeId, TypeKind, TypeTable, Value,
};

use crate::{RawPointerValue, UndefinedBehaviorKind, VerificationWriteKind};

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
    pub result: Option<Value>,
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
    I64(i64),
    RawPointer(RawPointerValue),
    TrackedFixture(u64),
    Struct(Vec<RuntimeValue>),
}

impl RuntimeValue {
    fn from_constant(value: &Value) -> Self {
        match value {
            Value::Bool(value) => Self::Bool(*value),
            Value::I64(value) => Self::I64(*value),
            Value::TrackedFixture(value) => Self::TrackedFixture(*value),
            Value::Struct(values) => Self::Struct(values.iter().map(Self::from_constant).collect()),
        }
    }

    fn into_public_value(self) -> Value {
        match self {
            Self::Bool(value) => Value::Bool(value),
            Self::I64(value) => Value::I64(value),
            Self::TrackedFixture(value) => Value::TrackedFixture(value),
            Self::Struct(values) => Value::Struct(
                values
                    .into_iter()
                    .map(Self::into_public_value)
                    .collect(),
            ),
            Self::RawPointer(_) => {
                unreachable!("validated entry result is call-transfer-safe and pointer-free")
            }
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

/// Executable abstract machine for validated interprocedural Core programs.
pub struct Machine {
    program: ValidatedProgram,
    frames: Vec<Frame>,
    next_activation: u64,
    next_storage_instance: u64,
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
            next_activation: 1,
            next_storage_instance: 1,
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
                Terminator::Goto(target) => {
                    self.frames[frame_index].current = target;
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
                            result: result.map(RuntimeValue::into_public_value),
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
            Statement::RawRead { pointer } => self.raw_read(frame_index, pointer),
            Statement::RawAssign { pointer, src } => {
                self.raw_assign(frame_index, pointer, src)
            }
            Statement::Assign { dst, src } => {
                self.replace(frame_index, dst, src, VerificationWriteKind::Assign)
            }
            Statement::InteriorAssign { dst, src } => {
                self.replace(frame_index, dst, src, VerificationWriteKind::InteriorAssign)
            }
            Statement::Drop { place } => {
                self.drop_explicit(frame_index, place);
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

    fn read(&mut self, frame_index: usize, src: &PlaceAccess) {
        let src = self.resolve_access(frame_index, src);
        self.record(frame_index, VerificationEventKind::Read(src));
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
                    take_value(
                        types,
                        src_ty,
                        place_state_mut(&mut frame.locals, &src),
                    )
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
            Operand::RawMove(pointer_access) => {
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
            Operand::Copy(src) => {
                let src = self.resolve_access(frame_index, src);
                let src_ty = self.place_type(frame_index, &src);
                let value = clone_value(
                    &self.program.as_program().types,
                    src_ty,
                    place_state(&self.frames[frame_index].locals, &src),
                );
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
        }
    }

    fn raw_pointer_at(&self, frame_index: usize, place: &Place) -> RawPointerValue {
        match place_state(&self.frames[frame_index].locals, place) {
            ObjectState::Leaf(LeafState::Live(RuntimeValue::RawPointer(pointer))) => {
                pointer.clone()
            }
            _ => unreachable!("validated raw-pointer access reaches a fully-live pointer value"),
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
        }
    }

    fn drop_place_contents(&mut self, frame_index: usize, place: &Place) {
        let ty = self.place_type(frame_index, place);
        let activation = self.frames[frame_index].activation;
        let types = &self.program.as_program().types;
        let state = place_state_mut(&mut self.frames[frame_index].locals, place);
        drop_live_values(
            types,
            ty,
            state,
            place,
            activation,
            &mut self.verification_events,
        );
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

    fn record(&mut self, frame_index: usize, kind: VerificationEventKind) {
        self.verification_events.push(VerificationEvent {
            activation: self.frames[frame_index].activation,
            kind,
        });
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
        (TypeKind::Scalar(ScalarType::I64), ObjectState::Leaf(leaf), RuntimeValue::I64(value)) => {
            *leaf = LeafState::Live(RuntimeValue::I64(value));
        }
        (
            TypeKind::Scalar(ScalarType::RawPointer(_)),
            ObjectState::Leaf(leaf),
            RuntimeValue::RawPointer(value),
        ) => *leaf = LeafState::Live(RuntimeValue::RawPointer(value)),
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

fn clone_value(types: &TypeTable, ty: TypeId, state: &ObjectState) -> RuntimeValue {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(LeafState::Live(value))) => value.clone(),
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            RuntimeValue::Struct(
                fields
                    .iter()
                    .zip(states)
                    .map(|(field, state)| clone_value(types, field.ty, state))
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
                    trace,
                );
            }
        }
        _ => unreachable!("validated Core MIR guarantees state shape matches its type"),
    }
}
