mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, BorrowKind, LoanDecl,
    LoanId, LocalDecl, LocalId, MirValidationErrorKind, NumericContract, Operand, Place,
    PlaceAccess, Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
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

fn zero_value(scalar: ScalarType) -> Value {
    let zero = BinaryFloatValue::Zero(BinaryFloatSign::Positive);
    match scalar {
        ScalarType::F16 => Value::F16(zero),
        ScalarType::F32 => Value::F32(zero),
        ScalarType::F64 => Value::F64(zero),
        _ => unreachable!("floating-add fixture requests only represented floating kinds"),
    }
}

#[test]
fn float_add_accepts_all_three_scalar_kinds_all_contracts_and_immutable_vacant_destinations() {
    for (index, scalar) in [ScalarType::F16, ScalarType::F32, ScalarType::F64]
        .into_iter()
        .enumerate()
    {
        for contract in [
            NumericContract::Standard,
            NumericContract::Reproducible,
            NumericContract::Fast,
        ] {
            let mut types = TypeTable::new();
            let ty = types.push(TypeDef::scalar(format!("float-{index}"), scalar));
            let program = one_block(
                types,
                vec![LocalDecl::new("result", ty, false)],
                vec![Statement::FloatAdd {
                    contract,
                    dst: Place::local(LocalId(0)),
                    left: Operand::Constant(zero_value(scalar)),
                    right: Operand::Constant(zero_value(scalar)),
                }],
            );
            validate_program(program)
                .expect("represented same-format FloatAdd contract must validate");
        }
    }
}

#[test]
fn float_add_rejects_non_floating_destination_with_specific_error() {
    for scalar in [
        ScalarType::Bool,
        ScalarType::I32,
        ScalarType::TrackedFixture,
    ] {
        let mut types = TypeTable::new();
        let ty = types.push(TypeDef::scalar("not-float", scalar));
        let program = one_block(
            types,
            vec![LocalDecl::new("result", ty, false)],
            vec![Statement::FloatAdd {
                contract: NumericContract::Standard,
                dst: Place::local(LocalId(0)),
                left: Operand::Constant(Value::Bool(false)),
                right: Operand::Constant(Value::Bool(false)),
            }],
        );
        let error =
            validate_program(program).expect_err("non-floating destination must fail first");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::FloatAddRequiresFloat(ty)
        );
    }
}

#[test]
fn float_add_requires_exact_destination_type_identity_for_both_operands() {
    let mut types = TypeTable::new();
    let destination_ty = types.push(TypeDef::scalar("left-f32", ScalarType::F32));
    let distinct_f32 = types.push(TypeDef::scalar("right-f32", ScalarType::F32));
    let source = Place::local(LocalId(1));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("result", destination_ty, false),
            LocalDecl::new("source", distinct_f32, false),
        ],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::F32(BinaryFloatValue::Zero(
                    BinaryFloatSign::Positive,
                ))),
            },
            Statement::FloatAdd {
                contract: NumericContract::Standard,
                dst: Place::local(LocalId(0)),
                left: Operand::Move(source.into()),
                right: Operand::Constant(Value::F32(BinaryFloatValue::Zero(
                    BinaryFloatSign::Positive,
                ))),
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
fn float_add_rejects_cross_format_operand() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let f64_ty = types.push(TypeDef::scalar("f64", ScalarType::F64));
    let source = Place::local(LocalId(1));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("result", f32_ty, false),
            LocalDecl::new("source", f64_ty, false),
        ],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(Value::F64(BinaryFloatValue::Zero(
                    BinaryFloatSign::Positive,
                ))),
            },
            Statement::FloatAdd {
                contract: NumericContract::Standard,
                dst: Place::local(LocalId(0)),
                left: Operand::Move(source.into()),
                right: Operand::Constant(Value::F32(BinaryFloatValue::Zero(
                    BinaryFloatSign::Positive,
                ))),
            },
        ],
    );

    let error = validate_program(program).expect_err("cross-format FloatAdd must fail");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: f32_ty }
    );
}

#[test]
fn float_add_checks_vacancy_before_operand_state() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let destination = Place::local(LocalId(0));
    let missing = Place::local(LocalId(1));
    let zero = Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("result", f32_ty, false),
            LocalDecl::new("missing", f32_ty, false),
        ],
        vec![
            Statement::Init {
                dst: destination.clone(),
                src: Operand::Constant(zero.clone()),
            },
            Statement::FloatAdd {
                contract: NumericContract::Standard,
                dst: destination.clone(),
                left: Operand::Move(missing.into()),
                right: Operand::Constant(zero),
            },
        ],
    );

    let error = validate_program(program).expect_err("occupied destination must reject first");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::FloatAddRequiresVacant(destination)
    );
}

#[test]
fn float_add_checks_direct_destination_authority_before_operands() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let destination = Place::local(LocalId(0));
    let zero = Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let program = one_function_program(
        types,
        Body {
            locals: vec![
                LocalDecl::new("result", f32_ty, false),
                LocalDecl::new("moved", f32_ty, false),
            ],
            loans: vec![LoanDecl::new("loan", f32_ty)],
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: destination.clone(),
                        src: Operand::Constant(zero.clone()),
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
                    Statement::FloatAdd {
                        contract: NumericContract::Standard,
                        dst: destination.clone(),
                        left: Operand::Move(Place::local(LocalId(1)).into()),
                        right: Operand::Constant(zero),
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
fn float_add_evaluates_left_state_before_right_state() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let source = Place::local(LocalId(0));
    let result = Place::local(LocalId(1));
    let zero = Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let program = one_block(
        types,
        vec![
            LocalDecl::new("source", f32_ty, false),
            LocalDecl::new("result", f32_ty, false),
        ],
        vec![
            Statement::Init {
                dst: source.clone(),
                src: Operand::Constant(zero),
            },
            Statement::FloatAdd {
                contract: NumericContract::Standard,
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
fn float_add_result_becomes_live_once_after_both_operands() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let result = Place::local(LocalId(0));
    let zero = Value::F32(BinaryFloatValue::Zero(BinaryFloatSign::Positive));
    let program = one_block(
        types,
        vec![LocalDecl::new("result", f32_ty, false)],
        vec![
            Statement::FloatAdd {
                contract: NumericContract::Standard,
                dst: result.clone(),
                left: Operand::Constant(zero.clone()),
                right: Operand::Constant(zero),
            },
            Statement::Read { src: result.into() },
        ],
    );

    validate_program(program)
        .expect("successful FloatAdd initializes its destination exactly once");
}
