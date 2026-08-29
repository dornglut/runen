use std::collections::{HashMap, HashSet, VecDeque};

use crate::interprocedural::{Body, Function, Program, Terminator};
use crate::{
    BasicBlockId, BorrowKind, FunctionId, LoanDecl, LoanId, LocalId, Operand, Place, PlaceAccess,
    Projection, ReferenceAccess, ReferencePermission, ScalarType, Statement, TypeId, TypeKind,
    TypeTable, Value,
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
    InvalidReferenceProjection {
        projections: Vec<Projection>,
    },
    UnknownType(TypeId),
    RecursiveType(TypeId),
    DuplicateReferenceType(TypeId),
    ParameterTransferUnsafe(TypeId),
    ResultTransferUnsafe(TypeId),
    MissingSharedReferenceResultOrigin,
    UnexpectedSharedReferenceResultOrigin,
    InvalidSharedReferenceResultOriginSlot(usize),
    SharedReferenceResultRequiresShared(TypeId),
    SharedReferenceResultOriginRequiresShared(TypeId),
    SharedReferenceResultOriginTypeMismatch {
        expected: TypeId,
        found: TypeId,
    },
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
    ReferenceAccessRequiresReference(TypeId),
    ReferencePermissionRequired(ReferencePermission),
    ReferenceTargetNotLive,
    ReferenceAuthorityNotActive,
    ReferenceAuthorityIncomplete,
    SharedReferenceResultOriginMismatch,
    ReferenceFormationConflictsWithLoan {
        place: Place,
        loan: LoanId,
    },
    ReferenceFormationConflictsWithReferenceAuthority {
        place: Place,
    },
    BorrowConflictWithReferenceAuthority {
        place: Place,
    },
    DirectAccessConflictWithReferenceAuthority {
        place: Place,
    },
    ReferenceAccessDelegated,
    ReferenceOutlivesStorage(LocalId),
    ExternalReferentNotLive,
    IntegerAddRequiresInteger(TypeId),
    IntegerSubRequiresInteger(TypeId),
    IntegerMulRequiresInteger(TypeId),
    IntegerXorRequiresInteger(TypeId),
    IntegerOrRequiresInteger(TypeId),
    FloatAddRequiresFloat(TypeId),
    FloatSubRequiresFloat(TypeId),
    FloatMulRequiresFloat(TypeId),
    FloatDivRequiresFloat(TypeId),
    AssignToImmutable(LocalId),
    InteriorMutationRequiresMarkedRegion(Place),
    InteriorMutationRequiresMarkedReferenceRegion,
    InitRequiresVacant(Place),
    IntegerAddRequiresVacant(Place),
    IntegerSubRequiresVacant(Place),
    IntegerMulRequiresVacant(Place),
    IntegerXorRequiresVacant(Place),
    IntegerOrRequiresVacant(Place),
    FloatAddRequiresVacant(Place),
    FloatSubRequiresVacant(Place),
    FloatMulRequiresVacant(Place),
    FloatDivRequiresVacant(Place),
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
    let mut reference_pairs = HashMap::new();
    for index in 0..types.len() {
        let ty = TypeId(u32::try_from(index).expect("type index exceeds u32::MAX"));
        let definition = types
            .get(ty)
            .expect("type index was derived from the type-table length");
        match &definition.kind {
            TypeKind::Scalar(ScalarType::RawPointer(pointee)) => {
                if types.get(*pointee).is_none() {
                    return Err(program_error(MirValidationErrorKind::UnknownType(*pointee)));
                }
            }
            TypeKind::Scalar(ScalarType::Reference {
                referent,
                permission,
            }) => {
                if types.get(*referent).is_none() {
                    return Err(program_error(MirValidationErrorKind::UnknownType(
                        *referent,
                    )));
                }
                if reference_pairs
                    .insert((*referent, *permission), ty)
                    .is_some()
                {
                    return Err(program_error(
                        MirValidationErrorKind::DuplicateReferenceType(ty),
                    ));
                }
            }
            TypeKind::Scalar(_) | TypeKind::Struct(_) => {}
        }
    }

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
        require_known_parameter_type(types, function_id, local.ty)?;
    }

    validate_result_contract(types, function_id, function)
}

fn require_known_parameter_type(
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
    if !types.is_parameter_transfer_safe(ty) {
        return Err(function_error(
            function,
            MirValidationErrorKind::ParameterTransferUnsafe(ty),
        ));
    }
    Ok(())
}

fn validate_result_contract(
    types: &TypeTable,
    function_id: FunctionId,
    function: &Function,
) -> Result<(), MirValidationError> {
    let Some(result) = function.result else {
        return if function.shared_reference_result_origin.is_none() {
            Ok(())
        } else {
            Err(function_error(
                function_id,
                MirValidationErrorKind::UnexpectedSharedReferenceResultOrigin,
            ))
        };
    };

    let definition = types
        .get(result)
        .ok_or_else(|| function_error(function_id, MirValidationErrorKind::UnknownType(result)))?;

    if types.is_result_transfer_safe(result) {
        return if function.shared_reference_result_origin.is_none() {
            Ok(())
        } else {
            Err(function_error(
                function_id,
                MirValidationErrorKind::UnexpectedSharedReferenceResultOrigin,
            ))
        };
    }

    let TypeKind::Scalar(ScalarType::Reference { permission, .. }) = &definition.kind else {
        return Err(function_error(
            function_id,
            MirValidationErrorKind::ResultTransferUnsafe(result),
        ));
    };

    if *permission != ReferencePermission::Shared {
        return Err(function_error(
            function_id,
            MirValidationErrorKind::SharedReferenceResultRequiresShared(result),
        ));
    }

    let Some(origin_slot) = function.shared_reference_result_origin else {
        return Err(function_error(
            function_id,
            MirValidationErrorKind::MissingSharedReferenceResultOrigin,
        ));
    };
    let Some(parameter) = function.parameters.get(origin_slot) else {
        return Err(function_error(
            function_id,
            MirValidationErrorKind::InvalidSharedReferenceResultOriginSlot(origin_slot),
        ));
    };
    let found = function
        .body
        .local(*parameter)
        .expect("parameter validation establishes the designated origin local")
        .ty;
    if let Some((_, permission)) = types.reference(found)
        && permission != ReferencePermission::Shared
    {
        return Err(function_error(
            function_id,
            MirValidationErrorKind::SharedReferenceResultOriginRequiresShared(found),
        ));
    }
    if found != result {
        return Err(function_error(
            function_id,
            MirValidationErrorKind::SharedReferenceResultOriginTypeMismatch {
                expected: result,
                found,
            },
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
        Operand::Move(src) => is_bool_type(types, access_type(types, body, src, point)?),
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
        Operand::ReferenceRoot { place, .. } => {
            place_type(types, body, place, point)?;
            false
        }
        Operand::ReferenceReborrow { permission, src } => {
            validate_reference_reborrow_shape(types, body, *permission, src, None, point)?;
            false
        }
        Operand::ReferenceMove(src) => {
            let (_, _, selected) = reference_access_type(types, body, src, point)?;
            is_bool_type(types, selected)
        }
        Operand::ReferenceCopy(src) => {
            let (_, _, selected) = reference_access_type(types, body, src, point)?;
            if !types.is_copy(selected) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::CopyOfNonCopy(selected),
                ));
            }
            is_bool_type(types, selected)
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

fn is_float_type(types: &TypeTable, ty: TypeId) -> bool {
    matches!(
        types.get(ty).map(|definition| &definition.kind),
        Some(TypeKind::Scalar(
            ScalarType::F16 | ScalarType::F32 | ScalarType::F64
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
        Statement::IntegerAdd { dst, left, right } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_integer_type,
            MirValidationErrorKind::IntegerAddRequiresInteger,
            point,
        ),
        Statement::IntegerSub { dst, left, right } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_integer_type,
            MirValidationErrorKind::IntegerSubRequiresInteger,
            point,
        ),
        Statement::IntegerMul { dst, left, right } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_integer_type,
            MirValidationErrorKind::IntegerMulRequiresInteger,
            point,
        ),
        Statement::IntegerXor { dst, left, right } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_integer_type,
            MirValidationErrorKind::IntegerXorRequiresInteger,
            point,
        ),
        Statement::IntegerOr { dst, left, right } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_integer_type,
            MirValidationErrorKind::IntegerOrRequiresInteger,
            point,
        ),
        Statement::FloatAdd {
            dst, left, right, ..
        } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_float_type,
            MirValidationErrorKind::FloatAddRequiresFloat,
            point,
        ),
        Statement::FloatSub {
            dst, left, right, ..
        } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_float_type,
            MirValidationErrorKind::FloatSubRequiresFloat,
            point,
        ),
        Statement::FloatMul {
            dst, left, right, ..
        } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_float_type,
            MirValidationErrorKind::FloatMulRequiresFloat,
            point,
        ),
        Statement::FloatDiv {
            dst, left, right, ..
        } => validate_static_binary_numeric(
            types,
            body,
            (dst, left, right),
            is_float_type,
            MirValidationErrorKind::FloatDivRequiresFloat,
            point,
        ),
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
        Statement::ReferenceRead { src } => {
            reference_access_type(types, body, src, point)?;
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
        Statement::ReferenceAssign { dst, src } => {
            let (_, permission, expected) = reference_access_type(types, body, dst, point)?;
            require_reference_permission(permission, ReferencePermission::ExclusiveReplace, point)?;
            validate_operand_type(types, body, src, expected, point)
        }
        Statement::InteriorAssign { dst, src } => {
            let expected = access_type(types, body, dst, point)?;
            if let PlaceAccess::Direct(place) = dst {
                require_interior_mutable_place(types, body, place, point)?;
            }
            validate_operand_type(types, body, src, expected, point)
        }
        Statement::ReferenceInteriorAssign { dst, src } => {
            let (referent, _, expected) = reference_access_type(types, body, dst, point)?;
            if !is_interior_mutable_from_root(types, referent, &dst.projections) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::InteriorMutationRequiresMarkedReferenceRegion,
                ));
            }
            validate_operand_type(types, body, src, expected, point)
        }
        Statement::Drop { place } => {
            access_type(types, body, place, point)?;
            Ok(())
        }
        Statement::ReferenceDrop { place } => {
            let (_, permission, _) = reference_access_type(types, body, place, point)?;
            if matches!(
                permission,
                ReferencePermission::Exclusive | ReferencePermission::ExclusiveReplace
            ) {
                Ok(())
            } else {
                Err(point_error(
                    point,
                    MirValidationErrorKind::ReferencePermissionRequired(
                        ReferencePermission::Exclusive,
                    ),
                ))
            }
        }
    }
}

fn validate_static_binary_numeric(
    types: &TypeTable,
    body: &Body,
    operands: (&Place, &Operand, &Operand),
    predicate: fn(&TypeTable, TypeId) -> bool,
    error: fn(TypeId) -> MirValidationErrorKind,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    let (dst, left, right) = operands;
    let expected = place_type(types, body, dst, point)?;
    if !predicate(types, expected) {
        return Err(point_error(point, error(expected)));
    }
    validate_operand_type(types, body, left, expected, point)?;
    validate_operand_type(types, body, right, expected, point)
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
            require_type_match(access_type(types, body, src, point)?, expected, point)
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
        Operand::ReferenceRoot { permission, place } => {
            let actual_referent = place_type(types, body, place, point)?;
            if *permission == ReferencePermission::ExclusiveReplace {
                require_mutable_local(body, place, point)?;
            }
            let Some((expected_referent, expected_permission)) = types.reference(expected) else {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::TypeMismatch { expected },
                ));
            };
            if actual_referent == expected_referent && *permission == expected_permission {
                Ok(())
            } else {
                Err(point_error(
                    point,
                    MirValidationErrorKind::TypeMismatch { expected },
                ))
            }
        }
        Operand::ReferenceReborrow { permission, src } => {
            validate_reference_reborrow_shape(types, body, *permission, src, Some(expected), point)
        }
        Operand::ReferenceMove(src) => {
            let (_, permission, actual) = reference_access_type(types, body, src, point)?;
            if permission == ReferencePermission::Shared {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ReferencePermissionRequired(
                        ReferencePermission::Exclusive,
                    ),
                ));
            }
            require_type_match(actual, expected, point)
        }
        Operand::ReferenceCopy(src) => {
            let (_, _, actual) = reference_access_type(types, body, src, point)?;
            require_type_match(actual, expected, point)?;
            if !types.is_copy(actual) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::CopyOfNonCopy(actual),
                ));
            }
            Ok(())
        }
    }
}

fn validate_reference_reborrow_shape(
    types: &TypeTable,
    body: &Body,
    requested: ReferencePermission,
    src: &ReferenceAccess,
    expected: Option<TypeId>,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    let (_, parent_permission, selected) = reference_access_type(types, body, src, point)?;
    if !parent_permission.permits(requested) {
        return Err(point_error(
            point,
            MirValidationErrorKind::ReferencePermissionRequired(requested),
        ));
    }
    if let Some(expected) = expected {
        let Some((referent, permission)) = types.reference(expected) else {
            return Err(point_error(
                point,
                MirValidationErrorKind::TypeMismatch { expected },
            ));
        };
        if referent != selected || permission != requested {
            return Err(point_error(
                point,
                MirValidationErrorKind::TypeMismatch { expected },
            ));
        }
    }
    Ok(())
}

fn require_reference_permission(
    actual: ReferencePermission,
    required: ReferencePermission,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if actual == required {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::ReferencePermissionRequired(required),
        ))
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
    if is_interior_mutable_from_root(
        types,
        body.local(place.local).unwrap().ty,
        &place.projections,
    ) {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::InteriorMutationRequiresMarkedRegion(place.clone()),
        ))
    }
}

fn is_interior_mutable_from_root(
    types: &TypeTable,
    root_ty: TypeId,
    projections: &[Projection],
) -> bool {
    let mut current = root_ty;
    if types
        .get(current)
        .expect("validated type identity is known")
        .interior_mutable
    {
        return true;
    }
    for projection in projections {
        match projection {
            Projection::Field(index) => {
                let TypeKind::Struct(fields) = &types
                    .get(current)
                    .expect("validated projected type is known")
                    .kind
                else {
                    return false;
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

fn reference_access_type(
    types: &TypeTable,
    body: &Body,
    access: &ReferenceAccess,
    point: &MirPoint,
) -> Result<(TypeId, ReferencePermission, TypeId), MirValidationError> {
    let reference_ty = access_type(types, body, &access.reference, point)?;
    let Some((referent, permission)) = types.reference(reference_ty) else {
        return Err(point_error(
            point,
            MirValidationErrorKind::ReferenceAccessRequiresReference(reference_ty),
        ));
    };
    let selected = types
        .project_type(referent, &access.projections)
        .ok_or_else(|| {
            point_error(
                point,
                MirValidationErrorKind::InvalidReferenceProjection {
                    projections: access.projections.clone(),
                },
            )
        })?;
    Ok((referent, permission, selected))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ExternalRegionId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ValidationReferenceAuthorityId(u32);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ValidationRegionRoot {
    Local(LocalId),
    External(ExternalRegionId),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ValidationRegion {
    root: ValidationRegionRoot,
    projections: Vec<Projection>,
}

impl ValidationRegion {
    fn local(place: &Place) -> Self {
        Self {
            root: ValidationRegionRoot::Local(place.local),
            projections: place.projections.clone(),
        }
    }

    fn external(id: ExternalRegionId) -> Self {
        Self {
            root: ValidationRegionRoot::External(id),
            projections: Vec::new(),
        }
    }

    fn projected(&self, projections: &[Projection]) -> Self {
        let mut region = self.clone();
        region.projections.extend(projections.iter().copied());
        region
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.root == other.root
            && (self.projections.starts_with(&other.projections)
                || other.projections.starts_with(&self.projections))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ValidationScalar {
    NonPointer,
    RawPointer(Place),
    Reference(ValidationReferenceAuthorityId),
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

    fn fully_live_unknown(types: &TypeTable, ty: TypeId) -> Self {
        let definition = types
            .get(ty)
            .expect("validated type graph references only known types");
        match &definition.kind {
            TypeKind::Scalar(ScalarType::RawPointer(_) | ScalarType::Reference { .. }) => {
                unreachable!("reference external domains are raw-free and reference-free")
            }
            TypeKind::Scalar(_) => Self::Leaf(LeafState::Live(ValidationScalar::NonPointer)),
            TypeKind::Struct(fields) => Self::Aggregate(
                fields
                    .iter()
                    .map(|field| Self::fully_live_unknown(types, field.ty))
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
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExternalRegionState {
    ty: TypeId,
    state: ObjectState,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ActiveLoan {
    kind: BorrowKind,
    place: Place,
    parent: Option<LoanId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ActiveReferenceAuthority {
    target: ValidationRegion,
    permission: ReferencePermission,
    parent: Option<ValidationReferenceAuthorityId>,
    carriers: u32,
    is_result_origin: bool,
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
    external_regions: Vec<ExternalRegionState>,
    active_loans: Vec<Option<ActiveLoan>>,
    reference_authorities: Vec<Option<ActiveReferenceAuthority>>,
}

fn validate_path_state(
    program: &Program,
    function_id: FunctionId,
    function: &Function,
) -> Result<(), MirValidationError> {
    let types = &program.types;
    let body = &function.body;
    let mut state = ValidationState {
        current: body.entry,
        locals: body
            .locals
            .iter()
            .map(|local| ObjectState::uninitialized(types, local.ty))
            .collect(),
        external_regions: Vec::new(),
        active_loans: vec![None; body.loans.len()],
        reference_authorities: Vec::new(),
    };

    for (slot, parameter) in function.parameters.iter().enumerate() {
        let ty = body
            .local(*parameter)
            .expect("declaration validation establishes parameter local")
            .ty;
        let value = parameter_validation_value(
            types,
            ty,
            &mut state,
            function.shared_reference_result_origin == Some(slot),
        );
        write_validation_value(
            types,
            ty,
            place_state_mut(&mut state.locals, &Place::local(*parameter)),
            value,
        );
    }

    let mut worklist = VecDeque::from([state]);
    let mut seen = HashSet::new();

    'worklist: while let Some(mut state) = worklist.pop_front() {
        normalize_reference_authorities(&mut state.reference_authorities);
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
                validate_state_statement(types, body, &mut state, statement, &point)?,
                DefinedStep::NoDefinedContinuation
            ) {
                continue 'worklist;
            }
            normalize_reference_authorities(&mut state.reference_authorities);
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
                    validate_operand_state(types, body, &mut state, condition, &point)?,
                    DefinedStep::NoDefinedContinuation
                ) {
                    continue 'worklist;
                }
                normalize_reference_authorities(&mut state.reference_authorities);
                let mut false_state = state.clone();
                state.current = *true_target;
                false_state.current = *false_target;
                worklist.push_back(state);
                worklist.push_back(false_state);
            }
            Terminator::Fault(_) => {
                cleanup_function(types, body, &mut state, &point)?;
            }
            Terminator::Return(value) => {
                let result = if let Some(value) = value {
                    let DefinedStep::Continue(value) =
                        validate_operand_state(types, body, &mut state, value, &point)?
                    else {
                        continue 'worklist;
                    };
                    Some(value)
                } else {
                    None
                };
                if function.shared_reference_result_origin.is_some() {
                    require_shared_reference_result_origin(
                        result
                            .as_ref()
                            .expect("static validation establishes a contract-bearing result"),
                        &state,
                        &point,
                    )?;
                }
                require_external_regions_fully_live(&state, &point)?;
                cleanup_function(types, body, &mut state, &point)?;
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
                        &state.reference_authorities,
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

                let callee = program
                    .function(*target_function)
                    .expect("static validation establishes call target");
                let mut held = Vec::with_capacity(arguments.len());
                for (argument, parameter) in arguments.iter().zip(&callee.parameters) {
                    let parameter_ty = callee
                        .body
                        .local(*parameter)
                        .expect("declaration validation establishes parameter local")
                        .ty;
                    let DefinedStep::Continue(value) =
                        validate_operand_state(types, body, &mut state, argument, &point)?
                    else {
                        continue 'worklist;
                    };
                    held.push((parameter_ty, value));
                }

                for (ty, value) in &held {
                    require_call_value_admissible(types, *ty, value, &state, &point)?;
                }

                let mut fault_state = state.clone();
                destroy_transient_values(types, &held, &mut fault_state);
                cleanup_function(types, body, &mut fault_state, &point)?;

                for (ty, value) in &held {
                    restore_transferred_referents(types, *ty, value, &mut state);
                }

                let returned_result = callee
                    .shared_reference_result_origin
                    .map(|slot| held[slot].1.clone());
                if let Some(origin_slot) = callee.shared_reference_result_origin {
                    destroy_transient_values_except(types, &held, origin_slot, &mut state);
                } else {
                    destroy_transient_values(types, &held, &mut state);
                }

                if let Some(destination) = destination {
                    let result_ty = callee
                        .result
                        .expect("static validation establishes result destination shape");
                    let value = returned_result
                        .unwrap_or_else(|| unknown_result_validation_value(types, result_ty));
                    write_validation_value(
                        types,
                        result_ty,
                        place_state_mut(&mut state.locals, destination),
                        value,
                    );
                }
                normalize_reference_authorities(&mut state.reference_authorities);
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
    state: &mut ValidationState,
    statement: &Statement,
    point: &MirPoint,
) -> Result<DefinedStep<()>, MirValidationError> {
    match statement {
        Statement::Init { dst, src } => {
            authorize_direct_access(
                &state.active_loans,
                &state.reference_authorities,
                dst,
                AccessRequirement::Exclusive,
                point,
            )?;
            if !place_state(&state.locals, dst).wholly_vacant() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::InitRequiresVacant(dst.clone()),
                ));
            }
            let dst_ty = place_type(types, body, dst, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, state, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            write_validation_value(
                types,
                dst_ty,
                place_state_mut(&mut state.locals, dst),
                value,
            );
        }
        Statement::IntegerAdd { dst, left, right } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::IntegerAddRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::IntegerSub { dst, left, right } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::IntegerSubRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::IntegerMul { dst, left, right } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::IntegerMulRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::IntegerXor { dst, left, right } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::IntegerXorRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::IntegerOr { dst, left, right } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::IntegerOrRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::FloatAdd {
            dst, left, right, ..
        } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::FloatAddRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::FloatSub {
            dst, left, right, ..
        } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::FloatSubRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::FloatMul {
            dst, left, right, ..
        } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::FloatMulRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::FloatDiv {
            dst, left, right, ..
        } => {
            if matches!(
                validate_state_binary_numeric(
                    types,
                    body,
                    state,
                    (dst, left, right),
                    MirValidationErrorKind::FloatDivRequiresVacant,
                    point,
                )?,
                DefinedStep::NoDefinedContinuation
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::Borrow { loan, kind, src } => {
            let loan_index = loan.0 as usize;
            if state
                .active_loans
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
                    if let Some(conflicting) = borrow_conflict(&state.active_loans, place, *kind) {
                        return Err(point_error(
                            point,
                            MirValidationErrorKind::BorrowConflict {
                                place: place.clone(),
                                loan: conflicting,
                            },
                        ));
                    }
                    if reference_conflict_with_local(&state.reference_authorities, place, *kind) {
                        return Err(point_error(
                            point,
                            MirValidationErrorKind::BorrowConflictWithReferenceAuthority {
                                place: place.clone(),
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
                        resolve_authorized_access(
                            &state.active_loans,
                            &state.reference_authorities,
                            src,
                            requirement,
                            point,
                        )?,
                        Some(*parent),
                    )
                }
            };

            if !place_state(&state.locals, &place).fully_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::BorrowOfUninitialized(place),
                ));
            }

            state.active_loans[loan_index] = Some(ActiveLoan {
                kind: *kind,
                place,
                parent,
            });
        }
        Statement::EndBorrow { loan } => {
            if state
                .active_loans
                .get(loan.0 as usize)
                .expect("static validation guarantees a declared loan identity")
                .is_none()
            {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::LoanNotActive(*loan),
                ));
            }
            if let Some(child) = active_child(&state.active_loans, *loan) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::LoanHasActiveChild { loan: *loan, child },
                ));
            }
            state.active_loans[loan.0 as usize] = None;
        }
        Statement::Read { src } => {
            let place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                src,
                AccessRequirement::Shared,
                point,
            )?;
            require_fully_live(&state.locals, &place, point)?;
        }
        Statement::ReferenceRead { src } => {
            let resolved = resolve_reference_access(
                types,
                body,
                state,
                src,
                AccessRequirement::Shared,
                None,
                point,
            )?;
            require_region_fully_live(state, &resolved.target, point)?;
        }
        Statement::RawRead { pointer } => {
            let pointer_place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                pointer,
                AccessRequirement::Shared,
                point,
            )?;
            require_fully_live(&state.locals, &pointer_place, point)?;
            let target = raw_pointer_target_at(&state.locals, &pointer_place);
            if !place_state(&state.locals, &target).fully_live()
                || raw_target_conflicts(
                    &state.active_loans,
                    &state.reference_authorities,
                    &target,
                    AccessRequirement::Shared,
                )
            {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
        }
        Statement::RawAssign { pointer, src } => {
            let pointer_place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                pointer,
                AccessRequirement::Shared,
                point,
            )?;
            require_fully_live(&state.locals, &pointer_place, point)?;
            let target = raw_pointer_target_at(&state.locals, &pointer_place);
            let pointer_ty = place_type(types, body, &pointer_place, point)?;
            let pointee_ty = types
                .raw_pointer_pointee(pointer_ty)
                .expect("static validation guarantees a raw-pointer RawAssign operand");
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, state, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            if raw_target_conflicts(
                &state.active_loans,
                &state.reference_authorities,
                &target,
                AccessRequirement::Exclusive,
            ) {
                return Ok(DefinedStep::NoDefinedContinuation);
            }
            destroy_place_live(types, body, state, &target);
            write_validation_value(
                types,
                pointee_ty,
                place_state_mut(&mut state.locals, &target),
                value,
            );
        }
        Statement::Assign { dst, src } => {
            let place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                dst,
                AccessRequirement::Exclusive,
                point,
            )?;
            require_mutable_local(body, &place, point)?;
            let dst_ty = place_type(types, body, &place, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, state, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            destroy_place_live(types, body, state, &place);
            write_validation_value(
                types,
                dst_ty,
                place_state_mut(&mut state.locals, &place),
                value,
            );
        }
        Statement::ReferenceAssign { dst, src } => {
            let resolved = resolve_reference_access(
                types,
                body,
                state,
                dst,
                AccessRequirement::Exclusive,
                Some(ReferencePermission::ExclusiveReplace),
                point,
            )?;
            let dst_ty = resolved.selected_ty;
            let target = resolved.target;
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, state, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            destroy_region_live(types, body, state, &target);
            write_region_value(types, body, state, &target, dst_ty, value);
        }
        Statement::InteriorAssign { dst, src } => {
            let place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                dst,
                AccessRequirement::Shared,
                point,
            )?;
            require_interior_mutable_place(types, body, &place, point)?;
            let dst_ty = place_type(types, body, &place, point)?;
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, state, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            destroy_place_live(types, body, state, &place);
            write_validation_value(
                types,
                dst_ty,
                place_state_mut(&mut state.locals, &place),
                value,
            );
        }
        Statement::ReferenceInteriorAssign { dst, src } => {
            let resolved = resolve_reference_access(
                types,
                body,
                state,
                dst,
                AccessRequirement::Shared,
                None,
                point,
            )?;
            if !is_interior_mutable_region(types, body, state, &resolved.target) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::InteriorMutationRequiresMarkedReferenceRegion,
                ));
            }
            let dst_ty = resolved.selected_ty;
            let target = resolved.target;
            let DefinedStep::Continue(value) =
                validate_operand_state(types, body, state, src, point)?
            else {
                return Ok(DefinedStep::NoDefinedContinuation);
            };
            destroy_region_live(types, body, state, &target);
            write_region_value(types, body, state, &target, dst_ty, value);
        }
        Statement::Drop { place } => {
            let place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                place,
                AccessRequirement::Exclusive,
                point,
            )?;
            if !place_state(&state.locals, &place).any_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::DropOfUninitialized(place),
                ));
            }
            destroy_place_live(types, body, state, &place);
        }
        Statement::ReferenceDrop { place } => {
            let resolved = resolve_reference_access(
                types,
                body,
                state,
                place,
                AccessRequirement::Exclusive,
                None,
                point,
            )?;
            if !region_state(state, &resolved.target).any_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ReferenceTargetNotLive,
                ));
            }
            destroy_region_live(types, body, state, &resolved.target);
        }
    }
    Ok(DefinedStep::Continue(()))
}

fn validate_state_binary_numeric(
    types: &TypeTable,
    body: &Body,
    state: &mut ValidationState,
    operands: (&Place, &Operand, &Operand),
    vacancy_error: fn(Place) -> MirValidationErrorKind,
    point: &MirPoint,
) -> Result<DefinedStep<()>, MirValidationError> {
    let (dst, left, right) = operands;
    authorize_direct_access(
        &state.active_loans,
        &state.reference_authorities,
        dst,
        AccessRequirement::Exclusive,
        point,
    )?;
    if !place_state(&state.locals, dst).wholly_vacant() {
        return Err(point_error(point, vacancy_error(dst.clone())));
    }
    let dst_ty = place_type(types, body, dst, point)?;
    let DefinedStep::Continue(_) = validate_operand_state(types, body, state, left, point)? else {
        return Ok(DefinedStep::NoDefinedContinuation);
    };
    let DefinedStep::Continue(_) = validate_operand_state(types, body, state, right, point)? else {
        return Ok(DefinedStep::NoDefinedContinuation);
    };
    let value = unknown_result_validation_value(types, dst_ty);
    write_validation_value(
        types,
        dst_ty,
        place_state_mut(&mut state.locals, dst),
        value,
    );
    Ok(DefinedStep::Continue(()))
}

fn validate_operand_state(
    types: &TypeTable,
    body: &Body,
    state: &mut ValidationState,
    operand: &Operand,
    point: &MirPoint,
) -> Result<DefinedStep<ValidationValue>, MirValidationError> {
    match operand {
        Operand::Constant(value) => {
            Ok(DefinedStep::Continue(validation_value_from_constant(value)))
        }
        Operand::Move(src) => {
            let place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                src,
                AccessRequirement::Exclusive,
                point,
            )?;
            require_fully_live(&state.locals, &place, point)?;
            let ty = place_type(types, body, &place, point)?;
            Ok(DefinedStep::Continue(take_validation_value(
                types,
                ty,
                place_state_mut(&mut state.locals, &place),
            )))
        }
        Operand::RawMove(pointer) => {
            let pointer_place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                pointer,
                AccessRequirement::Shared,
                point,
            )?;
            require_fully_live(&state.locals, &pointer_place, point)?;
            let target = raw_pointer_target_at(&state.locals, &pointer_place);
            if !place_state(&state.locals, &target).fully_live()
                || raw_target_conflicts(
                    &state.active_loans,
                    &state.reference_authorities,
                    &target,
                    AccessRequirement::Exclusive,
                )
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
                place_state_mut(&mut state.locals, &target),
            )))
        }
        Operand::Copy(src) => {
            let place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                src,
                AccessRequirement::Shared,
                point,
            )?;
            require_fully_live(&state.locals, &place, point)?;
            let ty = place_type(types, body, &place, point)?;
            let value = clone_validation_value(
                types,
                ty,
                place_state(&state.locals, &place),
                &mut state.reference_authorities,
            );
            Ok(DefinedStep::Continue(value))
        }
        Operand::AddressOf(src) => {
            let place = resolve_authorized_access(
                &state.active_loans,
                &state.reference_authorities,
                src,
                AccessRequirement::Shared,
                point,
            )?;
            Ok(DefinedStep::Continue(ValidationValue::Scalar(
                ValidationScalar::RawPointer(place),
            )))
        }
        Operand::ReferenceRoot { permission, place } => {
            require_fully_live(&state.locals, place, point)?;
            if *permission == ReferencePermission::ExclusiveReplace {
                require_mutable_local(body, place, point)?;
            }
            let requested = permission.alias_kind();
            if let Some(loan) = borrow_conflict(&state.active_loans, place, requested) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ReferenceFormationConflictsWithLoan {
                        place: place.clone(),
                        loan,
                    },
                ));
            }
            if reference_conflict_with_local(&state.reference_authorities, place, requested) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ReferenceFormationConflictsWithReferenceAuthority {
                        place: place.clone(),
                    },
                ));
            }
            let authority = allocate_reference_authority(
                &mut state.reference_authorities,
                ActiveReferenceAuthority {
                    target: ValidationRegion::local(place),
                    permission: *permission,
                    parent: None,
                    carriers: 1,
                    is_result_origin: false,
                },
            );
            Ok(DefinedStep::Continue(ValidationValue::Scalar(
                ValidationScalar::Reference(authority),
            )))
        }
        Operand::ReferenceReborrow { permission, src } => {
            let requirement = match permission.alias_kind() {
                BorrowKind::Shared => AccessRequirement::Shared,
                BorrowKind::Exclusive => AccessRequirement::Exclusive,
            };
            let resolved =
                resolve_reference_access(types, body, state, src, requirement, None, point)?;
            if !resolved.permission.permits(*permission) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ReferencePermissionRequired(*permission),
                ));
            }
            let authority = allocate_reference_authority(
                &mut state.reference_authorities,
                ActiveReferenceAuthority {
                    target: resolved.target,
                    permission: *permission,
                    parent: Some(resolved.authority),
                    carriers: 1,
                    is_result_origin: false,
                },
            );
            Ok(DefinedStep::Continue(ValidationValue::Scalar(
                ValidationScalar::Reference(authority),
            )))
        }
        Operand::ReferenceMove(src) => {
            let resolved = resolve_reference_access(
                types,
                body,
                state,
                src,
                AccessRequirement::Exclusive,
                None,
                point,
            )?;
            require_region_fully_live(state, &resolved.target, point)?;
            let value =
                take_region_value(types, body, state, &resolved.target, resolved.selected_ty);
            Ok(DefinedStep::Continue(value))
        }
        Operand::ReferenceCopy(src) => {
            let resolved = resolve_reference_access(
                types,
                body,
                state,
                src,
                AccessRequirement::Shared,
                None,
                point,
            )?;
            require_region_fully_live(state, &resolved.target, point)?;
            let value =
                clone_region_value(types, body, state, &resolved.target, resolved.selected_ty);
            Ok(DefinedStep::Continue(value))
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

fn parameter_validation_value(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ValidationState,
    is_result_origin: bool,
) -> ValidationValue {
    match &types.get(ty).expect("validated type identity exists").kind {
        TypeKind::Scalar(ScalarType::RawPointer(_)) => {
            unreachable!("parameter-transfer validation excludes raw-pointer values")
        }
        TypeKind::Scalar(ScalarType::Reference {
            referent,
            permission,
        }) => {
            let external = ExternalRegionId(
                u32::try_from(state.external_regions.len())
                    .expect("external region count exceeds u32::MAX"),
            );
            state.external_regions.push(ExternalRegionState {
                ty: *referent,
                state: ObjectState::fully_live_unknown(types, *referent),
            });
            let authority = allocate_reference_authority(
                &mut state.reference_authorities,
                ActiveReferenceAuthority {
                    target: ValidationRegion::external(external),
                    permission: *permission,
                    parent: None,
                    carriers: 1,
                    is_result_origin,
                },
            );
            ValidationValue::Scalar(ValidationScalar::Reference(authority))
        }
        TypeKind::Scalar(_) => {
            debug_assert!(!is_result_origin);
            ValidationValue::Scalar(ValidationScalar::NonPointer)
        }
        TypeKind::Struct(fields) => {
            debug_assert!(!is_result_origin);
            ValidationValue::Struct(
                fields
                    .iter()
                    .map(|field| parameter_validation_value(types, field.ty, state, false))
                    .collect(),
            )
        }
    }
}

fn unknown_result_validation_value(types: &TypeTable, ty: TypeId) -> ValidationValue {
    match &types.get(ty).expect("validated type identity exists").kind {
        TypeKind::Scalar(ScalarType::RawPointer(_) | ScalarType::Reference { .. }) => {
            unreachable!("ordinary result-transfer validation excludes pointer/reference values")
        }
        TypeKind::Scalar(_) => ValidationValue::Scalar(ValidationScalar::NonPointer),
        TypeKind::Struct(fields) => ValidationValue::Struct(
            fields
                .iter()
                .map(|field| unknown_result_validation_value(types, field.ty))
                .collect(),
        ),
    }
}

fn clone_validation_value(
    types: &TypeTable,
    ty: TypeId,
    state: &ObjectState,
    authorities: &mut [Option<ActiveReferenceAuthority>],
) -> ValidationValue {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(LeafState::Live(value))) => {
            if let ValidationScalar::Reference(authority) = value {
                let active = authorities[authority.0 as usize]
                    .as_mut()
                    .expect("live reference carrier names an active authority");
                active.carriers = active
                    .carriers
                    .checked_add(1)
                    .expect("validation reference carrier count overflow");
            }
            ValidationValue::Scalar(value.clone())
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states))
            if fields.len() == states.len() =>
        {
            ValidationValue::Struct(
                fields
                    .iter()
                    .zip(states)
                    .map(|(field, state)| {
                        clone_validation_value(types, field.ty, state, authorities)
                    })
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
            TypeKind::Scalar(ScalarType::Reference { .. }),
            ObjectState::Leaf(leaf),
            ValidationValue::Scalar(value @ ValidationScalar::Reference(_)),
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

fn destroy_place_live(types: &TypeTable, body: &Body, state: &mut ValidationState, place: &Place) {
    let ty = body
        .local(place.local)
        .and_then(|local| types.project_type(local.ty, &place.projections))
        .expect("structural validation establishes the destroyed place type");
    let (locals, authorities) = (&mut state.locals, &mut state.reference_authorities);
    let object = place_state_mut(locals, place);
    destroy_object_live(types, ty, object, authorities);
    normalize_reference_authorities(authorities);
}

fn destroy_region_live(
    types: &TypeTable,
    body: &Body,
    state: &mut ValidationState,
    region: &ValidationRegion,
) {
    let ty = region_type(types, body, state, region);
    match region.root {
        ValidationRegionRoot::Local(local) => {
            let place = Place {
                local,
                projections: region.projections.clone(),
            };
            let (locals, authorities) = (&mut state.locals, &mut state.reference_authorities);
            destroy_object_live(types, ty, place_state_mut(locals, &place), authorities);
        }
        ValidationRegionRoot::External(id) => {
            let index = id.0 as usize;
            let (external_regions, authorities) = (
                &mut state.external_regions,
                &mut state.reference_authorities,
            );
            let object =
                projected_state_mut(&mut external_regions[index].state, &region.projections);
            destroy_object_live(types, ty, object, authorities);
        }
    }
    normalize_reference_authorities(&mut state.reference_authorities);
}

fn destroy_object_live(
    types: &TypeTable,
    ty: TypeId,
    state: &mut ObjectState,
    authorities: &mut [Option<ActiveReferenceAuthority>],
) {
    let definition = types
        .get(ty)
        .expect("validated Core MIR references only known types");
    match (&definition.kind, state) {
        (TypeKind::Scalar(_), ObjectState::Leaf(leaf)) => {
            let old = std::mem::replace(leaf, LeafState::Dead);
            if let LeafState::Live(ValidationScalar::Reference(authority)) = old {
                remove_reference_carrier(authorities, authority);
            } else if matches!(old, LeafState::NeverInitialized) {
                *leaf = LeafState::NeverInitialized;
            }
        }
        (TypeKind::Struct(fields), ObjectState::Aggregate(states)) => {
            for (field, field_state) in fields.iter().zip(states.iter_mut()).rev() {
                destroy_object_live(types, field.ty, field_state, authorities);
            }
        }
        _ => unreachable!("validated object state matches its Core type"),
    }
}

fn destroy_validation_value(
    types: &TypeTable,
    ty: TypeId,
    value: &ValidationValue,
    authorities: &mut [Option<ActiveReferenceAuthority>],
) {
    let definition = types
        .get(ty)
        .expect("validated transient value references only known types");
    match (&definition.kind, value) {
        (
            TypeKind::Scalar(ScalarType::Reference { .. }),
            ValidationValue::Scalar(ValidationScalar::Reference(authority)),
        ) => remove_reference_carrier(authorities, *authority),
        (TypeKind::Scalar(_), ValidationValue::Scalar(_)) => {}
        (TypeKind::Struct(fields), ValidationValue::Struct(values)) => {
            for (field, value) in fields.iter().zip(values).rev() {
                destroy_validation_value(types, field.ty, value, authorities);
            }
        }
        _ => unreachable!("validated transient value matches its Core type"),
    }
}

fn destroy_transient_values(
    types: &TypeTable,
    held: &[(TypeId, ValidationValue)],
    state: &mut ValidationState,
) {
    for (ty, value) in held.iter().rev() {
        destroy_validation_value(types, *ty, value, &mut state.reference_authorities);
    }
    normalize_reference_authorities(&mut state.reference_authorities);
}

fn destroy_transient_values_except(
    types: &TypeTable,
    held: &[(TypeId, ValidationValue)],
    excluded: usize,
    state: &mut ValidationState,
) {
    for (index, (ty, value)) in held.iter().enumerate().rev() {
        if index == excluded {
            continue;
        }
        destroy_validation_value(types, *ty, value, &mut state.reference_authorities);
    }
    normalize_reference_authorities(&mut state.reference_authorities);
}

fn write_region_value(
    types: &TypeTable,
    body: &Body,
    state: &mut ValidationState,
    region: &ValidationRegion,
    ty: TypeId,
    value: ValidationValue,
) {
    debug_assert_eq!(region_type(types, body, state, region), ty);
    match region.root {
        ValidationRegionRoot::Local(local) => {
            let place = Place {
                local,
                projections: region.projections.clone(),
            };
            write_validation_value(types, ty, place_state_mut(&mut state.locals, &place), value);
        }
        ValidationRegionRoot::External(id) => {
            let external = &mut state.external_regions[id.0 as usize];
            let object = projected_state_mut(&mut external.state, &region.projections);
            write_validation_value(types, ty, object, value);
        }
    }
}

fn take_region_value(
    types: &TypeTable,
    body: &Body,
    state: &mut ValidationState,
    region: &ValidationRegion,
    ty: TypeId,
) -> ValidationValue {
    debug_assert_eq!(region_type(types, body, state, region), ty);
    match region.root {
        ValidationRegionRoot::Local(local) => {
            let place = Place {
                local,
                projections: region.projections.clone(),
            };
            take_validation_value(types, ty, place_state_mut(&mut state.locals, &place))
        }
        ValidationRegionRoot::External(id) => {
            let external = &mut state.external_regions[id.0 as usize];
            take_validation_value(
                types,
                ty,
                projected_state_mut(&mut external.state, &region.projections),
            )
        }
    }
}

fn clone_region_value(
    types: &TypeTable,
    body: &Body,
    state: &mut ValidationState,
    region: &ValidationRegion,
    ty: TypeId,
) -> ValidationValue {
    debug_assert_eq!(region_type(types, body, state, region), ty);
    match region.root {
        ValidationRegionRoot::Local(local) => {
            let place = Place {
                local,
                projections: region.projections.clone(),
            };
            let (locals, authorities) = (&state.locals, &mut state.reference_authorities);
            clone_validation_value(types, ty, place_state(locals, &place), authorities)
        }
        ValidationRegionRoot::External(id) => {
            let index = id.0 as usize;
            let (external_regions, authorities) =
                (&state.external_regions, &mut state.reference_authorities);
            clone_validation_value(
                types,
                ty,
                projected_state(&external_regions[index].state, &region.projections),
                authorities,
            )
        }
    }
}

fn region_type(
    types: &TypeTable,
    body: &Body,
    state: &ValidationState,
    region: &ValidationRegion,
) -> TypeId {
    let root_ty = match region.root {
        ValidationRegionRoot::Local(local) => {
            body.local(local)
                .expect("validated local region names a current-function local")
                .ty
        }
        ValidationRegionRoot::External(id) => state.external_regions[id.0 as usize].ty,
    };
    types
        .project_type(root_ty, &region.projections)
        .expect("validated reference region projection is structurally valid")
}

fn region_state<'a>(state: &'a ValidationState, region: &ValidationRegion) -> &'a ObjectState {
    match region.root {
        ValidationRegionRoot::Local(local) => {
            let place = Place {
                local,
                projections: region.projections.clone(),
            };
            place_state(&state.locals, &place)
        }
        ValidationRegionRoot::External(id) => projected_state(
            &state.external_regions[id.0 as usize].state,
            &region.projections,
        ),
    }
}

fn region_state_mut<'a>(
    state: &'a mut ValidationState,
    region: &ValidationRegion,
) -> &'a mut ObjectState {
    match region.root {
        ValidationRegionRoot::Local(local) => {
            let place = Place {
                local,
                projections: region.projections.clone(),
            };
            place_state_mut(&mut state.locals, &place)
        }
        ValidationRegionRoot::External(id) => projected_state_mut(
            &mut state.external_regions[id.0 as usize].state,
            &region.projections,
        ),
    }
}

fn projected_state<'a>(state: &'a ObjectState, projections: &[Projection]) -> &'a ObjectState {
    let mut current = state;
    for projection in projections {
        match (current, projection) {
            (ObjectState::Aggregate(fields), Projection::Field(index)) => {
                current = &fields[*index as usize];
            }
            (ObjectState::Leaf(_), Projection::Field(_)) => {
                unreachable!("structural validation rejects projections from scalar/empty state");
            }
        }
    }
    current
}

fn projected_state_mut<'a>(
    state: &'a mut ObjectState,
    projections: &[Projection],
) -> &'a mut ObjectState {
    let mut current = state;
    for projection in projections {
        match (current, projection) {
            (ObjectState::Aggregate(fields), Projection::Field(index)) => {
                current = &mut fields[*index as usize];
            }
            (ObjectState::Leaf(_), Projection::Field(_)) => {
                unreachable!("structural validation rejects projections from scalar/empty state");
            }
        }
    }
    current
}

fn require_region_fully_live(
    state: &ValidationState,
    region: &ValidationRegion,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if region_state(state, region).fully_live() {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::ReferenceTargetNotLive,
        ))
    }
}

fn is_interior_mutable_region(
    types: &TypeTable,
    body: &Body,
    state: &ValidationState,
    region: &ValidationRegion,
) -> bool {
    let root_ty = match region.root {
        ValidationRegionRoot::Local(local) => {
            body.local(local)
                .expect("validated local region names a current-function local")
                .ty
        }
        ValidationRegionRoot::External(id) => state.external_regions[id.0 as usize].ty,
    };
    is_interior_mutable_from_root(types, root_ty, &region.projections)
}

#[derive(Clone, Debug)]
struct ResolvedReferenceAccess {
    authority: ValidationReferenceAuthorityId,
    permission: ReferencePermission,
    target: ValidationRegion,
    selected_ty: TypeId,
}

fn resolve_reference_access(
    types: &TypeTable,
    body: &Body,
    state: &ValidationState,
    access: &ReferenceAccess,
    requirement: AccessRequirement,
    exact_permission: Option<ReferencePermission>,
    point: &MirPoint,
) -> Result<ResolvedReferenceAccess, MirValidationError> {
    let carrier_place = resolve_authorized_access(
        &state.active_loans,
        &state.reference_authorities,
        &access.reference,
        AccessRequirement::Shared,
        point,
    )?;
    require_fully_live(&state.locals, &carrier_place, point)?;
    let carrier_ty = access_type(types, body, &access.reference, point)?;
    let Some((referent, permission)) = types.reference(carrier_ty) else {
        return Err(point_error(
            point,
            MirValidationErrorKind::ReferenceAccessRequiresReference(carrier_ty),
        ));
    };
    let authority = match place_state(&state.locals, &carrier_place) {
        ObjectState::Leaf(LeafState::Live(ValidationScalar::Reference(authority))) => *authority,
        _ => unreachable!("live stored reference carrier has reference validation metadata"),
    };
    let active = state
        .reference_authorities
        .get(authority.0 as usize)
        .and_then(Option::as_ref)
        .ok_or_else(|| point_error(point, MirValidationErrorKind::ReferenceAuthorityNotActive))?;
    debug_assert_eq!(active.permission, permission);

    if requirement == AccessRequirement::Exclusive && permission == ReferencePermission::Shared {
        return Err(point_error(
            point,
            MirValidationErrorKind::ReferencePermissionRequired(ReferencePermission::Exclusive),
        ));
    }
    if let Some(required) = exact_permission
        && permission != required
    {
        return Err(point_error(
            point,
            MirValidationErrorKind::ReferencePermissionRequired(required),
        ));
    }

    let selected_ty = types
        .project_type(referent, &access.projections)
        .expect("static reference-access validation establishes projection type");
    let target = active.target.projected(&access.projections);
    if reference_delegated_child(
        &state.reference_authorities,
        authority,
        &target,
        requirement,
    ) {
        return Err(point_error(
            point,
            MirValidationErrorKind::ReferenceAccessDelegated,
        ));
    }

    Ok(ResolvedReferenceAccess {
        authority,
        permission,
        target,
        selected_ty,
    })
}

fn allocate_reference_authority(
    authorities: &mut Vec<Option<ActiveReferenceAuthority>>,
    authority: ActiveReferenceAuthority,
) -> ValidationReferenceAuthorityId {
    if let Some(index) = authorities.iter().position(Option::is_none) {
        authorities[index] = Some(authority);
        ValidationReferenceAuthorityId(
            u32::try_from(index).expect("reference authority index exceeds u32::MAX"),
        )
    } else {
        let index = authorities.len();
        authorities.push(Some(authority));
        ValidationReferenceAuthorityId(
            u32::try_from(index).expect("reference authority index exceeds u32::MAX"),
        )
    }
}

fn remove_reference_carrier(
    authorities: &mut [Option<ActiveReferenceAuthority>],
    authority: ValidationReferenceAuthorityId,
) {
    let active = authorities[authority.0 as usize]
        .as_mut()
        .expect("live carrier removal names an active reference authority");
    active.carriers = active
        .carriers
        .checked_sub(1)
        .expect("reference carrier count underflow");
    normalize_reference_authorities(authorities);
}

fn normalize_reference_authorities(authorities: &mut [Option<ActiveReferenceAuthority>]) {
    loop {
        let mut changed = false;

        for index in 0..authorities.len() {
            let Some(active) = authorities[index].as_ref() else {
                continue;
            };
            if active.carriers == 0 && reference_children(authorities, index).is_empty() {
                authorities[index] = None;
                changed = true;
                break;
            }
        }
        if changed {
            continue;
        }

        for index in 0..authorities.len() {
            let Some(active) = authorities[index].as_ref() else {
                continue;
            };
            if active.carriers != 0 || active.is_result_origin {
                continue;
            }
            let children = reference_children(authorities, index);
            if children.len() != 1 {
                continue;
            }
            let child_index = children[0];
            let child = authorities[child_index]
                .as_ref()
                .expect("child index names an active authority");
            if child.target != active.target
                || child.permission.alias_kind() != active.permission.alias_kind()
            {
                continue;
            }
            let parent = active.parent;
            authorities[child_index]
                .as_mut()
                .expect("child index remains active")
                .parent = parent;
            authorities[index] = None;
            changed = true;
            break;
        }

        if !changed {
            break;
        }
    }
}

fn reference_children(
    authorities: &[Option<ActiveReferenceAuthority>],
    parent_index: usize,
) -> Vec<usize> {
    let parent = ValidationReferenceAuthorityId(
        u32::try_from(parent_index).expect("reference authority index exceeds u32::MAX"),
    );
    authorities
        .iter()
        .enumerate()
        .filter_map(|(index, active)| (active.as_ref()?.parent == Some(parent)).then_some(index))
        .collect()
}

fn reference_delegated_child(
    authorities: &[Option<ActiveReferenceAuthority>],
    parent: ValidationReferenceAuthorityId,
    target: &ValidationRegion,
    requirement: AccessRequirement,
) -> bool {
    authorities.iter().any(|active| {
        let Some(active) = active.as_ref() else {
            return false;
        };
        active.parent == Some(parent)
            && active.target.overlaps(target)
            && (requirement == AccessRequirement::Exclusive
                || active.permission.alias_kind() == BorrowKind::Exclusive)
    })
}

fn authority_retains_full_advertised_permission(
    authorities: &[Option<ActiveReferenceAuthority>],
    authority: ValidationReferenceAuthorityId,
    advertised: ReferencePermission,
) -> bool {
    let Some(active) = authorities
        .get(authority.0 as usize)
        .and_then(Option::as_ref)
    else {
        return false;
    };
    if active.permission != advertised {
        return false;
    }
    let requirement = match advertised.alias_kind() {
        BorrowKind::Shared => AccessRequirement::Shared,
        BorrowKind::Exclusive => AccessRequirement::Exclusive,
    };
    !reference_delegated_child(authorities, authority, &active.target, requirement)
}

fn require_shared_reference_result_origin(
    value: &ValidationValue,
    state: &ValidationState,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    let ValidationValue::Scalar(ValidationScalar::Reference(authority)) = value else {
        unreachable!("static validation establishes a scalar Shared-reference result")
    };
    let Some(active) = state
        .reference_authorities
        .get(authority.0 as usize)
        .and_then(Option::as_ref)
    else {
        return Err(point_error(
            point,
            MirValidationErrorKind::SharedReferenceResultOriginMismatch,
        ));
    };
    if !active.is_result_origin {
        return Err(point_error(
            point,
            MirValidationErrorKind::SharedReferenceResultOriginMismatch,
        ));
    }
    if !authority_retains_full_advertised_permission(
        &state.reference_authorities,
        *authority,
        ReferencePermission::Shared,
    ) {
        return Err(point_error(
            point,
            MirValidationErrorKind::ReferenceAuthorityIncomplete,
        ));
    }
    Ok(())
}

fn require_call_value_admissible(
    types: &TypeTable,
    ty: TypeId,
    value: &ValidationValue,
    state: &ValidationState,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    let definition = types
        .get(ty)
        .expect("validated call value type identity exists");
    match (&definition.kind, value) {
        (
            TypeKind::Scalar(ScalarType::Reference { permission, .. }),
            ValidationValue::Scalar(ValidationScalar::Reference(authority)),
        ) => {
            if !authority_retains_full_advertised_permission(
                &state.reference_authorities,
                *authority,
                *permission,
            ) {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ReferenceAuthorityIncomplete,
                ));
            }
            let target = &state.reference_authorities[authority.0 as usize]
                .as_ref()
                .expect("held reference carrier names an active authority")
                .target;
            if !region_state(state, target).fully_live() {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::ReferenceTargetNotLive,
                ));
            }
            Ok(())
        }
        (TypeKind::Scalar(_), ValidationValue::Scalar(_)) => Ok(()),
        (TypeKind::Struct(fields), ValidationValue::Struct(values)) => {
            for (field, value) in fields.iter().zip(values) {
                require_call_value_admissible(types, field.ty, value, state, point)?;
            }
            Ok(())
        }
        _ => unreachable!("static validation guarantees held call value/type compatibility"),
    }
}

fn restore_transferred_referents(
    types: &TypeTable,
    ty: TypeId,
    value: &ValidationValue,
    state: &mut ValidationState,
) {
    let definition = types
        .get(ty)
        .expect("validated call value type identity exists");
    match (&definition.kind, value) {
        (
            TypeKind::Scalar(ScalarType::Reference { referent, .. }),
            ValidationValue::Scalar(ValidationScalar::Reference(authority)),
        ) => {
            let target = state.reference_authorities[authority.0 as usize]
                .as_ref()
                .expect("held reference carrier names an active authority")
                .target
                .clone();
            *region_state_mut(state, &target) = ObjectState::fully_live_unknown(types, *referent);
        }
        (TypeKind::Scalar(_), ValidationValue::Scalar(_)) => {}
        (TypeKind::Struct(fields), ValidationValue::Struct(values)) => {
            for (field, value) in fields.iter().zip(values) {
                restore_transferred_referents(types, field.ty, value, state);
            }
        }
        _ => unreachable!("static validation guarantees held call value/type compatibility"),
    }
}

fn require_external_regions_fully_live(
    state: &ValidationState,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if state
        .external_regions
        .iter()
        .all(|external| external.state.fully_live())
    {
        Ok(())
    } else {
        Err(point_error(
            point,
            MirValidationErrorKind::ExternalReferentNotLive,
        ))
    }
}

fn cleanup_function(
    types: &TypeTable,
    body: &Body,
    state: &mut ValidationState,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    state.active_loans.fill(None);
    for index in (0..body.locals.len()).rev() {
        let local = LocalId(u32::try_from(index).expect("local index exceeds u32::MAX"));
        let ty = body.locals[index].ty;
        {
            let (locals, authorities) = (&mut state.locals, &mut state.reference_authorities);
            destroy_object_live(types, ty, &mut locals[index], authorities);
        }
        normalize_reference_authorities(&mut state.reference_authorities);
        if state.reference_authorities.iter().any(|active| {
            active.as_ref().is_some_and(|active| {
                matches!(active.target.root, ValidationRegionRoot::Local(target) if target == local)
            })
        }) {
            return Err(point_error(
                point,
                MirValidationErrorKind::ReferenceOutlivesStorage(local),
            ));
        }
    }
    Ok(())
}

fn raw_pointer_target_at(locals: &[ObjectState], place: &Place) -> Place {
    match place_state(locals, place) {
        ObjectState::Leaf(LeafState::Live(ValidationScalar::RawPointer(target))) => target.clone(),
        _ => unreachable!("validated raw-pointer access reaches exact live pointer metadata"),
    }
}

fn raw_target_conflicts(
    active_loans: &[Option<ActiveLoan>],
    authorities: &[Option<ActiveReferenceAuthority>],
    place: &Place,
    requirement: AccessRequirement,
) -> bool {
    let loan_conflict = active_loans.iter().any(|active| {
        active.as_ref().is_some_and(|active| {
            active.place.overlaps(place)
                && (requirement == AccessRequirement::Exclusive
                    || active.kind == BorrowKind::Exclusive)
        })
    });
    if loan_conflict {
        return true;
    }
    let target = ValidationRegion::local(place);
    authorities.iter().any(|active| {
        active.as_ref().is_some_and(|active| {
            active.target.overlaps(&target)
                && (requirement == AccessRequirement::Exclusive
                    || active.permission.alias_kind() == BorrowKind::Exclusive)
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

fn reference_conflict_with_local(
    authorities: &[Option<ActiveReferenceAuthority>],
    place: &Place,
    requested: BorrowKind,
) -> bool {
    let target = ValidationRegion::local(place);
    authorities.iter().any(|active| {
        active.as_ref().is_some_and(|active| {
            active.target.overlaps(&target)
                && (requested == BorrowKind::Exclusive
                    || active.permission.alias_kind() == BorrowKind::Exclusive)
        })
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
    authorities: &[Option<ActiveReferenceAuthority>],
    access: &PlaceAccess,
    requirement: AccessRequirement,
    point: &MirPoint,
) -> Result<Place, MirValidationError> {
    match access {
        PlaceAccess::Direct(place) => {
            authorize_direct_access(active_loans, authorities, place, requirement, point)?;
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
    authorities: &[Option<ActiveReferenceAuthority>],
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

    let target = ValidationRegion::local(place);
    if authorities.iter().any(|active| {
        active.as_ref().is_some_and(|active| {
            active.target.overlaps(&target)
                && (requirement == AccessRequirement::Exclusive
                    || active.permission.alias_kind() == BorrowKind::Exclusive)
        })
    }) {
        return Err(point_error(
            point,
            MirValidationErrorKind::DirectAccessConflictWithReferenceAuthority {
                place: place.clone(),
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
                unreachable!("structural validation rejects projections from scalar/empty state");
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
                unreachable!("structural validation rejects projections from scalar/empty state");
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
