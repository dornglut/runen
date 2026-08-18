use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, ScalarType, Statement, Terminator,
    TypeDef, TypeTable, Value, validate_body,
};

fn marked_body(statements: Vec<Statement>, loans: Vec<LoanDecl>) -> Body {
    let mut types = TypeTable::new();
    let ty = types.push(
        TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability(),
    );
    Body {
        types,
        locals: vec![LocalDecl::new("value", ty, true)],
        loans,
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, Terminator::Return)],
    }
}

#[test]
fn interior_capability_does_not_allow_ordinary_assignment_through_shared_loan() {
    let value = Place::local(LocalId(0));
    let mut types = TypeTable::new();
    let ty = types.push(
        TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability(),
    );
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", ty, true)],
        loans: vec![LoanDecl::new("shared", ty)],
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: value.clone(),
                    src: Operand::Constant(Value::I64(1)),
                },
                Statement::Borrow {
                    loan: LoanId(0),
                    kind: BorrowKind::Shared,
                    src: value.into(),
                },
                Statement::Assign {
                    dst: PlaceAccess::loan(LoanId(0)),
                    src: Operand::Constant(Value::I64(2)),
                },
            ],
            Terminator::Return,
        )],
    };

    assert_eq!(
        validate_body(body).expect_err("shared authority cannot ordinary-assign").kind,
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn interior_capability_does_not_allow_drop_through_shared_loan() {
    let value = Place::local(LocalId(0));
    let mut types = TypeTable::new();
    let ty = types.push(
        TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability(),
    );
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", ty, true)],
        loans: vec![LoanDecl::new("shared", ty)],
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: value.clone(),
                    src: Operand::Constant(Value::I64(1)),
                },
                Statement::Borrow {
                    loan: LoanId(0),
                    kind: BorrowKind::Shared,
                    src: value.into(),
                },
                Statement::Drop {
                    place: PlaceAccess::loan(LoanId(0)),
                },
            ],
            Terminator::Return,
        )],
    };

    assert_eq!(
        validate_body(body).expect_err("shared authority cannot drop").kind,
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn interior_capability_does_not_allow_exclusive_reborrow_from_shared_loan() {
    let value = Place::local(LocalId(0));
    let mut types = TypeTable::new();
    let ty = types.push(
        TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability(),
    );
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", ty, true)],
        loans: vec![
            LoanDecl::new("shared", ty),
            LoanDecl::new("exclusive_child", ty),
        ],
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: value.clone(),
                    src: Operand::Constant(Value::I64(1)),
                },
                Statement::Borrow {
                    loan: LoanId(0),
                    kind: BorrowKind::Shared,
                    src: value.into(),
                },
                Statement::Borrow {
                    loan: LoanId(1),
                    kind: BorrowKind::Exclusive,
                    src: PlaceAccess::loan(LoanId(0)),
                },
            ],
            Terminator::Return,
        )],
    };

    assert_eq!(
        validate_body(body)
            .expect_err("interior capability cannot upgrade shared alias authority")
            .kind,
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn stable_interior_assignment_loop_adds_no_hidden_validation_state() {
    let mut types = TypeTable::new();
    let ty = types.push(
        TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability(),
    );
    let value = Place::local(LocalId(0));
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", ty, false)],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: value.clone(),
                    src: Operand::Constant(Value::I64(1)),
                }],
                Terminator::Goto(BasicBlockId(1)),
            ),
            BasicBlock::new(
                vec![Statement::InteriorAssign {
                    dst: value.into(),
                    src: Operand::Constant(Value::I64(2)),
                }],
                Terminator::Goto(BasicBlockId(1)),
            ),
        ],
    };

    validate_body(body).expect("interior assignment adds no hidden state to loop validation");
}
