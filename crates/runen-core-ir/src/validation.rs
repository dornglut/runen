use std::collections::HashSet;

use crate::{
    BasicBlockId, Body, BorrowKind, LoanId, LocalId, Operand, Place, PlaceAccess, Projection,
    Statement, Terminator, TypeId, TypeKind, TypeTable,
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
    InvalidLoan(LoanId),
    InvalidProjection(Place),
    InvalidLoanProjection {
        loan: LoanId,
        projections: Vec<Projection>,
    },
    UnknownType(TypeId),
    RecursiveType(TypeId),
    TypeMismatch {
        expected: TypeId,
    },
    CopyOfNonCopy(TypeId),
    AssignToImmutable(LocalId),
    InitRequiresNeverInitialized(Place),
    UseOfUninitialized(Place),
    DropOfUninitialized(Place),
    BorrowOfUninitialized(Place),
    LoanAlreadyActive(LoanId),
    LoanNotActive(LoanId),
    BorrowConflict {
        place: Place,
        loan: LoanId,
    },
    DirectAccessConflict {
        place: Place,
        loan: LoanId,
    },
    ExclusiveLoanRequired(LoanId),
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
/// The boundary covers all structural, typing, copyability, mutability,
/// initialization-state, root-borrow, and place-access rules expressible by the
/// currently represented Core MIR. Later language domains extend validation in
/// their owning slices rather than being anticipated here.
pub fn validate_body(body: Body) -> Result<ValidatedBody, MirValidationError> {
    validate_type_table(&body.types)?;
    validate_local_declarations(&body)?;
    validate_loan_declarations(&body)?;
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

fn validate_loan_declarations(body: &Body) -> Result<(), MirValidationError> {
    for loan in &body.loans {
        if body.types.get(loan.ty).is_none() {
            return Err(type_error(MirValidationErrorKind::UnknownType(loan.ty)));
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
        Statement::Borrow { loan, place, .. } => {
            let declaration = loan_decl(body, *loan, point)?;
            let actual = place_type(body, place, point)?;
            require_type_match(actual, declaration.ty, point)
        }
        Statement::EndBorrow { loan } => {
            loan_decl(body, *loan, point)?;
            Ok(())
        }
        Statement::Read { src } => {
            access_type(body, src, point)?;
            Ok(())
        }
        Statement::Assign { dst, src } => {
            let expected = access_type(body, dst, point)?;
            if let PlaceAccess::Direct(place) = dst {
                require_mutable_local(body, place, point)?;
            }
            validate_operand_type(body, src, expected, point)
        }
        Statement::Drop { place } => {
            access_type(body, place, point)?;
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
            let actual = access_type(body, src, point)?;
            require_type_match(actual, expected, point)
        }
        Operand::Copy(src) => {
            let actual = access_type(body, src, point)?;
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

fn require_mutable_local(
    body: &Body,
    place: &Place,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    let local = body
        .local(place.local)
        .expect("validated place references a known containing local");
    if local.mutable {
        Ok(())
    } else {
        Err(MirValidationError {
            point: Some(point.clone()),
            kind: MirValidationErrorKind::AssignToImmutable(place.local),
        })
    }
}

fn loan_decl<'a>(
    body: &'a Body,
    loan: LoanId,
    point: &MirPoint,
) -> Result<&'a crate::LoanDecl, MirValidationError> {
    body.loan(loan).ok_or_else(|| MirValidationError {
        point: Some(point.clone()),
        kind: MirValidationErrorKind::InvalidLoan(loan),
    })
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

fn access_type(
    body: &Body,
    access: &PlaceAccess,
    point: &MirPoint,
) -> Result<TypeId, MirValidationError> {
    match access {
        PlaceAccess::Direct(place) => place_type(body, place, point),
        PlaceAccess::Loan { loan, projections } => {
            let declaration = loan_decl(body, *loan, point)?;
            body.types
                .project_type(declaration.ty, projections)
                .ok_or_else(|| MirValidationError {
                    point: Some(point.clone()),
                    kind: MirValidationErrorKind::InvalidLoanProjection {
                        loan: *loan,
                        projections: projections.clone(),
                    },
                })
        }
    }
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ActiveLoan {
    kind: BorrowKind,
    place: Place,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccessMode {
    Read,
    Consume,
}

fn validate_path_state(body: &Body) -> Result<(), MirValidationError> {
    let mut locals = body
        .locals
        .iter()
        .map(|local| ObjectState::uninitialized(&body.types, local.ty))
        .collect::<Vec<_>>();
    let mut active_loans = vec![None; body.loans.len()];
    let mut current = body.entry;
    let mut seen = HashSet::new();

    loop {
        if !seen.insert((current, locals.clone(), active_loans.clone())) {
            // The current MIR has one control-flow successor per block. Reaching the
            // same block with the same abstract storage and root-loan state therefore
            // proves that future validation repeats forever without exposing a new
            // invalid state.
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
            validate_state_statement(body, &mut locals, &mut active_loans, statement, &point)?;
        }

        match &block.terminator {
            Terminator::Goto(target) => current = *target,
            Terminator::Return | Terminator::Fault(_) => return Ok(()),
        }
    }
}

fn validate_state_statement(
    body: &Body,
    locals: &mut [ObjectState],
    active_loans: &mut [Option<ActiveLoan>],
    statement: &Statement,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match statement {
        Statement::Init { dst, src } => {
            authorize_direct_access(active_loans, dst, AccessMode::Consume, point)?;
            if !place_state(locals, dst).all_never_initialized() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::InitRequiresNeverInitialized(dst.clone()),
                ));
            }
            validate_operand_state(body, locals, active_loans, src, point)?;
            place_state_mut(locals, dst).mark_live();
        }
        Statement::Borrow { loan, kind, place } => {
            let loan_index = loan.0 as usize;
            if active_loans
                .get(loan_index)
                .expect("static validation guarantees a declared loan identity")
                .is_some()
            {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::LoanAlreadyActive(*loan),
                ));
            }

            if !place_state(locals, place).fully_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::BorrowOfUninitialized(place.clone()),
                ));
            }

            if let Some(conflicting) = borrow_conflict(active_loans, place, *kind) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::BorrowConflict {
                        place: place.clone(),
                        loan: conflicting,
                    },
                ));
            }

            active_loans[loan_index] = Some(ActiveLoan {
                kind: *kind,
                place: place.clone(),
            });
        }
        Statement::EndBorrow { loan } => {
            let slot = active_loans
                .get_mut(loan.0 as usize)
                .expect("static validation guarantees a declared loan identity");
            if slot.take().is_none() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::LoanNotActive(*loan),
                ));
            }
        }
        Statement::Read { src } => {
            let place = resolve_authorized_access(active_loans, src, AccessMode::Read, point)?;
            require_fully_live(locals, &place, point)?;
        }
        Statement::Assign { dst, src } => {
            // Authorization and mutability are properties of the destination access,
            // not effects on storage. Resolve them before source evaluation while
            // preserving the accepted source-first value-state transition order.
            let place = resolve_authorized_access(active_loans, dst, AccessMode::Consume, point)?;
            require_mutable_local(body, &place, point)?;
            validate_operand_state(body, locals, active_loans, src, point)?;
            place_state_mut(locals, &place).mark_live();
        }
        Statement::Drop { place } => {
            let place = resolve_authorized_access(active_loans, place, AccessMode::Consume, point)?;
            if !place_state(locals, &place).any_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::DropOfUninitialized(place),
                ));
            }
            place_state_mut(locals, &place).drop_live();
        }
    }
    Ok(())
}

fn validate_operand_state(
    _body: &Body,
    locals: &mut [ObjectState],
    active_loans: &[Option<ActiveLoan>],
    operand: &Operand,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match operand {
        Operand::Constant(_) => Ok(()),
        Operand::Move(src) => {
            let place = resolve_authorized_access(active_loans, src, AccessMode::Consume, point)?;
            require_fully_live(locals, &place, point)?;
            place_state_mut(locals, &place).mark_dead();
            Ok(())
        }
        Operand::Copy(src) => {
            let place = resolve_authorized_access(active_loans, src, AccessMode::Read, point)?;
            require_fully_live(locals, &place, point)
        }
    }
}

fn borrow_conflict(
    active_loans: &[Option<ActiveLoan>],
    place: &Place,
    requested: BorrowKind,
) -> Option<LoanId> {
    active_loans.iter().enumerate().find_map(|(index, active)| {
        let active = active.as_ref()?;
        let conflicts = active.place.overlaps(place)
            && (requested == BorrowKind::Exclusive || active.kind == BorrowKind::Exclusive);
        conflicts.then(|| LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")))
    })
}

fn resolve_authorized_access(
    active_loans: &[Option<ActiveLoan>],
    access: &PlaceAccess,
    mode: AccessMode,
    point: &MirPoint,
) -> Result<Place, MirValidationError> {
    match access {
        PlaceAccess::Direct(place) => {
            authorize_direct_access(active_loans, place, mode, point)?;
            Ok(place.clone())
        }
        PlaceAccess::Loan { loan, projections } => {
            let active = active_loans
                .get(loan.0 as usize)
                .and_then(Option::as_ref)
                .ok_or_else(|| point_error(point, MirValidationErrorKind::LoanNotActive(*loan)))?;

            if mode == AccessMode::Consume && active.kind != BorrowKind::Exclusive {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ExclusiveLoanRequired(*loan),
                ));
            }

            let mut place = active.place.clone();
            place.projections.extend(projections.iter().copied());
            Ok(place)
        }
    }
}

fn authorize_direct_access(
    active_loans: &[Option<ActiveLoan>],
    place: &Place,
    mode: AccessMode,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if let Some((index, _)) = active_loans.iter().enumerate().find(|(_, active)| {
        active.as_ref().is_some_and(|active| {
            active.place.overlaps(place)
                && (mode == AccessMode::Consume || active.kind == BorrowKind::Exclusive)
        })
    }) {
        return Err(point_error(
            point,
            MirValidationErrorKind::DirectAccessConflict {
                place: place.clone(),
                loan: LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")),
            },
        ));
    }
    Ok(())
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
