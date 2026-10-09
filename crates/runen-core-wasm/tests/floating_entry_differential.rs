mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, Field, Function,
    FunctionId, LocalDecl, LocalId, NumericContract, Operand, Place, Program,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId,
    TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{ExecutionOutcome, ExecutionValue, FloatingScalarValue, RealizedProgram};
use runen_reference::{Machine, TerminalStatus};

fn validated(types: TypeTable, result: TypeId, body: Body) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(result),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body,
        }],
    })
    .expect("floating observation fixture must validate")
}

fn constant_result(value: Value) -> Body {
    Body {
        locals: Vec::new(),
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(value))),
        )],
    }
}

fn compare_once_validated_core(program: ValidatedProgram) -> ExecutionOutcome {
    let reference = Machine::new(program.clone(), FunctionId(0))
        .expect("reference entry admitted")
        .execute()
        .expect("reference execution succeeds");
    assert_eq!(reference.terminal, TerminalStatus::Returned);
    let expected = ExecutionOutcome::Returned(
        reference.result.map(support::from_reference_observation),
    );
    let actual = RealizedProgram::new(&program)
        .expect("Core-Wasm realizes exactly the same Core program")
        .execute(FunctionId(0))
        .expect("Core-Wasm executes exactly the same Core program");
    assert_eq!(actual, expected);
    actual
}

#[test]
fn direct_and_nested_floating_observations_match_reference_without_carrier_bits() {
    for (scalar, make_value, precision) in [
        (ScalarType::F16, Value::F16 as fn(BinaryFloatValue) -> Value, 11),
        (ScalarType::F32, Value::F32 as fn(BinaryFloatValue) -> Value, 24),
        (ScalarType::F64, Value::F64 as fn(BinaryFloatValue) -> Value, 53),
    ] {
        let values = [
            BinaryFloatValue::Zero(BinaryFloatSign::Positive),
            BinaryFloatValue::Zero(BinaryFloatSign::Negative),
            BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Positive,
                significand: 1,
            },
            BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Negative,
                significand: 1_u64 << (precision - 1),
                exponent: 0,
            },
            BinaryFloatValue::Infinity(BinaryFloatSign::Positive),
            BinaryFloatValue::Infinity(BinaryFloatSign::Negative),
        ];
        for value in values {
            let mut direct_types = TypeTable::new();
            let direct_ty = direct_types.push(TypeDef::scalar("Float", scalar.clone()));
            compare_once_validated_core(validated(
                direct_types,
                direct_ty,
                constant_result(make_value(value)),
            ));

            let mut nested_types = TypeTable::new();
            let leaf_ty = nested_types.push(TypeDef::scalar("Float", scalar.clone()));
            let empty_ty = nested_types.push(TypeDef::structure("Empty", vec![]));
            let inner_ty = nested_types.push(TypeDef::structure(
                "Inner",
                vec![Field::new("empty", empty_ty), Field::new("value", leaf_ty)],
            ));
            let outer_ty = nested_types.push(TypeDef::structure(
                "Outer",
                vec![Field::new("inner", inner_ty)],
            ));
            let nested = Value::Struct(vec![Value::Struct(vec![
                Value::Struct(vec![]),
                make_value(value),
            ])]);
            compare_once_validated_core(validated(
                nested_types,
                outer_ty,
                constant_result(nested),
            ));
        }
    }
}

#[test]
fn arithmetic_generated_nan_class_matches_reference_in_all_formats() {
    for (scalar, make_value, expected) in [
        (
            ScalarType::F16,
            Value::F16 as fn(BinaryFloatValue) -> Value,
            ExecutionValue::F16(FloatingScalarValue::NaNClass),
        ),
        (
            ScalarType::F32,
            Value::F32 as fn(BinaryFloatValue) -> Value,
            ExecutionValue::F32(FloatingScalarValue::NaNClass),
        ),
        (
            ScalarType::F64,
            Value::F64 as fn(BinaryFloatValue) -> Value,
            ExecutionValue::F64(FloatingScalarValue::NaNClass),
        ),
    ] {
        let mut types = TypeTable::new();
        let ty = types.push(TypeDef::scalar("Float", scalar));
        let zero = Operand::Constant(make_value(BinaryFloatValue::Zero(
            BinaryFloatSign::Positive,
        )));
        let program = validated(
            types,
            ty,
            Body {
                locals: vec![LocalDecl::new("result", ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::FloatDiv {
                        dst: Place::local(LocalId(0)),
                        left: zero.clone(),
                        right: zero,
                        contract: NumericContract::Standard,
                    }],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            },
        );
        assert_eq!(
            compare_once_validated_core(program),
            ExecutionOutcome::Returned(Some(expected))
        );
    }
}
