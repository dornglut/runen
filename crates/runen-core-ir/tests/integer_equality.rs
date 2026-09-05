mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, Program, ReferenceAccess, ReferencePermission,
    ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable, Value, validate_program,
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

fn equality(
    dst: Place,
    operand_type: TypeId,
    left: Operand,
    right: Operand,
) -> Statement {
    Statement::IntegerEq {
        dst,
        operand_type,
        left,
        right,
    }
}

#[test]
fn integer_eq_accepts_all_eight_integer_operand_kinds_with_bool_destination() {
    let cases: [(ScalarType, Value, Value); 8] = [
        (ScalarType::I8, Value::I8(1), Value::I8(1)),
        (ScalarType::I16, Value::I16(-2), Value::I16(-2)),
        (ScalarType::I32, Value::I32(3), Value::I32(4)),
        (ScalarType::I64, Value::I64(-5), Value::I64(6)),
        (ScalarType::U8, Value::U8(7), Value::U8(7)),
        (ScalarType::U16, Value::U16(8), Value::U16(9)),
        (ScalarType::U32, Value::U32(10), Value::U32(10)),
        (ScalarType::U64, Value::U64(11), Value::U64(12)),
    ];

    for (index, (scalar, left, right)) in cases.into_iter().enumerate() {
        let mut types = TypeTable::new();
        let operand_type = types.push(TypeDef::scalar(format!("integer-{index}"), scalar));
        let bool_type = types.push(TypeDef::scalar("bool-result", ScalarType::Bool));
        let program = one_block(
            types,
            vec![LocalDecl::new("result", bool_type, false)],
            vec![equality(
                Place::local(LocalId(0)),
                operand_type,
                Operand::Constant(left),
                Operand::Constant(right),
            )],
        );

        validate_program(program).expect("fixed-width integer equality must validate");
    }
}

#[test]
fn integer_eq_rejects_known_non_bool_destination_before_operands() {
    for scalar in [ScalarType::I8, ScalarType::F32, ScalarType::TrackedFixture] {
        let mut types = TypeTable::new();
        let operand_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
        let destination_type = types.push(TypeDef::scalar("not-bool", scalar));
        let program = one_block(
            types,
            vec![LocalDecl::new("result", destination_type, false)],
            vec![equality(
                Place::local(LocalId(0)),
                operand_type,
                Operand::Constant(Value::I8(1)),
                Operand::Constant(Value::I8(1)),
            )],
        );

        let error = validate_program(program).expect_err("non-Bool equality destination must fail");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::IntegerEqRequiresBoolDestination(destination_type)
        );
    }
}

#[test]
fn integer_eq_rejects_unknown_operand_type_before_operands() {
    let mut types = TypeTable::new();
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let unknown = TypeId(999);
    let program = one_block(
        types,
        vec![LocalDecl::new("result", bool_type, false)],
        vec![equality(
            Place::local(LocalId(0)),
            unknown,
            Operand::Constant(Value::I8(1)),
            Operand::Constant(Value::I8(1)),
        )],
    );

    let error = validate_program(program).expect_err("unknown operand TypeId must fail");
    assert_eq!(error.kind, MirValidationErrorKind::UnknownType(unknown));
}

#[test]
fn integer_eq_rejects_known_non_integer_operand_types_before_operands() {
    let mut types = TypeTable::new();
    let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let float_type = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let tracked_type = types.push(TypeDef::scalar("tracked", ScalarType::TrackedFixture));
    let raw_type = types.push(TypeDef::raw_pointer("raw-i8", i8_type));
    let reference_type = types.push(TypeDef::reference(
        "shared-i8",
        i8_type,
        ReferencePermission::Shared,
    ));

    for operand_type in [bool_type, float_type, tracked_type, raw_type, reference_type] {
        let program = one_block(
            types.clone(),
            vec![LocalDecl::new("result", bool_type, false)],
            vec![equality(
                Place::local(LocalId(0)),
                operand_type,
                Operand::Constant(Value::I8(1)),
                Operand::Constant(Value::I8(1)),
            )],
        );
        let error = validate_program(program).expect_err("known non-integer operand type must fail");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::IntegerEqRequiresIntegerOperands(operand_type)
        );
    }
}

#[test]
fn integer_eq_uses_explicit_operand_type_for_constants_and_place_constant_pairs() {
    for place_on_left in [true, false] {
        let mut types = TypeTable::new();
        let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
        let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
        let source = Place::local(LocalId(0));
        let result = Place::local(LocalId(1));
        let program = one_block(
            types,
            vec![
                LocalDecl::new("source", i8_type, false),
                LocalDecl::new("result", bool_type, false),
            ],
            vec![
                Statement::Init {
                    dst: source.clone(),
                    src: Operand::Constant(Value::I8(7)),
                },
                equality(
                    result,
                    i8_type,
                    if place_on_left {
                        Operand::Move(source.clone().into())
                    } else {
                        Operand::Constant(Value::I8(7))
                    },
                    if place_on_left {
                        Operand::Constant(Value::I8(7))
                    } else {
                        Operand::Move(source.into())
                    },
                ),
            ],
        );

        validate_program(program).expect("place/constant equality must use explicit operand TypeId");
    }
}

#[test]
fn integer_eq_requires_exact_operand_type_identity_for_each_place_operand() {
    for mismatch_left in [true, false] {
        let mut types = TypeTable::new();
        let operand_type = types.push(TypeDef::scalar("selected-i8", ScalarType::I8));
        let distinct_i8 = types.push(TypeDef::scalar("distinct-i8", ScalarType::I8));
        let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
        let source = Place::local(LocalId(0));
        let result = Place::local(LocalId(1));
        let program = one_block(
            types,
            vec![
                LocalDecl::new("source", distinct_i8, false),
                LocalDecl::new("result", bool_type, false),
            ],
            vec![
                Statement::Init {
                    dst: source.clone(),
                    src: Operand::Constant(Value::I8(7)),
                },
                equality(
                    result,
                    operand_type,
                    if mismatch_left {
                        Operand::Move(source.clone().into())
                    } else {
                        Operand::Constant(Value::I8(7))
                    },
                    if mismatch_left {
                        Operand::Constant(Value::I8(7))
                    } else {
                        Operand::Move(source.into())
                    },
                ),
            ],
        );

        let error = validate_program(program).expect_err("distinct same-kind TypeId must fail");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::TypeMismatch {
                expected: operand_type,
            }
        );
    }
}

#[test]
fn integer_eq_rejects_constant_mismatch_against_explicit_operand_type() {
    let mut types = TypeTable::new();
    let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let program = one_block(
        types,
        vec![LocalDecl::new("result", bool_type, false)],
        vec![equality(
            Place::local(LocalId(0)),
            i8_type,
            Operand::Constant(Value::U8(1)),
            Operand::Constant(Value::I8(1)),
        )],
    );

    let error = validate_program(program).expect_err("constant must match explicit operand type");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: i8_type }
    );
}

#[test]
fn integer_eq_checks_vacancy_before_operand_state() {
    let mut types = TypeTable::new();
    let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let destination = Place::local(LocalId(0));
    let missing = Place::local(LocalId(1));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("result", bool_type, false),
            LocalDecl::new("missing", i8_type, false),
        ],
        vec![
            Statement::Init {
                dst: destination.clone(),
                src: Operand::Constant(Value::Bool(false)),
            },
            equality(
                destination.clone(),
                i8_type,
                Operand::Move(missing.into()),
                Operand::Constant(Value::I8(1)),
            ),
        ],
    );

    let error = validate_program(program).expect_err("occupied destination must reject first");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::IntegerEqRequiresVacant(destination)
    );
}

#[test]
fn shared_and_exclusive_overlapping_loans_block_direct_eq_destination_first() {
    for kind in [BorrowKind::Shared, BorrowKind::Exclusive] {
        let mut types = TypeTable::new();
        let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
        let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
        let destination = Place::local(LocalId(0));
        let missing = Place::local(LocalId(1));
        let program = one_function_program(
            types,
            Body {
                locals: vec![
                    LocalDecl::new("result", bool_type, false),
                    LocalDecl::new("missing", i8_type, false),
                ],
                loans: vec![LoanDecl::new("loan", bool_type)],
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: destination.clone(),
                            src: Operand::Constant(Value::Bool(false)),
                        },
                        Statement::Borrow {
                            loan: LoanId(0),
                            kind,
                            src: destination.clone().into(),
                        },
                        equality(
                            destination.clone(),
                            i8_type,
                            Operand::Move(missing.clone().into()),
                            Operand::Move(missing.clone().into()),
                        ),
                    ],
                    Terminator::Return(None),
                )],
            },
        );

        let error = validate_program(program).expect_err("overlapping loan blocks direct equality dst");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::DirectAccessConflict {
                place: destination,
                loan: LoanId(0),
            }
        );
    }
}

#[test]
fn integer_eq_evaluates_left_state_before_right_state() {
    let mut types = TypeTable::new();
    let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let source = Place::local(LocalId(0));
    let result = Place::local(LocalId(1));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("source", i8_type, false),
            LocalDecl::new("result", bool_type, false),
        ],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::I8(5)),
            },
            equality(
                result,
                i8_type,
                Operand::Move(source.clone().into()),
                Operand::Move(source.clone().into()),
            ),
        ],
    );

    let error = validate_program(program).expect_err("right move observes the left move");
    assert_eq!(error.kind, MirValidationErrorKind::UseOfUninitialized(source));
}

#[test]
fn integer_eq_result_becomes_live_as_bool() {
    let mut types = TypeTable::new();
    let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let result = Place::local(LocalId(0));
    let program = one_block(
        types,
        vec![LocalDecl::new("result", bool_type, false)],
        vec![
            equality(
                result.clone(),
                i8_type,
                Operand::Constant(Value::I8(3)),
                Operand::Constant(Value::I8(3)),
            ),
            Statement::Read { src: result.into() },
        ],
    );

    validate_program(program).expect("successful IntegerEq initializes a Bool destination once");
}

#[test]
fn integer_eq_preserves_raw_move_integer_operand_semantics() {
    let mut types = TypeTable::new();
    let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let raw_i8 = types.push(TypeDef::raw_pointer("raw-i8", i8_type));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("target", i8_type, false),
            LocalDecl::new("pointer", raw_i8, false),
            LocalDecl::new("result", bool_type, false),
        ],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I8(5)),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(target),
            },
            equality(
                result.clone(),
                i8_type,
                Operand::RawMove(pointer.into()),
                Operand::Constant(Value::I8(5)),
            ),
            Statement::Read { src: result.into() },
        ],
    );

    validate_program(program).expect("IntegerEq must preserve existing RawMove integer semantics");
}

#[test]
fn integer_eq_preserves_reference_copy_and_move_permission_semantics() {
    let mut types = TypeTable::new();
    let i8_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let shared_i8 = types.push(TypeDef::reference(
        "shared-i8",
        i8_type,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let access = ReferenceAccess::new(reference.clone());
    let valid = one_block(
        types.clone(),
        vec![
            LocalDecl::new("target", i8_type, false),
            LocalDecl::new("reference", shared_i8, false),
            LocalDecl::new("result", bool_type, false),
        ],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I8(9)),
            },
            Statement::Init {
                dst: reference.clone(),
                src: Operand::ReferenceRoot {
                    permission: ReferencePermission::Shared,
                    place: target,
                },
            },
            equality(
                result,
                i8_type,
                Operand::ReferenceCopy(access.clone()),
                Operand::Constant(Value::I8(9)),
            ),
        ],
    );
    validate_program(valid).expect("Shared ReferenceCopy remains a valid integer producer");

    let invalid = one_block(
        types,
        vec![
            LocalDecl::new("target", i8_type, false),
            LocalDecl::new("reference", shared_i8, false),
            LocalDecl::new("result", bool_type, false),
        ],
        vec![equality(
            Place::local(LocalId(2)),
            i8_type,
            Operand::ReferenceMove(access),
            Operand::Constant(Value::I8(9)),
        )],
    );
    let error = validate_program(invalid).expect_err("Shared ReferenceMove remains permission-invalid");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferencePermissionRequired(ReferencePermission::Exclusive)
    );
}
