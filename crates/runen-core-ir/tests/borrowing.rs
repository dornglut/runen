use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
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
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![
            LoanDecl::new("a", i64_ty),
            LoanDecl::new("b", i64_ty),
        ],
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
            Statement::Read {
                src: value.into(),
            },
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_body(body).expect("overlapping shared loans and direct reads are valid");
}

#[test]
fn exclusive_root_borrow_conflicts_with_active_shared_loan() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, true)],
        vec![
            LoanDecl::new("shared", i64_ty),
            LoanDecl::new("exclusive", i64_ty),
        ],
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

    let error = validate_body(body).expect_err("exclusive borrow must conflict with shared loan");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::BorrowConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn shared_root_borrow_conflicts_with_active_exclusive_loan() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, true)],
        vec![
            LoanDecl::new("exclusive", i64_ty),
            LoanDecl::new("shared", i64_ty),
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
                kind: BorrowKind::Shared,
                place: value.clone(),
            },
        ],
    );

    let error = validate_body(body).expect_err("shared borrow must conflict with exclusive loan");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::BorrowConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn disjoint_exclusive_field_loans_are_valid() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let root = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair, true)],
        vec![
            LoanDecl::new("left", i64_ty),
            LoanDecl::new("right", i64_ty),
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

    validate_body(body).expect("disjoint sibling fields may be borrowed exclusively");
}

#[test]
fn direct_read_and_copy_are_valid_under_shared_borrow() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let source = Place::local(LocalId(0));
    let target = Place::local(LocalId(1));

    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", i64_ty, false),
            LocalDecl::new("target", i64_ty, false),
        ],
        vec![LoanDecl::new("shared", i64_ty)],
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
                dst: target,
                src: Operand::Copy(source.into()),
            },
        ],
    );

    validate_body(body).expect("shared borrowing permits direct non-consuming access");
}

#[test]
fn direct_move_is_rejected_under_shared_borrow() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let source = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", i64_ty, false),
            LocalDecl::new("target", i64_ty, false),
        ],
        vec![LoanDecl::new("shared", i64_ty)],
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

    let error = validate_body(body).expect_err("move must conflict with shared borrow");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: source,
            loan: LoanId(0),
        }
    );
}

#[test]
fn direct_assignment_is_rejected_under_shared_borrow() {
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
                place: value.clone(),
            },
            Statement::Assign {
                dst: value.clone().into(),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    let error = validate_body(body).expect_err("assignment must conflict with shared borrow");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn direct_read_is_rejected_under_exclusive_borrow() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, true)],
        vec![LoanDecl::new("exclusive", i64_ty)],
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

    let error = validate_body(body).expect_err("exclusive borrow blocks direct reads");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn shared_loan_rejects_consuming_access() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", i64_ty, true),
            LocalDecl::new("target", i64_ty, false),
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
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(PlaceAccess::loan(LoanId(0))),
            },
        ],
    );

    let error = validate_body(body).expect_err("shared loan cannot consume storage");
    assert_eq!(
        error.kind,
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

    validate_body(body).expect("exclusive loan controls storage across value replacement");
}

#[test]
fn loan_access_after_explicit_end_is_rejected() {
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
                place: value,
            },
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
        ],
    );

    let error = validate_body(body).expect_err("ended loan cannot authorize access");
    assert_eq!(error.kind, MirValidationErrorKind::LoanNotActive(LoanId(0)));
}

#[test]
fn inactive_loan_identity_may_be_reused() {
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

    validate_body(body).expect("inactive typed loan identity may begin another interval");
}

#[test]
fn borrowing_uninitialized_storage_is_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![LoanDecl::new("shared", i64_ty)],
        vec![Statement::Borrow {
            loan: LoanId(0),
            kind: BorrowKind::Shared,
            place: value.clone(),
        }],
    );

    let error = validate_body(body).expect_err("borrow requires fully Live storage");
    assert_eq!(error.kind, MirValidationErrorKind::BorrowOfUninitialized(value));
}

#[test]
fn exclusive_borrow_of_immutable_local_is_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
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
        ],
    );

    let error = validate_body(body).expect_err("exclusive borrow requires mutable local");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ExclusiveBorrowOfImmutable(LocalId(0))
    );
}

#[test]
fn loan_projection_is_typed_even_in_unreachable_mir() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let access = PlaceAccess::loan(LoanId(0)).field(0);

    let body = Body {
        types,
        locals: Vec::new(),
        loans: vec![LoanDecl::new("scalar", i64_ty)],
        entry: BasicBlockId(0),
        blocks: vec![
            BasicBlock::new(Vec::new(), Terminator::Return),
            BasicBlock::new(vec![Statement::Read { src: access }], Terminator::Return),
        ],
    };

    let error = validate_body(body).expect_err("loan projection must be statically typed");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidLoanProjection {
            loan: LoanId(0),
            projections: vec![runen_core_ir::Projection::Field(0)],
        }
    );
}

#[test]
fn stable_loop_state_includes_active_loans() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", i64_ty, false)],
        loans: vec![LoanDecl::new("shared", i64_ty)],
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

    validate_body(body).expect("repeated storage and active-loan state is valid divergence");
}
