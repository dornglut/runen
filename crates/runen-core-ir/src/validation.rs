use std::collections::HashSet;

use crate::{
    BasicBlockId, Body, LocalId, Operand, Place, Projection, Statement, Terminator, TypeId,
    TypeKind, TypeTable,
};

/// Location within Core MIR used by validation diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirPoint {
    pub block: BasicBlockId,
    pub statement: Option<usize>,
}

/// Reasons that raw Core MIR is not valid for the currently represented Core subset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirValidationErrorKind {
    InvalidEntryBlock(BasicBlockId),
    InvalidTargetBlock(BasicBlockId),
    InvalidLocal(LocalId),
    InvalidProjection(Place),
    UnknownType(TypeId),
    RecursiveType(TypeId),
    TypeMismatch { expected: TypeId },
    CopyOfNonCopy(TypeId),
    AssignToImmutable(LocalId),
    InitRequiresNeverInitialized(Place),
    UseOfUninitialized(Place),
    DropOfUninitialized(Place),
}

/// Diagnostic produced while validating raw Core MIR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirValidationError {
    pub point: Option<MirPoint>,
    pub kind: MirValidationErrorKind,
}

/// Core MIR that passed the structural and language-validity checks represented here.
///
/// The wrapper is intentionally constructible only through [`validate_body`] and
/// exposes the validated body read-only so execution cannot invalidate the proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedBody {
    body: Body,
}

impl ValidatedBody {
    #[must_use]
    pub fn as_body(&self) -> &Body {
        &self.body
    }
}

/// Validates raw Core MIR before it may enter the reference machine.
///
/// The boundary covers all structural, typing, copyability, mutability, and
/// initialization-state rules expressible by the currently represented A0 MIR.
/// Later language domains extend validation in their owning slices rather than
/// being anticipated here.
pub fn validate_body(body: Body) -> Result<ValidatedBody, MirValidationError> {
    validate_type_table(&body.types)?;
    validate_local_declarations(&body)?;
    validate_structure_and_static_rules(&body)?;
    validate_path_state(&body)?;
    Ok(ValidatedBody { body })
}

fn validate_type_table(types: &TypeTable) -> Result<(), MirValidationError> {
    // 0 = unseen, 1 = active DFS ancestor, 2 = fully validated.
    let mut marks = vec![0_u8; types.len()];

    for root_index in 0..types.len() {
        if marks[root_index] != 0 {
            continue;
        }

        let root = TypeId(u32::try_from(root_index).expect("type index exceeds u32::MAX"));
        marks[root_index] = 1;
        let mut stack = vec![(root, 0_usize)];

        while let Some((current, next_field)) = stack.pop() {
            let current_index = current.0 as usize;
            let definition = types
                .get(current)
                .expect("type-validation stack contains a known table index");

            match &definition.kind {
                TypeKind::Scalar(_) => {
                    marks[current_index] = 2;
                }
                TypeKind::Struct(fields) if next_field == fields.len() => {
                    marks[current_index] = 2;
                }
                TypeKind::Struct(fields) => {
                    stack.push((current, next_field + 1));
                    let child = fields[next_field].ty;
                    let child_index = child.0 as usize;

                    if child_index >= types.len() {
                        return Err(type_error(MirValidationErrorKind::UnknownType(child)));
                    }

                    match marks[child_index] {
                        0 => {
                            marks[child_index] = 1;
                            stack.push((child, 0));
                        }
                        1 => {
                            return Err(type_error(MirValidationErrorKind::RecursiveType(child)));
                        }
                        2 => {}
                        _ => unreachable!("type-validation mark outside declared state space"),
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_local_declarations(body: &Body) -> Result<(), MirValidationError> {
    for local in &body.locals {
        if body.types.get(local.ty).is_none() {
            return Err(type_error(MirValidationErrorKind::UnknownType(local.ty)));
        }
    }
    Ok(())
}

fn validate_structure_and_static_rules(body: &Body) -> Result<(), MirValidationError> {
    if body.block(body.entry).is_none() {
        return Err(MirValidationError {
            point: None,
            kind: MirValidationErrorKind::InvalidEntryBlock(body.entry),
        });
    }

    for (block_index, block) in body.blocks.iter().enumerate() {
        let block_id = block_id(block_index);

        for (statement_index, statement) in block.statements.iter().enumerate() {
            let point = MirPoint {
                block: block_id,
                statement: Some(statement_index),
            };
            validate_static_statement(body, statement, &point)?;
        }

        if let Terminator::Goto(target) = &block.terminator
            && body.block(*target).is_none()
        {
            return Err(MirValidationError {
                point: Some(MirPoint {
                    block: block_id,
                    statement: None,
                }),
                kind: MirValidationErrorKind::InvalidTargetBlock(*target),
            });
        }
    }

    Ok(())
}

fn validate_static_statement(
    body: &Body,
    statement: &Statement,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match statement {
        Statement::Init { dst, src } => {
            let expected = place_type(body, dst, point)?;
            validate_operand_type(body, src, expected, point)
        }
        Statement::Read { src } => {
            place_type(body, src, point)?;
            Ok(())
        }
        Statement::Assign { dst, src } => {
            let expected = place_type(body, dst, point)?;
            let local = body
                .local(dst.local)
                .expect("validated place references a known containing local");
            if !local.mutable {
                return Err(MirValidationError {
                    point: Some(point.clone()),
                    kind: MirValidationErrorKind::AssignToImmutable(dst.local),
                });
            }
            validate_operand_type(body, src, expected, point)
        }
        Statement::Drop { place } => {
            place_type(body, place, point)?;
            Ok(())
        }
    }
}

fn validate_operand_type(
    body: &Body,
    operand: &Operand,
    expected: TypeId,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match operand {
        Operand::Constant(value) => {
            if body.types.value_matches(expected, value) {
                Ok(())
            } else {
                Err(MirValidationError {
                    point: Some(point.clone()),
                    kind: MirValidationErrorKind::TypeMismatch { expected },
                })
            }
        }
        Operand::Move(src) => {
            let actual = place_type(body, src, point)?;
            require_type_match(actual, expected, point)
        }
        Operand::Copy(src) => {
            let actual = place_type(body, src, point)?;
            require_type_match(actual, expected, point)?;
            if !body.types.is_copy(actual) {
                return Err(MirValidationError {
                    point: Some(point.clone()),
                    kind: MirValidationErrorKind::CopyOfNonCopy(actual),
                });
            }
            Ok(())
        }
    }
}

fn require_type_match(
    actual: TypeId,
    expected: TypeId,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if actual == expected {
        Ok(())
    } else {
        Err(MirValidationError {
            point: Some(point.clone()),
            kind: MirValidationErrorKind::TypeMismatch { expected },
        })
    }
}

fn place_type(body: &Body, place: &Place, point: &MirPoint) -> Result<TypeId, MirValidationError> {
    let local = body.local(place.local).ok_or_else(|| MirValidationError {
        point: Some(point.clone()),
        kind: MirValidationErrorKind::InvalidLocal(place.local),
    })?;

    body.types
        .project_type(local.ty, &place.projections)
        .ok_or_else(|| MirValidationError {
            point: Some(point.clone()),
            kind: MirValidationErrorKind::InvalidProjection(place.clone()),
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LeafState {
    NeverInitialized,
    Live,
    Dead,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ObjectState {
    Leaf(LeafState),
    Aggregate(Vec<ObjectState>),
}

impl ObjectState {
    fn uninitialized(types: &TypeTable, ty: TypeId) -> Self {
        let definition = types
            .get(ty)
            .expect("validated type graph references only known types");
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

    fn all_never_initialized(&self) -> bool {
        match self {
            Self::Leaf(LeafState::NeverInitialized) => true,
            Self::Leaf(LeafState::Live | LeafState::Dead) => false,
            Self::Aggregate(fields) => fields.iter().all(Self::all_never_initialized),
        }
    }

    fn fully_live(&self) -> bool {
        match self {
            Self::Leaf(LeafState::Live) => true,
            Self::Leaf(LeafState::NeverInitialized | LeafState::Dead) => false,
            Self::Aggregate(fields) => fields.iter().all(Self::fully_live),
        }
    }

    fn any_live(&self) -> bool {
        match self {
            Self::Leaf(LeafState::Live) => true,
            Self::Leaf(LeafState::NeverInitialized | LeafState::Dead) => false,
            Self::Aggregate(fields) => fields.iter().any(Self::any_live),
        }
    }

    fn mark_live(&mut self) {
        match self {
            Self::Leaf(state) => *state = LeafState::Live,
            Self::Aggregate(fields) => fields.iter_mut().for_each(Self::mark_live),
        }
    }

    fn mark_dead(&mut self) {
        match self {
            Self::Leaf(state) => *state = LeafState::Dead,
            Self::Aggregate(fields) => fields.iter_mut().for_each(Self::mark_dead),
        }
    }

    fn drop_live(&mut self) {
        match self {
            Self::Leaf(state) => {
                if *state == LeafState::Live {
                    *state = LeafState::Dead;
                }
            }
            Self::Aggregate(fields) => fields.iter_mut().for_each(Self::drop_live),
        }
    }
}

fn validate_path_state(body: &Body) -> Result<(), MirValidationError> {
    let mut locals = body
        .locals
        .iter()
        .map(|local| ObjectState::uninitialized(&body.types, local.ty))
        .collect::<Vec<_>>();
    let mut current = body.entry;
    let mut seen = HashSet::new();

    loop {
        if !seen.insert((current, locals.clone())) {
            // The current MIR has one control-flow successor per block. Reaching the
            // same block with the same abstract storage state therefore proves that
            // future validation repeats forever without exposing a new invalid state.
            return Ok(());
        }

        let block = body
            .block(current)
            .expect("structurally validated Core MIR reaches only known blocks");

        for (statement_index, statement) in block.statements.iter().enumerate() {
            let point = MirPoint {
                block: current,
                statement: Some(statement_index),
            };
            validate_state_statement(&mut locals, statement, &point)?;
        }

        match &block.terminator {
            Terminator::Goto(target) => current = *target,
            Terminator::Return | Terminator::Fault(_) => return Ok(()),
        }
    }
}

fn validate_state_statement(
    locals: &mut [ObjectState],
    statement: &Statement,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match statement {
        Statement::Init { dst, src } => {
            if !place_state(locals, dst).all_never_initialized() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::InitRequiresNeverInitialized(dst.clone()),
                ));
            }
            validate_operand_state(locals, src, point)?;
            place_state_mut(locals, dst).mark_live();
        }
        Statement::Read { src } => {
            require_fully_live(locals, src, point)?;
        }
        Statement::Assign { dst, src } => {
            validate_operand_state(locals, src, point)?;
            place_state_mut(locals, dst).mark_live();
        }
        Statement::Drop { place } => {
            if !place_state(locals, place).any_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::DropOfUninitialized(place.clone()),
                ));
            }
            place_state_mut(locals, place).drop_live();
        }
    }
    Ok(())
}

fn validate_operand_state(
    locals: &mut [ObjectState],
    operand: &Operand,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match operand {
        Operand::Constant(_) => Ok(()),
        Operand::Move(src) => {
            require_fully_live(locals, src, point)?;
            place_state_mut(locals, src).mark_dead();
            Ok(())
        }
        Operand::Copy(src) => require_fully_live(locals, src, point),
    }
}

fn require_fully_live(
    locals: &[ObjectState],
    place: &Place,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if place_state(locals, place).fully_live() {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::UseOfUninitialized(place.clone()),
        ))
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
                unreachable!("structural validation rejects projections from scalar state");
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
                unreachable!("structural validation rejects projections from scalar state");
            }
        }
    }
    state
}

fn block_id(index: usize) -> BasicBlockId {
    BasicBlockId(u32::try_from(index).expect("basic block index exceeds u32::MAX"))
}

fn point_error(point: &MirPoint, kind: MirValidationErrorKind) -> MirValidationError {
    MirValidationError {
        point: Some(point.clone()),
        kind,
    }
}

fn type_error(kind: MirValidationErrorKind) -> MirValidationError {
    MirValidationError { point: None, kind }
}
