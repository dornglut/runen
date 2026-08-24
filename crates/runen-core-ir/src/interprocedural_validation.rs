use std::collections::{HashSet, VecDeque};

use crate::interprocedural::{Body, Function, Program, Terminator};
use crate::{
    BasicBlockId, BorrowKind, FunctionId, LoanDecl, LoanId, LocalId, Operand, Place, PlaceAccess,
    Projection, ScalarType, Statement, TypeId, TypeKind, TypeTable, Value,
};

/// Function-scoped location within program-level Core MIR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirPoint {
    pub function: FunctionId,
    pub block: BasicBlockId,
    pub statement: Option<usize>,
}

/// Unambiguous location for a program-validation diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirLocation {
    Program,
    Function(FunctionId),
    Point(MirPoint),
}

/// Reasons program-level Core MIR is not valid for the represented subset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirValidationErrorKind {
    InvalidFunction(FunctionId),
    InvalidEntryBlock(BasicBlockId),
    InvalidTargetBlock(BasicBlockId),
    InvalidLocal(LocalId),
    DuplicateParameter(LocalId),
    InvalidLoan(LoanId),
    InvalidProjection(Place),
    InvalidLoanProjection {
        loan: LoanId,
        projections: Vec<Projection>,
    },
    UnknownType(TypeId),
    RecursiveType(TypeId),
    CallTransferUnsafe(TypeId),
    ArgumentCount {
        expected: usize,
        found: usize,
    },
    MissingResultDestination,
    UnexpectedResultDestination,
    MissingReturnValue,
    UnexpectedReturnValue,
    TypeMismatch {
        expected: TypeId,
    },
    BranchConditionNotBool,
    CopyOfNonCopy(TypeId),
    RawReadRequiresPointer(TypeId),
    RawMoveRequiresPointer(TypeId),
    RawAssignRequiresPointer(TypeId),
    IntegerAddRequiresInteger(TypeId),
    IntegerSubRequiresInteger(TypeId),
    IntegerMulRequiresInteger(TypeId),
    AssignToImmutable(LocalId),
    InteriorMutationRequiresMarkedRegion(Place),
    InitRequiresVacant(Place),
    IntegerAddRequiresVacant(Place),
    IntegerSubRequiresVacant(Place),
    IntegerMulRequiresVacant(Place),
    CallResultRequiresVacant(Place),
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

/// Diagnostic produced while validating one raw Core program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirValidationError {
    pub location: MirLocation,
    pub kind: MirValidationErrorKind,
}

/// Core Program that passed represented structural and language-validity checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedProgram {
    program: Program,
}

impl ValidatedProgram {
    #[must_use]
    pub fn as_program(&self) -> &Program {
        &self.program
    }
}

/// Validate one complete represented Core program.
///
/// Function bodies are validated independently. Call sites consume only target
/// callable structure; call-graph cycles are never recursively expanded or rejected.
pub fn validate_program(program: Program) -> Result<ValidatedProgram, MirValidationError> {
    validate_type_table(&program.types)?;

    for (index, function) in program.functions.iter().enumerate() {
        let function_id = function_id(index);
        validate_function_declarations(&program.types, function_id, function)?;
        validate_structure_and_static_rules(&program, function_id, function)?;
    }

    for (index, function) in program.functions.iter().enumerate() {
        validate_path_state(&program, function_id(index), function)?;
    }

    Ok(ValidatedProgram { program })
}

fn validate_type_table(types: &TypeTable) -> Result<(), MirValidationError> {
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
                        return Err(program_error(MirValidationErrorKind::UnknownType(*pointee)));
                    }
                    marks[current_index] = 2;
                }
                TypeKind::Scalar(_) => marks[current_index] = 2,
                TypeKind::Struct(fields) if next_field == fields.len() => {
                    marks[current_index] = 2;
                }
                TypeKind::Struct(fields) => {
                    stack.push((current, next_field + 1));
                    let child = fields[next_field].ty;
                    let child_index = child.0 as usize;
                    if child_index >= types.len() {
                        return Err(program_error(MirValidationErrorKind::UnknownType(child)));
                    }
                    match marks[child_index] {
                        0 => {
                            marks[child_index] = 1;
                            stack.push((child, 0));
                        }
                        1 => {
                            return Err(program_error(MirValidationErrorKind::RecursiveType(
                                child,
                            )));
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

fn validate_function_declarations(
    types: &TypeTable,
    function_id: FunctionId,
    function: &Function,
) -> Result<(), MirValidationError> {
    for local in &function.body.locals {
        if types.get(local.ty).is_none() {
            return Err(function_error(
                function_id,
                MirValidationErrorKind::UnknownType(local.ty),
            ));
        }
    }

    for loan in &function.body.loans {
        if types.get(loan.ty).is_none() {
            return Err(function_error(
                function_id,
                MirValidationErrorKind::UnknownType(loan.ty),
            ));
        }
    }

    if let Some(result) = function.result {
        require_known_call_type(types, function_id, result)?;
    }

    let mut parameters = HashSet::new();
    for parameter in &function.parameters {
        if !parameters.insert(*parameter) {
            return Err(function_error(
                function_id,
                MirValidationErrorKind::DuplicateParameter(*parameter),
            ));
        }
        let local = function.body.local(*parameter).ok_or_else(|| {
            function_error(
                function_id,
                MirValidationErrorKind::InvalidLocal(*parameter),
            )
        })?;
        require_known_call_type(types, function_id, local.ty)?;
    }

    Ok(())
}

fn require_known_call_type(
    types: &TypeTable,
    function: FunctionId,
    ty: TypeId,
) -> Result<(), MirValidationError> {
    if types.get(ty).is_none() {
        return Err(function_error(
            function,
            MirValidationErrorKind::UnknownType(ty),
        ));
    }
    if !types.is_call_transfer_safe(ty) {
        return Err(function_error(
            function,
            MirValidationErrorKind::CallTransferUnsafe(ty),
        ));
    }
    Ok(())
}

fn validate_structure_and_static_rules(
    program: &Program,
    function_id: FunctionId,
    function: &Function,
) -> Result<(), MirValidationError> {
    let body = &function.body;
    if body.block(body.entry).is_none() {
        return Err(function_error(
            function_id,
            MirValidationErrorKind::InvalidEntryBlock(body.entry),
        ));
    }

    for (block_index, block) in body.blocks.iter().enumerate() {
        let block_id = block_id(block_index);
        for (statement_index, statement) in block.statements.iter().enumerate() {
            let point = MirPoint {
                function: function_id,
                block: block_id,
                statement: Some(statement_index),
            };
            validate_static_statement(&program.types, body, statement, &point)?;
        }

        let point = MirPoint {
            function: function_id,
            block: block_id,
            statement: None,
        };
        validate_static_terminator(program, function, &block.terminator, &point)?;
    }

    Ok(())
}

fn validate_static_terminator(
    program: &Program,
    function: &Function,
    terminator: &Terminator,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    let body = &function.body;
    match terminator {
        Terminator::Goto(target) => require_target(body, *target, point),
        Terminator::Branch {
            condition,
            true_target,
            false_target,
        } => {
            require_target(body, *true_target, point)?;
            require_target(body, *false_target, point)?;
            validate_branch_condition(&program.types, body, condition, point)
        }
        Terminator::Fault(_) => Ok(()),
        Terminator::Return(value) => match (function.result, value) {
            (None, None) => Ok(()),
            (None, Some(_)) => Err(point_error(
                point,
                MirValidationErrorKind::UnexpectedReturnValue,
            )),
            (Some(_), None) => Err(point_error(
                point,
                MirValidationErrorKind::MissingReturnValue,
            )),
            (Some(expected), Some(value)) => {
                validate_operand_type(&program.types, body, value, expected, point)
            }
        },
        Terminator::Call {
            function: target_id,
            arguments,
            destination,
            target,
        } => {
            require_target(body, *target, point)?;
            let callee = program.function(*target_id).ok_or_else(|| {
                point_error(point, MirValidationErrorKind::InvalidFunction(*target_id))
            })?;
            if arguments.len() != callee.parameters.len() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ArgumentCount {
                        expected: callee.parameters.len(),
                        found: arguments.len(),
                    },
                ));
            }
            for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                let expected = callee
                    .body
                    .local(*parameter)
                    .expect("function-declaration validation establishes parameter local")
                    .ty;
                validate_operand_type(&program.types, body, argument, expected, point)?;
            }
            match (callee.result, destination) {
                (None, None) => Ok(()),
                (None, Some(_)) => Err(point_error(
                    point,
                    MirValidationErrorKind::UnexpectedResultDestination,
                )),
                (Some(_), None) => Err(point_error(
                    point,
                    MirValidationErrorKind::MissingResultDestination,
                )),
                (Some(expected), Some(destination)) => {
                    let actual = place_type(&program.types, body, destination, point)?;
                    require_type_match(actual, expected, point)
                }
            }
        }
    }
}

fn require_target(
    body: &Body,
    target: BasicBlockId,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if body.block(target).is_some() {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::InvalidTargetBlock(target),
        ))
    }
}

fn validate_branch_condition(
    types: &TypeTable,
    body: &Body,
    operand: &Operand,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    let bool_valued = match operand {
        Operand::Constant(Value::Bool(_)) => true,
        Operand::Constant(_) => false,
        Operand::Move(src) => {
            let actual = access_type(types, body, src, point)?;
            is_bool_type(types, actual)
        }
        Operand::Copy(src) => {
            let actual = access_type(types, body, src, point)?;
            if !types.is_copy(actual) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::CopyOfNonCopy(actual),
                ));
            }
            is_bool_type(types, actual)
        }
        Operand::RawMove(pointer) => {
            let pointer_ty = access_type(types, body, pointer, point)?;
            let Some(pointee) = types.raw_pointer_pointee(pointer_ty) else {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::RawMoveRequiresPointer(pointer_ty),
                ));
            };
            is_bool_type(types, pointee)
        }
        Operand::AddressOf(src) => {
            access_type(types, body, src, point)?;
            false
        }
    };

    if bool_valued {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::BranchConditionNotBool,
        ))
    }
}

fn is_bool_type(types: &TypeTable, ty: TypeId) -> bool {
    matches!(
        types.get(ty).map(|definition| &definition.kind),
        Some(TypeKind::Scalar(ScalarType::Bool))
    )
}

fn is_integer_type(types: &TypeTable, ty: TypeId) -> bool {
    matches!(
        types.get(ty).map(|definition| &definition.kind),
        Some(TypeKind::Scalar(
            ScalarType::I8
                | ScalarType::I16
                | ScalarType::I32
                | ScalarType::I64
                | ScalarType::U8
                | ScalarType::U16
                | ScalarType::U32
                | ScalarType::U64
        ))
    )
}

fn validate_static_statement(
    types: &TypeTable,
    body: &Body,
    statement: &Statement,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match statement {
        Statement::Init { dst, src } => {
            let expected = place_type(types, body, dst, point)?;
            validate_operand_type(types, body, src, expected, point)
        }
        Statement::IntegerAdd { dst, left, right } => {
            let expected = place_type(types, body, dst, point)?;
            if !is_integer_type(types, expected) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::IntegerAddRequiresInteger(expected),
                ));
            }
            validate_operand_type(types, body, left, expected, point)?;
            validate_operand_type(types, body, right, expected, point)
        }
        Statement::IntegerSub { dst, left, right } => {
            let expected = place_type(types, body, dst, point)?;
            if !is_integer_type(types, expected) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::IntegerSubRequiresInteger(expected),
                ));
            }
            validate_operand_type(types, body, left, expected, point)?;
            validate_operand_type(types, body, right, expected, point)
        }
        Statement::IntegerMul { dst, left, right } => {
            let expected = place_type(types, body, dst, point)?;
            if !is_integer_type(types, expected) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::IntegerMulRequiresInteger(expected),
                ));
            }
            validate_operand_type(types, body, left, expected, point)?;
            validate_operand_type(types, body, right, expected, point)
        }
        Statement::Borrow { loan, src, .. } => {
            let declaration = loan_decl(body, *loan, point)?;
            let actual = access_type(types, body, src, point)?;
            require_type_match(actual, declaration.ty, point)
        }
        Statement::EndBorrow { loan } => {
            loan_decl(body, *loan, point)?;
            Ok(())
        }
        Statement::Read { src } => {
            access_type(types, body, src, point)?;
            Ok(())
        }
        Statement::RawRead { pointer } => {
            let actual = access_type(types, body, pointer, point)?;
            if types.raw_pointer_pointee(actual).is_some() {
                Ok(())
            } else {
                Err(point_error(
                    point,
                    MirValidationErrorKind::RawReadRequiresPointer(actual),
                ))
            }
        }
        Statement::RawAssign { pointer, src } => {
            let actual = access_type(types, body, pointer, point)?;
            let Some(pointee) = types.raw_pointer_pointee(actual) else {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::RawAssignRequiresPointer(actual),
                ));
            };
            validate_operand_type(types, body, src, pointee, point)
        }
        Statement::Assign { dst, src } => {
            let expected = access_type(types, body, dst, point)?;
            if let PlaceAccess::Direct(place) = dst {
                require_mutable_local(body, place, point)?;
            }
            validate_operand_type(types, body, src, expected, point)
        }
        Statement::InteriorAssign { dst, src } => {
            let expected = access_type(types, body, dst, point)?;
            if let PlaceAccess::Direct(place) = dst {
                require_interior_mutable_place(types, body, place, point)?;
            }
            validate_operand_type(types, body, src, expected, point)
        }
        Statement::Drop { place } => {
            access_type(types, body, place, point)?;
            Ok(())
        }
    }
}

fn validate_operand_type(
    types: &TypeTable,
    body: &Body,
    operand: &Operand,
    expected: TypeId,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match operand {
        Operand::Constant(value) => {
            if types.value_matches(expected, value) {
                Ok(())
            } else {
                Err(point_error(
                    point,
                    MirValidationErrorKind::TypeMismatch { expected },
                ))
            }
        }
        Operand::Move(src) => {
            let actual = access_type(types, body, src, point)?;
            require_type_match(actual, expected, point)
        }
        Operand::RawMove(pointer) => {
            let pointer_ty = access_type(types, body, pointer, point)?;
            let Some(actual) = types.raw_pointer_pointee(pointer_ty) else {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::RawMoveRequiresPointer(pointer_ty),
                ));
            };
            require_type_match(actual, expected, point)
        }
        Operand::Copy(src) => {
            let actual = access_type(types, body, src, point)?;
            require_type_match(actual, expected, point)?;
            if !types.is_copy(actual) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::CopyOfNonCopy(actual),
                ));
            }
            Ok(())
        }
        Operand::AddressOf(src) => {
            let actual = access_type(types, body, src, point)?;
            if types.raw_pointer_pointee(expected) == Some(actual) {
                Ok(())
            } else {
                Err(point_error(
                    point,
                    MirValidationErrorKind::TypeMismatch { expected },
                ))
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
        Err(point_error(
            point,
            MirValidationErrorKind::TypeMismatch { expected },
        ))
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
        Err(point_error(
            point,
            MirValidationErrorKind::AssignToImmutable(place.local),
        ))
    }
}

fn require_interior_mutable_place(
    types: &TypeTable,
    body: &Body,
    place: &Place,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if is_interior_mutable_place(types, body, place) {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::InteriorMutationRequiresMarkedRegion(place.clone()),
        ))
    }
}

fn is_interior_mutable_place(types: &TypeTable, body: &Body, place: &Place) -> bool {
    let local = body
        .local(place.local)
        .expect("validated place references a known containing local");
    let mut current = local.ty;

    if types
        .get(current)
        .expect("validated local type is known")
        .interior_mutable
    {
        return true;
    }

    for projection in &place.projections {
        match projection {
            Projection::Field(index) => {
                let TypeKind::Struct(fields) = &types
                    .get(current)
                    .expect("validated projected type is known")
                    .kind
                else {
                    unreachable!("structural validation rejects field projection from scalar type");
                };
                current = fields[*index as usize].ty;
                if types
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
) -> Result<&'a LoanDecl, MirValidationError> {
    body.loan(loan)
        .ok_or_else(|| point_error(point, MirValidationErrorKind::InvalidLoan(loan)))
}

fn place_type(
    types: &TypeTable,
    body: &Body,
    place: &Place,
    point: &MirPoint,
) -> Result<TypeId, MirValidationError> {
    let local = body
        .local(place.local)
        .ok_or_else(|| point_error(point, MirValidationErrorKind::InvalidLocal(place.local)))?;

    types
        .project_type(local.ty, &place.projections)
        .ok_or_else(|| {
            point_error(
                point,
                MirValidationErrorKind::InvalidProjection(place.clone()),
            )
        })
}

fn access_type(
    types: &TypeTable,
    body: &Body,
    access: &PlaceAccess,
    point: &MirPoint,
) -> Result<TypeId, MirValidationError> {
    match access {
        PlaceAccess::Direct(place) => place_type(types, body, place, point),
        PlaceAccess::Loan { loan, projections } => {
            let declaration = loan_decl(body, *loan, point)?;
            types
                .project_type(declaration.ty, projections)
                .ok_or_else(|| {
                    point_error(
                        point,
                        MirValidationErrorKind::InvalidLoanProjection {
                            loan: *loan,
                            projections: projections.clone(),
                        },
                    )
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

    fn wholly_vacant(&self) -> bool {
        match self {
            Self::Leaf(LeafState::NeverInitialized | LeafState::Dead) => true,
            Self::Leaf(LeafState::Live(_)) => false,
            Self::Aggregate(fields) => fields.iter().all(Self::wholly_vacant),
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ValidationState {
    current: BasicBlockId,
    locals: Vec<ObjectState>,
    active_loans: Vec<Option<ActiveLoan>>,
}

fn validate_path_state(
    program: &Program,
    function_id: FunctionId,
    function: &Function,
) -> Result<(), MirValidationError> {
    let types = &program.types;
    let body = &function.body;
    let mut locals = body
        .locals
        .iter()
        .map(|local| ObjectState::uninitialized(types, local.ty))
        .collect::<Vec<_>>();

    for parameter in &function.parameters {
        let ty = body
            .local(*parameter)
            .expect("declaration validation establishes parameter local")
            .ty;
        let value = unknown_validation_value(types, ty);
        write_validation_value(
            types,
            ty,
            place_state_mut(&mut locals, &Place::local(*parameter)),
            value,
        );
    }

    let initial = ValidationState {
        current: body.entry,
        locals,
        active_loans: vec![None; body.loans.len()],
    };
    let mut worklist = VecDeque::from([initial]);
    let mut seen = HashSet::new();

    'worklist: while let Some(mut state) = worklist.pop_front() {
        if !seen.insert(state.clone()) {
            continue;
        }

        let current = state.current;
        let block = body
            .block(current)
            .expect("structurally validated Core MIR reaches only known blocks");

        for (statement_index, statement) in block.statements.iter().enumerate() {
            let point = MirPoint {
                function: function_id,
                block: current,
                statement: Some(statement_index),
            };
            if matches!(
                validate_state_statement(
                    types,
                    body,
                    &mut state.locals,
                    &mut state.active_loans,
                    statement,
                    &point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                continue 'worklist;
            }
        }

        let point = MirPoint {
            function: function_id,
            block: current,
            statement: None,
        };
        match &block.terminator {
            Terminator::Goto(target) => {
                state.current = *target;
                worklist.push_back(state);
            }
            Terminator::Branch {
                condition,
                true_target,
                false_target,
            } => {
                if matches!(
                    validate_operand_state(
                        types,
                        body,
                        &mut state.locals,
                        &state.active_loans,
                        condition,
                        &point,
                    )?,
                    DefinedStep::NoDefinedContinuation
                ) {
                    continue 'worklist;
                }

                let mut false_state = state.clone();
                state.current = *true_target;
                false_state.current = *false_target;
                worklist.push_back(state);
                worklist.push_back(false_state);
            }
            Terminator::Fault(_) => {}
            Terminator::Return(value) => {
                if let Some(value) = value
                    && matches!(
                        validate_operand_state(
                            types,
                            body,
                            &mut state.locals,
                            &state.active_loans,
                            value,
                            &point,
                        )?,
                        DefinedStep::NoDefinedContinuation
                    )
                {
                    continue 'worklist;
                }
            }
            Terminator::Call {
                function: target_function,
                arguments,
                destination,
                target,
            } => {
                if let Some(destination) = destination {
                    authorize_direct_access(
                        &state.active_loans,
                        destination,
                        AccessRequirement::Exclusive,
                        &point,
                    )?;
                    if !place_state(&state.locals, destination).wholly_vacant() {
                        return Err(point_error(
                            &point,
                            MirValidationErrorKind::CallResultRequiresVacant(destination.clone()),
                        ));
                    }
                }

                for argument in arguments {
                    if matches!(
                        validate_operand_state(
                            types,
                            body,
                            &mut state.locals,
                            &state.active_loans,
                            argument,
                            &point,
                        )?,
                        DefinedStep::NoDefinedContinuation
                    ) {
                        continue 'worklist;
                    }
                }

                if let Some(destination) = destination {
                    let result_ty = program
                        .function(*target_function)
                        .expect("static validation establishes call target")
                        .result
                        .expect("static validation establishes result destination shape");
                    let value = unknown_validation_value(types, result_ty);
                    write_validation_value(
                        types,
                        result_ty,
                        place_state_mut(&mut state.locals, destination),
                        value,
                    );
                }
                state.current = *target;
                worklist.push_back(state);
            }
        }
    }

    Ok(())
}

fn validate_state_statement(
    types: &TypeTable,
    body: &Body,
    locals: &mut [ObjectState],
    active_loans: &mut [Option<ActiveLoan>],
    statement: &Statement,
    point: &MirPoint,
) -> Result<DefinedStep<()>, MirValidationError> {
    match statement {
        Statement::Init { dst, src } => {
            authorize_direct_access(active_loans, dst, AccessRequirement::Exclusive, point)?;
            if !place_state(locals, dst).wholly_vacant() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::InitRequiresVacant(dst.clone()),
                ));
            }
            let dst_ty = place_type(types, body, dst, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, locals, active_loans, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            write_validation_value(types, dst_ty, place_state_mut(locals, dst), value);
        }
        Statement::IntegerAdd { dst, left, right } => {
            authorize_direct_access(active_loans, dst, AccessRequirement::Exclusive, point)?;
            if !place_state(locals, dst).wholly_vacant() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::IntegerAddRequiresVacant(dst.clone()),
                ));
            }
            let dst_ty = place_type(types, body, dst, point)?;
            let DefinedStep::Continue(_) =
                validate_operand_state(types, body, locals, active_loans, left, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            let DefinedStep::Continue(_) =
                validate_operand_state(types, body, locals, active_loans, right, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            let value = unknown_validation_value(types, dst_ty);
            write_validation_value(types, dst_ty, place_state_mut(locals, dst), value);
        }
        Statement::IntegerSub { dst, left, right } => {
            authorize_direct_access(active_loans, dst, AccessRequirement::Exclusive, point)?;
            if !place_state(locals, dst).wholly_vacant() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::IntegerSubRequiresVacant(dst.clone()),
                ));
            }
            let dst_ty = place_type(types, body, dst, point)?;
            let DefinedStep::Continue(_) =
                validate_operand_state(types, body, locals, active_loans, left, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            let DefinedStep::Continue(_) =
                validate_operand_state(types, body, locals, active_loans, right, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            let value = unknown_validation_value(types, dst_ty);
            write_validation_value(types, dst_ty, place_state_mut(locals, dst), value);
        }
        Statement::IntegerMul { dst, left, right } => {
            authorize_direct_access(active_loans, dst, AccessRequirement::Exclusive, point)?;
            if !place_state(locals, dst).wholly_vacant() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::IntegerMulRequiresVacant(dst.clone()),
                ));
            }
            let dst_ty = place_type(types, body, dst, point)?;
            let DefinedStep::Continue(_) =
                validate_operand_state(types, body, locals, active_loans, left, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            let DefinedStep::Continue(_) =
                validate_operand_state(types, body, locals, active_loans, right, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            let value = unknown_validation_value(types, dst_ty);
            write_validation_value(types, dst_ty, place_state_mut(locals, dst), value);
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
            let pointer_place =
                resolve_authorized_access(active_loans, pointer, AccessRequirement::Shared, point)?;
            require_fully_live(locals, &pointer_place, point)?;
            let target = raw_pointer_target_at(locals, &pointer_place);
            let pointer_ty = place_type(types, body, &pointer_place, point)?;
            let pointee_ty = types
                .raw_pointer_pointee(pointer_ty)
                .expect("static validation guarantees a raw-pointer RawAssign operand");
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, locals, active_loans, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            if raw_target_conflicts(active_loans, &target, AccessRequirement::Exclusive) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
            write_validation_value(types, pointee_ty, place_state_mut(locals, &target), value);
        }
        Statement::Assign { dst, src } => {
            let place =
                resolve_authorized_access(active_loans, dst, AccessRequirement::Exclusive, point)?;
            require_mutable_local(body, &place, point)?;
            let dst_ty = place_type(types, body, &place, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, locals, active_loans, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            write_validation_value(types, dst_ty, place_state_mut(locals, &place), value);
        }
        Statement::InteriorAssign { dst, src } => {
            let place =
                resolve_authorized_access(active_loans, dst, AccessRequirement::Shared, point)?;
            require_interior_mutable_place(types, body, &place, point)?;
            let dst_ty = place_type(types, body, &place, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, locals, active_loans, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            write_validation_value(types, dst_ty, place_state_mut(locals, &place), value);
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
    types: &TypeTable,
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
            let ty = place_type(types, body, &place, point)?;
            Ok(DefinedStep::Continue(take_validation_value(
                types,
                ty,
                place_state_mut(locals, &place),
            )))
        }
        Operand::RawMove(pointer) => {
            let pointer_place =
                resolve_authorized_access(active_loans, pointer, AccessRequirement::Shared, point)?;
            require_fully_live(locals, &pointer_place, point)?;
            let target = raw_pointer_target_at(locals, &pointer_place);
            if !place_state(locals, &target).fully_live()
                || raw_target_conflicts(active_loans, &target, AccessRequirement::Exclusive)
            {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
            let pointer_ty = place_type(types, body, &pointer_place, point)?;
            let pointee_ty = types
                .raw_pointer_pointee(pointer_ty)
                .expect("static validation guarantees a raw-pointer RawMove operand");
            Ok(DefinedStep::Continue(take_validation_value(
                types,
                pointee_ty,
                place_state_mut(locals, &target),
            )))
        }
        Operand::Copy(src) => {
            let place =
                resolve_authorized_access(active_loans, src, AccessRequirement::Shared, point)?;
            require_fully_live(locals, &place, point)?;
            let ty = place_type(types, body, &place, point)?;
            Ok(DefinedStep::Continue(clone_validation_value(
                types,
                ty,
                place_state(locals, &place),
            )))
        }
        Operand::AddressOf(src) => {
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
        Value::Bool(_)
        | Value::I8(_)
        | Value::I16(_)
        | Value::I32(_)
        | Value::I64(_)
        | Value::U8(_)
        | Value::U16(_)
        | Value::U32(_)
        | Value::U64(_)
        | Value::F16(_)
        | Value::F32(_)
        | Value::F64(_)
        | Value::TrackedFixture(_) => ValidationValue::Scalar(ValidationScalar::NonPointer),
        Value::Struct(values) => {
            ValidationValue::Struct(values.iter().map(validation_value_from_constant).collect())
        }
    }
}

fn unknown_validation_value(types: &TypeTable, ty: TypeId) -> ValidationValue {
    match &types.get(ty).expect("validated type identity exists").kind {
        TypeKind::Scalar(ScalarType::RawPointer(_)) => {
            unreachable!("call-transfer validation excludes raw-pointer values")
        }
        TypeKind::Scalar(_) => ValidationValue::Scalar(ValidationScalar::NonPointer),
        TypeKind::Struct(fields) => ValidationValue::Struct(
            fields
                .iter()
                .map(|field| unknown_validation_value(types, field.ty))
                .collect(),
        ),
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
        ) => *leaf = LeafState::Live(value),
        (
            TypeKind::Scalar(_),
            ObjectState::Leaf(leaf),
            ValidationValue::Scalar(ValidationScalar::NonPointer),
        ) => *leaf = LeafState::Live(ValidationScalar::NonPointer),
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

fn function_id(index: usize) -> FunctionId {
    FunctionId(u32::try_from(index).expect("function index exceeds u32::MAX"))
}

fn block_id(index: usize) -> BasicBlockId {
    BasicBlockId(u32::try_from(index).expect("basic block index exceeds u32::MAX"))
}

fn point_error(point: &MirPoint, kind: MirValidationErrorKind) -> MirValidationError {
    MirValidationError {
        location: MirLocation::Point(point.clone()),
        kind,
    }
}

fn function_error(function: FunctionId, kind: MirValidationErrorKind) -> MirValidationError {
    MirValidationError {
        location: MirLocation::Function(function),
        kind,
    }
}

fn program_error(kind: MirValidationErrorKind) -> MirValidationError {
    MirValidationError {
        location: MirLocation::Program,
        kind,
    }
}
