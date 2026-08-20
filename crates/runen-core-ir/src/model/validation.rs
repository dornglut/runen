use std::collections::HashSet;

use crate::{
    BasicBlockId, Body, BorrowKind, LoanId, LocalId, Operand, Place, PlaceAccess, Projection,
    ScalarType, Statement, Terminator, TypeId, TypeKind, TypeTable, Value,
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
    RawReadRequiresPointer(TypeId),
    RawMoveRequiresPointer(TypeId),
    RawAssignRequiresPointer(TypeId),
    AssignToImmutable(LocalId),
    InteriorMutationRequiresMarkedRegion(Place),
    InitRequiresNeverInitialized(Place),
    UseOfUninitialized(Place),
    DropOfUninitialized(Place),
    BorrowOfUninitialized(Place),
    LoanAlreadyActive(LoanId),
    LoanNotActive(LoanId),
    LoanHasActiveChild {
        loan: LoanId,
        child: LoanId,
    },
    BorrowConflict {
        place: Place,
        loan: LoanId,
    },
    DirectAccessConflict {
        place: Place,
        loan: LoanId,
    },
    LoanAccessDelegated {
        loan: LoanId,
        child: LoanId,
        place: Place,
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
/// The boundary covers all structural, typing, copyability, assignment-mutability,
/// interior-mutability, initialization-state, borrow-tree, and place-access rules
/// expressible by the currently represented Core MIR. Unsafe raw-target access
/// obligations remain execution preconditions rather than validation diagnostics.
/// Validation carries exact symbolic raw-pointer targets only as verification state
/// needed to propagate defined raw target state effects; that state is not an
/// independent provenance model or observable language state. When exact verification
/// state proves that a raw unsafe precondition already fails, path-state propagation
/// stops rather than fabricating post-UB state; static MIR validation still covers the
/// complete body. Later language domains extend validation in their owning slices
/// rather than being anticipated here.
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
                TypeKind::Scalar(ScalarType::RawPointer(pointee)) => {
                    if types.get(*pointee).is_none() {
                        return Err(type_error(MirValidationErrorKind::UnknownType(*pointee)));
                    }
                    // Raw-pointer pointee edges are semantic indirection rather than
                    // structural containment, so they do not participate in the
                    // direct-recursion DFS.
                    marks[current_index] = 2;
                }
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
        Statement::Borrow { loan, src, .. } => {
            let declaration = loan_decl(body, *loan, point)?;
            let actual = access_type(body, src, point)?;
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
        Statement::RawRead { pointer } => {
            let actual = access_type(body, pointer, point)?;
            if body.types.raw_pointer_pointee(actual).is_some() {
                Ok(())
            } else {
                Err(point_error(
                    point,
                    MirValidationErrorKind::RawReadRequiresPointer(actual),
                ))
            }
        }
        Statement::RawAssign { pointer, src } => {
            let actual = access_type(body, pointer, point)?;
            let Some(pointee) = body.types.raw_pointer_pointee(actual) else {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::RawAssignRequiresPointer(actual),
                ));
            };
            validate_operand_type(body, src, pointee, point)
        }
        Statement::Assign { dst, src } => {
            let expected = access_type(body, dst, point)?;
            if let PlaceAccess::Direct(place) = dst {
                require_mutable_local(body, place, point)?;
            }
            validate_operand_type(body, src, expected, point)
        }
        Statement::InteriorAssign { dst, src } => {
            let expected = access_type(body, dst, point)?;
            if let PlaceAccess::Direct(place) = dst {
                require_interior_mutable_place(body, place, point)?;
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
        Operand::RawMove(pointer) => {
            let pointer_ty = access_type(body, pointer, point)?;
            let Some(actual) = body.types.raw_pointer_pointee(pointer_ty) else {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::RawMoveRequiresPointer(pointer_ty),
                ));
            };
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
        Operand::AddressOf(src) => {
            let actual = access_type(body, src, point)?;
            if body.types.raw_pointer_pointee(expected) == Some(actual) {
                Ok(())
            } else {
                Err(MirValidationError {
                    point: Some(point.clone()),
                    kind: MirValidationErrorKind::TypeMismatch { expected },
                })
            }
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

fn require_interior_mutable_place(
    body: &Body,
    place: &Place,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if is_interior_mutable_place(body, place) {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::InteriorMutationRequiresMarkedRegion(place.clone()),
        ))
    }
}

fn is_interior_mutable_place(body: &Body, place: &Place) -> bool {
    let local = body
        .local(place.local)
        .expect("validated place references a known containing local");
    let mut current = local.ty;

    if body
        .types
        .get(current)
        .expect("validated local type is known")
        .interior_mutable
    {
        return true;
    }

    for projection in &place.projections {
        match projection {
            Projection::Field(index) => {
                let TypeKind::Struct(fields) = &body
                    .types
                    .get(current)
                    .expect("validated projected type is known")
                    .kind
                else {
                    unreachable!("structural validation rejects field projection from scalar type");
                };
                current = fields[*index as usize].ty;
                if body
                    .types
                    .get(current)
                    .expect("validated field type is known")
                    .interior_mutable
                {
                    return true;
                }
            }
        }
    }

    false
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ValidationScalar {
    NonPointer,
    RawPointer(Place),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ValidationValue {
    Scalar(ValidationScalar),
    Struct(Vec<ValidationValue>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DefinedStep<T> {
    Continue(T),
    NoDefinedContinuation,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum LeafState {
    NeverInitialized,
    Live(ValidationScalar),
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

    fn drop_live(&mut self) {
        match self {
            Self::Leaf(state) => {
                if matches!(state, LeafState::Live(_)) {
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
    parent: Option<LoanId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AccessRequirement {
    Shared,
    Exclusive,
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
            // same block with the same abstract storage, exact raw-pointer targets,
            // and active-loan forest state proves that future validation repeats
            // forever without exposing a new invalid state.
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
            if matches!(
                validate_state_statement(body, &mut locals, &mut active_loans, statement, &point)?,
                DefinedStep::NoDefinedContinuation
            ) {
                // Static validation already covered the complete MIR. Path-state
                // propagation is only about defined executions, so a statically evident
                // unsafe violation has no post-state to validate.
                return Ok(());
            }
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
) -> Result<DefinedStep<()>, MirValidationError> {
    match statement {
        Statement::Init { dst, src } => {
            authorize_direct_access(active_loans, dst, AccessRequirement::Exclusive, point)?;
            if !place_state(locals, dst).all_never_initialized() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::InitRequiresNeverInitialized(dst.clone()),
                ));
            }
            let dst_ty = place_type(body, dst, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(body, locals, active_loans, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            write_validation_value(&body.types, dst_ty, place_state_mut(locals, dst), value);
        }
        Statement::Borrow { loan, kind, src } => {
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

            let (place, parent) = match src {
                PlaceAccess::Direct(place) => {
                    if let Some(conflicting) = borrow_conflict(active_loans, place, *kind) {
                        return Err(point_error(
                            point,
                            MirValidationErrorKind::BorrowConflict {
                                place: place.clone(),
                                loan: conflicting,
                            },
                        ));
                    }
                    (place.clone(), None)
                }
                PlaceAccess::Loan { loan: parent, .. } => {
                    let requirement = match kind {
                        BorrowKind::Shared => AccessRequirement::Shared,
                        BorrowKind::Exclusive => AccessRequirement::Exclusive,
                    };
                    (
                        resolve_authorized_access(active_loans, src, requirement, point)?,
                        Some(*parent),
                    )
                }
            };

            if !place_state(locals, &place).fully_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::BorrowOfUninitialized(place),
                ));
            }

            active_loans[loan_index] = Some(ActiveLoan {
                kind: *kind,
                place,
                parent,
            });
        }
        Statement::EndBorrow { loan } => {
            if active_loans
                .get(loan.0 as usize)
                .expect("static validation guarantees a declared loan identity")
                .is_none()
            {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::LoanNotActive(*loan),
                ));
            }

            if let Some(child) = active_child(active_loans, *loan) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::LoanHasActiveChild { loan: *loan, child },
                ));
            }

            active_loans[loan.0 as usize] = None;
        }
        Statement::Read { src } => {
            let place =
                resolve_authorized_access(active_loans, src, AccessRequirement::Shared, point)?;
            require_fully_live(locals, &place, point)?;
        }
        Statement::RawRead { pointer } => {
            let pointer_place =
                resolve_authorized_access(active_loans, pointer, AccessRequirement::Shared, point)?;
            require_fully_live(locals, &pointer_place, point)?;
            let target = raw_pointer_target_at(locals, &pointer_place);
            if !place_state(locals, &target).fully_live()
                || raw_target_conflicts(active_loans, &target, AccessRequirement::Shared)
            {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::RawAssign { pointer, src } => {
            // Snapshot the exact target before source evaluation, matching the
            // normative RawAssign ordering.
            let pointer_place =
                resolve_authorized_access(active_loans, pointer, AccessRequirement::Shared, point)?;
            require_fully_live(locals, &pointer_place, point)?;
            let target = raw_pointer_target_at(locals, &pointer_place);
            let pointer_ty = place_type(body, &pointer_place, point)?;
            let pointee_ty = body
                .types
                .raw_pointer_pointee(pointer_ty)
                .expect("static validation guarantees a raw-pointer RawAssign operand");
            let DefinedStep::Continue(value) =
                validate_operand_state(body, locals, active_loans, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            if raw_target_conflicts(active_loans, &target, AccessRequirement::Exclusive) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
            write_validation_value(
                &body.types,
                pointee_ty,
                place_state_mut(locals, &target),
                value,
            );
        }
        Statement::Assign { dst, src } => {
            // Authorization and assignment mutability are properties of the destination
            // access, not effects on storage. Resolve them before source evaluation while
            // preserving the accepted source-first value-state transition order.
            let place =
                resolve_authorized_access(active_loans, dst, AccessRequirement::Exclusive, point)?;
            require_mutable_local(body, &place, point)?;
            let dst_ty = place_type(body, &place, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(body, locals, active_loans, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            write_validation_value(&body.types, dst_ty, place_state_mut(locals, &place), value);
        }
        Statement::InteriorAssign { dst, src } => {
            // Interior mutability is independent of local assignment mutability. Shared
            // alias authority is sufficient only when the concrete destination lies
            // within an explicitly marked interior-mutable region.
            let place =
                resolve_authorized_access(active_loans, dst, AccessRequirement::Shared, point)?;
            require_interior_mutable_place(body, &place, point)?;
            let dst_ty = place_type(body, &place, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(body, locals, active_loans, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            write_validation_value(&body.types, dst_ty, place_state_mut(locals, &place), value);
        }
        Statement::Drop { place } => {
            let place = resolve_authorized_access(
                active_loans,
                place,
                AccessRequirement::Exclusive,
                point,
            )?;
            if !place_state(locals, &place).any_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::DropOfUninitialized(place),
                ));
            }
            place_state_mut(locals, &place).drop_live();
        }
    }
    Ok(DefinedStep::Continue(()))
}

fn validate_operand_state(
    body: &Body,
    locals: &mut [ObjectState],
    active_loans: &[Option<ActiveLoan>],
    operand: &Operand,
    point: &MirPoint,
) -> Result<DefinedStep<ValidationValue>, MirValidationError> {
    match operand {
        Operand::Constant(value) => {
            Ok(DefinedStep::Continue(validation_value_from_constant(value)))
        }
        Operand::Move(src) => {
            let place =
                resolve_authorized_access(active_loans, src, AccessRequirement::Exclusive, point)?;
            require_fully_live(locals, &place, point)?;
            let ty = place_type(body, &place, point)?;
            Ok(DefinedStep::Continue(take_validation_value(
                &body.types,
                ty,
                place_state_mut(locals, &place),
            )))
        }
        Operand::RawMove(pointer) => {
            // The pointer value itself is language-validated like other raw operations.
            // Target liveness and alias compatibility remain unsafe preconditions. If
            // exact verification state proves either fails, there is no defined value
            // to fabricate for the enclosing statement.
            let pointer_place =
                resolve_authorized_access(active_loans, pointer, AccessRequirement::Shared, point)?;
            require_fully_live(locals, &pointer_place, point)?;
            let target = raw_pointer_target_at(locals, &pointer_place);
            if !place_state(locals, &target).fully_live()
                || raw_target_conflicts(active_loans, &target, AccessRequirement::Exclusive)
            {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
            let pointer_ty = place_type(body, &pointer_place, point)?;
            let pointee_ty = body
                .types
                .raw_pointer_pointee(pointer_ty)
                .expect("static validation guarantees a raw-pointer RawMove operand");
            Ok(DefinedStep::Continue(take_validation_value(
                &body.types,
                pointee_ty,
                place_state_mut(locals, &target),
            )))
        }
        Operand::Copy(src) => {
            let place =
                resolve_authorized_access(active_loans, src, AccessRequirement::Shared, point)?;
            require_fully_live(locals, &place, point)?;
            let ty = place_type(body, &place, point)?;
            Ok(DefinedStep::Continue(clone_validation_value(
                &body.types,
                ty,
                place_state(locals, &place),
            )))
        }
        Operand::AddressOf(src) => {
            // Address formation authorizes access to existing storage but does not
            // read the pointee value, so NeverInitialized and Dead storage are valid
            // targets. The resolved Place is exact verification metadata for transport
            // and later raw state propagation, not a second provenance identity.
            let place =
                resolve_authorized_access(active_loans, src, AccessRequirement::Shared, point)?;
            Ok(DefinedStep::Continue(ValidationValue::Scalar(
                ValidationScalar::RawPointer(place),
            )))
        }
    }
}

fn validation_value_from_constant(value: &Value) -> ValidationValue {
    match value {
        Value::Bool(_) | Value::I64(_) | Value::TrackedFixture(_) => {
            ValidationValue::Scalar(ValidationScalar::NonPointer)
        }
        Value::Struct(values) => {
            ValidationValue::Struct(values.iter().map(validation_value_from_constant).collect())
        }
    }
}

fn clone_validation_value(types: &TypeTable, ty: TypeId, state: &ObjectState) -> ValidationValue {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(LeafState::Live(value))) => {
            ValidationValue::Scalar(value.clone())
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            ValidationValue::Struct(
                fields
                    .iter()
                    .zip(states)
                    .map(|(field, state)| clone_validation_value(types, field.ty, state))
                    .collect(),
            )
        }
        _ => unreachable!("validated Core MIR guarantees copied state is fully live"),
    }
}

fn take_validation_value(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ObjectState,
) -> ValidationValue {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(leaf)) => {
            match std::mem::replace(leaf, LeafState::Dead) {
                LeafState::Live(value) => ValidationValue::Scalar(value),
                LeafState::NeverInitialized | LeafState::Dead => {
                    unreachable!("validated Core MIR guarantees moved state is fully live")
                }
            }
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            ValidationValue::Struct(
                fields
                    .iter()
                    .zip(states)
                    .map(|(field, state)| take_validation_value(types, field.ty, state))
                    .collect(),
            )
        }
        _ => unreachable!("validated Core MIR guarantees moved state matches its type"),
    }
}

fn write_validation_value(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ObjectState,
    value: ValidationValue,
) {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state, value) {
        (
            TypeKind::Scalar(ScalarType::RawPointer(_)),
            ObjectState::Leaf(leaf),
            ValidationValue::Scalar(value @ ValidationScalar::RawPointer(_)),
        ) => {
            *leaf = LeafState::Live(value);
        }
        (
            TypeKind::Scalar(ScalarType::Bool | ScalarType::I64 | ScalarType::TrackedFixture),
            ObjectState::Leaf(leaf),
            ValidationValue::Scalar(ValidationScalar::NonPointer),
        ) => {
            *leaf = LeafState::Live(ValidationScalar::NonPointer);
        }
        (
            TypeKind::Struct(fields),
            ObjectState::Aggregate(states),
            ValidationValue::Struct(values),
        ) if fields.len() == states.len() && fields.len() == values.len() => {
            for ((field, field_state), field_value) in fields.iter().zip(states).zip(values) {
                write_validation_value(types, field.ty, field_state, field_value);
            }
        }
        _ => unreachable!("static validation guarantees value/type compatibility"),
    }
}

fn raw_pointer_target_at(locals: &[ObjectState], place: &Place) -> Place {
    match place_state(locals, place) {
        ObjectState::Leaf(LeafState::Live(ValidationScalar::RawPointer(target))) => target.clone(),
        _ => unreachable!("validated raw-pointer access reaches exact live pointer metadata"),
    }
}

fn raw_target_conflicts(
    active_loans: &[Option<ActiveLoan>],
    place: &Place,
    requirement: AccessRequirement,
) -> bool {
    active_loans.iter().any(|active| {
        active.as_ref().is_some_and(|active| {
            active.place.overlaps(place)
                && (requirement == AccessRequirement::Exclusive
                    || active.kind == BorrowKind::Exclusive)
        })
    })
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

fn active_child(active_loans: &[Option<ActiveLoan>], parent: LoanId) -> Option<LoanId> {
    active_loans.iter().enumerate().find_map(|(index, active)| {
        (active.as_ref()?.parent == Some(parent))
            .then(|| LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")))
    })
}

fn delegated_child(
    active_loans: &[Option<ActiveLoan>],
    parent: LoanId,
    place: &Place,
    requirement: AccessRequirement,
) -> Option<LoanId> {
    active_loans.iter().enumerate().find_map(|(index, active)| {
        let active = active.as_ref()?;
        let blocks = active.parent == Some(parent)
            && active.place.overlaps(place)
            && (requirement == AccessRequirement::Exclusive
                || active.kind == BorrowKind::Exclusive);
        blocks.then(|| LoanId(u32::try_from(index).expect("loan index exceeds u32::MAX")))
    })
}

fn resolve_authorized_access(
    active_loans: &[Option<ActiveLoan>],
    access: &PlaceAccess,
    requirement: AccessRequirement,
    point: &MirPoint,
) -> Result<Place, MirValidationError> {
    match access {
        PlaceAccess::Direct(place) => {
            authorize_direct_access(active_loans, place, requirement, point)?;
            Ok(place.clone())
        }
        PlaceAccess::Loan { loan, projections } => {
            let active = active_loans
                .get(loan.0 as usize)
                .and_then(Option::as_ref)
                .ok_or_else(|| point_error(point, MirValidationErrorKind::LoanNotActive(*loan)))?;

            if requirement == AccessRequirement::Exclusive && active.kind != BorrowKind::Exclusive {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ExclusiveLoanRequired(*loan),
                ));
            }

            let mut place = active.place.clone();
            place.projections.extend(projections.iter().copied());

            if let Some(child) = delegated_child(active_loans, *loan, &place, requirement) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::LoanAccessDelegated {
                        loan: *loan,
                        child,
                        place,
                    },
                ));
            }

            Ok(place)
        }
    }
}

fn authorize_direct_access(
    active_loans: &[Option<ActiveLoan>],
    place: &Place,
    requirement: AccessRequirement,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if let Some((index, _)) = active_loans.iter().enumerate().find(|(_, active)| {
        active.as_ref().is_some_and(|active| {
            active.place.overlaps(place)
                && (requirement == AccessRequirement::Exclusive
                    || active.kind == BorrowKind::Exclusive)
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
