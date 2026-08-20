mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, Program, Projection, ScalarType,
    Statement, Terminator, TypeDef, TypeId, TypeTable, Value, validate_program,
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

fn i64_type() -> (TypeTable, TypeId) {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    (types, ty)
}

fn scalar_body(mutable: bool, loan_count: u32, rest: Vec<Statement>) -> Program {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let mut statements = vec![Statement::Init {
        dst: value,
        src: Operand::Constant(Value::I64(1)),
    }];
    statements.extend(rest);
    let loans = (0..loan_count)
        .map(|index| LoanDecl::new(format!("loan{index}"), ty))
        .collect();
    one_block(
        types,
        vec![LocalDecl::new("value", ty, mutable)],
        loans,
        statements,
    )
}

fn pair_types() -> (TypeTable, TypeId, TypeId) {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", scalar), Field::new("right", scalar)],
    ));
    (types, scalar, pair)
}

fn error_kind(program: Program) -> MirValidationErrorKind {
    validate_program(program).expect_err("invalid MIR").kind
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
fn shared_child_is_valid_from_shared_or_exclusive_parent() {
    let value = Place::local(LocalId(0));
    for parent_kind in [BorrowKind::Shared, BorrowKind::Exclusive] {
        let body = scalar_body(
            false,
            2,
            vec![
                root(0, parent_kind, value.clone()),
                child(1, BorrowKind::Shared, 0),
                Statement::Read {
                    src: PlaceAccess::loan(LoanId(0)),
                },
                Statement::Read {
                    src: PlaceAccess::loan(LoanId(1)),
                },
            ],
        );
        validate_program(body).expect("shared child preserves read authority");
    }
}

#[test]
fn exclusive_child_requires_exclusive_parent() {
    let value = Place::local(LocalId(0));
    let body = scalar_body(
        false,
        2,
        vec![
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
    let value = Place::local(LocalId(0));
    let body = scalar_body(
        false,
        2,
        vec![
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
    let value = Place::local(LocalId(0));
    let body = scalar_body(
        true,
        2,
        vec![
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Shared, 0),
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    validate_program(body).expect("child end restores delegated parent authority");
}

#[test]
fn exclusive_child_suspends_overlapping_parent_access() {
    let value = Place::local(LocalId(0));
    let body = scalar_body(
        false,
        2,
        vec![
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
fn shared_child_downgrades_exclusive_parent_to_read_only() {
    let value = Place::local(LocalId(0));
    let readable = scalar_body(
        true,
        2,
        vec![
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Shared, 0),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
        ],
    );
    validate_program(readable).expect("shared child retains parent read authority");

    let consuming = scalar_body(
        true,
        2,
        vec![
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Shared, 0),
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );
    assert_eq!(
        error_kind(consuming),
        MirValidationErrorKind::LoanAccessDelegated {
            loan: LoanId(0),
            child: LoanId(1),
            place: value,
        }
    );
}

#[test]
fn disjoint_child_preserves_parent_authority_and_allows_exclusive_sibling() {
    let (types, scalar, pair) = pair_types();
    let value = Place::local(LocalId(0));
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
                dst: value.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            root(0, BorrowKind::Exclusive, value),
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(0)).field(0),
            },
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(0)).field(1),
                src: Operand::Constant(Value::I64(3)),
            },
            Statement::Borrow {
                loan: LoanId(2),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(0)).field(1),
            },
        ],
    );

    validate_program(body).expect("disjoint delegation preserves parent authority");
}

#[test]
fn overlapping_shared_children_coexist_but_block_exclusive_sibling() {
    let value = Place::local(LocalId(0));
    let coexist = scalar_body(
        false,
        3,
        vec![
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Shared, 0),
            child(2, BorrowKind::Shared, 0),
        ],
    );
    validate_program(coexist).expect("overlapping shared children may coexist");

    let blocked = scalar_body(
        false,
        3,
        vec![
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Shared, 0),
            child(2, BorrowKind::Exclusive, 0),
        ],
    );
    assert_eq!(
        error_kind(blocked),
        MirValidationErrorKind::LoanAccessDelegated {
            loan: LoanId(0),
            child: LoanId(1),
            place: value,
        }
    );
}

#[test]
fn grandchild_delegation_restores_authority_leaf_to_root() {
    let value = Place::local(LocalId(0));
    let body = scalar_body(
        false,
        3,
        vec![
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Exclusive, 0),
            child(2, BorrowKind::Shared, 1),
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

    validate_program(body).expect("nested delegation composes recursively");
}

#[test]
fn active_declaration_reuse_cannot_create_a_loan_cycle() {
    let value = Place::local(LocalId(0));
    let body = scalar_body(
        false,
        2,
        vec![
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
fn exclusive_child_controls_storage_across_value_replacement() {
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
        vec![
            LoanDecl::new("parent", tracked),
            LoanDecl::new("child", tracked),
        ],
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
        ],
    );

    validate_program(body).expect("exclusive child authority spans stored-value lifetimes");
}

#[test]
fn immutable_storage_supports_exclusive_reborrow_without_assignment_privilege() {
    let value = Place::local(LocalId(0));
    let valid = scalar_body(
        false,
        2,
        vec![
            root(0, BorrowKind::Exclusive, value.clone()),
            child(1, BorrowKind::Exclusive, 0),
            Statement::Read {
                src: PlaceAccess::loan(LoanId(1)),
            },
        ],
    );
    validate_program(valid).expect("exclusive delegation is independent from mutability");

    let invalid = scalar_body(
        false,
        2,
        vec![
            root(0, BorrowKind::Exclusive, value),
            child(1, BorrowKind::Exclusive, 0),
            Statement::Assign {
                dst: PlaceAccess::loan(LoanId(1)),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );
    assert_eq!(
        error_kind(invalid),
        MirValidationErrorKind::AssignToImmutable(LocalId(0))
    );
}

#[test]
fn reborrow_source_projection_is_typed_even_when_unreachable() {
    let (types, ty) = i64_type();
    let body = one_function_program(
        types,
        Body {
            locals: Vec::new(),
            loans: vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
                BasicBlock::new(
                    vec![Statement::Borrow {
                        loan: LoanId(1),
                        kind: BorrowKind::Shared,
                        src: PlaceAccess::loan(LoanId(0)).field(0),
                    }],
                    Terminator::Return(None),
                ),
            ],
        },
    );

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
    let value = Place::local(LocalId(0));
    let body = scalar_body(
        false,
        2,
        vec![
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

    validate_program(body).expect("each activation receives fresh parentage");
}

#[test]
fn stable_loop_state_includes_loan_tree_parentage() {
    let (types, ty) = i64_type();
    let value = Place::local(LocalId(0));
    let body = one_function_program(
        types,
        Body {
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
        },
    );

    validate_program(body).expect("repeated loan-tree state proves possible divergence");
}
