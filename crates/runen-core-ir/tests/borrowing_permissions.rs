use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, ScalarType, Statement, Terminator,
    TypeDef, TypeId, TypeTable, Value, validate_body,
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

fn i64_type() -> (TypeTable, TypeId) {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    (types, ty)
}

fn error_kind(body: Body) -> MirValidationErrorKind {
    validate_body(body).expect_err("test MIR must be rejected").kind
}

#[test]
fn overlapping_exclusive_root_loans_conflict() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("first", ty), LoanDecl::new("second", ty)],
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

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::BorrowConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn direct_drop_is_rejected_under_shared_borrow() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("shared", ty)],
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

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::DirectAccessConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn shared_loan_allows_read_and_copy() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", ty, false),
            LocalDecl::new("copy", ty, false),
        ],
        vec![LoanDecl::new("shared", ty)],
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
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("shared", ty)],
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

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn shared_loan_rejects_drop() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("shared", ty)],
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

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn exclusive_loan_allows_read_copy_and_drop() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", ty, true),
            LocalDecl::new("copy", ty, false),
        ],
        vec![LoanDecl::new("exclusive", ty)],
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
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("shared", ty)],
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

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanAlreadyActive(LoanId(0))
    );
}

#[test]
fn ending_inactive_loan_is_rejected() {
    let (types, ty) = i64_type();
    let body = one_block(
        types,
        Vec::new(),
        vec![LoanDecl::new("shared", ty)],
        vec![Statement::EndBorrow { loan: LoanId(0) }],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanNotActive(LoanId(0))
    );
}
