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

fn root(loan: u32, kind: BorrowKind, place: Place) -> Statement {
    Statement::Borrow {
        loan: LoanId(loan),
        kind,
        src: place.into(),
    }
}

fn child(loan: u32, kind: BorrowKind, parent: u32) -> Statement {
    Statement::Borrow {
        loan: LoanId(loan),
        kind,
        src: PlaceAccess::loan(LoanId(parent)),
    }
}

#[test]
fn shared_child_from_shared_parent_is_valid() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Shared, value),
            child(1, BorrowKind::Shared, 0),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(1)),
            },
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_body(body).expect("shared authority may be reborrowed as shared");
}

#[test]
fn shared_child_from_exclusive_parent_downgrades_only_overlapping_authority() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Shared, 0),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(1)),
            },
        ],
    );

    validate_body(body).expect("shared child leaves overlapping parent read authority");
}

#[test]
fn exclusive_child_requires_exclusive_parent() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Shared, value),
            child(1, BorrowKind::Exclusive, 0),
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::ExclusiveLoanRequired(LoanId(0))
    );
}

#[test]
fn parent_cannot_end_before_child() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Shared, value),
            child(1, BorrowKind::Shared, 0),
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanHasActiveChild {
            loan: LoanId(0),
            child: LoanId(1),
        }
    );
}

#[test]
fn ending_child_restores_exclusive_parent_authority() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Shared, 0),
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    validate_body(body).expect("ending the child restores delegated parent authority");
}

#[test]
fn exclusive_child_suspends_overlapping_parent_access() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Exclusive, 0),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanAccessDelegated {
            loan: LoanId(0),
            child: LoanId(1),
            place: value,
        }
    );
}

#[test]
fn shared_child_blocks_parent_consuming_access() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Shared, 0),
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanAccessDelegated {
            loan: LoanId(0),
            child: LoanId(1),
            place: value,
        }
    );
}

#[test]
fn disjoint_parent_authority_remains_available() {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", scalar), Field::new("right", scalar)],
    ));
    let root_place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair, true)],
        vec![LoanDecl::new("parent", pair), LoanDecl::new("left", scalar)],
        vec![
            Statement::Init {
                dst: root_place.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            root(0, BorrowKind::Exclusive, root_place),
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(0)).field(0),
            },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)).field(1),
                src: Operand::Constant(Value::I64(3)),
            },
        ],
    );

    validate_body(body).expect("delegation over left does not suspend disjoint right authority");
}

#[test]
fn overlapping_shared_children_may_coexist() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![
            LoanDecl::new("parent", ty),
            LoanDecl::new("a", ty),
            LoanDecl::new("b", ty),
        ],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Shared, 0),
            child(2, BorrowKind::Shared, 0),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(1)),
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(2)),
            },
        ],
    );

    validate_body(body).expect("multiple shared delegations may overlap");
}

#[test]
fn existing_shared_child_blocks_exclusive_sibling() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![
            LoanDecl::new("parent", ty),
            LoanDecl::new("shared", ty),
            LoanDecl::new("exclusive", ty),
        ],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Shared, 0),
            child(2, BorrowKind::Exclusive, 0),
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanAccessDelegated {
            loan: LoanId(0),
            child: LoanId(1),
            place: value,
        }
    );
}

#[test]
fn disjoint_exclusive_children_may_coexist() {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", scalar), Field::new("right", scalar)],
    ));
    let root_place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair, true)],
        vec![
            LoanDecl::new("parent", pair),
            LoanDecl::new("left", scalar),
            LoanDecl::new("right", scalar),
        ],
        vec![
            Statement::Init {
                dst: root_place.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            root(0, BorrowKind::Exclusive, root_place),
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(0)).field(0),
            },
            Statement::Borrow {
                loan: LoanId(2),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(0)).field(1),
            },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(1)),
                src: Operand::Constant(Value::I64(3)),
            },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(2)),
                src: Operand::Constant(Value::I64(4)),
            },
        ],
    );

    validate_body(body).expect("disjoint exclusive child delegations may coexist");
}

#[test]
fn grandchild_delegation_composes_recursively() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![
            LoanDecl::new("root", ty),
            LoanDecl::new("child", ty),
            LoanDecl::new("grandchild", ty),
        ],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Exclusive, 0),
            child(2, BorrowKind::Shared, 1),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(2)),
            },
            Statement::EndBorrow { loan: LoanId(2) },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(1)),
            },
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
        ],
    );

    validate_body(body).expect("nested delegation restores authority leaf-to-root");
}

#[test]
fn loan_cycle_cannot_be_created_by_reusing_active_declaration() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("root", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Exclusive, 0),
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(1)),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::LoanAlreadyActive(LoanId(0))
    );
}

#[test]
fn child_source_must_be_fully_live() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", ty, false),
            LocalDecl::new("taken", ty, false),
        ],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value.clone()),
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(PlaceAccess::loan(LoanId(0))),
            },
            child(1, BorrowKind::Shared, 0),
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::BorrowOfUninitialized(value)
    );
}

#[test]
fn exclusive_child_can_replace_value_without_ending_interval() {
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
        vec![LoanDecl::new("parent", tracked), LoanDecl::new("child", tracked)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Exclusive, 0),
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(PlaceAccess::loan(LoanId(1))),
            },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(1)),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(1)),
            },
        ],
    );

    validate_body(body).expect("exclusive child controls storage across value replacement");
}

#[test]
fn immutable_storage_supports_exclusive_reborrow_without_assignment_privilege() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Exclusive, 0),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(1)),
            },
        ],
    );

    validate_body(body).expect("exclusive delegation is independent from assignment mutability");
}

#[test]
fn reborrow_source_projection_is_typed_even_when_unreachable() {
    let (types, ty) = i64_type();
    let body = Body {
        types,
        locals: Vec::new(),
        loans: vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        entry: BasicBlockId(0),
        blocks: vec![
            BasicBlock::new(Vec::new(), Terminator::Return),
            BasicBlock::new(
                vec![Statement::Borrow {
                    loan: LoanId(1),
                    kind: BorrowKind::Shared,
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
fn sequential_loan_id_reuse_gets_fresh_parentage() {
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
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Shared, 0),
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::EndBorrow { loan: LoanId(0) },
            root(1, BorrowKind::Exclusive, value),
            child(0, BorrowKind::Shared, 1),
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::EndBorrow { loan: LoanId(1) },
        ],
    );

    validate_body(body).expect("each dynamic interval replaces stale parentage");
}

#[test]
fn stable_loop_state_includes_loan_tree_parentage() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = Body {
        types,
        locals: vec![LocalDecl::new("value", ty, false)],
        loans: vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        entry: BasicBlockId(0),
        blocks: vec![
            BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: value.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    root(0, BorrowKind::Shared, value),
                    child(1, BorrowKind::Shared, 0),
                ],
                Terminator::Goto(BasicBlockId(1)),
            ),
            BasicBlock::new(
                vec![Statement::Read {
                    src: PlaceAccess::loan(LoanId(1)),
                }],
                Terminator::Goto(BasicBlockId(1)),
            ),
        ],
    };

    validate_body(body).expect("repeated loan-tree state proves possible divergence");
}