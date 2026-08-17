#![forbid(unsafe_code)]
//! Executable reference semantics for validated Runen Core MIR.

use runen_core_ir::{
    BorrowKind, LoanId, LocalId, Operand, Place, PlaceAccess, Projection, ScalarType, Statement,
    Terminator, TypeId, TypeKind, TypeTable, ValidatedBody, Value,
};

/// Why a write occurred in verification instrumentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationWriteKind {
    Init,
    Assign,
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
    Move(Place),
    Copy(Place),
    Write {
        place: Place,
        kind: VerificationWriteKind,
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum LeafState {
    NeverInitialized,
    Live(Value),
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveLoan {
    place: Place,
}

/// Small executable abstract machine for validated Core MIR.
pub struct Machine {
    body: ValidatedBody,
    locals: Vec<ObjectState>,
    active_loans: Vec<Option<ActiveLoan>>,
    verification_events: Vec<VerificationEvent>,
}

impl Machine {
    /// Constructs the reference machine from Core MIR that passed language validation.
    #[must_use]
    pub fn new(body: ValidatedBody) -> Self {
        let locals = body
            .as_body()
            .locals
            .iter()
            .map(|local| ObjectState::uninitialized(&body.as_body().types, local.ty))
            .collect();
        let active_loans = vec![None; body.as_body().loans.len()];

        Self {
            body,
            locals,
            active_loans,
            verification_events: Vec::new(),
        }
    }

    /// Executes validated MIR until defined `Return` or defined `Fault` termination.
    ///
    /// Cyclic MIR may diverge and therefore never return an `ExecutionReport`.
    #[must_use]
    pub fn execute(mut self) -> ExecutionReport {
        let mut current = self.body.as_body().entry;

        loop {
            let block = self
                .body
                .as_body()
                .block(current)
                .expect("validated Core MIR reaches only known basic blocks")
                .clone();

            for statement in &block.statements {
                self.execute_statement(statement);
            }

            match block.terminator {
                Terminator::Goto(target) => current = target,
                Terminator::Return => {
                    self.end_all_borrows();
                    self.cleanup_all_locals();
                    return ExecutionReport {
                        terminal: TerminalStatus::Returned,
                        verification_events: self.verification_events,
                    };
                }
                Terminator::Fault(fault) => {
                    self.end_all_borrows();
                    self.cleanup_all_locals();
                    return ExecutionReport {
                        terminal: TerminalStatus::Faulted(fault.code),
                        verification_events: self.verification_events,
                    };
                }
            }
        }
    }

    fn execute_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Init { dst, src } => self.initialize(dst, src),
            Statement::Borrow { loan, kind, place } => self.begin_borrow(*loan, *kind, place),
            Statement::EndBorrow { loan } => self.end_borrow(*loan),
            Statement::Read { src } => self.read(src),
            Statement::Assign { dst, src } => self.assign(dst, src),
            Statement::Drop { place } => self.drop_explicit(place),
        }
    }

    fn initialize(&mut self, dst: &Place, src: &Operand) {
        let dst_ty = self.place_type(dst);
        let value = self.evaluate_operand(src);
        let dst_state = place_state_mut(&mut self.locals, dst);
        write_value(&self.body.as_body().types, dst_ty, dst_state, value);
        self.verification_events.push(VerificationEvent::Write {
            place: dst.clone(),
            kind: VerificationWriteKind::Init,
        });
    }

    fn begin_borrow(&mut self, loan: LoanId, kind: BorrowKind, place: &Place) {
        let slot = self
            .active_loans
            .get_mut(loan.0 as usize)
            .expect("validated Core MIR references only declared loans");
        assert!(
            slot.is_none(),
            "validated Core MIR cannot begin an already-active loan"
        );
        *slot = Some(ActiveLoan {
            place: place.clone(),
        });
        self.verification_events
            .push(VerificationEvent::BorrowStart {
                loan,
                kind,
                place: place.clone(),
            });
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

    fn assign(&mut self, dst: &PlaceAccess, src: &Operand) {
        let value = self.evaluate_operand(src);
        let dst = self.resolve_access(dst);
        let dst_ty = self.place_type(&dst);

        self.drop_place_contents(&dst);
        let dst_state = place_state_mut(&mut self.locals, &dst);
        write_value(&self.body.as_body().types, dst_ty, dst_state, value);
        self.verification_events.push(VerificationEvent::Write {
            place: dst,
            kind: VerificationWriteKind::Assign,
        });
    }

    fn read(&mut self, src: &PlaceAccess) {
        let src = self.resolve_access(src);
        self.verification_events.push(VerificationEvent::Read(src));
    }

    fn drop_explicit(&mut self, access: &PlaceAccess) {
        let place = self.resolve_access(access);
        self.drop_place_contents(&place);
    }

    fn evaluate_operand(&mut self, operand: &Operand) -> Value {
        match operand {
            Operand::Constant(value) => value.clone(),
            Operand::Move(src) => {
                let src = self.resolve_access(src);
                let src_ty = self.place_type(&src);
                let src_state = place_state_mut(&mut self.locals, &src);
                let value = take_value(&self.body.as_body().types, src_ty, src_state);
                self.verification_events
                    .push(VerificationEvent::Move(src));
                value
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
                    .push(VerificationEvent::Copy(src));
                value
            }
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

fn place_state<'a>(locals: &'a [ObjectState], place: &Place) -> &'a ObjectState {
    let mut state = &locals[place.local.0 as usize];
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

fn place_state_mut<'a>(locals: &'a mut [ObjectState], place: &Place) -> &'a mut ObjectState {
    let mut state = &mut locals[place.local.0 as usize];
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

fn write_value(types: &TypeTable, ty: TypeId, state: &mut ObjectState, value: Value) {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");

    match (&definition.kind, state, value) {
        (TypeKind::Scalar(ScalarType::Bool), ObjectState::Leaf(leaf), Value::Bool(value)) => {
            *leaf = LeafState::Live(Value::Bool(value));
        }
        (TypeKind::Scalar(ScalarType::I64), ObjectState::Leaf(leaf), Value::I64(value)) => {
            *leaf = LeafState::Live(Value::I64(value));
        }
        (
            TypeKind::Scalar(ScalarType::TrackedFixture),
            ObjectState::Leaf(leaf),
            Value::TrackedFixture(value),
        ) => {
            *leaf = LeafState::Live(Value::TrackedFixture(value));
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states), Value::Struct(values))
            if fields.len() == states.len() && fields.len() == values.len() =>
        {
            for ((field, field_state), field_value) in fields.iter().zip(states).zip(values) {
                write_value(types, field.ty, field_state, field_value);
            }
        }
        _ => unreachable!("validated Core MIR guarantees value/type compatibility"),
    }
}

fn clone_value(types: &TypeTable, ty: TypeId, state: &ObjectState) -> Value {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");

    match (&definition.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(LeafState::Live(value))) => value.clone(),
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            Value::Struct(
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

fn take_value(types: &TypeTable, ty: TypeId, state: &mut ObjectState) -> Value {
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
            Value::Struct(
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
                LeafState::Live(Value::TrackedFixture(id)) => {
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
