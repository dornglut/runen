use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_body,
};

fn one_block(
    types: TypeTable,
    locals: Vec<LocalDecl>,
    loans: Vec<LoanDecl>,
    statements: Vec<Statement>,
) -> Body {
    Body {
        types,
        locals,
        loans,
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, Terminator::Return)],
    }
}

#[test]
fn overlapping_exclusive_root_loans_conflict() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, true)],
        vec![
            LoanDecl::new("first", i64_ty),
            LoanDecl::new("second", i64_ty),
        ],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                place: value.clone(),
            },
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                place: value.clone(),
            },
        ],
    );

    let error = validate_body(body).expect_err("overlapping exclusive loans must conflict");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::BorrowConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn direct_drop_is_rejected_under_shared_borrow() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![LoanDecl::new("shared", i64_ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value.clone(),
            },
            Statement::Drop {
                place: value.clone().into(),
            },
        ],
    );

    let error = validate_body(body).expect_err("shared borrow blocks direct destruction");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn shared_loan_allows_read_and_copy() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", i64_ty, false),
            LocalDecl::new("copy", i64_ty, false),
        ],
        vec![LoanDecl::new("shared", i64_ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value,
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(PlaceAccess::loan(LoanId(0))),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_body(body).expect("shared loan permits read and copy");
}

#[test]
fn shared_loan_rejects_assignment() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, true)],
        vec![LoanDecl::new("shared", i64_ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value,
            },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    let error = validate_body(body).expect_err("shared loan cannot assign");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn shared_loan_rejects_drop() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, true)],
        vec![LoanDecl::new("shared", i64_ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value,
            },
            Statement::Drop {
                place: PlaceAccess::loan(LoanId(0)),
            },
        ],
    );

    let error = validate_body(body).expect_err("shared loan cannot destroy");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn exclusive_loan_allows_copy_and_drop() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", i64_ty, true),
            LocalDecl::new("copy", i64_ty, false),
        ],
        vec![LoanDecl::new("exclusive", i64_ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                place: value,
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(PlaceAccess::loan(LoanId(0))),
            },
            Statement::Drop {
                place: PlaceAccess::loan(LoanId(0)),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_body(body).expect("exclusive loan permits read, copy, and drop");
}

#[test]
fn already_active_loan_cannot_begin_again() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![LoanDecl::new("shared", i64_ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value.clone(),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value,
            },
        ],
    );

    let error = validate_body(body).expect_err("active loan identity cannot restart");
    assert_eq!(error.kind, MirValidationErrorKind::LoanAlreadyActive(LoanId(0)));
}

#[test]
fn ending_inactive_loan_is_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));

    let body = one_block(
        types,
        Vec::new(),
        vec![LoanDecl::new("shared", i64_ty)],
        vec![Statement::EndBorrow { loan: LoanId(0) }],
    );

    let error = validate_body(body).expect_err("inactive loan cannot end");
    assert_eq!(error.kind, MirValidationErrorKind::LoanNotActive(LoanId(0)));
}
