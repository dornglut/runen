use std::sync::{Arc, Mutex};

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Field, Function, FunctionId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, PersistentDecl, PersistentId, Place, Program,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable,
    ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{
    ExecutionOutcome, ExternalProviderBinding, ExternalScalarValue, FloatingScalarValue,
    RealizationError, RealizedProgram,
};

fn interface(parameters: Vec<TypeId>, result: Option<TypeId>) -> CallableInterface {
    CallableInterface::new(parameters, result, SafeReferenceResultContract::None)
}

fn function(
    name: &str,
    parameters: Vec<LocalId>,
    result: Option<TypeId>,
    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
) -> Function {
    Function {
        name: name.into(),
        parameters,
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals,
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks,
        },
    }
}

fn validated(
    types: TypeTable,
    external_callables: Vec<ExternalCallableDecl>,
    functions: Vec<Function>,
) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables,
        functions,
    })
    .expect("floating aggregate fixture must be valid Core")
}

fn represented(value: BinaryFloatValue) -> FloatingScalarValue {
    FloatingScalarValue::Represented(value)
}

#[test]
fn represented_all_format_aggregate_lifecycle_and_direct_call_transport_are_exact() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("F16", ScalarType::F16));
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let f64_ty = types.push(TypeDef::scalar("F64", ScalarType::F64));
    let aggregate_ty = types.push(TypeDef::structure(
        "Floats",
        vec![
            Field::new("half", f16_ty),
            Field::new("single", f32_ty),
            Field::new("double", f64_ty),
        ],
    ));

    let f16_value = BinaryFloatValue::Zero(BinaryFloatSign::Negative);
    let initial_f32 = BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Positive,
        significand: 1,
    };
    let assigned_f32 = BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Negative,
        significand: 1_u64 << 23,
        exponent: -126,
    };
    let f64_value = BinaryFloatValue::Infinity(BinaryFloatSign::Negative);

    let sink16 = interface(vec![f16_ty], None);
    let sink32 = interface(vec![f32_ty], None);
    let sink64 = interface(vec![f64_ty], None);
    let source = Place::local(LocalId(0));
    let copied = Place::local(LocalId(1));
    let round_trip = Place::local(LocalId(2));

    let program = validated(
        types,
        vec![
            ExternalCallableDecl::new(sink16.clone()),
            ExternalCallableDecl::new(sink32.clone()),
            ExternalCallableDecl::new(sink64.clone()),
        ],
        vec![
            function(
                "entry",
                Vec::new(),
                None,
                vec![
                    LocalDecl::new("source", aggregate_ty, false),
                    LocalDecl::new("copied", aggregate_ty, true),
                    LocalDecl::new("round_trip", aggregate_ty, false),
                ],
                vec![
                    BasicBlock::new(
                        vec![
                            Statement::Init {
                                dst: source.clone(),
                                src: Operand::Constant(Value::Struct(vec![
                                    Value::F16(f16_value),
                                    Value::F32(initial_f32),
                                    Value::F64(f64_value),
                                ])),
                            },
                            Statement::Init {
                                dst: copied.clone(),
                                src: Operand::Copy(source.clone().into()),
                            },
                            Statement::Read {
                                src: source.clone().field(0).into(),
                            },
                            Statement::Assign {
                                dst: copied.clone().field(1).into(),
                                src: Operand::Constant(Value::F32(assigned_f32)),
                            },
                            Statement::Drop {
                                place: source.field(2).into(),
                            },
                        ],
                        Terminator::Call {
                            function: FunctionId(1),
                            arguments: vec![Operand::Move(copied.into())],
                            destination: Some(round_trip.clone()),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Copy(round_trip.clone().field(0).into())],
                            destination: None,
                            target: BasicBlockId(2),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(1),
                            arguments: vec![Operand::Copy(round_trip.clone().field(1).into())],
                            destination: None,
                            target: BasicBlockId(3),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(2),
                            arguments: vec![Operand::Move(round_trip.field(2).into())],
                            destination: None,
                            target: BasicBlockId(4),
                        },
                    ),
                    BasicBlock::new(Vec::new(), Terminator::Return(None)),
                ],
            ),
            function(
                "identity",
                vec![LocalId(0)],
                Some(aggregate_ty),
                vec![LocalDecl::new("value", aggregate_ty, false)],
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        ],
    );

    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen16 = Arc::clone(&seen);
    let seen32 = Arc::clone(&seen);
    let seen64 = Arc::clone(&seen);
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), sink16, move |arguments| {
                seen16.lock().unwrap().push(arguments.to_vec());
            }),
            ExternalProviderBinding::no_result(ExternalCallableId(1), sink32, move |arguments| {
                seen32.lock().unwrap().push(arguments.to_vec());
            }),
            ExternalProviderBinding::no_result(ExternalCallableId(2), sink64, move |arguments| {
                seen64.lock().unwrap().push(arguments.to_vec());
            }),
        ],
    )
    .expect("floating aggregate lifecycle fixture must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(None)
    );
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            vec![ExternalScalarValue::F16(represented(f16_value))],
            vec![ExternalScalarValue::F32(represented(assigned_f32))],
            vec![ExternalScalarValue::F64(represented(f64_value))],
        ]
    );
}

#[test]
fn indirect_call_round_trips_float_bearing_aggregate_without_changing_callable_identity() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("F16", ScalarType::F16));
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let f64_ty = types.push(TypeDef::scalar("F64", ScalarType::F64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let aggregate_ty = types.push(TypeDef::structure(
        "Floats",
        vec![
            Field::new("half", f16_ty),
            Field::new("single", f32_ty),
            Field::new("double", f64_ty),
        ],
    ));
    let callable = types.push(TypeDef::callable(
        "AggregateIdentity",
        interface(vec![aggregate_ty], Some(aggregate_ty)),
    ));
    let classify = interface(vec![f16_ty, f32_ty, f64_ty], Some(bool_ty));

    let f16_value = BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Positive,
        significand: 1,
    };
    let f32_value = BinaryFloatValue::Infinity(BinaryFloatSign::Positive);
    let f64_value = BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Negative,
        significand: 1_u64 << 52,
        exponent: -1022,
    };

    let result = Place::local(LocalId(1));
    let classified = Place::local(LocalId(2));
    let program = validated(
        types,
        vec![ExternalCallableDecl::new(classify.clone())],
        vec![
            function(
                "entry",
                Vec::new(),
                Some(bool_ty),
                vec![
                    LocalDecl::new("callee", callable, false),
                    LocalDecl::new("result", aggregate_ty, false),
                    LocalDecl::new("classified", bool_ty, false),
                ],
                vec![
                    BasicBlock::new(
                        vec![Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::FunctionValue(FunctionId(1)),
                        }],
                        Terminator::IndirectCall {
                            callable,
                            callee: Operand::Move(Place::local(LocalId(0)).into()),
                            arguments: vec![Operand::Constant(Value::Struct(vec![
                                Value::F16(f16_value),
                                Value::F32(f32_value),
                                Value::F64(f64_value),
                            ]))],
                            destination: Some(result.clone()),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![
                                Operand::Copy(result.clone().field(0).into()),
                                Operand::Copy(result.clone().field(1).into()),
                                Operand::Move(result.field(2).into()),
                            ],
                            destination: Some(classified.clone()),
                            target: BasicBlockId(2),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(classified.into()))),
                    ),
                ],
            ),
            function(
                "identity",
                vec![LocalId(0)],
                Some(aggregate_ty),
                vec![LocalDecl::new("value", aggregate_ty, false)],
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        ],
    );

    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            classify,
            move |arguments| {
                assert_eq!(
                    arguments,
                    &[
                        ExternalScalarValue::F16(represented(f16_value)),
                        ExternalScalarValue::F32(represented(f32_value)),
                        ExternalScalarValue::F64(represented(f64_value)),
                    ]
                );
                ExternalScalarValue::Bool(true)
            },
        )],
    )
    .expect("floating aggregate indirect-call fixture must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::Bool(true)))
    );
}

#[test]
fn provider_nan_class_survives_structural_transport_and_projection() {
    let mut types = TypeTable::new();
    let u8_ty = types.push(TypeDef::scalar("U8", ScalarType::U8));
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let aggregate_ty = types.push(TypeDef::structure(
        "TaggedFloat",
        vec![Field::new("tag", u8_ty), Field::new("value", f32_ty)],
    ));
    let produce = interface(Vec::new(), Some(f32_ty));
    let classify = interface(vec![f32_ty], Some(bool_ty));

    let nan = Place::local(LocalId(0));
    let aggregate = Place::local(LocalId(1));
    let copied = Place::local(LocalId(2));
    let classified = Place::local(LocalId(3));
    let program = validated(
        types,
        vec![
            ExternalCallableDecl::new(produce.clone()),
            ExternalCallableDecl::new(classify.clone()),
        ],
        vec![function(
            "entry",
            Vec::new(),
            Some(bool_ty),
            vec![
                LocalDecl::new("nan", f32_ty, false),
                LocalDecl::new("aggregate", aggregate_ty, false),
                LocalDecl::new("copied", aggregate_ty, false),
                LocalDecl::new("classified", bool_ty, false),
            ],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: Vec::new(),
                        destination: Some(nan.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: aggregate.clone().field(0),
                            src: Operand::Constant(Value::U8(7)),
                        },
                        Statement::Init {
                            dst: aggregate.clone().field(1),
                            src: Operand::Move(nan.into()),
                        },
                        Statement::Init {
                            dst: copied.clone(),
                            src: Operand::Copy(aggregate.clone().into()),
                        },
                        Statement::Drop {
                            place: aggregate.into(),
                        },
                    ],
                    Terminator::ExternalCall {
                        external: ExternalCallableId(1),
                        arguments: vec![Operand::Move(copied.field(1).into())],
                        destination: Some(classified.clone()),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(classified.into()))),
                ),
            ],
        )],
    );

    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![
            ExternalProviderBinding::scalar_result(ExternalCallableId(0), produce, |_| {
                ExternalScalarValue::F32(FloatingScalarValue::NaNClass)
            }),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), classify, |arguments| {
                assert_eq!(
                    arguments,
                    &[ExternalScalarValue::F32(FloatingScalarValue::NaNClass)]
                );
                ExternalScalarValue::Bool(true)
            }),
        ],
    )
    .expect("NaN-class floating aggregate fixture must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::Bool(true)))
    );
}

#[test]
fn public_float_bearing_aggregate_entry_results_are_rejected_recursively_for_all_formats() {
    let cases = [
        (
            "F16",
            ScalarType::F16,
            Value::F16(BinaryFloatValue::Zero(BinaryFloatSign::Negative)),
        ),
        (
            "F32",
            ScalarType::F32,
            Value::F32(BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Positive,
                significand: 1,
            }),
        ),
        (
            "F64",
            ScalarType::F64,
            Value::F64(BinaryFloatValue::Infinity(BinaryFloatSign::Positive)),
        ),
    ];

    for (name, scalar, value) in cases {
        let mut types = TypeTable::new();
        let float_ty = types.push(TypeDef::scalar(name, scalar));
        let inner_ty = types.push(TypeDef::structure(
            "Inner",
            vec![Field::new("value", float_ty)],
        ));
        let outer_ty = types.push(TypeDef::structure(
            "Outer",
            vec![Field::new("inner", inner_ty)],
        ));
        let program = validated(
            types,
            Vec::new(),
            vec![function(
                "entry",
                Vec::new(),
                Some(outer_ty),
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(Value::Struct(vec![
                        Value::Struct(vec![value]),
                    ])))),
                )],
            )],
        );

        let realized = RealizedProgram::new(&program)
            .expect("float-bearing aggregate result must remain internally realizable");
        assert_eq!(
            realized.execute(FunctionId(0)),
            Err(RealizationError::EntryResultUnsupported(FunctionId(0)))
        );
    }
}

#[test]
fn aggregate_external_interfaces_remain_outside_provider_transfer_scope() {
    let mut parameter_types = TypeTable::new();
    let parameter_f32 = parameter_types.push(TypeDef::scalar("F32", ScalarType::F32));
    let parameter_aggregate = parameter_types.push(TypeDef::structure(
        "FloatWrapper",
        vec![Field::new("value", parameter_f32)],
    ));
    let parameter_error = validate_program(Program {
        types: parameter_types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(interface(
            vec![parameter_aggregate],
            None,
        ))],
        functions: vec![function(
            "entry",
            Vec::new(),
            None,
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        )],
    })
    .expect_err("aggregate external parameter must remain invalid Core");
    assert_eq!(
        parameter_error.kind,
        MirValidationErrorKind::InvalidExternalCallableType {
            external: ExternalCallableId(0),
            ty: parameter_aggregate,
        }
    );

    let mut result_types = TypeTable::new();
    let result_f64 = result_types.push(TypeDef::scalar("F64", ScalarType::F64));
    let result_aggregate = result_types.push(TypeDef::structure(
        "FloatWrapper",
        vec![Field::new("value", result_f64)],
    ));
    let result_error = validate_program(Program {
        types: result_types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(interface(
            Vec::new(),
            Some(result_aggregate),
        ))],
        functions: vec![function(
            "entry",
            Vec::new(),
            None,
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        )],
    })
    .expect_err("aggregate external result must remain invalid Core");
    assert_eq!(
        result_error.kind,
        MirValidationErrorKind::InvalidExternalCallableType {
            external: ExternalCallableId(0),
            ty: result_aggregate,
        }
    );
}

#[test]
fn aggregate_persistents_remain_rejected_by_canonical_core_validation() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let aggregate_ty = types.push(TypeDef::structure(
        "FloatWrapper",
        vec![Field::new("value", f32_ty)],
    ));
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(
            aggregate_ty,
            Value::Struct(vec![Value::F32(BinaryFloatValue::Zero(
                BinaryFloatSign::Positive,
            ))]),
        )],
        external_callables: Vec::new(),
        functions: vec![function(
            "entry",
            Vec::new(),
            None,
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        )],
    })
    .expect_err("aggregate persistent must remain invalid Core");

    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidPersistentType {
            persistent: PersistentId(0),
            ty: aggregate_ty,
        }
    );
}
