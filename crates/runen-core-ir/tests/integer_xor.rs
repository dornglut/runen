mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, Program, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_program,
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
fn integer_xor_accepts_all_eight_scalar_kinds_and_immutable_vacant_destinations() {
    let cases: [(ScalarType, Value, Value); 8] = [
        (ScalarType::I8, Value::I8(1), Value::I8(2)),
        (ScalarType::I16, Value::I16(1), Value::I16(2)),
        (ScalarType::I32, Value::I32(1), Value::I32(2)),
        (ScalarType::I64, Value::I64(1), Value::I64(2)),
        (ScalarType::U8, Value::U8(1), Value::U8(2)),
        (ScalarType::U16, Value::U16(1), Value::U16(2)),
        (ScalarType::U32, Value::U32(1), Value::U32(2)),
        (ScalarType::U64, Value::U64(1), Value::U64(2)),
    ];

    for (index, (scalar, left, right)) in cases.into_iter().enumerate() {
        let mut types = TypeTable::new();
        let ty = types.push(TypeDef::scalar(format!("integer-{index}"), scalar));
        let program = one_block(
            types,
            vec![LocalDecl::new("result", ty, false)],
            vec![Statement::IntegerXor {
                dst: Place::local(LocalId(0)),
                left: Operand::Constant(left),
                right: Operand::Constant(right),
            }],
        );
        validate_program(program).expect("fixed-width integer XOR must validate");
    }
}

#[test]
fn integer_xor_rejects_non_integer_destination_with_specific_error() {
    for scalar in [
        ScalarType::Bool,
        ScalarType::F32,
        ScalarType::TrackedFixture,
    ] {
        let mut types = TypeTable::new();
        let ty = types.push(TypeDef::scalar("not-integer", scalar));
        let program = one_block(
            types,
            vec![LocalDecl::new("result", ty, false)],
            vec![Statement::IntegerXor {
                dst: Place::local(LocalId(0)),
                left: Operand::Constant(Value::Bool(false)),
                right: Operand::Constant(Value::Bool(false)),
            }],
        );
        let error = validate_program(program).expect_err("non-integer destination must fail first");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::IntegerXorRequiresInteger(ty)
        );
    }
}

#[test]
fn integer_xor_requires_exact_destination_type_identity_for_each_operand() {
    for mismatch_left in [true, false] {
        let mut types = TypeTable::new();
        let destination_ty = types.push(TypeDef::scalar("destination-i8", ScalarType::I8));
        let distinct_i8 = types.push(TypeDef::scalar("distinct-i8", ScalarType::I8));
        let program = one_block(
            types,
            vec![
                LocalDecl::new("result", destination_ty, false),
                LocalDecl::new("source", distinct_i8, false),
            ],
            vec![
                Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::Constant(Value::I8(7)),
                },
                Statement::IntegerXor {
                    dst: Place::local(LocalId(0)),
                    left: if mismatch_left {
                        Operand::Move(Place::local(LocalId(1)).into())
                    } else {
                        Operand::Constant(Value::I8(2))
                    },
                    right: if mismatch_left {
                        Operand::Constant(Value::I8(2))
                    } else {
                        Operand::Move(Place::local(LocalId(1)).into())
                    },
                },
            ],
        );

        let error = validate_program(program).expect_err("distinct integer TypeId must fail");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::TypeMismatch {
                expected: destination_ty,
            }
        );
    }
}

#[test]
fn integer_xor_checks_vacancy_before_operand_state() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let destination = Place::local(LocalId(0));
    let missing = Place::local(LocalId(1));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("result", i8_ty, false),
            LocalDecl::new("missing", i8_ty, false),
        ],
        vec![
            Statement::Init {
                dst: destination.clone(),
                src: Operand::Constant(Value::I8(9)),
            },
            Statement::IntegerXor {
                dst: destination.clone(),
                left: Operand::Move(missing.into()),
                right: Operand::Constant(Value::I8(1)),
            },
        ],
    );

    let error = validate_program(program).expect_err("occupied destination must reject first");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::IntegerXorRequiresVacant(destination)
    );
}

#[test]
fn shared_and_exclusive_overlapping_loans_block_direct_xor_destination_before_vacancy() {
    for kind in [BorrowKind::Shared, BorrowKind::Exclusive] {
        let mut types = TypeTable::new();
        let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
        let destination = Place::local(LocalId(0));
        let program = one_function_program(
            types,
            Body {
                locals: vec![LocalDecl::new("result", i8_ty, false)],
                loans: vec![LoanDecl::new("loan", i8_ty)],
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: destination.clone(),
                            src: Operand::Constant(Value::I8(3)),
                        },
                        Statement::Borrow {
                            loan: LoanId(0),
                            kind,
                            src: destination.clone().into(),
                        },
                        Statement::IntegerXor {
                            dst: destination.clone(),
                            left: Operand::Constant(Value::I8(1)),
                            right: Operand::Constant(Value::I8(2)),
                        },
                    ],
                    Terminator::Return(None),
                )],
            },
        );

        let error = validate_program(program).expect_err("overlapping loan blocks direct XOR dst");
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
fn integer_xor_evaluates_left_state_before_right_state() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let source = Place::local(LocalId(0));
    let result = Place::local(LocalId(1));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("source", i8_ty, false),
            LocalDecl::new("result", i8_ty, false),
        ],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::I8(5)),
            },
            Statement::IntegerXor {
                dst: result,
                left: Operand::Move(source.clone().into()),
                right: Operand::Move(source.clone().into()),
            },
        ],
    );

    let error = validate_program(program).expect_err("right move observes the left move");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(source)
    );
}

#[test]
fn integer_xor_result_becomes_live_once_after_both_operands() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let result = Place::local(LocalId(0));
    let program = one_block(
        types,
        vec![LocalDecl::new("result", i8_ty, false)],
        vec![
            Statement::IntegerXor {
                dst: result.clone(),
                left: Operand::Constant(Value::I8(2)),
                right: Operand::Constant(Value::I8(3)),
            },
            Statement::Read { src: result.into() },
        ],
    );

    validate_program(program).expect("successful IntegerXor initializes its destination once");
}
