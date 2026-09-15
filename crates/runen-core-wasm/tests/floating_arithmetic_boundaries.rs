use std::sync::{Arc, Mutex};

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LocalDecl, LocalId,
    NumericContract, Operand, Place, Program, SafeReferenceResultContract, ScalarType, Statement,
    Terminator, TypeDef, TypeId, TypeTable, Value, validate_program,
};
use runen_core_wasm::{
    ExecutionOutcome, ExternalProviderBinding, ExternalScalarValue, FloatingScalarValue,
    RealizedProgram,
};

#[derive(Clone, Copy)]
enum Operation {
    Add,
    Sub,
    Mul,
    Div,
}

fn interface(parameters: Vec<TypeId>, result: Option<TypeId>) -> CallableInterface {
    CallableInterface::new(parameters, result, SafeReferenceResultContract::None)
}

fn statement(
    operation: Operation,
    dst: Place,
    left: Operand,
    right: Operand,
    contract: NumericContract,
) -> Statement {
    match operation {
        Operation::Add => Statement::FloatAdd {
            dst,
            left,
            right,
            contract,
        },
        Operation::Sub => Statement::FloatSub {
            dst,
            left,
            right,
            contract,
        },
        Operation::Mul => Statement::FloatMul {
            dst,
            left,
            right,
            contract,
        },
        Operation::Div => Statement::FloatDiv {
            dst,
            left,
            right,
            contract,
        },
    }
}

fn value(scalar: &ScalarType, value: BinaryFloatValue) -> Value {
    match scalar {
        ScalarType::F16 => Value::F16(value),
        ScalarType::F32 => Value::F32(value),
        ScalarType::F64 => Value::F64(value),
        _ => panic!("floating arithmetic fixture requires a represented float"),
    }
}

fn format(scalar: &ScalarType) -> (u32, i16, i16) {
    match scalar {
        ScalarType::F16 => (11, -14, 15),
        ScalarType::F32 => (24, -126, 127),
        ScalarType::F64 => (53, -1022, 1023),
        _ => panic!("boundary fixture only supports represented binary floats"),
    }
}

fn normal(sign: BinaryFloatSign, significand: u64, exponent: i16) -> BinaryFloatValue {
    BinaryFloatValue::Normal {
        sign,
        significand,
        exponent,
    }
}

fn observe(
    scalar: &ScalarType,
    operation: Operation,
    contract: NumericContract,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> FloatingScalarValue {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("Float", scalar.clone()));
    let observer = interface(vec![ty], None);
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(observer.clone())],
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("result", ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![
                    BasicBlock::new(
                        vec![statement(
                            operation,
                            Place::local(LocalId(0)),
                            Operand::Constant(value(scalar, left)),
                            Operand::Constant(value(scalar, right)),
                            contract,
                        )],
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                            destination: None,
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(Vec::new(), Terminator::Return(None)),
                ],
            },
        }],
    })
    .expect("boundary fixture must be valid Core");

    let seen = Arc::new(Mutex::new(None));
    let capture = Arc::clone(&seen);
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            observer,
            move |arguments| {
                let [argument] = arguments else {
                    panic!("boundary observer expects one value");
                };
                let value = match argument {
                    ExternalScalarValue::F16(value)
                    | ExternalScalarValue::F32(value)
                    | ExternalScalarValue::F64(value) => *value,
                    other => panic!("boundary observer received {other:?}"),
                };
                *capture.lock().unwrap() = Some(value);
            },
        )],
    )
    .expect("represented floating boundary arithmetic must realize");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(None)
    );
    seen.lock()
        .unwrap()
        .expect("boundary observer must receive the result")
}

#[test]
fn baseline_rounding_subnormal_overflow_and_signed_zero_results_are_exact() {
    for scalar in [ScalarType::F16, ScalarType::F32, ScalarType::F64] {
        let (precision, emin, emax) = format(&scalar);
        let one = normal(BinaryFloatSign::Positive, 1_u64 << (precision - 1), 0);
        let two = normal(BinaryFloatSign::Positive, 1_u64 << (precision - 1), 1);
        let half_ulp_at_one = normal(
            BinaryFloatSign::Positive,
            1_u64 << (precision - 1),
            -(precision as i16),
        );
        let min_normal = normal(BinaryFloatSign::Positive, 1_u64 << (precision - 1), emin);
        let max_finite = normal(BinaryFloatSign::Positive, (1_u64 << precision) - 1, emax);
        let negative_zero = BinaryFloatValue::Zero(BinaryFloatSign::Negative);
        let positive_zero = BinaryFloatValue::Zero(BinaryFloatSign::Positive);

        for contract in [
            NumericContract::Standard,
            NumericContract::Reproducible,
            NumericContract::Fast,
        ] {
            assert_eq!(
                observe(&scalar, Operation::Add, contract, one, half_ulp_at_one),
                FloatingScalarValue::Represented(one),
                "half-ULP tie at one must round to the even significand"
            );
            assert_eq!(
                observe(
                    &scalar,
                    Operation::Add,
                    contract,
                    negative_zero,
                    negative_zero,
                ),
                FloatingScalarValue::Represented(negative_zero),
                "negative zero plus negative zero must stay negative zero"
            );
            assert_eq!(
                observe(
                    &scalar,
                    Operation::Sub,
                    contract,
                    negative_zero,
                    positive_zero,
                ),
                FloatingScalarValue::Represented(negative_zero),
                "negative zero minus positive zero must be negative zero"
            );
            assert_eq!(
                observe(&scalar, Operation::Mul, contract, negative_zero, one),
                FloatingScalarValue::Represented(negative_zero),
                "zero multiplication must preserve the product sign"
            );
            assert_eq!(
                observe(&scalar, Operation::Div, contract, negative_zero, one),
                FloatingScalarValue::Represented(negative_zero),
                "zero division must preserve the quotient sign"
            );
            assert_eq!(
                observe(&scalar, Operation::Div, contract, min_normal, two),
                FloatingScalarValue::Represented(BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Positive,
                    significand: 1_u64 << (precision - 2),
                }),
                "baseline realization must preserve a subnormal result, including Fast"
            );
            assert_eq!(
                observe(&scalar, Operation::Add, contract, max_finite, max_finite),
                FloatingScalarValue::Represented(BinaryFloatValue::Infinity(
                    BinaryFloatSign::Positive,
                )),
                "finite overflow must round to positive infinity"
            );
            assert_eq!(
                observe(&scalar, Operation::Div, contract, one, negative_zero),
                FloatingScalarValue::Represented(BinaryFloatValue::Infinity(
                    BinaryFloatSign::Negative,
                )),
                "division by negative zero must produce negative infinity"
            );
        }
    }
}

#[test]
fn f16_underflow_normal_transition_and_overflow_boundaries_are_directly_exercised() {
    let scalar = ScalarType::F16;
    let min_subnormal = BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Positive,
        significand: 1,
    };
    let max_subnormal = BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Positive,
        significand: 1023,
    };
    let positive_zero = BinaryFloatValue::Zero(BinaryFloatSign::Positive);
    let min_normal = normal(BinaryFloatSign::Positive, 1_u64 << 10, -14);
    let normal_boundary_numerator = normal(BinaryFloatSign::Positive, 2047, -14);
    let one = normal(BinaryFloatSign::Positive, 1_u64 << 10, 0);
    let below_two = normal(BinaryFloatSign::Positive, 2047, 0);
    let two = normal(BinaryFloatSign::Positive, 1_u64 << 10, 1);
    let above_two = normal(BinaryFloatSign::Positive, 1025, 1);
    let max_finite = normal(BinaryFloatSign::Positive, 2047, 15);
    let sixteen = normal(BinaryFloatSign::Positive, 1_u64 << 10, 4);
    let eight = normal(BinaryFloatSign::Positive, 1_u64 << 10, 3);

    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            min_subnormal,
            two,
        ),
        FloatingScalarValue::Represented(positive_zero),
        "exact 2^-25 underflow midpoint ties to even zero"
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            min_subnormal,
            below_two,
        ),
        FloatingScalarValue::Represented(min_subnormal),
        "quotient immediately above the zero/min-subnormal midpoint rounds upward"
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            min_subnormal,
            above_two,
        ),
        FloatingScalarValue::Represented(positive_zero),
        "quotient immediately below the zero/min-subnormal midpoint rounds downward"
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            normal_boundary_numerator,
            two,
        ),
        FloatingScalarValue::Represented(min_normal),
        "exact max-subnormal/min-normal midpoint ties to the even minimum-normal significand"
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            normal_boundary_numerator,
            below_two,
        ),
        FloatingScalarValue::Represented(min_normal),
        "quotient above the subnormal/normal midpoint rounds to minimum normal"
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            normal_boundary_numerator,
            above_two,
        ),
        FloatingScalarValue::Represented(max_subnormal),
        "quotient below the subnormal/normal midpoint rounds to maximum subnormal"
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            min_normal,
            one,
        ),
        FloatingScalarValue::Represented(min_normal),
        "minimum normal survives direct narrowing"
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Add,
            NumericContract::Standard,
            max_finite,
            sixteen,
        ),
        FloatingScalarValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
        "exact 65520 overflow midpoint rounds to infinity"
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Add,
            NumericContract::Standard,
            max_finite,
            eight,
        ),
        FloatingScalarValue::Represented(max_finite),
        "value below the overflow midpoint rounds to maximum finite"
    );
}

#[test]
fn f16_division_rounds_recurring_and_adversarial_near_midpoint_quotients() {
    let scalar = ScalarType::F16;
    let one = normal(BinaryFloatSign::Positive, 1024, 0);
    let three = normal(BinaryFloatSign::Positive, 1536, 1);
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            one,
            three,
        ),
        FloatingScalarValue::Represented(normal(BinaryFloatSign::Positive, 1365, -2)),
        "one third rounds to the expected binary16 neighbor"
    );

    let left = normal(BinaryFloatSign::Positive, 1981, 0);
    let right = normal(BinaryFloatSign::Positive, 2025, 0);
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            left,
            right,
        ),
        FloatingScalarValue::Represented(normal(BinaryFloatSign::Positive, 2004, -1)),
        "a quotient very near an F16 midpoint must stay on the mathematically correct side"
    );
}

#[test]
fn f16_special_values_and_nan_class_are_normalized_semantically() {
    let scalar = ScalarType::F16;
    let positive_zero = BinaryFloatValue::Zero(BinaryFloatSign::Positive);
    let positive_infinity = BinaryFloatValue::Infinity(BinaryFloatSign::Positive);
    let negative_infinity = BinaryFloatValue::Infinity(BinaryFloatSign::Negative);
    let one = normal(BinaryFloatSign::Positive, 1024, 0);

    assert_eq!(
        observe(
            &scalar,
            Operation::Mul,
            NumericContract::Standard,
            positive_zero,
            positive_infinity,
        ),
        FloatingScalarValue::NaNClass
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            positive_infinity,
            positive_infinity,
        ),
        FloatingScalarValue::NaNClass
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Add,
            NumericContract::Standard,
            positive_infinity,
            negative_infinity,
        ),
        FloatingScalarValue::NaNClass
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Div,
            NumericContract::Standard,
            positive_zero,
            positive_zero,
        ),
        FloatingScalarValue::NaNClass
    );
    assert_eq!(
        observe(
            &scalar,
            Operation::Add,
            NumericContract::Standard,
            positive_infinity,
            one,
        ),
        FloatingScalarValue::Represented(positive_infinity)
    );
}
