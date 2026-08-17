#![forbid(unsafe_code)]
//! Executable reference semantics for the Runen A0 Core kernel.

use runen_core_ir::{
    BasicBlockId, Body, LocalId, Operand, Place, Projection, ScalarType, Statement, Terminator,
    TypeId, TypeKind, TypeTable, Value,
};

/// A precise location in Core MIR used by semantic diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramPoint {
    pub block: BasicBlockId,
    pub statement: Option<usize>,
}

/// Stable semantic failure categories for A0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticErrorKind {
    InvalidEntryBlock,
    InvalidTargetBlock(BasicBlockId),
    InvalidLocal(LocalId),
    InvalidProjection(Place),
    UnknownType(TypeId),
    RecursiveType(TypeId),
    InconsistentState(TypeId),
    TypeMismatch { expected: TypeId },
    InitRequiresNeverInitialized(Place),
    AssignRequiresPriorInitialization(Place),
    UseOfUninitialized(Place),
    CopyOfNonCopy(TypeId),
    AssignToImmutable(LocalId),
    DropOfUninitialized(Place),
}

/// Structured reference-machine diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticError {
    pub point: ProgramPoint,
    pub kind: SemanticErrorKind,
}

/// Why a write occurred in the semantic trace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteKind {
    Init,
    Assign,
}

/// Observable semantic trace used by A0 conformance tests.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceEvent {
    Read(Place),
    Move(Place),
    Copy(Place),
    Write {
        place: Place,
        kind: WriteKind,
    },
    /// Observable destructor event for `Tracked` values.
    DropTracked {
        place: Place,
        id: u64,
    },
}

/// Terminal status of a successful reference execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalStatus {
    Returned,
    Faulted(String),
}

/// Complete result of a successful reference execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionReport {
    pub terminal: TerminalStatus,
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
    fn uninitialized(types: &TypeTable, ty: TypeId) -> Result<Self, SemanticErrorKind> {
        Self::uninitialized_inner(types, ty, &mut Vec::new())
    }

    fn uninitialized_inner(
        types: &TypeTable,
        ty: TypeId,
        active: &mut Vec<TypeId>,
    ) -> Result<Self, SemanticErrorKind> {
        if active.contains(&ty) {
            return Err(SemanticErrorKind::RecursiveType(ty));
        }

        let Some(def) = types.get(ty) else {
            return Err(SemanticErrorKind::UnknownType(ty));
        };

        match &def.kind {
            TypeKind::Scalar(_) => Ok(Self::Leaf(LeafState::NeverInitialized)),
            TypeKind::Struct(fields) => {
                active.push(ty);
                let result = fields
                    .iter()
                    .map(|field| Self::uninitialized_inner(types, field.ty, active))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Self::Aggregate);
                active.pop();
                result
            }
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

/// Small executable abstract machine for typed A0 Core MIR.
pub struct Machine {
    body: Body,
    locals: Vec<ObjectState>,
    trace: Vec<TraceEvent>,
    point: ProgramPoint,
}

impl Machine {
    /// Constructs a reference machine. Type-shape failures are reported on execution.
    #[must_use]
    pub fn new(body: Body) -> Self {
        Self {
            point: ProgramPoint {
                block: body.entry,
                statement: None,
            },
            body,
            locals: Vec::new(),
            trace: Vec::new(),
        }
    }

    pub fn execute(mut self) -> Result<ExecutionReport, SemanticError> {
        self.initialize_locals()?;

        if self.body.block(self.body.entry).is_none() {
            return Err(self.error(SemanticErrorKind::InvalidEntryBlock));
        }

        let mut current = self.body.entry;

        loop {
            let Some(block) = self.body.block(current).cloned() else {
                self.point = ProgramPoint {
                    block: current,
                    statement: None,
                };
                return Err(self.error(SemanticErrorKind::InvalidTargetBlock(current)));
            };

            for (statement_index, statement) in block.statements.iter().enumerate() {
                self.point = ProgramPoint {
                    block: current,
                    statement: Some(statement_index),
                };
                self.execute_statement(statement)?;
            }

            self.point = ProgramPoint {
                block: current,
                statement: None,
            };

            match block.terminator {
                Terminator::Goto(target) => {
                    if self.body.block(target).is_none() {
                        return Err(self.error(SemanticErrorKind::InvalidTargetBlock(target)));
                    }
                    current = target;
                }
                Terminator::Return => {
                    self.cleanup_all_locals()?;
                    return Ok(ExecutionReport {
                        terminal: TerminalStatus::Returned,
                        trace: self.trace,
                    });
                }
                Terminator::Fault(fault) => {
                    self.cleanup_all_locals()?;
                    return Ok(ExecutionReport {
                        terminal: TerminalStatus::Faulted(fault.code),
                        trace: self.trace,
                    });
                }
            }
        }
    }

    fn initialize_locals(&mut self) -> Result<(), SemanticError> {
        let mut locals = Vec::with_capacity(self.body.locals.len());
        for local in &self.body.locals {
            let state = ObjectState::uninitialized(&self.body.types, local.ty)
                .map_err(|kind| self.error(kind))?;
            locals.push(state);
        }
        self.locals = locals;
        Ok(())
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<(), SemanticError> {
        match statement {
            Statement::Init { dst, src } => self.initialize(dst, src),
            Statement::Read { src } => self.read(src),
            Statement::Assign { dst, src } => self.assign(dst, src),
            Statement::Drop { place } => self.drop_explicit(place),
        }
    }

    fn initialize(&mut self, dst: &Place, src: &Operand) -> Result<(), SemanticError> {
        let dst_ty = self.place_type(dst)?;
        {
            let dst_state = self.place_state(dst)?;
            if !dst_state.all_never_initialized() {
                return Err(self.error(SemanticErrorKind::InitRequiresNeverInitialized(
                    dst.clone(),
                )));
            }
        }

        let value = self.evaluate_operand(src, dst_ty)?;
        let point = self.point.clone();
        let types = &self.body.types;
        let dst_state = place_state_mut(&mut self.locals, dst, &point)?;
        write_value(types, dst_ty, dst_state, value).map_err(|kind| SemanticError {
            point: point.clone(),
            kind,
        })?;
        self.trace.push(TraceEvent::Write {
            place: dst.clone(),
            kind: WriteKind::Init,
        });
        Ok(())
    }

    fn assign(&mut self, dst: &Place, src: &Operand) -> Result<(), SemanticError> {
        let local = self
            .body
            .local(dst.local)
            .ok_or_else(|| self.error(SemanticErrorKind::InvalidLocal(dst.local)))?;
        if !local.mutable {
            return Err(self.error(SemanticErrorKind::AssignToImmutable(dst.local)));
        }

        let dst_ty = self.place_type(dst)?;
        {
            let dst_state = self.place_state(dst)?;
            if dst_state.all_never_initialized() {
                return Err(self.error(SemanticErrorKind::AssignRequiresPriorInitialization(
                    dst.clone(),
                )));
            }
        }
        let value = self.evaluate_operand(src, dst_ty)?;

        self.drop_place_contents(dst, false)?;
        let point = self.point.clone();
        let types = &self.body.types;
        let dst_state = place_state_mut(&mut self.locals, dst, &point)?;
        write_value(types, dst_ty, dst_state, value).map_err(|kind| SemanticError {
            point: point.clone(),
            kind,
        })?;
        self.trace.push(TraceEvent::Write {
            place: dst.clone(),
            kind: WriteKind::Assign,
        });
        Ok(())
    }

    fn read(&mut self, src: &Place) -> Result<(), SemanticError> {
        let state = self.place_state(src)?;
        if !state.fully_live() {
            return Err(self.error(SemanticErrorKind::UseOfUninitialized(src.clone())));
        }
        self.trace.push(TraceEvent::Read(src.clone()));
        Ok(())
    }

    fn drop_explicit(&mut self, place: &Place) -> Result<(), SemanticError> {
        let state = self.place_state(place)?;
        if !state.any_live() {
            return Err(self.error(SemanticErrorKind::DropOfUninitialized(place.clone())));
        }
        self.drop_place_contents(place, false)
    }

    fn evaluate_operand(
        &mut self,
        operand: &Operand,
        expected: TypeId,
    ) -> Result<Value, SemanticError> {
        match operand {
            Operand::Constant(value) => {
                if !self.body.types.value_matches(expected, value) {
                    return Err(self.error(SemanticErrorKind::TypeMismatch { expected }));
                }
                Ok(value.clone())
            }
            Operand::Move(src) => {
                let src_ty = self.place_type(src)?;
                if src_ty != expected {
                    return Err(self.error(SemanticErrorKind::TypeMismatch { expected }));
                }
                let point = self.point.clone();
                let types = &self.body.types;
                let src_state = place_state_mut(&mut self.locals, src, &point)?;
                if !src_state.fully_live() {
                    return Err(SemanticError {
                        point,
                        kind: SemanticErrorKind::UseOfUninitialized(src.clone()),
                    });
                }
                let value = take_value(types, src_ty, src_state).map_err(|kind| SemanticError {
                    point: point.clone(),
                    kind,
                })?;
                self.trace.push(TraceEvent::Move(src.clone()));
                Ok(value)
            }
            Operand::Copy(src) => {
                let src_ty = self.place_type(src)?;
                if src_ty != expected {
                    return Err(self.error(SemanticErrorKind::TypeMismatch { expected }));
                }
                if !self.body.types.is_copy(src_ty) {
                    return Err(self.error(SemanticErrorKind::CopyOfNonCopy(src_ty)));
                }
                let src_state = self.place_state(src)?;
                if !src_state.fully_live() {
                    return Err(self.error(SemanticErrorKind::UseOfUninitialized(src.clone())));
                }
                let value = clone_value(&self.body.types, src_ty, src_state)
                    .map_err(|kind| self.error(kind))?;
                self.trace.push(TraceEvent::Copy(src.clone()));
                Ok(value)
            }
        }
    }

    fn cleanup_all_locals(&mut self) -> Result<(), SemanticError> {
        for local_index in (0..self.locals.len()).rev() {
            let local_id =
                LocalId(u32::try_from(local_index).expect("local index exceeds u32::MAX"));
            let place = Place::local(local_id);
            self.drop_place_contents(&place, true)?;
        }
        Ok(())
    }

    fn drop_place_contents(&mut self, place: &Place, cleanup: bool) -> Result<(), SemanticError> {
        let ty = self.place_type(place)?;
        let point = self.point.clone();
        let types = &self.body.types;
        let state = place_state_mut(&mut self.locals, place, &point)?;

        if !cleanup && !state.any_live() {
            return Ok(());
        }

        drop_live_values(types, ty, state, place, &mut self.trace)
            .map_err(|kind| SemanticError { point, kind })
    }

    fn place_type(&self, place: &Place) -> Result<TypeId, SemanticError> {
        let local = self
            .body
            .local(place.local)
            .ok_or_else(|| self.error(SemanticErrorKind::InvalidLocal(place.local)))?;
        self.body
            .types
            .project_type(local.ty, &place.projections)
            .ok_or_else(|| self.error(SemanticErrorKind::InvalidProjection(place.clone())))
    }

    fn place_state(&self, place: &Place) -> Result<&ObjectState, SemanticError> {
        let mut state = self
            .locals
            .get(place.local.0 as usize)
            .ok_or_else(|| self.error(SemanticErrorKind::InvalidLocal(place.local)))?;

        for projection in &place.projections {
            match (state, projection) {
                (ObjectState::Aggregate(fields), Projection::Field(index)) => {
                    state = fields.get(*index as usize).ok_or_else(|| {
                        self.error(SemanticErrorKind::InvalidProjection(place.clone()))
                    })?;
                }
                (ObjectState::Leaf(_), Projection::Field(_)) => {
                    return Err(self.error(SemanticErrorKind::InvalidProjection(place.clone())));
                }
            }
        }
        Ok(state)
    }

    fn error(&self, kind: SemanticErrorKind) -> SemanticError {
        SemanticError {
            point: self.point.clone(),
            kind,
        }
    }
}

fn place_state_mut<'a>(
    locals: &'a mut [ObjectState],
    place: &Place,
    point: &ProgramPoint,
) -> Result<&'a mut ObjectState, SemanticError> {
    let Some(mut state) = locals.get_mut(place.local.0 as usize) else {
        return Err(SemanticError {
            point: point.clone(),
            kind: SemanticErrorKind::InvalidLocal(place.local),
        });
    };

    for projection in &place.projections {
        match (state, projection) {
            (ObjectState::Aggregate(fields), Projection::Field(index)) => {
                let Some(next) = fields.get_mut(*index as usize) else {
                    return Err(SemanticError {
                        point: point.clone(),
                        kind: SemanticErrorKind::InvalidProjection(place.clone()),
                    });
                };
                state = next;
            }
            (ObjectState::Leaf(_), Projection::Field(_)) => {
                return Err(SemanticError {
                    point: point.clone(),
                    kind: SemanticErrorKind::InvalidProjection(place.clone()),
                });
            }
        }
    }
    Ok(state)
}

fn write_value(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ObjectState,
    value: Value,
) -> Result<(), SemanticErrorKind> {
    let Some(def) = types.get(ty) else {
        return Err(SemanticErrorKind::UnknownType(ty));
    };

    match (&def.kind, state, value) {
        (TypeKind::Scalar(ScalarType::Bool), ObjectState::Leaf(leaf), Value::Bool(value)) => {
            *leaf = LeafState::Live(Value::Bool(value));
            Ok(())
        }
        (TypeKind::Scalar(ScalarType::I64), ObjectState::Leaf(leaf), Value::I64(value)) => {
            *leaf = LeafState::Live(Value::I64(value));
            Ok(())
        }
        (TypeKind::Scalar(ScalarType::Tracked), ObjectState::Leaf(leaf), Value::Tracked(value)) => {
            *leaf = LeafState::Live(Value::Tracked(value));
            Ok(())
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states), Value::Struct(values))
            if fields.len() == states.len() && fields.len() == values.len() =>
        {
            for ((field, field_state), field_value) in fields.iter().zip(states).zip(values) {
                write_value(types, field.ty, field_state, field_value)?;
            }
            Ok(())
        }
        _ => Err(SemanticErrorKind::TypeMismatch { expected: ty }),
    }
}

fn clone_value(
    types: &TypeTable,
    ty: TypeId,
    state: &ObjectState,
) -> Result<Value, SemanticErrorKind> {
    let Some(def) = types.get(ty) else {
        return Err(SemanticErrorKind::UnknownType(ty));
    };

    match (&def.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(LeafState::Live(value))) => Ok(value.clone()),
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            let values = fields
                .iter()
                .zip(states)
                .map(|(field, state)| clone_value(types, field.ty, state))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Struct(values))
        }
        _ => Err(SemanticErrorKind::InconsistentState(ty)),
    }
}

fn take_value(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ObjectState,
) -> Result<Value, SemanticErrorKind> {
    let Some(def) = types.get(ty) else {
        return Err(SemanticErrorKind::UnknownType(ty));
    };

    match (&def.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(leaf)) => {
            match std::mem::replace(leaf, LeafState::Dead) {
                LeafState::Live(value) => Ok(value),
                previous @ (LeafState::NeverInitialized | LeafState::Dead) => {
                    *leaf = previous;
                    Err(SemanticErrorKind::InconsistentState(ty))
                }
            }
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            let mut values = Vec::with_capacity(fields.len());
            for (field, state) in fields.iter().zip(states) {
                values.push(take_value(types, field.ty, state)?);
            }
            Ok(Value::Struct(values))
        }
        _ => Err(SemanticErrorKind::TypeMismatch { expected: ty }),
    }
}

fn drop_live_values(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ObjectState,
    place: &Place,
    trace: &mut Vec<TraceEvent>,
) -> Result<(), SemanticErrorKind> {
    let Some(def) = types.get(ty) else {
        return Err(SemanticErrorKind::UnknownType(ty));
    };

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
            Ok(())
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            for index in (0..fields.len()).rev() {
                let field = &fields[index];
                let field_place = place.with_projection(Projection::Field(
                    u32::try_from(index).expect("field index exceeds u32::MAX"),
                ));
                drop_live_values(types, field.ty, &mut states[index], &field_place, trace)?;
            }
            Ok(())
        }
        _ => Err(SemanticErrorKind::TypeMismatch { expected: ty }),
    }
}
