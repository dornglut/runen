use std::sync::{Arc, Mutex};

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, ExternalCallableDecl, ExternalCallableId,
    Function, FunctionId, LocalDecl, LocalId, Operand, PersistentDecl, PersistentId, Place,
    Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId,
    TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{
    BackendPhase, CoverageErrorKind, CoverageLocation, ExecutionOutcome,
    ExternalProviderAdmissionError, ExternalProviderBinding, ExternalProviderFailure,
    ExternalScalarValue, RealizationError, RealizedProgram, UnsupportedTypeCategory,
};
use runen_reference::{
    ExternalProviderBinding as ReferenceProviderBinding,
    ExternalScalarValue as ReferenceScalarValue, Machine, ObservedValue, TerminalStatus,
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
    .expect("external provider fixture must be valid Core")
}

fn no_result_entry(
    types: TypeTable,
    external_callables: Vec<ExternalCallableDecl>,
) -> ValidatedProgram {
    validated(
        types,
        Vec::new(),
        external_callables,
        vec![function(
            "entry",
            Vec::new(),
            None,
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        )],
    )
}

#[test]
fn provider_admission_requires_exactly_one_matching_binding_for_every_declaration() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let first = interface(vec![i64_ty], None);
    let second = interface(Vec::new(), Some(i64_ty));
    let program = no_result_entry(
        types,
        vec![
            ExternalCallableDecl::new(first.clone()),
            ExternalCallableDecl::new(second.clone()),
        ],
    );

    assert!(matches!(
        RealizedProgram::new(&program),
        Err(RealizationError::ProviderAdmission(
            ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(0))
        ))
    ));

    assert!(matches!(
        RealizedProgram::new_with_external_providers(
            &program,
            vec![ExternalProviderBinding::no_result(
                ExternalCallableId(0),
                first.clone(),
                |_| {},
            )],
        ),
        Err(RealizationError::ProviderAdmission(
            ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(1))
        ))
    ));

    assert!(matches!(
        RealizedProgram::new_with_external_providers(
            &program,
            vec![
                ExternalProviderBinding::no_result(ExternalCallableId(0), first.clone(), |_| {},),
                ExternalProviderBinding::no_result(ExternalCallableId(0), first.clone(), |_| {},),
                ExternalProviderBinding::scalar_result(
                    ExternalCallableId(1),
                    second.clone(),
                    |_| ExternalScalarValue::I64(1),
                ),
            ],
        ),
        Err(RealizationError::ProviderAdmission(
            ExternalProviderAdmissionError::DuplicateProvider(ExternalCallableId(0))
        ))
    ));

    let wrong = interface(Vec::new(), None);
    assert!(matches!(
        RealizedProgram::new_with_external_providers(
            &program,
            vec![
                ExternalProviderBinding::no_result(ExternalCallableId(0), wrong.clone(), |_| {},),
                ExternalProviderBinding::scalar_result(
                    ExternalCallableId(1),
                    second.clone(),
                    |_| ExternalScalarValue::I64(1),
                ),
            ],
        ),
        Err(RealizationError::ProviderAdmission(
            ExternalProviderAdmissionError::InterfaceMismatch {
                external: ExternalCallableId(0),
                expected,
                found,
            }
        )) if expected == first && found == wrong
    ));

    assert!(matches!(
        RealizedProgram::new_with_external_providers(
            &program,
            vec![
                ExternalProviderBinding::no_result(ExternalCallableId(0), first.clone(), |_| {},),
                ExternalProviderBinding::scalar_result(
                    ExternalCallableId(1),
                    second.clone(),
                    |_| ExternalScalarValue::I64(1),
                ),
                ExternalProviderBinding::no_result(
                    ExternalCallableId(2),
                    interface(Vec::new(), None),
                    |_| {},
                ),
            ],
        ),
        Err(RealizationError::ProviderAdmission(
            ExternalProviderAdmissionError::UnknownProvider(ExternalCallableId(2))
        ))
    ));

    RealizedProgram::new_with_external_providers(
        &program,
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), first, |_| {}),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), second, |_| {
                ExternalScalarValue::I64(1)
            }),
        ],
    )
    .expect("complete provider environment must admit even when declarations are unused");
}

#[test]
fn no_result_and_scalar_result_providers_preserve_order_and_match_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let sink = interface(vec![i64_ty], None);
    let transform = interface(vec![i64_ty], Some(i64_ty));
    let program = validated(
        types,
        Vec::new(),
        vec![
            ExternalCallableDecl::new(sink.clone()),
            ExternalCallableDecl::new(transform.clone()),
        ],
        vec![function(
            "entry",
            Vec::new(),
            Some(i64_ty),
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: vec![Operand::Constant(Value::I64(2))],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(1),
                        arguments: vec![Operand::Constant(Value::I64(7))],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        )],
    );

    let wasm_seen = Arc::new(Mutex::new(Vec::new()));
    let sink_seen = Arc::clone(&wasm_seen);
    let transform_seen = Arc::clone(&wasm_seen);
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), sink.clone(), move |args| {
                sink_seen.lock().unwrap().push(args.to_vec());
            }),
            ExternalProviderBinding::scalar_result(
                ExternalCallableId(1),
                transform.clone(),
                move |args| {
                    transform_seen.lock().unwrap().push(args.to_vec());
                    let [ExternalScalarValue::I64(value)] = args else {
                        panic!("exact i64 provider argument");
                    };
                    ExternalScalarValue::I64(value + 1)
                },
            ),
        ],
    )
    .expect("matching provider environment must realize");
    assert_eq!(
        realized
            .execute(FunctionId(0))
            .expect("provider calls return normally"),
        ExecutionOutcome::Returned(Some(Value::I64(8)))
    );
    assert_eq!(
        *wasm_seen.lock().unwrap(),
        vec![
            vec![ExternalScalarValue::I64(2)],
            vec![ExternalScalarValue::I64(7)],
        ]
    );

    let report = Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![
            ReferenceProviderBinding::no_result(ExternalCallableId(0), sink, |_| {}),
            ReferenceProviderBinding::scalar_result(ExternalCallableId(1), transform, |args| {
                let [ReferenceScalarValue::I64(value)] = args else {
                    panic!("exact reference i64 provider argument");
                };
                ReferenceScalarValue::I64(value + 1)
            }),
        ],
    )
    .expect("reference provider environment must admit")
    .execute()
    .expect("reference external calls are defined");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(8)));
}

fn scalar_round_trip(
    scalar: ScalarType,
    core_value: Value,
    provider_value: ExternalScalarValue,
) -> ExecutionOutcome {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("Scalar", scalar));
    let external = interface(vec![ty], Some(ty));
    let program = validated(
        types,
        Vec::new(),
        vec![ExternalCallableDecl::new(external.clone())],
        vec![function(
            "entry",
            Vec::new(),
            Some(ty),
            vec![LocalDecl::new("result", ty, false)],
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
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        )],
    );
    let expected_argument = provider_value;
    RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            external,
            move |args| {
                assert_eq!(args, &[expected_argument]);
                expected_argument
            },
        )],
    )
    .expect("scalar provider fixture must realize")
    .execute(FunctionId(0))
    .expect("scalar provider fixture must execute")
}

#[test]
fn every_admitted_scalar_width_round_trips_semantically() {
    let cases = [
        (
            ScalarType::Bool,
            Value::Bool(true),
            ExternalScalarValue::Bool(true),
            Value::Bool(true),
        ),
        (
            ScalarType::I8,
            Value::I8(i8::MIN),
            ExternalScalarValue::I8(i8::MIN),
            Value::I8(i8::MIN),
        ),
        (
            ScalarType::I16,
            Value::I16(i16::MIN),
            ExternalScalarValue::I16(i16::MIN),
            Value::I16(i16::MIN),
        ),
        (
            ScalarType::I32,
            Value::I32(i32::MIN),
            ExternalScalarValue::I32(i32::MIN),
            Value::I32(i32::MIN),
        ),
        (
            ScalarType::I64,
            Value::I64(i64::MIN),
            ExternalScalarValue::I64(i64::MIN),
            Value::I64(i64::MIN),
        ),
        (
            ScalarType::U8,
            Value::U8(u8::MAX),
            ExternalScalarValue::U8(u8::MAX),
            Value::U8(u8::MAX),
        ),
        (
            ScalarType::U16,
            Value::U16(u16::MAX),
            ExternalScalarValue::U16(u16::MAX),
            Value::U16(u16::MAX),
        ),
        (
            ScalarType::U32,
            Value::U32(u32::MAX),
            ExternalScalarValue::U32(u32::MAX),
            Value::U32(u32::MAX),
        ),
        (
            ScalarType::U64,
            Value::U64(u64::MAX),
            ExternalScalarValue::U64(u64::MAX),
            Value::U64(u64::MAX),
        ),
    ];

    for (scalar, core_value, provider_value, expected) in cases {
        assert_eq!(
            scalar_round_trip(scalar, core_value, provider_value),
            ExecutionOutcome::Returned(Some(expected))
        );
    }
}

#[test]
fn equal_interfaces_keep_external_declaration_identity_distinct() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let same = interface(Vec::new(), Some(i64_ty));
    let program = validated(
        types,
        Vec::new(),
        vec![
            ExternalCallableDecl::new(same.clone()),
            ExternalCallableDecl::new(same.clone()),
        ],
        vec![function(
            "entry",
            Vec::new(),
            Some(i64_ty),
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(1),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        )],
    );
    let first_count = Arc::new(Mutex::new(0_u32));
    let second_count = Arc::new(Mutex::new(0_u32));
    let first_seen = Arc::clone(&first_count);
    let second_seen = Arc::clone(&second_count);
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![
            ExternalProviderBinding::scalar_result(
                ExternalCallableId(0),
                same.clone(),
                move |_| {
                    *first_seen.lock().unwrap() += 1;
                    ExternalScalarValue::I64(11)
                },
            ),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), same, move |_| {
                *second_seen.lock().unwrap() += 1;
                ExternalScalarValue::I64(22)
            }),
        ],
    )
    .expect("distinct provider identities must admit");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(22)))
    );
    assert_eq!(*first_count.lock().unwrap(), 0);
    assert_eq!(*second_count.lock().unwrap(), 1);
}

#[test]
fn imports_shift_direct_function_indices_without_changing_core_call_targets() {
    let external = interface(Vec::new(), None);
    let program = validated(
        TypeTable::new(),
        Vec::new(),
        vec![ExternalCallableDecl::new(external.clone())],
        vec![
            function(
                "entry",
                Vec::new(),
                None,
                Vec::new(),
                vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Call {
                            function: FunctionId(1),
                            arguments: Vec::new(),
                            destination: None,
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(Vec::new(), Terminator::Return(None)),
                ],
            ),
            function(
                "target",
                Vec::new(),
                None,
                Vec::new(),
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        ],
    );
    let provider_count = Arc::new(Mutex::new(0_u32));
    let count = Arc::clone(&provider_count);
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            external,
            move |_| *count.lock().unwrap() += 1,
        )],
    )
    .expect("unused provider import must coexist with direct calls");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(None)
    );
    assert_eq!(
        *provider_count.lock().unwrap(),
        0,
        "direct Core call must not accidentally invoke shifted import index"
    );
}

#[test]
fn imports_shift_element_function_indices_but_callable_table_slots_stay_function_ids() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "Thunk",
        interface(Vec::new(), Some(i64_ty)),
    ));
    let external = interface(Vec::new(), None);
    let program = validated(
        types,
        Vec::new(),
        vec![ExternalCallableDecl::new(external.clone())],
        vec![
            function(
                "entry",
                Vec::new(),
                Some(i64_ty),
                vec![
                    LocalDecl::new("callee", callable, false),
                    LocalDecl::new("result", i64_ty, false),
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
                            arguments: Vec::new(),
                            destination: Some(Place::local(LocalId(1))),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
                    ),
                ],
            ),
            function(
                "target",
                Vec::new(),
                Some(i64_ty),
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(Value::I64(73)))),
                )],
            ),
        ],
    );
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            external,
            |_| {},
        )],
    )
    .expect("unused import must coexist with private callable table");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(73)))
    );
}

#[test]
fn callable_direct_result_transport_stays_correct_in_a_module_with_imports() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "Thunk",
        interface(Vec::new(), Some(i64_ty)),
    ));
    let external = interface(Vec::new(), None);
    let program = validated(
        types,
        Vec::new(),
        vec![ExternalCallableDecl::new(external.clone())],
        vec![
            function(
                "entry",
                Vec::new(),
                Some(i64_ty),
                vec![
                    LocalDecl::new("returned", callable, false),
                    LocalDecl::new("result", i64_ty, false),
                ],
                vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::Call {
                            function: FunctionId(1),
                            arguments: Vec::new(),
                            destination: Some(Place::local(LocalId(0))),
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::IndirectCall {
                            callable,
                            callee: Operand::Move(Place::local(LocalId(0)).into()),
                            arguments: Vec::new(),
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
                "producer",
                Vec::new(),
                Some(callable),
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::FunctionValue(FunctionId(2)))),
                )],
            ),
            function(
                "target",
                Vec::new(),
                Some(i64_ty),
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
                )],
            ),
        ],
    );
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            external,
            |_| {},
        )],
    )
    .expect("imports must not change callable payload identity");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn persistent_reads_compose_with_external_imports() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let external = interface(vec![i64_ty], Some(i64_ty));
    let program = validated(
        types,
        vec![PersistentDecl::new(i64_ty, Value::I64(41))],
        vec![ExternalCallableDecl::new(external.clone())],
        vec![function(
            "entry",
            Vec::new(),
            Some(i64_ty),
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: vec![Operand::PersistentRead(PersistentId(0))],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        )],
    );
    let realized = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            external,
            |args| {
                let [ExternalScalarValue::I64(value)] = args else {
                    panic!("persistent read must transfer as i64");
                };
                ExternalScalarValue::I64(value + 1)
            },
        )],
    )
    .expect("persistent/global and import sections must compose");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn provider_failures_and_wrong_result_types_remain_realization_errors() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let external = interface(Vec::new(), Some(i64_ty));
    let program = validated(
        types,
        Vec::new(),
        vec![ExternalCallableDecl::new(external.clone())],
        vec![function(
            "entry",
            Vec::new(),
            Some(i64_ty),
            vec![LocalDecl::new("result", i64_ty, false)],
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
                    Terminator::Return(Some(Operand::Constant(Value::I64(99)))),
                ),
            ],
        )],
    );

    let failing = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::try_scalar_result(
            ExternalCallableId(0),
            external.clone(),
            |_| Err(ExternalProviderFailure::new("host failed")),
        )],
    )
    .expect("fallible provider environment admits");
    assert!(matches!(
        failing.execute(FunctionId(0)),
        Err(RealizationError::Backend {
            phase: BackendPhase::Execute,
            ..
        })
    ));

    let wrong_type = RealizedProgram::new_with_external_providers(
        &program,
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            external,
            |_| ExternalScalarValue::Bool(true),
        )],
    )
    .expect("interface admission is independent from provider return execution");
    assert!(matches!(
        wrong_type.execute(FunctionId(0)),
        Err(RealizationError::Backend {
            phase: BackendPhase::Execute,
            ..
        })
    ));
}

#[test]
fn floating_external_interface_remains_a_structured_realization_exclusion() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let external = interface(vec![f32_ty], Some(f32_ty));
    let program = no_result_entry(types, vec![ExternalCallableDecl::new(external)]);

    assert!(matches!(
        RealizedProgram::new(&program),
        Err(RealizationError::Coverage(error))
            if error.location == CoverageLocation::ExternalCallable(ExternalCallableId(0))
                && error.kind
                    == CoverageErrorKind::UnsupportedExternalParameterType {
                        external: ExternalCallableId(0),
                        parameter: 0,
                        ty: f32_ty,
                        category: UnsupportedTypeCategory::Floating,
                    }
    ));
}
