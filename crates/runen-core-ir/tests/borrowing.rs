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

#[test]
fn structural_place_overlap_is_address_independent() {
    let root = Place::local(LocalId(0));
    let left = root.clone().field(0);
    let left_nested = left.clone().field(0);
    let right = root.clone().field(1);
    let other = Place::local(LocalId(1));

    assert!(root.overlaps(&root));
    assert!(root.overlaps(&left));
    assert!(left.overlaps(&root));
    assert!(left.overlaps(&left_nested));
    assert!(left_nested.overlaps(&left));
    assert!(!left.overlaps(&right));
    assert!(!root.overlaps(&other));
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
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value.clone(),
            },
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Shared,
                place: value.clone(),
            },
            Statement::Read { src: value.into() },
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_body(body).expect("overlapping shared loans are valid");
}

#[test]
fn exclusive_root_borrow_conflicts_with_shared_loan() {
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
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
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
fn shared_root_borrow_conflicts_with_exclusive_loan() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("exclusive", ty), LoanDecl::new("shared", ty)],
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
                kind: BorrowKind::Shared,
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
        vec![
            LoanDecl::new("left", scalar),
            LoanDecl::new("right", scalar),
        ],
        vec![
            Statement::Init {
                dst: root.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                place: root.clone().field(0),
            },
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                place: root.field(1),
            },
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

    validate_body(body).expect("disjoint sibling fields may be exclusive");
}

#[test]
fn direct_read_and_copy_are_valid_under_shared_borrow() {
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
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: source.clone(),
            },
            Statement::Read {
                src: source.clone().into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(source.into()),
            },
        ],
    );

    validate_body(body).expect("shared loan permits direct non-consuming access");
}

#[test]
fn direct_move_is_rejected_under_shared_borrow() {
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
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: source.clone(),
            },
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
fn direct_assignment_is_rejected_under_shared_borrow() {
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
                place: value.clone(),
            },
            Statement::Assign {
                dst: value.clone().into(),
                src: Operand::Constant(Value::I64(2)),
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
fn direct_read_is_rejected_under_exclusive_borrow() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("exclusive", ty)],
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
fn shared_loan_rejects_consuming_access() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", ty, true),
            LocalDecl::new("target", ty, false),
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
fn exclusive_loan_survives_move_and_replacement() {
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
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                place: value,
            },
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
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_body(body).expect("exclusive loan persists across replacement");
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
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value,
            },
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
fn inactive_loan_identity_may_be_reused() {
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
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                place: value,
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_body(body).expect("inactive typed loan identity may be reused");
}

#[test]
fn borrowing_uninitialized_storage_is_rejected() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("shared", ty)],
        vec![Statement::Borrow {
            loan: LoanId(0),
            kind: BorrowKind::Shared,
            place: value.clone(),
        }],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::BorrowOfUninitialized(value)
    );
}

#[test]
fn exclusive_borrow_of_immutable_local_is_rejected() {
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
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                place: value,
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::ExclusiveBorrowOfImmutable(LocalId(0))
    );
}

#[test]
fn loan_projection_is_typed_even_in_unreachable_mir() {
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
                    Statement::Borrow {
                        loan: LoanId(0),
                        kind: BorrowKind::Shared,
                        place: value,
                    },
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

    validate_body(body).expect("stable active-loan loop is valid divergence");
}
