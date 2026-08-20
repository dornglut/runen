mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, Program, Projection, ScalarType, Statement,
    Terminator, TypeDef, TypeId, TypeTable, Value, validate_program,
};
use support::one_function_program;

fn one_block(
    types: TypeTable,
    locals: Vec<LocalDecl>,
    loans: Vec<LoanDecl>,
    statements: Vec<Statement>,
) -> Program {
    one_function_program(
        types,
        Body {
            locals,
            loans,
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(statements, Terminator::Return(None))],
        },
    )
}

fn error_kind(program: Program) -> MirValidationErrorKind {
    validate_program(program).expect_err("invalid MIR").kind
}

#[test]
fn raw_pointer_pointee_recursion_is_not_structural_recursion() {
    let mut types = TypeTable::new();
    let node = TypeId(0);
    let node_pointer = TypeId(1);
    assert_eq!(
        types.push(TypeDef::structure(
            "Node",
            vec![Field::new("next", node_pointer)],
        )),
        node
    );
    assert_eq!(types.push(TypeDef::raw_pointer("NodePtr", node)), node_pointer);

    validate_program(one_block(types, Vec::new(), Vec::new(), Vec::new()))
        .expect("raw-pointer indirection may close an otherwise finite recursive shape");
}

#[test]
fn direct_structural_recursion_remains_invalid() {
    let mut types = TypeTable::new();
    let recursive = TypeId(0);
    assert_eq!(
        types.push(TypeDef::structure(
            "Recursive",
            vec![Field::new("self", recursive)],
        )),
        recursive
    );

    assert_eq!(
        error_kind(one_block(types, Vec::new(), Vec::new(), Vec::new())),
        MirValidationErrorKind::RecursiveType(recursive)
    );
}

#[test]
fn raw_pointer_pointee_type_must_exist() {
    let mut types = TypeTable::new();
    let unknown = TypeId(42);
    types.push(TypeDef::raw_pointer("DanglingType", unknown));

    assert_eq!(
        error_kind(one_block(types, Vec::new(), Vec::new(), Vec::new())),
        MirValidationErrorKind::UnknownType(unknown)
    );
}

#[test]
fn address_of_requires_pointer_to_exact_target_type() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let bool_ty = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let i64_pointer = types.push(TypeDef::raw_pointer("i64_ptr", i64_ty));
    let dst = Place::local(LocalId(0));
    let bool_storage = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("ptr", i64_pointer, false),
            LocalDecl::new("flag", bool_ty, false),
        ],
        Vec::new(),
        vec![Statement::Init {
            dst,
            src: Operand::AddressOf(bool_storage.into()),
        }],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::TypeMismatch {
            expected: i64_pointer,
        }
    );
}

#[test]
fn address_of_does_not_require_initialized_pointee_storage() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![Statement::Init {
            dst: pointer,
            src: Operand::AddressOf(target.into()),
        }],
    );

    validate_program(body).expect("address formation names storage rather than reading its value");
}

#[test]
fn address_of_dead_storage_is_valid_while_storage_extent_continues() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let moved = Place::local(LocalId(1));
    let pointer = Place::local(LocalId(2));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("moved", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: moved,
                src: Operand::Move(target.clone().into()),
            },
            Statement::Init {
                dst: pointer,
                src: Operand::AddressOf(target.into()),
            },
        ],
    );

    validate_program(body).expect("ending a stored-value lifetime does not end local storage extent");
}

#[test]
fn direct_address_of_can_coexist_with_shared_loan() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![LoanDecl::new("shared", value_ty)],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: target.clone().into(),
            },
            Statement::Init {
                dst: pointer,
                src: Operand::AddressOf(target.into()),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_program(body).expect("address formation has only a shared alias requirement");
}

#[test]
fn direct_address_of_is_blocked_by_overlapping_exclusive_loan() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![LoanDecl::new("exclusive", value_ty)],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: target.clone().into(),
            },
            Statement::Init {
                dst: pointer,
                src: Operand::AddressOf(target.clone().into()),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::DirectAccessConflict {
            place: target,
            loan: LoanId(0),
        }
    );
}

#[test]
fn address_of_through_shared_or_exclusive_loan_uses_shared_authority() {
    for kind in [BorrowKind::Shared, BorrowKind::Exclusive] {
        let mut types = TypeTable::new();
        let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
        let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
        let target = Place::local(LocalId(0));
        let pointer = Place::local(LocalId(1));
        let body = one_block(
            types,
            vec![
                LocalDecl::new("target", value_ty, false),
                LocalDecl::new("pointer", pointer_ty, false),
            ],
            vec![LoanDecl::new("loan", value_ty)],
            vec![
                Statement::Init {
                    dst: target.clone(),
                    src: Operand::Constant(Value::I64(1)),
                },
                Statement::Borrow {
                    loan: LoanId(0),
                    kind,
                    src: target.into(),
                },
                Statement::Init {
                    dst: pointer,
                    src: Operand::AddressOf(PlaceAccess::loan(LoanId(0))),
                },
                Statement::EndBorrow { loan: LoanId(0) },
            ],
        );

        validate_program(body).expect("both loan kinds include shared address-formation authority");
    }
}

#[test]
fn raw_pointer_values_are_copyable() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let first = Place::local(LocalId(1));
    let second = Place::local(LocalId(2));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("first", pointer_ty, false),
            LocalDecl::new("second", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: first.clone(),
                src: Operand::AddressOf(target.into()),
            },
            Statement::Init {
                dst: second,
                src: Operand::Copy(first.into()),
            },
        ],
    );

    validate_program(body).expect("raw-pointer values are ordinary copyable owned values");
}

#[test]
fn address_of_rejects_invalid_structural_projection() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let pointer = Place::local(LocalId(1));
    let invalid = Place {
        local: LocalId(0),
        projections: vec![Projection::Field(0)],
    };
    let body = one_block(
        types,
        vec![
            LocalDecl::new("scalar", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![Statement::Init {
            dst: pointer,
            src: Operand::AddressOf(invalid.clone().into()),
        }],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::InvalidProjection(invalid)
    );
}
