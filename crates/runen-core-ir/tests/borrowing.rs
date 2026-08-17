use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, Projection, ScalarType, Statement,
    Terminator, TypeDef, TypeId, TypeTable, Value, validate_body,
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
    validate_body(body).expect_err("invalid MIR").kind
}

fn borrow(loan: u32, kind: BorrowKind, place: Place) -> Statement {
    Statement::Borrow {
        loan: LoanId(loan),
        kind,
        src: place.into(),
    }
}

#[test]
fn structural_place_overlap_is_address_independent() {
    let root = Place::local(LocalId(0));
    let left = root.clone().field(0);
    let nested = left.clone().field(0);
    let right = root.clone().field(1);

    assert!(root.overlaps(&left));
    assert!(left.overlaps(&root));
    assert!(left.overlaps(&nested));
    assert!(!left.overlaps(&right));
    assert!(!root.overlaps(&Place::local(LocalId(1))));
}

#[test]
fn overlapping_shared_root_loans_are_valid() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("a", ty), LoanDecl::new("b", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            borrow(0, BorrowKind::Shared, value.clone()),
            borrow(1, BorrowKind::Shared, value.clone()),
            Statement::Read { src: value.into() },
        ],
    );

    validate_body(body).expect("overlapping shared root loans are valid");
}

#[test]
fn exclusive_root_borrow_conflicts_with_overlapping_shared_loan() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("shared", ty), LoanDecl::new("exclusive", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            borrow(0, BorrowKind::Shared, value.clone()),
            borrow(1, BorrowKind::Exclusive, value.clone()),
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
fn disjoint_exclusive_field_loans_are_valid() {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", scalar), Field::new("right", scalar)],
    ));
    let root = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair, true)],
        vec![LoanDecl::new("left", scalar), LoanDecl::new("right", scalar)],
        vec![
            Statement::Init {
                dst: root.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            borrow(0, BorrowKind::Exclusive, root.clone().field(0)),
            borrow(1, BorrowKind::Exclusive, root.field(1)),
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(3)),
            },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(1)),
                src: Operand::Constant(Value::I64(4)),
            },
        ],
    );

    validate_body(body).expect("disjoint sibling fields may be borrowed exclusively");
}

#[test]
fn direct_non_consuming_access_survives_shared_borrow() {
    let (types, ty) = i64_type();
    let source = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", ty, false),
            LocalDecl::new("copy", ty, false),
        ],
        vec![LoanDecl::new("shared", ty)],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            borrow(0, BorrowKind::Shared, source.clone()),
            Statement::Read {
                src: source.clone().into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(source.into()),
            },
        ],
    );

    validate_body(body).expect("shared root loan permits direct read/copy");
}

#[test]
fn direct_consuming_access_is_blocked_by_shared_borrow() {
    let (types, ty) = i64_type();
    let source = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", ty, false),
            LocalDecl::new("target", ty, false),
        ],
        vec![LoanDecl::new("shared", ty)],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            borrow(0, BorrowKind::Shared, source.clone()),
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(source.clone().into()),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::DirectAccessConflict {
            place: source,
            loan: LoanId(0),
        }
    );
}

#[test]
fn direct_read_is_blocked_by_exclusive_borrow() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("exclusive", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            borrow(0, BorrowKind::Exclusive, value.clone()),
            Statement::Read {
                src: value.clone().into(),
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
fn shared_loan_cannot_consume() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", ty, false),
            LocalDecl::new("target", ty, false),
        ],
        vec![LoanDecl::new("shared", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            borrow(0, BorrowKind::Shared, value),
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(PlaceAccess::loan(LoanId(0))),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn exclusive_loan_survives_value_replacement() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", tracked, true),
            LocalDecl::new("taken", tracked, false),
        ],
        vec![LoanDecl::new("exclusive", tracked)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            borrow(0, BorrowKind::Exclusive, value),
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(PlaceAccess::loan(LoanId(0))),
            },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
        ],
    );

    validate_body(body).expect("exclusive authority is over storage, not one value lifetime");
}

#[test]
fn loan_end_and_sequential_reuse_are_explicit() {
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
            borrow(0, BorrowKind::Shared, value.clone()),
            Statement::EndBorrow { loan: LoanId(0) },
            borrow(0, BorrowKind::Shared, value),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
        ],
    );

    validate_body(body).expect("inactive declaration may begin a new borrow interval");
}

#[test]
fn loan_access_after_explicit_end_is_rejected() {
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
            borrow(0, BorrowKind::Shared, value),
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanNotActive(LoanId(0))
    );
}

#[test]
fn borrowing_requires_fully_live_storage() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("shared", ty)],
        vec![borrow(0, BorrowKind::Shared, value.clone())],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::BorrowOfUninitialized(value)
    );
}

#[test]
fn exclusive_borrow_does_not_grant_assignment_mutability() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("exclusive", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            borrow(0, BorrowKind::Exclusive, value),
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::AssignToImmutable(LocalId(0))
    );
}

#[test]
fn loan_projection_is_typed_even_when_unreachable() {
    let (types, ty) = i64_type();
    let body = Body {
        types,
        locals: Vec::new(),
        loans: vec![LoanDecl::new("scalar", ty)],
        entry: BasicBlockId(0),
        blocks: vec![
            BasicBlock::new(Vec::new(), Terminator::Return),
            BasicBlock::new(
                vec![Statement::Read {
                    src: PlaceAccess::loan(LoanId(0)).field(0),
                }],
                Terminator::Return,
            ),
        ],
    };

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::InvalidLoanProjection {
            loan: LoanId(0),
            projections: vec![Projection::Field(0)],
        }
    );
}

#[test]
fn stable_loop_state_includes_active_loans() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", ty, false)],
        loans: vec![LoanDecl::new("shared", ty)],
        entry: BasicBlockId(0),
        blocks: vec![
            BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: value.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    borrow(0, BorrowKind::Shared, value),
                ],
                Terminator::Goto(BasicBlockId(1)),
            ),
            BasicBlock::new(
                vec![Statement::Read {
                    src: PlaceAccess::loan(LoanId(0)),
                }],
                Terminator::Goto(BasicBlockId(1)),
            ),
        ],
    };

    validate_body(body).expect("repeated active-loan state proves possible divergence");
}