use std::cell::RefCell;
use std::rc::Rc;

use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, CallableInterface,
    ExternalCallableDecl, ExternalCallableId, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, SafeReferenceResultContract, ScalarType, Terminator, TypeDef, TypeId,
    TypeTable, Value, validate_program,
};
use runen_reference::{
    EntryError, ExternalProviderAdmissionError, ExternalProviderBinding, ExternalScalarValue,
    Machine, ObservedBinaryFloatValue, ObservedValue, TerminalStatus,
};

fn interface(parameters: Vec<TypeId>, result: Option<TypeId>) -> CallableInterface {
    CallableInterface {
        parameters,
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
    }
}

fn no_result_entry(
    types: TypeTable,
    external_callables: Vec<ExternalCallableDecl>,
) -> runen_core_ir::ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables,
        functions: vec![Function {
            name: "main".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: Vec::new(),
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            },
        }],
    })
    .expect("test program is valid")
}

#[test]
fn admission_requires_exactly_one_matching_provider_for_every_declaration() {
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

    assert_eq!(
        Machine::new(program.clone(), FunctionId(0)).err(),
        Some(EntryError::ExternalProviderAdmission(
            ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(0))
        ))
    );

    let missing_unused = Machine::new_with_external_providers(
        program.clone(),
        FunctionId(0),
        vec![ExternalProviderBinding::no_result(
            ExternalCallableId(0),
            first.clone(),
            |_| {},
        )],
    )
    .err();
    assert_eq!(
        missing_unused,
        Some(EntryError::ExternalProviderAdmission(
            ExternalProviderAdmissionError::MissingProvider(ExternalCallableId(1))
        ))
    );

    let duplicate = Machine::new_with_external_providers(
        program.clone(),
        FunctionId(0),
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), first.clone(), |_| {}),
            ExternalProviderBinding::no_result(ExternalCallableId(0), first.clone(), |_| {}),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), second.clone(), |_| {
                ExternalScalarValue::I64(1)
            }),
        ],
    )
    .err();
    assert_eq!(
        duplicate,
        Some(EntryError::ExternalProviderAdmission(
            ExternalProviderAdmissionError::DuplicateProvider(ExternalCallableId(0))
        ))
    );

    let wrong_interface = interface(Vec::new(), None);
    let mismatch = Machine::new_with_external_providers(
        program.clone(),
        FunctionId(0),
        vec![
            ExternalProviderBinding::no_result(
                ExternalCallableId(0),
                wrong_interface.clone(),
                |_| {},
            ),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), second.clone(), |_| {
                ExternalScalarValue::I64(1)
            }),
        ],
    )
    .err();
    assert_eq!(
        mismatch,
        Some(EntryError::ExternalProviderAdmission(
            ExternalProviderAdmissionError::InterfaceMismatch {
                external: ExternalCallableId(0),
                expected: first.clone(),
                found: wrong_interface,
            }
        ))
    );

    Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), first, |_| {}),
            ExternalProviderBinding::scalar_result(ExternalCallableId(1), second, |_| {
                ExternalScalarValue::I64(1)
            }),
        ],
    )
    .expect("complete matching provider set is admitted before execution");
}

#[test]
fn external_calls_invoke_once_in_program_order_without_creating_a_callee_activation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let sink = interface(vec![i64_ty], None);
    let transform = interface(vec![i64_ty], Some(i64_ty));
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![
            ExternalCallableDecl::new(sink.clone()),
            ExternalCallableDecl::new(transform.clone()),
        ],
        functions: vec![Function {
            name: "main".into(),
            parameters: Vec::new(),
            result: Some(i64_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("result", i64_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![
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
            },
        }],
    })
    .expect("external-call execution fixture is valid");

    let seen = Rc::new(RefCell::new(Vec::new()));
    let sink_seen = Rc::clone(&seen);
    let transform_seen = Rc::clone(&seen);
    let machine = Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![
            ExternalProviderBinding::no_result(ExternalCallableId(0), sink, move |arguments| {
                sink_seen.borrow_mut().push(arguments.to_vec());
            }),
            ExternalProviderBinding::scalar_result(
                ExternalCallableId(1),
                transform,
                move |arguments| {
                    transform_seen.borrow_mut().push(arguments.to_vec());
                    let [ExternalScalarValue::I64(value)] = arguments else {
                        panic!("exact admitted scalar arguments");
                    };
                    ExternalScalarValue::I64(value + 1)
                },
            ),
        ],
    )
    .expect("providers admitted");
    let report = machine
        .execute()
        .expect("defined external calls return normally");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(8)));
    assert_eq!(
        *seen.borrow(),
        vec![
            vec![ExternalScalarValue::I64(2)],
            vec![ExternalScalarValue::I64(7)],
        ]
    );
    assert!(
        report
            .verification_events
            .iter()
            .all(|event| event.activation.0 == 1),
        "external provider invocation creates no Core callee activation"
    );
}

#[test]
fn floating_external_provider_observes_semantic_value_and_may_return_nan_class() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let float_interface = interface(vec![f32_ty], Some(f32_ty));
    let zero = BinaryFloatValue::Zero(BinaryFloatSign::Negative);
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(float_interface.clone())],
        functions: vec![Function {
            name: "main".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("result", f32_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![
                    BasicBlock::new(
                        Vec::new(),
                        Terminator::ExternalCall {
                            external: ExternalCallableId(0),
                            arguments: vec![Operand::Constant(Value::F32(zero))],
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
        }],
    })
    .expect("floating external-call fixture is valid");

    let machine = Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            float_interface,
            move |arguments| {
                assert_eq!(
                    arguments,
                    &[ExternalScalarValue::F32(
                        ObservedBinaryFloatValue::Represented(zero)
                    )]
                );
                ExternalScalarValue::F32(ObservedBinaryFloatValue::NaNClass)
            },
        )],
    )
    .expect("floating provider admitted");
    let report = machine.execute().expect("provider returns normally");
    assert_eq!(
        report.result,
        Some(ObservedValue::F32(ObservedBinaryFloatValue::NaNClass))
    );
}

#[test]
fn nested_runen_caller_preserves_bool_round_trip_argument_order_and_single_invocation() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let external = interface(vec![bool_ty, i64_ty], Some(bool_ty));

    let main = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(bool_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("result", bool_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
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
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        },
    };
    let wrapper = Function {
        name: "wrapper".into(),
        parameters: Vec::new(),
        result: Some(bool_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("result", bool_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: vec![
                            Operand::Constant(Value::Bool(true)),
                            Operand::Constant(Value::I64(7)),
                        ],
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
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(external.clone())],
        functions: vec![main, wrapper],
    })
    .expect("nested external-call fixture is valid");

    let seen = Rc::new(RefCell::new(Vec::new()));
    let provider_seen = Rc::clone(&seen);
    let machine = Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            external,
            move |arguments| {
                provider_seen.borrow_mut().push(arguments.to_vec());
                ExternalScalarValue::Bool(false)
            },
        )],
    )
    .expect("matching provider is admitted");
    let report = machine
        .execute()
        .expect("nested external call returns normally");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::Bool(false)));
    assert_eq!(
        *seen.borrow(),
        vec![vec![
            ExternalScalarValue::Bool(true),
            ExternalScalarValue::I64(7)
        ]]
    );
    assert!(
        report
            .verification_events
            .iter()
            .any(|event| event.activation.0 == 2),
        "ordinary nested Runen callee must create its own activation"
    );
    assert!(
        report
            .verification_events
            .iter()
            .all(|event| event.activation.0 <= 2),
        "external provider invocation must not create an additional Core activation"
    );
}
