#![forbid(unsafe_code)]
//! Executable reference semantics for the Runen A0 Core kernel.

use runen_core_ir::{
    BasicBlockId, Body, LocalId, MirPoint, Operand, Place, Projection, ScalarType, Statement,
    Terminator, TypeId, TypeKind, TypeTable, ValidatedBody, Value,
};

/// Path-state violation encountered while exercising admitted A0 proving MIR.
///
/// These violations are not defined Runen `Fault` outcomes. They exist so the
/// executable oracle can exercise negative A0 state-transition cases directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionViolationKind {
    InitRequiresNeverInitialized(Place),
    UseOfUninitialized(Place),
    DropOfUninitialized(Place),
}

/// Structured execution-time proving violation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionViolation {
    pub point: MirPoint,
    pub kind: ExecutionViolationKind,
}

/// Why a write occurred in the verification trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteKind {
    Init,
    Assign,
}

/// Verification-only trace event emitted by the A0 executable oracle.
///
/// These events expose internal semantic transitions to conformance tests. They
/// are not the observable program trace defined by the Runen language semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceEvent {
    Read(Place),
    Move(Place),
    Copy(Place),
    Write {
        place: Place,
        kind: WriteKind,
    },
    /// Verification event for destruction of the synthetic `Tracked` fixture.
    DropTracked {
        place: Place,
        id: u64,
    },
}

/// Terminal status of a successful defined reference execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalStatus {
    Returned,
    Faulted(String),
}

/// Complete result of a successful defined reference execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReport {
    pub terminal: TerminalStatus,
    /// Verification instrumentation, not Runen-observable program behavior.
    pub trace: Vec<TraceEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LeafState {
    /// This storage path has never been initialized in the current object lifetime.
    NeverInitialized,
    /// The value is currently alive.
    Live(Value),
    /// The path was initialized previously and has since been moved/dropped.
    Dead,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ObjectState {
    Leaf(LeafState),
    Aggregate(Vec<ObjectState>),
}

impl ObjectState {
    fn uninitialized(types: &TypeTable, ty: TypeId) -> Self {
        let def = types
            .get(ty)
            .expect("validated Core MIR references an unknown type");

        match &def.kind {
            TypeKind::Scalar(_) => Self::Leaf(LeafState::NeverInitialized),
            TypeKind::Struct(fields) => Self::Aggregate(
                fields
                    .iter()
                    .map(|field| Self::uninitialized(types, field.ty))
                    .collect(),
            ),
        }
    }

    fn all_never_initialized(&self) -> bool {
        match self {
            Self::Leaf(LeafState::NeverInitialized) => true,
            Self::Leaf(LeafState::Live(_) | LeafState::Dead) => false,
            Self::Aggregate(fields) => fields.iter().all(Self::all_never_initialized),
        }
    }

    fn fully_live(&self) -> bool {
        match self {
            Self::Leaf(LeafState::Live(_)) => true,
            Self::Leaf(LeafState::NeverInitialized | LeafState::Dead) => false,
            Self::Aggregate(fields) => fields.iter().all(Self::fully_live),
        }
    }

    fn any_live(&self) -> bool {
        match self {
            Self::Leaf(LeafState::Live(_)) => true,
            Self::Leaf(LeafState::NeverInitialized | LeafState::Dead) => false,
            Self::Aggregate(fields) => fields.iter().any(Self::any_live),
        }
    }
}

/// Small executable abstract machine for admitted A0 Core MIR.
pub struct Machine {
    body: Body,
    locals: Vec<ObjectState>,
    trace: Vec<TraceEvent>,
    point: MirPoint,
}

impl Machine {
    /// Constructs the reference machine from Core MIR that has passed admission.
    #[must_use]
    pub fn new(body: ValidatedBody) -> Self {
        let body = body.into_body();
        let locals = body
            .locals
            .iter()
            .map(|local| ObjectState::uninitialized(&body.types, local.ty))
            .collect();

        Self {
            point: MirPoint {
                block: body.entry,
                statement: None,
            },
            body,
            locals,
            trace: Vec::new(),
        }
    }

    pub fn execute(mut self) -> Result<ExecutionReport, ExecutionViolation> {
        let mut current = self.body.entry;

        loop {
            let block = self
                .body
                .block(current)
                .expect("validated Core MIR reached an unknown basic block")
                .clone();

            for (statement_index, statement) in block.statements.iter().enumerate() {
                self.point = MirPoint {
                    block: current,
                    statement: Some(statement_index),
                };
                self.execute_statement(statement)?;
            }

            self.point = MirPoint {
                block: current,
                statement: None,
            };

            match block.terminator {
                Terminator::Goto(target) => {
                    current = target;
                }
                Terminator::Return => {
                    self.cleanup_all_locals();
                    return Ok(ExecutionReport {
                        terminal: TerminalStatus::Returned,
                        trace: self.trace,
                    });
                }
                Terminator::Fault(fault) => {
                    self.cleanup_all_locals();
                    return Ok(ExecutionReport {
                        terminal: TerminalStatus::Faulted(fault.code),
                        trace: self.trace,
                    });
                }
            }
        }
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<(), ExecutionViolation> {
        match statement {
            Statement::Init { dst, src } => self.initialize(dst, src),
            Statement::Read { src } => self.read(src),
            Statement::Assign { dst, src } => self.assign(dst, src),
            Statement::Drop { place } => self.drop_explicit(place),
        }
    }

    fn initialize(&mut self, dst: &Place, src: &Operand) -> Result<(), ExecutionViolation> {
        let dst_ty = self.place_type(dst);
        if !self.place_state(dst).all_never_initialized() {
            return Err(self.violation(ExecutionViolationKind::InitRequiresNeverInitialized(
                dst.clone(),
            )));
        }

        let value = self.evaluate_operand(src)?;
        let dst_state = place_state_mut(&mut self.locals, dst);
        write_value(&self.body.types, dst_ty, dst_state, value);
        self.trace.push(TraceEvent::Write {
            place: dst.clone(),
            kind: WriteKind::Init,
        });
        Ok(())
    }

    fn assign(&mut self, dst: &Place, src: &Operand) -> Result<(), ExecutionViolation> {
        let dst_ty = self.place_type(dst);
        let value = self.evaluate_operand(src)?;

        self.drop_place_contents(dst);
        let dst_state = place_state_mut(&mut self.locals, dst);
        write_value(&self.body.types, dst_ty, dst_state, value);
        self.trace.push(TraceEvent::Write {
            place: dst.clone(),
            kind: WriteKind::Assign,
        });
        Ok(())
    }

    fn read(&mut self, src: &Place) -> Result<(), ExecutionViolation> {
        if !self.place_state(src).fully_live() {
            return Err(self.violation(ExecutionViolationKind::UseOfUninitialized(src.clone())));
        }
        self.trace.push(TraceEvent::Read(src.clone()));
        Ok(())
    }

    fn drop_explicit(&mut self, place: &Place) -> Result<(), ExecutionViolation> {
        if !self.place_state(place).any_live() {
            return Err(self.violation(ExecutionViolationKind::DropOfUninitialized(
                place.clone(),
            )));
        }
        self.drop_place_contents(place);
        Ok(())
    }

    fn evaluate_operand(&mut self, operand: &Operand) -> Result<Value, ExecutionViolation> {
        match operand {
            Operand::Constant(value) => Ok(value.clone()),
            Operand::Move(src) => {
                if !self.place_state(src).fully_live() {
                    return Err(
                        self.violation(ExecutionViolationKind::UseOfUninitialized(src.clone()))
                    );
                }
                let src_ty = self.place_type(src);
                let src_state = place_state_mut(&mut self.locals, src);
                let value = take_value(&self.body.types, src_ty, src_state);
                self.trace.push(TraceEvent::Move(src.clone()));
                Ok(value)
            }
            Operand::Copy(src) => {
                if !self.place_state(src).fully_live() {
                    return Err(
                        self.violation(ExecutionViolationKind::UseOfUninitialized(src.clone()))
                    );
                }
                let src_ty = self.place_type(src);
                let value = clone_value(&self.body.types, src_ty, self.place_state(src));
                self.trace.push(TraceEvent::Copy(src.clone()));
                Ok(value)
            }
        }
    }

    fn cleanup_all_locals(&mut self) {
        for local_index in (0..self.locals.len()).rev() {
            let local_id =
                LocalId(u32::try_from(local_index).expect("local index exceeds u32::MAX"));
            let place = Place::local(local_id);
            self.drop_place_contents(&place);
        }
    }

    fn drop_place_contents(&mut self, place: &Place) {
        let ty = self.place_type(place);
        let state = place_state_mut(&mut self.locals, place);
        drop_live_values(&self.body.types, ty, state, place, &mut self.trace);
    }

    fn place_type(&self, place: &Place) -> TypeId {
        let local = self
            .body
            .local(place.local)
            .expect("validated Core MIR references an unknown local");
        self.body
            .types
            .project_type(local.ty, &place.projections)
            .expect("validated Core MIR contains an invalid projection")
    }

    fn place_state(&self, place: &Place) -> &ObjectState {
        let mut state = self
            .locals
            .get(place.local.0 as usize)
            .expect("validated Core MIR references an unknown local state");

        for projection in &place.projections {
            match (state, projection) {
                (ObjectState::Aggregate(fields), Projection::Field(index)) => {
                    state = fields
                        .get(*index as usize)
                        .expect("validated Core MIR projection exceeds state shape");
                }
                (ObjectState::Leaf(_), Projection::Field(_)) => {
                    unreachable!("validated Core MIR projects a field from scalar state");
                }
            }
        }
        state
    }

    fn violation(&self, kind: ExecutionViolationKind) -> ExecutionViolation {
        ExecutionViolation {
            point: self.point.clone(),
            kind,
        }
    }
}

fn place_state_mut<'a>(locals: &'a mut [ObjectState], place: &Place) -> &'a mut ObjectState {
    let mut state = locals
        .get_mut(place.local.0 as usize)
        .expect("validated Core MIR references an unknown local state");

    for projection in &place.projections {
        match (state, projection) {
            (ObjectState::Aggregate(fields), Projection::Field(index)) => {
                state = fields
                    .get_mut(*index as usize)
                    .expect("validated Core MIR projection exceeds state shape");
            }
            (ObjectState::Leaf(_), Projection::Field(_)) => {
                unreachable!("validated Core MIR projects a field from scalar state");
            }
        }
    }
    state
}

fn write_value(types: &TypeTable, ty: TypeId, state: &mut ObjectState, value: Value) {
    let def = types
        .get(ty)
        .expect("validated Core MIR references an unknown type");

    match (&def.kind, state, value) {
        (TypeKind::Scalar(ScalarType::Bool), ObjectState::Leaf(leaf), Value::Bool(value)) => {
            *leaf = LeafState::Live(Value::Bool(value));
        }
        (TypeKind::Scalar(ScalarType::I64), ObjectState::Leaf(leaf), Value::I64(value)) => {
            *leaf = LeafState::Live(Value::I64(value));
        }
        (TypeKind::Scalar(ScalarType::Tracked), ObjectState::Leaf(leaf), Value::Tracked(value)) => {
            *leaf = LeafState::Live(Value::Tracked(value));
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states), Value::Struct(values))
            if fields.len() == states.len() && fields.len() == values.len() =>
        {
            for ((field, field_state), field_value) in fields.iter().zip(states).zip(values) {
                write_value(types, field.ty, field_state, field_value);
            }
        }
        _ => unreachable!("validated Core MIR produced a value/state type mismatch"),
    }
}

fn clone_value(types: &TypeTable, ty: TypeId, state: &ObjectState) -> Value {
    let def = types
        .get(ty)
        .expect("validated Core MIR references an unknown type");

    match (&def.kind, state) {
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
        _ => unreachable!("fully-live validated state is structurally inconsistent"),
    }
}

fn take_value(types: &TypeTable, ty: TypeId, state: &mut ObjectState) -> Value {
    let def = types
        .get(ty)
        .expect("validated Core MIR references an unknown type");

    match (&def.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(leaf)) => {
            match std::mem::replace(leaf, LeafState::Dead) {
                LeafState::Live(value) => value,
                LeafState::NeverInitialized | LeafState::Dead => {
                    unreachable!("take_value called for state that is not fully live")
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
        _ => unreachable!("fully-live validated state is structurally inconsistent"),
    }
}

fn drop_live_values(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ObjectState,
    place: &Place,
    trace: &mut Vec<TraceEvent>,
) {
    let def = types
        .get(ty)
        .expect("validated Core MIR references an unknown type");

    match (&def.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(leaf)) => {
            let previous = std::mem::replace(leaf, LeafState::Dead);
            match previous {
                LeafState::Live(Value::Tracked(id)) => {
                    trace.push(TraceEvent::DropTracked {
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
        _ => unreachable!("validated Core MIR state shape does not match its declared type"),
    }
}
