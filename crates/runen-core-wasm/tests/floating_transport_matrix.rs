use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LocalDecl, LocalId, Operand,
    PersistentDecl, PersistentId, Place, Program, SafeReferenceResultContract, ScalarType,
    Statement, Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{
    ExecutionOutcome, ExternalProviderBinding, ExternalScalarValue, FloatingScalarValue,
    RealizedProgram,
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
    .expect("floating transport matrix fixture must be valid Core")
}

fn represented_cases() -> Vec<(ScalarType, Value, ExternalScalarValue)> {
    vec![
        (
            ScalarType::F16,
            Value::F16(BinaryFloatValue::Zero(BinaryFloatSign::Negative)),
            ExternalScalarValue::F16(FloatingScalarValue::Represented(BinaryFloatValue::Zero(
                BinaryFloatSign::Negative,
            ))),
        ),
        (
            ScalarType::F32,
            Value::F32(BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Positive,
                significand: 1_u64 << 23,
                exponent: -126,
            }),
            ExternalScalarValue::F32(FloatingScalarValue::Represented(BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Positive,
                significand: 1_u64 << 23,
                exponent: -126,
            })),
        ),
        (
            ScalarType::F64,
            Value::F64(BinaryFloatValue::Infinity(BinaryFloatSign::Negative)),
            ExternalScalarValue::F64(FloatingScalarValue::Represented(
                BinaryFloatValue::Infinity(BinaryFloatSign::Negative),
            )),
        ),
    ]
}

#[test]
fn represented_provider_results_round_trip_all_three_formats_through_local_operations() {
    for (scalar, core_value, provider_value) in represented_cases() {
        let mut types = TypeTable::new();
        let ty = types.push(TypeDef::scalar("Float", scalar));
        let transform = interface(vec![ty], Some(ty));
        let sink = interface(vec![ty], None);
        let program = validated(
            types,
            Vec::new(),
            vec![
                ExternalCallableDecl::new(transform.clone()),
                ExternalCallableDecl::new(sink.clone()),
            ],
            vec![function(
                "entry",
                Vec::new(),
                None,
                vec![
                    LocalDecl::new("provided", ty, false),
                    LocalDecl::new("transported", ty, true),
                ],
                vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Constant(core_value)],
                            destination: Some(Place::local(LocalId(0))),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        vec![
                            Statement::Init {
                                dst: Place::local(LocalId(1)),
                                src: Operand::Copy(Place::local(LocalId(0)).into()),
                            },
                            Statement::Read {
                                src: Place::local(LocalId(0)).into(),
                            },
                            Statement::Assign {
                                dst: Place::local(LocalId(1)).into(),
                                src: Operand::Copy(Place::local(LocalId(0)).into()),
                            },
                            Statement::Drop {
                                place: Place::local(LocalId(0)).into(),
                            },
                        ],
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
        );
        let transform_expected = provider_value;
        let sink_expected = provider_value;
        let realized = RealizedProgram::new_with_external_providers(
            &program,
            vec![
                ExternalProviderBinding::scalar_result(
                    ExternalCallableId(0),
                    transform,
                    move |arguments| {
                        assert_eq!(arguments, &[transform_expected]);
                        transform_expected
                    },
                ),
                ExternalProviderBinding::no_result(ExternalCallableId(1), sink, move |arguments| {
                    assert_eq!(arguments, &[sink_expected])
                }),
            ],
        )
        .expect("represented provider result matrix must realize");
        assert_eq!(
            realized.execute(FunctionId(0)).unwrap(),
            ExecutionOutcome::Returned(None)
        );
    }
}

#[test]
fn direct_function_parameter_and_result_transport_covers_all_three_float_formats() {
    for (scalar, core_value, provider_value) in represented_cases() {
        let mut types = TypeTable::new();
        let ty = types.push(TypeDef::scalar("Float", scalar));
        let sink = interface(vec![ty], None);
        let program = validated(
            types,
            Vec::new(),
            vec![ExternalCallableDecl::new(sink.clone())],
            vec![
                function(
                    "entry",
                    Vec::new(),
                    None,
                    vec![LocalDecl::new("returned", ty, false)],
                    vec![
                        BasicBlock::new(
                            Vec::new(),
                            Terminator::Call {
                                function: FunctionId(1),
                                arguments: vec![Operand::Constant(core_value)],
                                destination: Some(Place::local(LocalId(0))),
                                target: BasicBlockId(1),
                            },
                        ),
                        BasicBlock::new(
                            Vec::new(),
                            Terminator::ExternalCall {
                                external: ExternalCallableId(0),
                                arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                                destination: None,
                                target: BasicBlockId(2),
                            },
                        ),
                        BasicBlock::new(Vec::new(), Terminator::Return(None)),
                    ],
                ),
                function(
                    "identity",
                    vec![LocalId(0)],
                    Some(ty),
                    vec![LocalDecl::new("value", ty, false)],
                    vec![BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                    )],
                ),
            ],
        );
        let expected = provider_value;
        let realized = RealizedProgram::new_with_external_providers(
            &program,
            vec![ExternalProviderBinding::no_result(
                ExternalCallableId(0),
                sink,
                move |arguments| assert_eq!(arguments, &[expected]),
            )],
        )
        .expect("direct floating call format matrix must realize");
        assert_eq!(
            realized.execute(FunctionId(0)).unwrap(),
            ExecutionOutcome::Returned(None)
        );
    }
}

#[test]
fn persistent_initialization_and_read_cover_all_three_float_formats() {
    for (scalar, core_value, provider_value) in represented_cases() {
        let mut types = TypeTable::new();
        let ty = types.push(TypeDef::scalar("Float", scalar));
        let sink = interface(vec![ty], None);
        let program = validated(
            types,
            vec![PersistentDecl::new(ty, core_value)],
            vec![ExternalCallableDecl::new(sink.clone())],
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
                            arguments: vec![Operand::PersistentRead(PersistentId(0))],
                            destination: None,
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(Vec::new(), Terminator::Return(None)),
                ],
            )],
        );
        let expected = provider_value;
        let realized = RealizedProgram::new_with_external_providers(
            &program,
            vec![ExternalProviderBinding::no_result(
                ExternalCallableId(0),
                sink,
                move |arguments| assert_eq!(arguments, &[expected]),
            )],
        )
        .expect("floating persistent format matrix must realize");
        assert_eq!(
            realized.execute(FunctionId(0)).unwrap(),
            ExecutionOutcome::Returned(None)
        );
    }
}
