mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, FunctionId, LocalDecl, LocalId, MirValidationErrorKind,
    Operand, Place, Program, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable, Value,
    validate_program,
};
use support::one_function_program;

fn one_block(types: TypeTable, locals: Vec<LocalDecl>, statements: Vec<Statement>) -> Program {
    one_function_program(
        types,
        Body {
            locals,
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(statements, Terminator::Return(None))],
        },
    )
}

#[test]
fn valid_body_is_validated() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Init {
            dst: Place::local(LocalId(0)),
            src: Operand::Constant(Value::I64(1)),
        }],
    );

    let validated = validate_program(body).expect("valid body must pass validation");
    assert_eq!(
        validated
            .as_program()
            .function(FunctionId(0))
            .expect("one-function fixture")
            .body
            .entry,
        BasicBlockId(0)
    );
}

#[test]
fn invalid_entry_block_is_rejected() {
    let body = one_function_program(
        TypeTable::new(),
        Body {
            locals: Vec::new(),
            loans: Vec::new(),
            entry: BasicBlockId(1),
            blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        },
    );

    let error = validate_program(body).expect_err("entry outside the body must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidEntryBlock(BasicBlockId(1))
    );
}

#[test]
fn invalid_goto_target_is_rejected() {
    let body = one_function_program(
        TypeTable::new(),
        Body {
            locals: Vec::new(),
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                Vec::new(),
                Terminator::Goto(BasicBlockId(7)),
            )],
        },
    );

    let error = validate_program(body).expect_err("unknown target must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidTargetBlock(BasicBlockId(7))
    );
}

#[test]
fn unknown_local_type_is_rejected() {
    let body = one_block(
        TypeTable::new(),
        vec![LocalDecl::new("value", TypeId(9), false)],
        Vec::new(),
    );

    let error = validate_program(body).expect_err("unknown local type must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::UnknownType(TypeId(9)));
}

#[test]
fn statement_referencing_unknown_local_is_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Read {
            src: Place::local(LocalId(8)).into(),
        }],
    );

    let error = validate_program(body).expect_err("unknown local reference must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::InvalidLocal(LocalId(8)));
}

#[test]
fn struct_field_referencing_unknown_type_is_rejected() {
    let mut types = TypeTable::new();
    let aggregate = types.push(TypeDef::structure(
        "Broken",
        vec![Field::new("field", TypeId(9))],
    ));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", aggregate, false)],
        Vec::new(),
    );

    let error = validate_program(body).expect_err("unknown field type must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::UnknownType(TypeId(9)));
}

#[test]
fn recursive_by_value_type_is_rejected() {
    let mut types = TypeTable::new();
    let recursive = TypeId(0);
    assert_eq!(
        types.push(TypeDef::structure(
            "Recursive",
            vec![Field::new("next", recursive)]
        )),
        recursive
    );

    let body = one_block(
        types,
        vec![LocalDecl::new("value", recursive, false)],
        Vec::new(),
    );

    let error = validate_program(body).expect_err("recursive value type must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::RecursiveType(recursive));
}

#[test]
fn invalid_projection_is_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let place = Place::local(LocalId(0)).field(0);
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Read {
            src: place.clone().into(),
        }],
    );

    let error = validate_program(body).expect_err("field projection from scalar must be rejected");
    assert_eq!(error.kind, MirValidationErrorKind::InvalidProjection(place));
}

#[test]
fn operand_type_mismatch_is_rejected() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", bool_ty, false)],
        vec![Statement::Init {
            dst: Place::local(LocalId(0)),
            src: Operand::Constant(Value::I64(1)),
        }],
    );

    let error = validate_program(body).expect_err("typed MIR mismatch must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: bool_ty }
    );
}

#[test]
fn copy_of_noncopy_type_is_rejected_by_validation() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", tracked, false),
            LocalDecl::new("target", tracked, false),
        ],
        vec![Statement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Copy(Place::local(LocalId(0)).into()),
        }],
    );

    let error =
        validate_program(body).expect_err("non-copy Copy must be rejected by MIR validation");
    assert_eq!(error.kind, MirValidationErrorKind::CopyOfNonCopy(tracked));
}

#[test]
fn assignment_through_immutable_local_is_rejected_by_validation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Assign {
            dst: Place::local(LocalId(0)).into(),
            src: Operand::Constant(Value::I64(2)),
        }],
    );

    let error = validate_program(body)
        .expect_err("immutable assignment must be rejected by MIR validation");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::AssignToImmutable(LocalId(0))
    );
}

#[test]
fn read_after_move_is_rejected_by_validation() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let source = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", tracked, false),
            LocalDecl::new("target", tracked, false),
        ],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(source.clone().into()),
            },
            Statement::Read {
                src: source.clone().into(),
            },
        ],
    );

    let error = validate_program(body).expect_err("read after move must be invalid MIR");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(source)
    );
}

#[test]
fn init_after_move_reinitializes_vacant_storage() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let source = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![
            LocalDecl::new("source", tracked, false),
            LocalDecl::new("target", tracked, false),
        ],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(source.clone().into()),
            },
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
            Statement::Read { src: source.into() },
        ],
    );

    validate_program(body).expect("Init may begin a later lifetime in Dead storage");
}

#[test]
fn init_after_drop_reinitializes_vacant_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Drop {
                place: place.clone().into(),
            },
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Read { src: place.into() },
        ],
    );

    validate_program(body).expect("Init may begin a later lifetime after explicit destruction");
}

#[test]
fn init_accepts_mixed_never_and_dead_aggregate_when_no_leaf_is_live() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let pair = Place::local(LocalId(0));
    let left = pair.clone().field(0);
    let body = one_block(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("moved", i64_ty, false),
        ],
        vec![
            Statement::Init {
                dst: left.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(left.into()),
            },
            Statement::Init {
                dst: pair.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(2), Value::I64(3)])),
            },
            Statement::Read { src: pair.into() },
        ],
    );

    validate_program(body).expect("mixed Never/Dead aggregate with no Live leaf is vacant");
}

#[test]
fn init_rejects_aggregate_with_any_live_leaf() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let pair = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair_ty, false)],
        vec![
            Statement::Init {
                dst: pair.clone().field(0),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: pair.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(2), Value::I64(3)])),
            },
        ],
    );

    let error = validate_program(body).expect_err("partially Live aggregate is not wholly vacant");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InitRequiresVacant(pair)
    );
}

#[test]
fn projected_init_checks_only_selected_subplace_vacancy() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let pair = Place::local(LocalId(0));
    let right = pair.clone().field(1);
    let body = one_block(
        types,
        vec![LocalDecl::new("pair", pair_ty, false)],
        vec![
            Statement::Init {
                dst: pair.field(0),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: right.clone(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Read { src: right.into() },
        ],
    );

    validate_program(body).expect("Live sibling does not make a disjoint vacant field non-vacant");
}

#[test]
fn init_checks_vacancy_before_source_move() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: place.clone(),
                src: Operand::Move(place.clone().into()),
            },
        ],
    );

    let error = validate_program(body)
        .expect_err("source move cannot manufacture vacancy for the same Init");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InitRequiresVacant(place)
    );
}

#[test]
fn zero_leaf_structural_init_is_vacuously_vacant() {
    let mut types = TypeTable::new();
    let empty_ty = types.push(TypeDef::structure("Empty", Vec::new()));
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("empty", empty_ty, false)],
        vec![
            Statement::Init {
                dst: place.clone(),
                src: Operand::Constant(Value::Struct(Vec::new())),
            },
            Statement::Init {
                dst: place,
                src: Operand::Constant(Value::Struct(Vec::new())),
            },
        ],
    );

    validate_program(body).expect("zero-leaf structure has no Live leaf and remains wholly vacant");
}

#[test]
fn drop_without_live_subobject_is_rejected_by_validation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let place = Place::local(LocalId(0));
    let body = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![Statement::Drop {
            place: place.clone().into(),
        }],
    );

    let error =
        validate_program(body).expect_err("drop of never-initialized storage is invalid MIR");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DropOfUninitialized(place)
    );
}

#[test]
fn validation_rejects_repeated_loop_init_while_destination_remains_live() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let place = Place::local(LocalId(0));
    let body = one_function_program(
        types,
        Body {
            locals: vec![LocalDecl::new("value", i64_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: place.clone(),
                    src: Operand::Constant(Value::I64(1)),
                }],
                Terminator::Goto(BasicBlockId(0)),
            )],
        },
    );

    let error = validate_program(body).expect_err("second loop iteration reaches Live Init target");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InitRequiresVacant(place)
    );
}

#[test]
fn cyclic_scalar_and_aggregate_init_after_lifetime_end_are_valid() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let scalar = Place::local(LocalId(0));
    let pair = Place::local(LocalId(1));
    let body = one_function_program(
        types,
        Body {
            locals: vec![
                LocalDecl::new("scalar", i64_ty, false),
                LocalDecl::new("pair", pair_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: scalar.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: pair.clone(),
                        src: Operand::Constant(Value::Struct(vec![Value::I64(2), Value::I64(3)])),
                    },
                    Statement::Drop {
                        place: pair.into(),
                    },
                    Statement::Drop {
                        place: scalar.into(),
                    },
                ],
                Terminator::Goto(BasicBlockId(0)),
            )],
        },
    );

    validate_program(body)
        .expect("backedge may revisit scalar and aggregate Init after prior lifetimes ended");
}

#[test]
fn stable_valid_loop_state_is_accepted_as_possible_divergence() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let place = Place::local(LocalId(0));
    let body = one_function_program(
        types,
        Body {
            locals: vec![LocalDecl::new("value", i64_ty, true)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![Statement::Assign {
                    dst: place.into(),
                    src: Operand::Constant(Value::I64(1)),
                }],
                Terminator::Goto(BasicBlockId(0)),
            )],
        },
    );

    validate_program(body).expect("stable mutable assignment loop is valid MIR");
}
