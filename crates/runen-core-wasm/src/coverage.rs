use runen_core_ir::{
    BasicBlockId, Function, FunctionId, LocalId, Operand, PersistentId, Place, PlaceAccess,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeId, TypeKind, TypeTable,
    ValidatedProgram, Value,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoverageLocation {
    Program,
    Persistent(PersistentId),
    Function(FunctionId),
    Result {
        function: FunctionId,
    },
    Parameter {
        function: FunctionId,
        local: LocalId,
    },
    Local {
        function: FunctionId,
        local: LocalId,
    },
    Statement {
        function: FunctionId,
        block: BasicBlockId,
        statement: usize,
    },
    Terminator {
        function: FunctionId,
        block: BasicBlockId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedTypeCategory {
    Floating,
    RawPointer,
    SafeReference,
    Callable,
    TrackedFixture,
    StructuralAggregate,
    InteriorMutable,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedOperandKind {
    FloatingConstant,
    TrackedFixtureConstant,
    StructuralConstant,
    PersistentSharedRoot,
    FunctionValue,
    RawMove,
    AddressOf,
    ReferenceRoot,
    ReferenceReborrow,
    ReferenceMove,
    ReferenceCopy,
    LoanAccess,
    ProjectedAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedStatementKind {
    Floating,
    Borrowing,
    Reference,
    RawPointer,
    InteriorMutation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedTerminatorKind {
    ExternalCall,
    IndirectCall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoverageErrorKind {
    ProgramTooLarge,
    ExternalCallables,
    LoanDeclarations,
    UnsupportedType {
        ty: TypeId,
        category: UnsupportedTypeCategory,
    },
    UnsupportedSafeReferenceResultContract,
    UnsupportedOperand(UnsupportedOperandKind),
    UnsupportedStatement(UnsupportedStatementKind),
    UnsupportedTerminator(UnsupportedTerminatorKind),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageError {
    pub location: CoverageLocation,
    pub kind: CoverageErrorKind,
}

pub(crate) fn validate(program: &ValidatedProgram) -> Result<(), CoverageError> {
    let program = program.as_program();

    // Diagnose represented function behavior before passive whole-program declarations. This
    // preserves whole-program conservative rejection while making actively consumed unsupported
    // operands/statements/terminators observable at their precise Core location.
    for (function_index, function) in program.functions.iter().enumerate() {
        let function_id = checked_function_id(function_index)?;
        validate_function(&program.types, function_id, function)?;
    }

    for (persistent_index, persistent) in program.persistent.iter().enumerate() {
        let persistent_id = checked_persistent_id(persistent_index)?;
        require_supported_type(
            &program.types,
            persistent.ty,
            CoverageLocation::Persistent(persistent_id),
        )?;
    }

    if !program.external_callables.is_empty() {
        return Err(program_error(CoverageErrorKind::ExternalCallables));
    }

    Ok(())
}

fn validate_function(
    types: &TypeTable,
    function_id: FunctionId,
    function: &Function,
) -> Result<(), CoverageError> {
    if !matches!(
        function.safe_reference_result_contract,
        SafeReferenceResultContract::None
    ) {
        return Err(function_error(
            function_id,
            CoverageErrorKind::UnsupportedSafeReferenceResultContract,
        ));
    }

    // Behavior is inspected before passive declarations/types so the diagnostic identifies the
    // unsupported operation actually consumed by the function whenever one exists.
    for (block_index, block) in function.body.blocks.iter().enumerate() {
        let block_id = checked_block_id(block_index)?;
        for (statement_index, statement) in block.statements.iter().enumerate() {
            let location = CoverageLocation::Statement {
                function: function_id,
                block: block_id,
                statement: statement_index,
            };
            validate_statement(statement, &location)?;
        }
        let location = CoverageLocation::Terminator {
            function: function_id,
            block: block_id,
        };
        validate_terminator(&block.terminator, &location)?;
    }

    if !function.body.loans.is_empty() {
        return Err(function_error(
            function_id,
            CoverageErrorKind::LoanDeclarations,
        ));
    }

    if let Some(result) = function.result {
        require_supported_type(
            types,
            result,
            CoverageLocation::Result {
                function: function_id,
            },
        )?;
    }

    for (local_index, local) in function.body.locals.iter().enumerate() {
        let local_id = checked_local_id(local_index)?;
        let location = if function.parameters.contains(&local_id) {
            CoverageLocation::Parameter {
                function: function_id,
                local: local_id,
            }
        } else {
            CoverageLocation::Local {
                function: function_id,
                local: local_id,
            }
        };
        require_supported_type(types, local.ty, location)?;
    }

    Ok(())
}

fn require_supported_type(
    types: &TypeTable,
    ty: TypeId,
    location: CoverageLocation,
) -> Result<(), CoverageError> {
    let Some(definition) = types.get(ty) else {
        return Err(CoverageError {
            location,
            kind: CoverageErrorKind::UnsupportedType {
                ty,
                category: UnsupportedTypeCategory::Unknown,
            },
        });
    };
    if definition.interior_mutable {
        return Err(CoverageError {
            location,
            kind: CoverageErrorKind::UnsupportedType {
                ty,
                category: UnsupportedTypeCategory::InteriorMutable,
            },
        });
    }
    let category = match &definition.kind {
        TypeKind::Scalar(
            ScalarType::Bool
            | ScalarType::I8
            | ScalarType::I16
            | ScalarType::I32
            | ScalarType::I64
            | ScalarType::U8
            | ScalarType::U16
            | ScalarType::U32
            | ScalarType::U64,
        ) => return Ok(()),
        TypeKind::Scalar(ScalarType::F16 | ScalarType::F32 | ScalarType::F64) => {
            UnsupportedTypeCategory::Floating
        }
        TypeKind::Scalar(ScalarType::RawPointer(_)) => UnsupportedTypeCategory::RawPointer,
        TypeKind::Scalar(ScalarType::Reference { .. }) => UnsupportedTypeCategory::SafeReference,
        TypeKind::Scalar(ScalarType::Callable(_)) => UnsupportedTypeCategory::Callable,
        TypeKind::Scalar(ScalarType::TrackedFixture) => UnsupportedTypeCategory::TrackedFixture,
        TypeKind::Struct(_) => UnsupportedTypeCategory::StructuralAggregate,
    };
    Err(CoverageError {
        location,
        kind: CoverageErrorKind::UnsupportedType { ty, category },
    })
}

fn validate_statement(
    statement: &Statement,
    location: &CoverageLocation,
) -> Result<(), CoverageError> {
    match statement {
        Statement::Init { dst, src } => {
            validate_place(dst, location)?;
            validate_operand(src, location)
        }
        Statement::IntegerAdd {
            dst, left, right, ..
        }
        | Statement::IntegerSub {
            dst, left, right, ..
        }
        | Statement::IntegerMul {
            dst, left, right, ..
        }
        | Statement::IntegerXor {
            dst, left, right, ..
        }
        | Statement::IntegerOr {
            dst, left, right, ..
        }
        | Statement::IntegerEq {
            dst, left, right, ..
        }
        | Statement::IntegerLt {
            dst, left, right, ..
        } => {
            validate_place(dst, location)?;
            validate_operand(left, location)?;
            validate_operand(right, location)
        }
        Statement::Read { src } => validate_access(src, location),
        Statement::Assign { dst, src } => {
            validate_access(dst, location)?;
            validate_operand(src, location)
        }
        Statement::Drop { place } => validate_access(place, location),
        Statement::FloatAdd { .. }
        | Statement::FloatSub { .. }
        | Statement::FloatMul { .. }
        | Statement::FloatDiv { .. } => Err(CoverageError {
            location: location.clone(),
            kind: CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::Floating),
        }),
        Statement::Borrow { .. } | Statement::EndBorrow { .. } => Err(CoverageError {
            location: location.clone(),
            kind: CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::Borrowing),
        }),
        Statement::ReferenceRead { .. }
        | Statement::ReferenceAssign { .. }
        | Statement::ReferenceInteriorAssign { .. }
        | Statement::ReferenceDrop { .. } => Err(CoverageError {
            location: location.clone(),
            kind: CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::Reference),
        }),
        Statement::RawRead { .. } | Statement::RawAssign { .. } => Err(CoverageError {
            location: location.clone(),
            kind: CoverageErrorKind::UnsupportedStatement(UnsupportedStatementKind::RawPointer),
        }),
        Statement::InteriorAssign { .. } => Err(CoverageError {
            location: location.clone(),
            kind: CoverageErrorKind::UnsupportedStatement(
                UnsupportedStatementKind::InteriorMutation,
            ),
        }),
    }
}

fn validate_terminator(
    terminator: &Terminator,
    location: &CoverageLocation,
) -> Result<(), CoverageError> {
    match terminator {
        Terminator::Goto(_) | Terminator::Fault(_) => Ok(()),
        Terminator::Branch { condition, .. } => validate_operand(condition, location),
        Terminator::Call {
            arguments,
            destination,
            ..
        } => {
            for argument in arguments {
                validate_operand(argument, location)?;
            }
            if let Some(destination) = destination {
                validate_place(destination, location)?;
            }
            Ok(())
        }
        Terminator::Return(result) => {
            if let Some(result) = result {
                validate_operand(result, location)?;
            }
            Ok(())
        }
        Terminator::ExternalCall { .. } => Err(CoverageError {
            location: location.clone(),
            kind: CoverageErrorKind::UnsupportedTerminator(UnsupportedTerminatorKind::ExternalCall),
        }),
        Terminator::IndirectCall { .. } => Err(CoverageError {
            location: location.clone(),
            kind: CoverageErrorKind::UnsupportedTerminator(UnsupportedTerminatorKind::IndirectCall),
        }),
    }
}

fn validate_operand(operand: &Operand, location: &CoverageLocation) -> Result<(), CoverageError> {
    match operand {
        Operand::Constant(
            Value::Bool(_)
            | Value::I8(_)
            | Value::I16(_)
            | Value::I32(_)
            | Value::I64(_)
            | Value::U8(_)
            | Value::U16(_)
            | Value::U32(_)
            | Value::U64(_),
        ) => Ok(()),
        Operand::Constant(Value::F16(_) | Value::F32(_) | Value::F64(_)) => {
            unsupported_operand(location, UnsupportedOperandKind::FloatingConstant)
        }
        Operand::Constant(Value::TrackedFixture(_)) => {
            unsupported_operand(location, UnsupportedOperandKind::TrackedFixtureConstant)
        }
        Operand::Constant(Value::Struct(_)) => {
            unsupported_operand(location, UnsupportedOperandKind::StructuralConstant)
        }
        Operand::Move(access) | Operand::Copy(access) => validate_access(access, location),
        Operand::PersistentRead(_) => Ok(()),
        Operand::PersistentSharedRoot(_) => {
            unsupported_operand(location, UnsupportedOperandKind::PersistentSharedRoot)
        }
        Operand::FunctionValue(_) => {
            unsupported_operand(location, UnsupportedOperandKind::FunctionValue)
        }
        Operand::RawMove(_) => unsupported_operand(location, UnsupportedOperandKind::RawMove),
        Operand::AddressOf(_) => unsupported_operand(location, UnsupportedOperandKind::AddressOf),
        Operand::ReferenceRoot { .. } => {
            unsupported_operand(location, UnsupportedOperandKind::ReferenceRoot)
        }
        Operand::ReferenceReborrow { .. } => {
            unsupported_operand(location, UnsupportedOperandKind::ReferenceReborrow)
        }
        Operand::ReferenceMove(_) => {
            unsupported_operand(location, UnsupportedOperandKind::ReferenceMove)
        }
        Operand::ReferenceCopy(_) => {
            unsupported_operand(location, UnsupportedOperandKind::ReferenceCopy)
        }
    }
}

fn validate_access(access: &PlaceAccess, location: &CoverageLocation) -> Result<(), CoverageError> {
    match access {
        PlaceAccess::Direct(place) => validate_place(place, location),
        PlaceAccess::Loan { .. } => {
            unsupported_operand(location, UnsupportedOperandKind::LoanAccess)
        }
    }
}

fn validate_place(place: &Place, location: &CoverageLocation) -> Result<(), CoverageError> {
    if place.projections.is_empty() {
        Ok(())
    } else {
        unsupported_operand(location, UnsupportedOperandKind::ProjectedAccess)
    }
}

fn unsupported_operand(
    location: &CoverageLocation,
    kind: UnsupportedOperandKind,
) -> Result<(), CoverageError> {
    Err(CoverageError {
        location: location.clone(),
        kind: CoverageErrorKind::UnsupportedOperand(kind),
    })
}

fn checked_function_id(index: usize) -> Result<FunctionId, CoverageError> {
    u32::try_from(index)
        .map(FunctionId)
        .map_err(|_| program_error(CoverageErrorKind::ProgramTooLarge))
}

fn checked_persistent_id(index: usize) -> Result<PersistentId, CoverageError> {
    u32::try_from(index)
        .map(PersistentId)
        .map_err(|_| program_error(CoverageErrorKind::ProgramTooLarge))
}

fn checked_block_id(index: usize) -> Result<BasicBlockId, CoverageError> {
    u32::try_from(index)
        .map(BasicBlockId)
        .map_err(|_| program_error(CoverageErrorKind::ProgramTooLarge))
}

fn checked_local_id(index: usize) -> Result<LocalId, CoverageError> {
    u32::try_from(index)
        .map(LocalId)
        .map_err(|_| program_error(CoverageErrorKind::ProgramTooLarge))
}

fn program_error(kind: CoverageErrorKind) -> CoverageError {
    CoverageError {
        location: CoverageLocation::Program,
        kind,
    }
}

fn function_error(function: FunctionId, kind: CoverageErrorKind) -> CoverageError {
    CoverageError {
        location: CoverageLocation::Function(function),
        kind,
    }
}
