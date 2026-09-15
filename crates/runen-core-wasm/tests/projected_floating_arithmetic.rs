use std::sync::{Arc, Mutex};

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Field, Function, FunctionId, LocalDecl, LocalId,
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
        ScalarType::F16 => Value::F16(value),
        ScalarType::F32 => Value::F32(value),
        ScalarType::F64 => Value::F64(value),
        _ => panic!("projected floating arithmetic supports only F16/F32/F64"),
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

fn seen_from_wasm(value: &ExternalScalarValue) -> SeenFloat {
    match value {
        ExternalScalarValue::F16(FloatingScalarValue::Represented(value))
        | ExternalScalarValue::F32(FloatingScalarValue::Represented(value))
        | ExternalScalarValue::F64(FloatingScalarValue::Represented(value)) => {
            SeenFloat::Represented(*value)
        }
        ExternalScalarValue::F16(FloatingScalarValue::NaNClass)
        | ExternalScalarValue::F32(FloatingScalarValue::NaNClass)
        | ExternalScalarValue::F64(FloatingScalarValue::NaNClass) => SeenFloat::NaNClass,
        other => panic!("projected observer received non-floating Core-Wasm value: {other:?}"),
    }
}

fn seen_from_reference(value: &ReferenceScalarValue) -> SeenFloat {
    match value {
        ReferenceScalarValue::F16(ObservedBinaryFloatValue::Represented(value))
        | ReferenceScalarValue::F32(ObservedBinaryFloatValue::Represented(value))
        | ReferenceScalarValue::F64(ObservedBinaryFloatValue::Represented(value)) => {
            SeenFloat::Represented(*value)
        }
        ReferenceScalarValue::F16(ObservedBinaryFloatValue::NaNClass)
        | ReferenceScalarValue::F32(ObservedBinaryFloatValue::NaNClass)
        | ReferenceScalarValue::F64(ObservedBinaryFloatValue::NaNClass) => SeenFloat::NaNClass,
        other => panic!("projected observer received non-floating reference value: {other:?}"),
    }
}

fn format(scalar: &ScalarType) -> u32 {
    match scalar {
        ScalarType::F16 => 11,
        ScalarType::F32 => 24,
        ScalarType::F64 => 53,
        _ => panic!("projected floating arithmetic supports only F16/F32/F64"),
    }
}

fn normal(sign: BinaryFloatSign, significand: u64, exponent: i16) -> BinaryFloatValue {
    BinaryFloatValue::Normal {
        sign,
        significand,
        exponent,
    }
}

fn projected_program(
    scalar: &ScalarType,
    operation: Operation,
    contract: NumericContract,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> (ValidatedProgram, CallableInterface, BinaryFloatValue) {
    let mut types = TypeTable::new();
    let u8_ty = types.push(TypeDef::scalar("U8", ScalarType::U8));
    let float_ty = types.push(TypeDef::scalar("Float", scalar.clone()));
    let inner_ty = types.push(TypeDef::structure(
        "Inner",
        vec![
            Field::new("result", float_ty),
            Field::new("sibling", float_ty),
        ],
    ));
    let outer_ty = types.push(TypeDef::structure(
        "Outer",
        vec![Field::new("tag", u8_ty), Field::new("inner", inner_ty)],
    ));
    let observer = interface(vec![float_ty, u8_ty, float_ty], None);

    let precision = format(scalar);
    let sibling = normal(BinaryFloatSign::Negative, 1_u64 << (precision - 1), 0);
    let aggregate = Place::local(LocalId(0));
    let result = aggregate.clone().field(1).field(0);
    let sibling_place = aggregate.clone().field(1).field(1);
    let tag = aggregate.clone().field(0);

    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(observer.clone())],
        functions: vec![function(
            vec![LocalDecl::new("aggregate", outer_ty, false)],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: tag.clone(),
                            src: Operand::Constant(Value::U8(7)),
                        },
                        Statement::Init {
                            dst: sibling_place.clone(),
                            src: Operand::Constant(value(scalar, sibling)),
                        },
                        statement(
                            operation,
                            result.clone(),
                            Operand::Constant(value(scalar, left)),
                            Operand::Constant(value(scalar, right)),
                            contract,
                        ),
                    ],
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: vec![
                            Operand::Move(result.into()),
                            Operand::Move(tag.into()),
                            Operand::Move(sibling_place.into()),
                        ],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        )],
    })
    .expect("nested projected floating arithmetic fixture must be valid Core");

    (program, observer, sibling)
}

fn run_differential(
    scalar: &ScalarType,
    operation: Operation,
    contract: NumericContract,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> SeenFloat {
    let (program, observer, sibling) = projected_program(scalar, operation, contract, left, right);

    let wasm_seen = Arc::new(Mutex::new(None));
    let wasm_capture = Arc::clone(&wasm_seen);
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            observer.clone(),
            move |arguments| {
                let [result, tag, preserved_sibling] = arguments else {
                    panic!("projected observer expects result, tag, and sibling");
                };
                assert_eq!(tag, &ExternalScalarValue::U8(7));
                assert_eq!(
                    seen_from_wasm(preserved_sibling),
                    SeenFloat::Represented(sibling),
                    "projected Float* must not disturb an already-live disjoint sibling"
                );
                *wasm_capture.lock().unwrap() = Some(seen_from_wasm(result));
            },
        )],
    )
    .expect("nested projected F16/F32/F64 arithmetic fixture must realize");
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
                let [result, tag, preserved_sibling] = arguments else {
                    panic!("reference projected observer expects result, tag, and sibling");
                };
                assert_eq!(tag, &ReferenceScalarValue::U8(7));
                assert_eq!(
                    seen_from_reference(preserved_sibling),
                    SeenFloat::Represented(sibling),
                    "reference projected Float* must preserve the disjoint sibling"
                );
                *reference_capture.lock().unwrap() = Some(seen_from_reference(result));
            },
        )],
    )
    .expect("reference projected floating fixture must admit")
    .execute()
    .expect("reference projected floating fixture must execute");
    assert_eq!(report.terminal, TerminalStatus::Returned);

    let wasm = wasm_seen
        .lock()
        .unwrap()
        .expect("Core-Wasm projected observer must run");
    let reference = reference_seen
        .lock()
        .unwrap()
        .expect("reference projected observer must run");
    assert_eq!(wasm, reference);
    wasm
}

#[test]
fn nested_projected_destinations_cover_all_formats_operations_and_contracts_differentially() {
    for scalar in [ScalarType::F16, ScalarType::F32, ScalarType::F64] {
        let precision = format(&scalar);
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
fn projected_arithmetic_nan_reaches_provider_as_semantic_nan_class() {
    let zero = BinaryFloatValue::Zero(BinaryFloatSign::Positive);
    assert_eq!(
        run_differential(
            &ScalarType::F16,
            Operation::Div,
            NumericContract::Standard,
            zero,
            zero,
        ),
        SeenFloat::NaNClass
    );
}
