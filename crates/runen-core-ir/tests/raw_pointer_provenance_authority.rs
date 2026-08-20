mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, Program, ScalarType, Statement,
    Terminator, TypeDef, TypeTable, Value, validate_program,
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
fn shared_child_preserves_parent_shared_authority_for_address_formation() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![
            LoanDecl::new("parent", value_ty),
            LoanDecl::new("child", value_ty),
        ],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: target.into(),
            },
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Shared,
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(PlaceAccess::loan(LoanId(0))),
            },
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_program(body).expect("a shared child leaves parent shared authority available");
}

#[test]
fn exclusive_child_suspends_parent_address_formation_on_overlap() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![
            LoanDecl::new("parent", value_ty),
            LoanDecl::new("child", value_ty),
        ],
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
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(PlaceAccess::loan(LoanId(0))),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanAccessDelegated {
            loan: LoanId(0),
            child: LoanId(1),
            place: target,
        }
    );
}

#[test]
fn disjoint_exclusive_child_does_not_block_parent_address_formation_on_sibling() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", value_ty), Field::new("right", value_ty)],
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let root = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        vec![
            LoanDecl::new("parent", pair_ty),
            LoanDecl::new("left", value_ty),
        ],
        vec![
            Statement::Init {
                dst: root.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: root.into(),
            },
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(0)).field(0),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(PlaceAccess::loan(LoanId(0)).field(1)),
            },
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_program(body).expect("delegation constrains only overlapping structural storage");
}

#[test]
fn address_formation_through_exclusive_loan_does_not_require_live_pointee() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("moved", value_ty, false),
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
                src: target.into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(PlaceAccess::loan(LoanId(0))),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(PlaceAccess::loan(LoanId(0))),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_program(body).expect("loan authority may still name Dead storage for address formation");
}

#[test]
fn address_formation_through_inactive_loan_is_rejected() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let body = one_block(
        types,
        vec![LocalDecl::new("pointer", pointer_ty, false)],
        vec![LoanDecl::new("inactive", value_ty)],
        vec![Statement::Init {
            dst: Place::local(LocalId(0)),
            src: Operand::AddressOf(PlaceAccess::loan(LoanId(0))),
        }],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanNotActive(LoanId(0))
    );
}
