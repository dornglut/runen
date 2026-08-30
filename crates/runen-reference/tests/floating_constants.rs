use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, Field, Function, FunctionId,
    LocalDecl, LocalId, Operand, Place, Program, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_program,
};
use runen_reference::{
    Machine, ObservedBinaryFloatValue, ObservedValue, TerminalStatus, VerificationEventKind,
};

fn execute_direct_result(scalar: ScalarType, value: Value) -> runen_reference::ExecutionReport {
    let mut types = TypeTable::new();
    let result_ty = types.push(TypeDef::scalar("result", scalar));
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(result_ty),
            shared_reference_result_origin: None,
            body: Body {
                locals: Vec::new(),
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(value))),
                )],
            },
        }],
    };
    let validated = validate_program(program).expect("matching floating result must validate");
    Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("floating constant return is defined")
}

fn observed_from_constant(value: &Value) -> ObservedValue {
    match value {
        Value::Bool(value) => ObservedValue::Bool(*value),
        Value::I8(value) => ObservedValue::I8(*value),
        Value::I16(value) => ObservedValue::I16(*value),
        Value::I32(value) => ObservedValue::I32(*value),
        Value::I64(value) => ObservedValue::I64(*value),
        Value::U8(value) => ObservedValue::U8(*value),
        Value::U16(value) => ObservedValue::U16(*value),
        Value::U32(value) => ObservedValue::U32(*value),
        Value::U64(value) => ObservedValue::U64(*value),
        Value::F16(value) => ObservedValue::F16(ObservedBinaryFloatValue::Represented(*value)),
        Value::F32(value) => ObservedValue::F32(ObservedBinaryFloatValue::Represented(*value)),
        Value::F64(value) => ObservedValue::F64(ObservedBinaryFloatValue::Represented(*value)),
        Value::TrackedFixture(value) => ObservedValue::TrackedFixture(*value),
        Value::Struct(values) => {
            ObservedValue::Struct(values.iter().map(observed_from_constant).collect())
        }
    }
}

#[test]
fn direct_results_preserve_semantic_floating_classes_exactly() {
    let cases = [
        (
            ScalarType::F16,
            Value::F16(BinaryFloatValue::Zero(BinaryFloatSign::Negative)),
        ),
        (
            ScalarType::F16,
            Value::F16(BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Positive,
                significand: 1,
            }),
        ),
        (
            ScalarType::F32,
            Value::F32(BinaryFloatValue::Infinity(BinaryFloatSign::Negative)),
        ),
        (
            ScalarType::F32,
            Value::F32(BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Positive,
                significand: 1_u64 << 23,
                exponent: -126,
            }),
        ),
        (
            ScalarType::F64,
            Value::F64(BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Negative,
                significand: (1_u64 << 53) - 1,
                exponent: 1023,
            }),
        ),
    ];

    for (scalar, value) in cases {
        let expected = observed_from_constant(&value);
        let report = execute_direct_result(scalar, value);
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
        assert!(report.verification_events.is_empty());
    }
}

#[test]
fn floating_value_survives_init_copy_and_move_exactly() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let input = Value::F32(BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand: (1_u64 << 23) + 17,
        exponent: 5,
    });
    let expected = observed_from_constant(&input);
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
            shared_reference_result_origin: None,
            body: Body {
                locals: vec![
                    LocalDecl::new("source", f32_ty, false),
                    LocalDecl::new("target", f32_ty, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(input),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::Copy(Place::local(LocalId(0)).into()),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
                )],
            },
        }],
    };

    let validated = validate_program(program).expect("floating copy program must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("floating copy transport is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(expected));
}

#[test]
fn floating_value_survives_assignment_and_move_exactly() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let initial = Value::F32(BinaryFloatValue::Infinity(BinaryFloatSign::Positive));
    let replacement = Value::F32(BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Negative,
        significand: 37,
    });
    let expected = observed_from_constant(&replacement);
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
            shared_reference_result_origin: None,
            body: Body {
                locals: vec![LocalDecl::new("target", f32_ty, true)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(initial),
                        },
                        Statement::Assign {
                            dst: Place::local(LocalId(0)).into(),
                            src: Operand::Constant(replacement),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            },
        }],
    };

    let validated = validate_program(program).expect("floating assignment program must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("floating assignment transport is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(expected));
    assert!(
        !report
            .verification_events
            .iter()
            .any(|event| matches!(event.kind, VerificationEventKind::DropTrackedFixture { .. })),
        "ordinary floating transport must not invent floating-specific cleanup"
    );
}

#[test]
fn f64_round_trips_through_direct_call_argument_and_result() {
    let mut types = TypeTable::new();
    let f64_ty = types.push(TypeDef::scalar("f64", ScalarType::F64));
    let input = Value::F64(BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Negative,
        significand: 1_u64 << 52,
        exponent: -1022,
    });
    let expected = observed_from_constant(&input);

    let caller = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(f64_ty),
        shared_reference_result_origin: None,
        body: Body {
            locals: vec![LocalDecl::new("result", f64_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Constant(input)],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        },
    };
    let callee = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(f64_ty),
        shared_reference_result_origin: None,
        body: Body {
            locals: vec![LocalDecl::new("value", f64_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        },
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("f64 call transport must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("f64 call transport is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(expected));
}

#[test]
fn mixed_floating_struct_round_trips_through_storage() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("f16", ScalarType::F16));
    let f64_ty = types.push(TypeDef::scalar("f64", ScalarType::F64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("small", f16_ty), Field::new("large", f64_ty)],
    ));
    let input = Value::Struct(vec![
        Value::F16(BinaryFloatValue::Infinity(BinaryFloatSign::Positive)),
        Value::F64(BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Negative,
            significand: (1_u64 << 52) - 1,
        }),
    ]);
    let expected = observed_from_constant(&input);
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(pair_ty),
            shared_reference_result_origin: None,
            body: Body {
                locals: vec![LocalDecl::new("pair", pair_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(input),
                    }],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            },
        }],
    };

    let validated = validate_program(program).expect("mixed floating struct must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("mixed floating struct transport is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(expected));
}
