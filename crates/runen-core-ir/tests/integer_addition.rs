mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, LoanDecl, LoanId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, PlaceAccess, Program, ScalarType, Statement,
    Terminator, TypeDef, TypeTable, Value, validate_program,
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
fn integer_add_accepts_all_eight_scalar_kinds_and_immutable_vacant_destinations() {
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
            vec![Statement::IntegerAdd {
                dst: Place::local(LocalId(0)),
                left: Operand::Constant(left),
                right: Operand::Constant(right),
            }],
        );
        validate_program(program).expect("fixed-width integer add must validate");
    }
}

#[test]
fn integer_add_rejects_non_integer_destination_with_specific_error() {
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
            vec![Statement::IntegerAdd {
                dst: Place::local(LocalId(0)),
                left: Operand::Constant(Value::Bool(false)),
                right: Operand::Constant(Value::Bool(false)),
            }],
        );
        let error = validate_program(program).expect_err("non-integer destination must fail first");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::IntegerAddRequiresInteger(ty)
        );
    }
}

#[test]
fn integer_add_requires_exact_destination_type_identity_for_both_operands() {
    let mut types = TypeTable::new();
    let destination_ty = types.push(TypeDef::scalar("left-i8", ScalarType::I8));
    let distinct_i8 = types.push(TypeDef::scalar("right-i8", ScalarType::I8));
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
            Statement::IntegerAdd {
                dst: Place::local(LocalId(0)),
                left: Operand::Move(Place::local(LocalId(1)).into()),
                right: Operand::Constant(Value::I8(1)),
            },
        ],
    );

    let error = validate_program(program).expect_err("same scalar kind with distinct TypeId fails");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch {
            expected: destination_ty,
        }
    );
}

#[test]
fn integer_add_checks_vacancy_before_operand_state() {
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
            Statement::IntegerAdd {
                dst: destination.clone(),
                left: Operand::Move(missing.into()),
                right: Operand::Constant(Value::I8(1)),
            },
        ],
    );

    let error = validate_program(program).expect_err("occupied destination must reject first");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::IntegerAddRequiresVacant(destination)
    );
}

#[test]
fn integer_add_checks_direct_destination_authority_before_operands() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let destination = Place::local(LocalId(0));
    let program = one_function_program(
        types,
        Body {
            locals: vec![
                LocalDecl::new("result", i8_ty, false),
                LocalDecl::new("missing", i8_ty, false),
            ],
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
                        kind: BorrowKind::Exclusive,
                        src: destination.clone().into(),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Move(PlaceAccess::loan(LoanId(0))),
                    },
                    Statement::IntegerAdd {
                        dst: destination.clone(),
                        left: Operand::Move(Place::local(LocalId(1)).into()),
                        right: Operand::Constant(Value::I8(1)),
                    },
                ],
                Terminator::Return(None),
            )],
        },
    );

    let error = validate_program(program).expect_err("active overlapping loan blocks destination");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: destination,
            loan: LoanId(0),
        }
    );
}

#[test]
fn integer_add_evaluates_left_state_before_right_state() {
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
            Statement::IntegerAdd {
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
fn integer_add_result_becomes_live_once_after_both_operands() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let result = Place::local(LocalId(0));
    let program = one_block(
        types,
        vec![LocalDecl::new("result", i8_ty, false)],
        vec![
            Statement::IntegerAdd {
                dst: result.clone(),
                left: Operand::Constant(Value::I8(2)),
                right: Operand::Constant(Value::I8(3)),
            },
            Statement::Read { src: result.into() },
        ],
    );

    validate_program(program)
        .expect("successful IntegerAdd initializes its destination exactly once");
}
