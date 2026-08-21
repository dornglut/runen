mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, Program, ScalarType, Statement,
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

fn marked_i64() -> (TypeTable, TypeId) {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability());
    (types, ty)
}

fn plain_i64() -> (TypeTable, TypeId) {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    (types, ty)
}

fn error_kind(program: Program) -> MirValidationErrorKind {
    validate_program(program).expect_err("invalid MIR").kind
}

#[test]
fn interior_assignment_is_independent_of_local_assignment_mutability() {
    let (types, ty) = marked_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        Vec::new(),
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::InteriorAssign {
                dst: place.clone().into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Read { src: place.into() },
        ],
    );

    validate_program(body).expect("interior assignment does not require mutable-local permission");
}

#[test]
fn ordinary_assignment_still_requires_mutable_local_on_marked_storage() {
    let (types, ty) = marked_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        Vec::new(),
        vec![Statement::Assign {
            dst: place.into(),
            src: Operand::Constant(Value::I64(2)),
        }],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::AssignToImmutable(LocalId(0))
    );
}

#[test]
fn interior_assignment_requires_an_explicit_marked_region() {
    let (types, ty) = plain_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        Vec::new(),
        vec![Statement::InteriorAssign {
            dst: place.clone().into(),
            src: Operand::Constant(Value::I64(2)),
        }],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::InteriorMutationRequiresMarkedRegion(place)
    );
}

#[test]
fn exclusive_authority_does_not_imply_interior_mutability() {
    let (types, ty) = plain_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("exclusive", ty)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: place.clone().into(),
            },
            Statement::InteriorAssign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::InteriorMutationRequiresMarkedRegion(place)
    );
}

#[test]
fn shared_loan_can_interior_assign_without_gaining_exclusive_authority() {
    let (types, ty) = marked_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, true)],
        vec![LoanDecl::new("shared", ty)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: place.into(),
            },
            Statement::InteriorAssign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Read {
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_program(body)
        .expect("shared alias authority is sufficient for marked interior storage");
}

#[test]
fn shared_loan_still_cannot_move_from_marked_storage() {
    let (types, ty) = marked_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("value", ty, true),
            LocalDecl::new("taken", ty, false),
        ],
        vec![LoanDecl::new("shared", ty)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: place.into(),
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
fn direct_interior_assignment_can_coexist_with_shared_root_loan() {
    let (types, ty) = marked_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("shared", ty)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: place.clone().into(),
            },
            Statement::InteriorAssign {
                dst: place.into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_program(body).expect("shared aliases do not block marked direct interior replacement");
}

#[test]
fn direct_interior_assignment_is_blocked_by_exclusive_root_loan() {
    let (types, ty) = marked_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("exclusive", ty)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: place.clone().into(),
            },
            Statement::InteriorAssign {
                dst: place.clone().into(),
                src: Operand::Constant(Value::I64(2)),
            },
        ],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::DirectAccessConflict {
            place,
            loan: LoanId(0),
        }
    );
}

#[test]
fn marked_ancestor_grants_interior_mutability_to_structural_descendants() {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let marked_pair = types.push(
        TypeDef::structure(
            "InteriorPair",
            vec![Field::new("left", scalar), Field::new("right", scalar)],
        )
        .with_interior_mutability(),
    );
    let root = Place::local(LocalId(0));
    let left = root.clone().field(0);
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", marked_pair, false)],
        Vec::new(),
        vec![
            Statement::Init {
                dst: root,
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            Statement::InteriorAssign {
                dst: left.into(),
                src: Operand::Constant(Value::I64(3)),
            },
        ],
    );

    validate_program(body).expect("a marked structural ancestor owns its descendant region");
}

#[test]
fn marked_descendant_does_not_make_unmarked_aggregate_interior_mutable() {
    let mut types = TypeTable::new();
    let marked =
        types.push(TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability());
    let plain = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("interior", marked), Field::new("plain", plain)],
    ));
    let root = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair, false)],
        Vec::new(),
        vec![Statement::InteriorAssign {
            dst: root.clone().into(),
            src: Operand::Constant(Value::Struct(vec![Value::I64(3), Value::I64(4)])),
        }],
    );

    assert_eq!(
        error_kind(body),
        MirValidationErrorKind::InteriorMutationRequiresMarkedRegion(root)
    );
}

#[test]
fn marked_descendant_itself_is_interior_mutable_inside_unmarked_aggregate() {
    let mut types = TypeTable::new();
    let marked =
        types.push(TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability());
    let plain = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("interior", marked), Field::new("plain", plain)],
    ));
    let root = Place::local(LocalId(0));
    let interior = root.clone().field(0);
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair, false)],
        Vec::new(),
        vec![
            Statement::Init {
                dst: root,
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            Statement::InteriorAssign {
                dst: interior.into(),
                src: Operand::Constant(Value::I64(3)),
            },
        ],
    );

    validate_program(body).expect("only the marked descendant region gains the capability");
}

#[test]
fn shared_child_preserves_parent_shared_authority_for_interior_assignment() {
    let (types, ty) = marked_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: place.into(),
            },
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Shared,
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::InteriorAssign {
                dst: PlaceAccess::loan(LoanId(0)),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_program(body).expect("shared child leaves the parent's shared authority intact");
}

#[test]
fn exclusive_child_suspends_parent_interior_assignment() {
    let (types, ty) = marked_i64();
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", ty, false)],
        vec![LoanDecl::new("parent", ty), LoanDecl::new("child", ty)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Exclusive,
                src: place.clone().into(),
            },
            Statement::Borrow {
                loan: LoanId(1),
                kind: BorrowKind::Exclusive,
                src: PlaceAccess::loan(LoanId(0)),
            },
            Statement::InteriorAssign {
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
            place,
        }
    );
}

#[test]
fn disjoint_exclusive_child_does_not_block_parent_interior_assignment_on_sibling() {
    let mut types = TypeTable::new();
    let scalar = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(
        TypeDef::structure(
            "InteriorPair",
            vec![Field::new("left", scalar), Field::new("right", scalar)],
        )
        .with_interior_mutability(),
    );
    let root = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair, false)],
        vec![LoanDecl::new("parent", pair), LoanDecl::new("left", scalar)],
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
            Statement::InteriorAssign {
                dst: PlaceAccess::loan(LoanId(0)).field(1),
                src: Operand::Constant(Value::I64(3)),
            },
            Statement::EndBorrow { loan: LoanId(1) },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );

    validate_program(body)
        .expect("disjoint delegation does not constrain sibling shared authority");
}
