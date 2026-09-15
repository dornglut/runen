use std::sync::{Arc, Mutex};

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LocalDecl, LocalId,
    NumericContract, Operand, Place, Program, SafeReferenceResultContract, ScalarType, Statement,
    Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{
    ExecutionOutcome, ExternalProviderBinding, ExternalScalarValue, FloatingScalarValue,
    RealizedProgram,
};
use runen_reference::{
    ExternalProviderBinding as ReferenceProviderBinding,
    ExternalScalarValue as ReferenceScalarValue, Machine, ObservedBinaryFloatValue, TerminalStatus,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operation {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeenFloat {
    Represented(BinaryFloatValue),
    NaNClass,
}

fn interface(parameters: Vec<TypeId>, result: Option<TypeId>) -> CallableInterface {
    CallableInterface::new(parameters, result, SafeReferenceResultContract::None)
}

fn function(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Function {
    Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals,
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks,
        },
    }
}

fn value(scalar: &ScalarType, value: BinaryFloatValue) -> Value {
    match scalar {
        ScalarType::F32 => Value::F32(value),
        ScalarType::F64 => Value::F64(value),
        _ => panic!("floating arithmetic test only supports F32/F64"),
    }
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

fn represented_from_wasm(value: &ExternalScalarValue) -> SeenFloat {
    match value {
        ExternalScalarValue::F32(FloatingScalarValue::Represented(value))
        | ExternalScalarValue::F64(FloatingScalarValue::Represented(value)) => {
            SeenFloat::Represented(*value)
        }
        ExternalScalarValue::F32(FloatingScalarValue::NaNClass)
        | ExternalScalarValue::F64(FloatingScalarValue::NaNClass) => SeenFloat::NaNClass,
        other => panic!("observer received non-floating Core-Wasm value: {other:?}"),
    }
}

fn represented_from_reference(value: &ReferenceScalarValue) -> SeenFloat {
    match value {
        ReferenceScalarValue::F32(ObservedBinaryFloatValue::Represented(value))
        | ReferenceScalarValue::F64(ObservedBinaryFloatValue::Represented(value)) => {
            SeenFloat::Represented(*value)
        }
        ReferenceScalarValue::F32(ObservedBinaryFloatValue::NaNClass)
        | ReferenceScalarValue::F64(ObservedBinaryFloatValue::NaNClass) => SeenFloat::NaNClass,
        other => panic!("observer received non-floating reference value: {other:?}"),
    }
}

fn validated_observation_program(
    scalar: &ScalarType,
    operation: Operation,
    contract: NumericContract,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> (ValidatedProgram, CallableInterface) {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("Float", scalar.clone()));
    let observer = interface(vec![ty], None);
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(observer.clone())],
        functions: vec![function(
            vec![LocalDecl::new("result", ty, false)],
            vec![
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
        )],
    })
    .expect("floating arithmetic observation fixture must be valid Core");
    (program, observer)
}

fn run_differential(
    scalar: &ScalarType,
    operation: Operation,
    contract: NumericContract,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> SeenFloat {
    let (program, observer) =
        validated_observation_program(scalar, operation, contract, left, right);

    let wasm_seen = Arc::new(Mutex::new(None));
    let wasm_capture = Arc::clone(&wasm_seen);
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            observer.clone(),
            move |arguments| {
                let [argument] = arguments else {
                    panic!("floating observer expects exactly one argument");
                };
                *wasm_capture.lock().unwrap() = Some(represented_from_wasm(argument));
            },
        )],
    )
    .expect("F32/F64 arithmetic fixture must realize");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(None)
    );

    let reference_seen = Arc::new(Mutex::new(None));
    let reference_capture = Arc::clone(&reference_seen);
    let report = Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![ReferenceProviderBinding::no_result(
            ExternalCallableId(0),
            observer,
            move |arguments| {
                let [argument] = arguments else {
                    panic!("reference floating observer expects exactly one argument");
                };
                *reference_capture.lock().unwrap() = Some(represented_from_reference(argument));
            },
        )],
    )
    .expect("reference floating arithmetic fixture must admit")
    .execute()
    .expect("reference floating arithmetic fixture must execute");
    assert_eq!(report.terminal, TerminalStatus::Returned);

    let wasm = wasm_seen
        .lock()
        .unwrap()
        .expect("Core-Wasm observer must run");
    let reference = reference_seen
        .lock()
        .unwrap()
        .expect("reference observer must run");
    assert_eq!(wasm, reference);
    wasm
}

fn format(scalar: &ScalarType) -> (u32, i16, i16) {
    match scalar {
        ScalarType::F32 => (24, -126, 127),
        ScalarType::F64 => (53, -1022, 1023),
        _ => panic!("floating arithmetic test only supports F32/F64"),
    }
}

fn zero(sign: BinaryFloatSign) -> BinaryFloatValue {
    BinaryFloatValue::Zero(sign)
}

fn infinity(sign: BinaryFloatSign) -> BinaryFloatValue {
    BinaryFloatValue::Infinity(sign)
}

fn normal(sign: BinaryFloatSign, significand: u64, exponent: i16) -> BinaryFloatValue {
    BinaryFloatValue::Normal {
        sign,
        significand,
        exponent,
    }
}

#[test]
fn all_basic_operations_execute_for_f32_f64_and_all_numeric_contracts() {
    for scalar in [ScalarType::F32, ScalarType::F64] {
        let (precision, _, _) = format(&scalar);
        let one = normal(BinaryFloatSign::Positive, 1_u64 << (precision - 1), 0);
        let two = normal(BinaryFloatSign::Positive, 1_u64 << (precision - 1), 1);
        for contract in [
            NumericContract::Standard,
            NumericContract::Reproducible,
            NumericContract::Fast,
        ] {
            for (operation, left, right) in [
                (Operation::Add, one, two),
                (Operation::Sub, two, one),
                (Operation::Mul, two, two),
                (Operation::Div, two, one),
            ] {
                assert_ne!(
                    run_differential(&scalar, operation, contract, left, right),
                    SeenFloat::NaNClass
                );
            }
        }
    }
}

#[test]
fn boundary_rounding_signed_zero_subnormal_overflow_and_special_values_match_reference() {
    for scalar in [ScalarType::F32, ScalarType::F64] {
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
        let min_subnormal = BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1,
        };

        let cases = [
            (Operation::Add, one, half_ulp_at_one),
            (Operation::Sub, one, one),
            (Operation::Mul, zero(BinaryFloatSign::Negative), one),
            (Operation::Div, min_normal, two),
            (
                Operation::Add,
                min_subnormal,
                zero(BinaryFloatSign::Positive),
            ),
            (Operation::Add, max_finite, max_finite),
            (Operation::Div, one, zero(BinaryFloatSign::Negative)),
            (
                Operation::Add,
                infinity(BinaryFloatSign::Positive),
                infinity(BinaryFloatSign::Negative),
            ),
            (
                Operation::Sub,
                infinity(BinaryFloatSign::Positive),
                infinity(BinaryFloatSign::Positive),
            ),
            (
                Operation::Mul,
                zero(BinaryFloatSign::Positive),
                infinity(BinaryFloatSign::Positive),
            ),
            (
                Operation::Div,
                infinity(BinaryFloatSign::Positive),
                infinity(BinaryFloatSign::Positive),
            ),
            (
                Operation::Div,
                zero(BinaryFloatSign::Positive),
                zero(BinaryFloatSign::Positive),
            ),
        ];

        for contract in [
            NumericContract::Standard,
            NumericContract::Reproducible,
            NumericContract::Fast,
        ] {
            for (operation, left, right) in cases {
                run_differential(&scalar, operation, contract, left, right);
            }
        }
    }
}

fn provider_nan_value(scalar: &ScalarType) -> ExternalScalarValue {
    match scalar {
        ScalarType::F32 => ExternalScalarValue::F32(FloatingScalarValue::NaNClass),
        ScalarType::F64 => ExternalScalarValue::F64(FloatingScalarValue::NaNClass),
        _ => panic!("floating arithmetic test only supports F32/F64"),
    }
}

#[test]
fn provider_nan_class_is_an_arithmetic_input_and_returns_to_semantic_nan_class() {
    for scalar in [ScalarType::F32, ScalarType::F64] {
        let (precision, _, _) = format(&scalar);
        let one = normal(BinaryFloatSign::Positive, 1_u64 << (precision - 1), 0);
        for operation in [
            Operation::Add,
            Operation::Sub,
            Operation::Mul,
            Operation::Div,
        ] {
            let mut types = TypeTable::new();
            let ty = types.push(TypeDef::scalar("Float", scalar.clone()));
            let produce = interface(Vec::new(), Some(ty));
            let observe = interface(vec![ty], None);
            let program = validate_program(Program {
                types,
                persistent: Vec::new(),
                external_callables: vec![
                    ExternalCallableDecl::new(produce.clone()),
                    ExternalCallableDecl::new(observe.clone()),
                ],
                functions: vec![function(
                    vec![
                        LocalDecl::new("input", ty, false),
                        LocalDecl::new("result", ty, false),
                    ],
                    vec![
                        BasicBlock::new(
                            Vec::new(),
                            Terminator::ExternalCall {
                                external: ExternalCallableId(0),
                                arguments: Vec::new(),
                                destination: Some(Place::local(LocalId(0))),
                                target: BasicBlockId(1),
                            },
                        ),
                        BasicBlock::new(
                            vec![statement(
                                operation,
                                Place::local(LocalId(1)),
                                Operand::Move(Place::local(LocalId(0)).into()),
                                Operand::Constant(value(&scalar, one)),
                                NumericContract::Standard,
                            )],
                            Terminator::ExternalCall {
                                external: ExternalCallableId(1),
                                arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                                destination: None,
                                target: BasicBlockId(2),
                            },
                        ),
                        BasicBlock::new(Vec::new(), Terminator::Return(None)),
                    ],
                )],
            })
            .expect("provider-NaN arithmetic fixture must be valid Core");

            let expected_nan = provider_nan_value(&scalar);
            let realized = RealizedProgram::new_with_external_providers(
                &program,
                vec![
                    ExternalProviderBinding::scalar_result(
                        ExternalCallableId(0),
                        produce,
                        move |_| expected_nan,
                    ),
                    ExternalProviderBinding::no_result(
                        ExternalCallableId(1),
                        observe,
                        |arguments| {
                            let [argument] = arguments else {
                                panic!("NaN observer expects one argument");
                            };
                            assert_eq!(represented_from_wasm(argument), SeenFloat::NaNClass);
                        },
                    ),
                ],
            )
            .expect("provider NaN arithmetic must realize");
            assert_eq!(
                realized.execute(FunctionId(0)).unwrap(),
                ExecutionOutcome::Returned(None)
            );
        }
    }
}
