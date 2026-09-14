use std::sync::{Arc, Mutex};

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LocalDecl, LocalId, Operand,
    PersistentDecl, PersistentId, Place, Program, SafeReferenceResultContract, ScalarType,
    Statement, Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{
    BackendPhase, ExecutionOutcome, ExternalProviderAdmissionError, ExternalProviderBinding,
    ExternalScalarValue, FloatingScalarValue, RealizationError, RealizedProgram,
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
    persistent: Vec<PersistentDecl>,
    external_callables: Vec<ExternalCallableDecl>,
    functions: Vec<Function>,
) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent,
        external_callables,
        functions,
    })
    .expect("floating transport fixture must be valid Core")
}

#[test]
fn represented_external_parameters_preserve_all_three_float_formats() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("F16", ScalarType::F16));
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let f64_ty = types.push(TypeDef::scalar("F64", ScalarType::F64));
    let f16_value = BinaryFloatValue::Zero(BinaryFloatSign::Negative);
    let f32_value = BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Positive,
        significand: 1,
    };
    let f64_value = BinaryFloatValue::Infinity(BinaryFloatSign::Negative);
    let sink16 = interface(vec![f16_ty], None);
    let sink32 = interface(vec![f32_ty], None);
    let sink64 = interface(vec![f64_ty], None);
    let program = validated(
        types,
        Vec::new(),
        vec![
            ExternalCallableDecl::new(sink16.clone()),
            ExternalCallableDecl::new(sink32.clone()),
            ExternalCallableDecl::new(sink64.clone()),
        ],
        vec![function(
            "entry",
            Vec::new(),
            None,
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: vec![Operand::Constant(Value::F16(f16_value))],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(1),
                        arguments: vec![Operand::Constant(Value::F32(f32_value))],
                        destination: None,
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(2),
                        arguments: vec![Operand::Constant(Value::F64(f64_value))],
                        destination: None,
                        target: BasicBlockId(3),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        )],
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
    .expect("floating provider environment must admit");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(None)
    );
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            vec![ExternalScalarValue::F16(FloatingScalarValue::Represented(
                f16_value
            ))],
            vec![ExternalScalarValue::F32(FloatingScalarValue::Represented(
                f32_value
            ))],
            vec![ExternalScalarValue::F64(FloatingScalarValue::Represented(
                f64_value
            ))],
        ]
    );
}

#[test]
fn provider_nan_class_round_trips_to_a_later_provider_without_public_float_observation() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let u8_ty = types.push(TypeDef::scalar("U8", ScalarType::U8));
    let produce = interface(Vec::new(), Some(f32_ty));
    let classify = interface(vec![f32_ty], Some(u8_ty));
    let program = validated(
        types,
        Vec::new(),
        vec![
            ExternalCallableDecl::new(produce.clone()),
            ExternalCallableDecl::new(classify.clone()),
        ],
        vec![function(
            "entry",
            Vec::new(),
            Some(u8_ty),
            vec![
                LocalDecl::new("float", f32_ty, false),
                LocalDecl::new("class", u8_ty, false),
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
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
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
                ExternalScalarValue::U8(1)
            }),
        ],
    )
    .expect("NaN-class provider chain must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::U8(1)))
    );
}

#[test]
fn direct_function_float_parameter_and_result_transport_remain_internal() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let classify = interface(vec![f32_ty], Some(bool_ty));
    let value = BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand: 1_u64 << 23,
        exponent: 0,
    };
    let program = validated(
        types,
        Vec::new(),
        vec![ExternalCallableDecl::new(classify.clone())],
        vec![
            function(
                "entry",
                Vec::new(),
                Some(bool_ty),
                vec![
                    LocalDecl::new("round_trip", f32_ty, false),
                    LocalDecl::new("classified", bool_ty, false),
                ],
                vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Call {
                            function: FunctionId(1),
                            arguments: vec![Operand::Constant(Value::F32(value))],
                            destination: Some(Place::local(LocalId(0))),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                            destination: Some(Place::local(LocalId(1))),
                            target: BasicBlockId(2),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
                    ),
                ],
            ),
            function(
                "identity",
                vec![LocalId(0)],
                Some(f32_ty),
                vec![LocalDecl::new("value", f32_ty, false)],
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
                    &[ExternalScalarValue::F32(FloatingScalarValue::Represented(value))]
                );
                ExternalScalarValue::Bool(true)
            },
        )],
    )
    .expect("direct floating call transport must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::Bool(true)))
    );
}

#[test]
fn indirect_call_transports_direct_float_components_without_changing_callable_identity() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let callable = types.push(TypeDef::callable(
        "FloatIdentity",
        interface(vec![f32_ty], Some(f32_ty)),
    ));
    let classify = interface(vec![f32_ty], Some(bool_ty));
    let value = BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Negative,
        significand: 7,
    };
    let program = validated(
        types,
        Vec::new(),
        vec![ExternalCallableDecl::new(classify.clone())],
        vec![
            function(
                "entry",
                Vec::new(),
                Some(bool_ty),
                vec![
                    LocalDecl::new("callee", callable, false),
                    LocalDecl::new("round_trip", f32_ty, false),
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
                            arguments: vec![Operand::Constant(Value::F32(value))],
                            destination: Some(Place::local(LocalId(1))),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                            destination: Some(Place::local(LocalId(2))),
                            target: BasicBlockId(2),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
                    ),
                ],
            ),
            function(
                "identity",
                vec![LocalId(0)],
                Some(f32_ty),
                vec![LocalDecl::new("value", f32_ty, false)],
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
                    &[ExternalScalarValue::F32(FloatingScalarValue::Represented(value))]
                );
                ExternalScalarValue::Bool(true)
            },
        )],
    )
    .expect("indirect floating call transport must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::Bool(true)))
    );
}

#[test]
fn floating_persistent_and_local_operations_reuse_the_same_private_carrier() {
    let mut types = TypeTable::new();
    let f64_ty = types.push(TypeDef::scalar("F64", ScalarType::F64));
    let sink = interface(vec![f64_ty], None);
    let persistent_value = BinaryFloatValue::Zero(BinaryFloatSign::Negative);
    let replacement = BinaryFloatValue::Infinity(BinaryFloatSign::Positive);
    let program = validated(
        types,
        vec![PersistentDecl::new(f64_ty, Value::F64(persistent_value))],
        vec![ExternalCallableDecl::new(sink.clone())],
        vec![function(
            "entry",
            Vec::new(),
            None,
            vec![
                LocalDecl::new("first", f64_ty, false),
                LocalDecl::new("second", f64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::PersistentRead(PersistentId(0)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::Copy(Place::local(LocalId(0)).into()),
                        },
                        Statement::Read {
                            src: Place::local(LocalId(0)).into(),
                        },
                        Statement::Drop {
                            place: Place::local(LocalId(0)).into(),
                        },
                        Statement::Assign {
                            dst: Place::local(LocalId(1)).into(),
                            src: Operand::Constant(Value::F64(replacement)),
                        },
                    ],
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        )],
    );
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            sink,
            move |arguments| {
                assert_eq!(
                    arguments,
                    &[ExternalScalarValue::F64(FloatingScalarValue::Represented(
                        replacement
                    ))]
                );
            },
        )],
    )
    .expect("floating persistent/local transport must realize");

    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn public_floating_entry_result_remains_structurally_unobservable() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let value = BinaryFloatValue::Zero(BinaryFloatSign::Positive);
    let program = validated(
        types,
        Vec::new(),
        Vec::new(),
        vec![function(
            "entry",
            Vec::new(),
            Some(f32_ty),
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::F32(value)))),
            )],
        )],
    );
    let realized = RealizedProgram::new(&program).expect("floating result function must realize");
    assert_eq!(
        realized.execute(FunctionId(0)),
        Err(RealizationError::EntryResultUnsupported(FunctionId(0)))
    );
}

#[test]
fn floating_provider_contract_violations_are_backend_execution_failures() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("F16", ScalarType::F16));
    let external = interface(Vec::new(), Some(f16_ty));
    let program = validated(
        types,
        Vec::new(),
        vec![ExternalCallableDecl::new(external.clone())],
        vec![function(
            "entry",
            Vec::new(),
            None,
            vec![LocalDecl::new("float", f16_ty, false)],
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
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        )],
    );

    let wrong_variant = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            external.clone(),
            |_| ExternalScalarValue::F32(FloatingScalarValue::NaNClass),
        )],
    )
    .expect("provider interface admission precedes invocation");
    assert!(matches!(
        wrong_variant.execute(FunctionId(0)),
        Err(RealizationError::Backend {
            phase: BackendPhase::Execute,
            ..
        })
    ));

    let f32_only = BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand: 1_u64 << 23,
        exponent: -126,
    };
    let wrong_format = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            external,
            move |_| {
                ExternalScalarValue::F16(FloatingScalarValue::Represented(f32_only))
            },
        )],
    )
    .expect("provider interface admission does not pre-execute the provider");
    assert!(matches!(
        wrong_format.execute(FunctionId(0)),
        Err(RealizationError::Backend {
            phase: BackendPhase::Execute,
            ..
        })
    ));
}

#[test]
fn unused_floating_external_declaration_remains_a_hard_provider_requirement() {
    let mut types = TypeTable::new();
    let f64_ty = types.push(TypeDef::scalar("F64", ScalarType::F64));
    let external = interface(vec![f64_ty], Some(f64_ty));
    let program = validated(
        types,
        Vec::new(),
        vec![ExternalCallableDecl::new(external)],
        vec![function(
            "entry",
            Vec::new(),
            None,
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        )],
    );
    assert!(matches!(
        RealizedProgram::new(&program),
        Err(RealizationError::ProviderAdmission(
            ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(0))
        ))
    ));
}
