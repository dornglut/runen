#![forbid(unsafe_code)]
//! Executable reference semantics for validated Runen Core MIR.

use runen_core_ir::{
    BorrowKind, LoanId, LocalId, Operand, Place, PlaceAccess, Projection, ScalarType, Statement,
    StorageInstanceId, StorageRegion, Terminator, TypeId, TypeKind, TypeTable, ValidatedBody,
    Value,
};

/// Why a write occurred in verification instrumentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationWriteKind {
    Init,
    Assign,
    InteriorAssign,
    RawAssign,
}

/// Verification representation of one symbolic raw-pointer value.
///
/// `target` identifies the structural storage region selected when the pointer was
/// formed. The currently defined provenance root is represented once by
/// `target.instance`; the oracle does not materialize a duplicate provenance field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPointerValue {
    pub target: StorageRegion,
}

/// Verification-only event emitted by the executable reference oracle.
///
/// These events expose internal semantic transitions to conformance tests. They
/// are not the observable program trace defined by the Runen language semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationEvent {
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
    /// Verification event for destruction of the synthetic `TrackedFixture` fixture.
    DropTrackedFixture {
        place: Place,
        id: u64,
    },
}

/// Terminal status of a defined reference execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalStatus {
    Returned,
    Faulted(String),
}

/// Complete result of a terminating defined reference execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReport {
    pub terminal: TerminalStatus,
    /// Verification instrumentation, not Runen-observable program behavior.
    pub verification_events: Vec<VerificationEvent>,
}

/// Verification-only classification of detected undefined behavior.
///
/// These variants diagnose concrete unsafe-precondition violations in the reference
/// oracle. They are not Runen-observable outcomes and do not define a complete UB
/// taxonomy for the language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndefinedBehaviorKind {
    RawReadTargetNotLive { target: StorageRegion },
    RawReadConflictsWithExclusiveLoan { target: StorageRegion, loan: LoanId },
    RawMoveTargetNotLive { target: StorageRegion },
    RawMoveConflictsWithLoan { target: StorageRegion, loan: LoanId },
    RawAssignConflictsWithLoan { target: StorageRegion, loan: LoanId },
}

/// Reference-oracle report for an execution that entered undefined behavior.
///
/// The accumulated events are verification evidence only. The oracle stops at the
/// detected violation and does not perform defined Return/Fault cleanup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndefinedBehavior {
    pub kind: UndefinedBehaviorKind,
    pub verification_events: Vec<VerificationEvent>,
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
            .expect("validated Core MIR references only known types");
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

/// Small executable abstract machine for validated Core MIR.
pub struct Machine {
    body: ValidatedBody,
    locals: Vec<LocalStorage>,
    active_loans: Vec<Option<ActiveLoan>>,
    verification_events: Vec<VerificationEvent>,
}

impl Machine {
    /// Constructs the reference machine from Core MIR that passed language validation.
    #[must_use]
    pub fn new(body: ValidatedBody) -> Self {
        // The counter is an oracle representation of fresh execution-scoped storage
        // identity. Semantics depend only on uniqueness and stability, not on the
        // numeric values or their relation to MIR `LocalId`s.
        let mut next_storage_instance = 1_u64;
        let locals = body
            .as_body()
            .locals
            .iter()
            .map(|local| {
                let instance = StorageInstanceId(next_storage_instance);
                next_storage_instance = next_storage_instance
                    .checked_add(1)
                    .expect("reference storage-instance identity exhausted");
                LocalStorage {
                    instance,
                    state: ObjectState::uninitialized(&body.as_body().types, local.ty),
                }
            })
            .collect();
        let active_loans = vec![None; body.as_body().loans.len()];

        Self {
            body,
            locals,
            active_loans,
            verification_events: Vec::new(),
        }
    }

    /// Executes validated MIR until defined termination or detected undefined behavior.
    ///
    /// Cyclic MIR may diverge and therefore never return. `Err` is a verification-only
    /// UB report, not a defined Runen `Fault` or observable program outcome.
    pub fn execute(mut self) -> Result<ExecutionReport, UndefinedBehavior> {
        let mut current = self.body.as_body().entry;

        loop {
            let block = self
                .body
                .as_body()
                .block(current)
                .expect("validated Core MIR reaches only known basic blocks")
                .clone();

            for statement in &block.statements {
                if let Err(kind) = self.execute_statement(statement) {
                    return Err(UndefinedBehavior {
                        kind,
                        verification_events: self.verification_events,
                    });
                }
            }

            match block.terminator {
                Terminator::Goto(target) => current = target,
                Terminator::Return => {
                    self.end_all_borrows();
                    self.cleanup_all_locals();
                    return Ok(ExecutionReport {
                        terminal: TerminalStatus::Returned,
                        verification_events: self.verification_events,
                    });
                }
                Terminator::Fault(fault) => {
                    self.end_all_borrows();
                    self.cleanup_all_locals();
                    return Ok(ExecutionReport {
                        terminal: TerminalStatus::Faulted(fault.code),
                        verification_events: self.verification_events,
                    });
                }
            }
        }
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<(), UndefinedBehaviorKind> {
        match statement {
            Statement::Init { dst, src } => self.initialize(dst, src),
            Statement::Borrow { loan, kind, src } => {
                self.begin_borrow(*loan, *kind, src);
                Ok(())
            }
            Statement::EndBorrow { loan } => {
                self.end_borrow(*loan);
                Ok(())
            }
            Statement::Read { src } => {
                self.read(src);
                Ok(())
            }
            Statement::RawRead { pointer } => self.raw_read(pointer),
            Statement::RawAssign { pointer, src } => self.raw_assign(pointer, src),
            Statement::Assign { dst, src } => self.replace(dst, src, VerificationWriteKind::Assign),
            Statement::InteriorAssign { dst, src } => {
                self.replace(dst, src, VerificationWriteKind::InteriorAssign)
            }
            Statement::Drop { place } => {
                self.drop_explicit(place);
                Ok(())
            }
        }
    }

    fn initialize(&mut self, dst: &Place, src: &Operand) -> Result<(), UndefinedBehaviorKind> {
        let dst_ty = self.place_type(dst);
        let value = self.evaluate_operand(src)?;
        let dst_state = place_state_mut(&mut self.locals, dst);
        write_value(&self.body.as_body().types, dst_ty, dst_state, value);
        self.verification_events.push(VerificationEvent::Write {
            place: dst.clone(),
            kind: VerificationWriteKind::Init,
        });
        Ok(())
    }

    fn begin_borrow(&mut self, loan: LoanId, kind: BorrowKind, src: &PlaceAccess) {
        let place = self.resolve_access(src);
        let slot = self
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
        self.verification_events
            .push(VerificationEvent::BorrowStart { loan, kind, place });
    }

    fn end_borrow(&mut self, loan: LoanId) {
        let slot = self
            .active_loans
            .get_mut(loan.0 as usize)
            .expect("validated Core MIR references only declared loans");
        assert!(
            slot.take().is_some(),
            "validated Core MIR cannot end an inactive loan"
        );
        self.verification_events
            .push(VerificationEvent::BorrowEnd(loan));
    }

    fn replace(
        &mut self,
        dst: &PlaceAccess,
        src: &Operand,
        kind: VerificationWriteKind,
    ) -> Result<(), UndefinedBehaviorKind> {
        // All assignment forms share one accepted source-first replacement lifecycle.
        // Ordinary/interior legality is already established by validation. A failing
        // unsafe operand stops before destination destruction or write.
        let dst = self.resolve_access(dst);
        let value = self.evaluate_operand(src)?;
        self.replace_resolved(dst, value, kind);
        Ok(())
    }

    fn replace_resolved(&mut self, dst: Place, value: RuntimeValue, kind: VerificationWriteKind) {
        let dst_ty = self.place_type(&dst);
        self.drop_place_contents(&dst);
        let dst_state = place_state_mut(&mut self.locals, &dst);
        write_value(&self.body.as_body().types, dst_ty, dst_state, value);
        self.verification_events
            .push(VerificationEvent::Write { place: dst, kind });
    }

    fn read(&mut self, src: &PlaceAccess) {
        let src = self.resolve_access(src);
        self.verification_events.push(VerificationEvent::Read(src));
    }

    fn raw_read(&mut self, pointer_access: &PlaceAccess) -> Result<(), UndefinedBehaviorKind> {
        // Validation already guarantees that pointer_access reaches a fully-live
        // raw-pointer value with shared PlaceAccess authority. The target obligations
        // below are the unsafe runtime boundary and deliberately are not reclassified
        // as MIR-validation failures.
        let pointer_place = self.resolve_access(pointer_access);
        let pointer = self.raw_pointer_at(&pointer_place);
        let target = self.place_for_storage_region(&pointer.target);

        if !place_state(&self.locals, &target).fully_live() {
            return Err(UndefinedBehaviorKind::RawReadTargetNotLive {
                target: pointer.target,
            });
        }

        if let Some((index, _)) = self.active_loans.iter().enumerate().find(|(_, active)| {
            active.as_ref().is_some_and(|active| {
                active.kind == BorrowKind::Exclusive && active.place.overlaps(&target)
            })
        }) {
            return Err(UndefinedBehaviorKind::RawReadConflictsWithExclusiveLoan {
                target: pointer.target,
                loan: LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")),
            });
        }

        self.verification_events
            .push(VerificationEvent::RawRead { pointer, target });
        Ok(())
    }

    fn raw_assign(
        &mut self,
        pointer_access: &PlaceAccess,
        src: &Operand,
    ) -> Result<(), UndefinedBehaviorKind> {
        // The pointer value and target are snapshotted before source evaluation.
        // Validation owns the pointer-value access and source typing, while the raw
        // target's exclusive compatibility remains an unsafe execution precondition.
        let pointer_place = self.resolve_access(pointer_access);
        let pointer = self.raw_pointer_at(&pointer_place);
        let target = self.place_for_storage_region(&pointer.target);
        let value = self.evaluate_operand(src)?;

        if let Some((index, _)) = self.active_loans.iter().enumerate().find(|(_, active)| {
            active
                .as_ref()
                .is_some_and(|active| active.place.overlaps(&target))
        }) {
            return Err(UndefinedBehaviorKind::RawAssignConflictsWithLoan {
                target: pointer.target,
                loan: LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")),
            });
        }

        self.replace_resolved(target, value, VerificationWriteKind::RawAssign);
        Ok(())
    }

    fn drop_explicit(&mut self, access: &PlaceAccess) {
        let place = self.resolve_access(access);
        self.drop_place_contents(&place);
    }

    fn evaluate_operand(
        &mut self,
        operand: &Operand,
    ) -> Result<RuntimeValue, UndefinedBehaviorKind> {
        match operand {
            Operand::Constant(value) => Ok(RuntimeValue::from_constant(value)),
            Operand::Move(src) => {
                let src = self.resolve_access(src);
                let src_ty = self.place_type(&src);
                let src_state = place_state_mut(&mut self.locals, &src);
                let value = take_value(&self.body.as_body().types, src_ty, src_state);
                self.verification_events
                    .push(VerificationEvent::Move(src.clone()));
                if let RuntimeValue::RawPointer(pointer) = &value {
                    self.verification_events
                        .push(VerificationEvent::RawPointerMove {
                            place: src,
                            pointer: pointer.clone(),
                        });
                }
                Ok(value)
            }
            Operand::RawMove(pointer_access) => {
                // Snapshot the stored raw-pointer value before consuming any target
                // storage, including the self-targeting case.
                let pointer_place = self.resolve_access(pointer_access);
                let pointer = self.raw_pointer_at(&pointer_place);
                let target = self.place_for_storage_region(&pointer.target);

                if !place_state(&self.locals, &target).fully_live() {
                    return Err(UndefinedBehaviorKind::RawMoveTargetNotLive {
                        target: pointer.target,
                    });
                }

                if let Some((index, _)) =
                    self.active_loans.iter().enumerate().find(|(_, active)| {
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

                let target_ty = self.place_type(&target);
                let target_state = place_state_mut(&mut self.locals, &target);
                let value = take_value(&self.body.as_body().types, target_ty, target_state);
                self.verification_events.push(VerificationEvent::RawMove {
                    pointer,
                    target: target.clone(),
                });
                if let RuntimeValue::RawPointer(pointer) = &value {
                    self.verification_events
                        .push(VerificationEvent::RawPointerMove {
                            place: target,
                            pointer: pointer.clone(),
                        });
                }
                Ok(value)
            }
            Operand::Copy(src) => {
                let src = self.resolve_access(src);
                let src_ty = self.place_type(&src);
                let value = clone_value(
                    &self.body.as_body().types,
                    src_ty,
                    place_state(&self.locals, &src),
                );
                self.verification_events
                    .push(VerificationEvent::Copy(src.clone()));
                if let RuntimeValue::RawPointer(pointer) = &value {
                    self.verification_events
                        .push(VerificationEvent::RawPointerCopy {
                            place: src,
                            pointer: pointer.clone(),
                        });
                }
                Ok(value)
            }
            Operand::AddressOf(src) => {
                let place = self.resolve_access(src);
                let pointer = RawPointerValue {
                    target: self.storage_region(&place),
                };
                self.verification_events.push(VerificationEvent::AddressOf {
                    place,
                    pointer: pointer.clone(),
                });
                Ok(RuntimeValue::RawPointer(pointer))
            }
        }
    }

    fn raw_pointer_at(&self, place: &Place) -> RawPointerValue {
        match place_state(&self.locals, place) {
            ObjectState::Leaf(LeafState::Live(RuntimeValue::RawPointer(pointer))) => {
                pointer.clone()
            }
            _ => unreachable!("validated raw-pointer access reaches a fully-live pointer value"),
        }
    }

    fn resolve_access(&self, access: &PlaceAccess) -> Place {
        match access {
            PlaceAccess::Direct(place) => place.clone(),
            PlaceAccess::Loan { loan, projections } => {
                let active = self
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

    fn storage_region(&self, place: &Place) -> StorageRegion {
        let local = self
            .locals
            .get(place.local.0 as usize)
            .expect("validated Core MIR references only known local storage");
        StorageRegion {
            instance: local.instance,
            projections: place.projections.clone(),
        }
    }

    fn place_for_storage_region(&self, region: &StorageRegion) -> Place {
        let local_index = self
            .locals
            .iter()
            .position(|local| local.instance == region.instance)
            .expect("formed raw pointer targets a current local storage instance");
        Place {
            local: LocalId(u32::try_from(local_index).expect("local index exceeds u32::MAX")),
            projections: region.projections.clone(),
        }
    }

    fn end_all_borrows(&mut self) {
        self.active_loans.fill(None);
    }

    fn cleanup_all_locals(&mut self) {
        for local_index in (0..self.locals.len()).rev() {
            let local_id =
                LocalId(u32::try_from(local_index).expect("local index exceeds u32::MAX"));
            self.drop_place_contents(&Place::local(local_id));
        }
    }

    fn drop_place_contents(&mut self, place: &Place) {
        let ty = self.place_type(place);
        let state = place_state_mut(&mut self.locals, place);
        drop_live_values(
            &self.body.as_body().types,
            ty,
            state,
            place,
            &mut self.verification_events,
        );
    }

    fn place_type(&self, place: &Place) -> TypeId {
        let local = self
            .body
            .as_body()
            .local(place.local)
            .expect("validated Core MIR references only known locals");
        self.body
            .as_body()
            .types
            .project_type(local.ty, &place.projections)
            .expect("validated Core MIR contains only valid projections")
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
        ) => {
            *leaf = LeafState::Live(RuntimeValue::Bool(value));
        }
        (TypeKind::Scalar(ScalarType::I64), ObjectState::Leaf(leaf), RuntimeValue::I64(value)) => {
            *leaf = LeafState::Live(RuntimeValue::I64(value));
        }
        (
            TypeKind::Scalar(ScalarType::RawPointer(_)),
            ObjectState::Leaf(leaf),
            RuntimeValue::RawPointer(value),
        ) => {
            *leaf = LeafState::Live(RuntimeValue::RawPointer(value));
        }
        (
            TypeKind::Scalar(ScalarType::TrackedFixture),
            ObjectState::Leaf(leaf),
            RuntimeValue::TrackedFixture(value),
        ) => {
            *leaf = LeafState::Live(RuntimeValue::TrackedFixture(value));
        }
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
                    trace.push(VerificationEvent::DropTrackedFixture {
                        place: place.clone(),
                        id,
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
                drop_live_values(types, field.ty, &mut states[index], &field_place, trace);
            }
        }
        _ => unreachable!("validated Core MIR guarantees state shape matches its type"),
    }
}
