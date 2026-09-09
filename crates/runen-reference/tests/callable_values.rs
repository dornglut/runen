use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Fault, Function, FunctionId, LocalDecl,
    LocalId, Operand, Place, Program, ReferenceAccess, ReferencePermission,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_program,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus, VerificationEventKind};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn execute(program: Program) -> runen_reference::ExecutionReport {
    let validated = validate_program(program).expect("reference callable fixture must validate");
    Machine::new(validated, FunctionId(0))
        .expect("reference callable fixture has a zero-parameter entry")
        .execute()
        .expect("safe callable fixture has defined execution")
}

fn no_result_target(name: &str, terminator: Terminator) -> Function {
    Function {
        name: name.into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(Vec::new(), vec![BasicBlock::new(Vec::new(), terminator)]),
    }
}

#[test]
fn indirect_call_dispatches_function_value_and_returns_through_existing_activation_machine() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "ReturnsI64",
        CallableInterface::new(Vec::new(), Some(i64_ty), SafeReferenceResultContract::None),
    ));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::FunctionValue(FunctionId(1)),
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
        ),
    };
    let target = Function {
        name: "target".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
            )],
        ),
    };

    let report = execute(Program {
        types,
        functions: vec![caller, target],
    });
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn stored_copy_and_move_preserve_exact_function_identity() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "ReturnsI64",
        CallableInterface::new(Vec::new(), Some(i64_ty), SafeReferenceResultContract::None),
    ));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("source", callable, false),
                LocalDecl::new("copy", callable, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::FunctionValue(FunctionId(1)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::Copy(Place::local(LocalId(0)).into()),
                        },
                    ],
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::Move(Place::local(LocalId(1)).into()),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(2))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
                ),
            ],
        ),
    };
    let first = Function {
        name: "first".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(7)))),
            )],
        ),
    };
    let second = Function {
        name: "second".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(9)))),
            )],
        ),
    };

    let report = execute(Program {
        types,
        functions: vec![caller, first, second],
    });
    assert_eq!(report.result, Some(ObservedValue::I64(7)));
}

#[test]
fn top_level_callable_result_observes_function_entity_identity_only() {
    let mut types = TypeTable::new();
    let callable = types.push(TypeDef::callable(
        "NoArgs",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    let entry = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(callable),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::FunctionValue(FunctionId(1)))),
            )],
        ),
    };
    let target = no_result_target("target", Terminator::Return(None));

    let report = execute(Program {
        types,
        functions: vec![entry, target],
    });
    assert_eq!(report.result, Some(ObservedValue::Function(FunctionId(1))));
}

#[test]
fn indirect_runtime_evaluates_callee_before_arguments() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "TakesI64",
        CallableInterface::new(vec![i64_ty], None, SafeReferenceResultContract::None),
    ));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("callee", callable, false),
                LocalDecl::new("argument", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::FunctionValue(FunctionId(1)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::Constant(Value::I64(5)),
                        },
                    ],
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::Move(Place::local(LocalId(0)).into()),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let target = Function {
        name: "target".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let report = execute(Program {
        types,
        functions: vec![caller, target],
    });
    let moves = report
        .verification_events
        .iter()
        .filter_map(|event| match &event.kind {
            VerificationEventKind::Move(place) => Some(place.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        moves,
        vec![Place::local(LocalId(0)), Place::local(LocalId(1))]
    );
}

#[test]
fn indirect_fault_uses_existing_fault_propagation_and_skips_normal_continuation() {
    let mut types = TypeTable::new();
    let callable = types.push(TypeDef::callable(
        "NoArgs",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::FunctionValue(FunctionId(1)),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Fault(Fault::new("normal-continuation-ran")),
                ),
            ],
        ),
    };
    let target = no_result_target("faulting-target", Terminator::Fault(Fault::new("indirect")));

    let report = execute(Program {
        types,
        functions: vec![caller, target],
    });
    assert_eq!(report.terminal, TerminalStatus::Faulted("indirect".into()));
    assert_eq!(report.result, None);
}

#[test]
fn indirect_shared_identity_result_preserves_caller_reference_authority() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let callable = types.push(TypeDef::callable(
        "SharedIdentity",
        CallableInterface::new(
            vec![shared_i64],
            Some(shared_i64),
            SafeReferenceResultContract::SharedIdentity { origin: 0 },
        ),
    ));
    let target = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));
    let returned = Place::local(LocalId(2));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared_i64, false),
                LocalDecl::new("returned", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(29)),
                        },
                        Statement::Init {
                            dst: reference.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::FunctionValue(FunctionId(1)),
                        arguments: vec![Operand::Move(reference.into())],
                        destination: Some(returned.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::ReferenceRead {
                            src: ReferenceAccess::new(returned.clone()),
                        },
                        Statement::Drop {
                            place: returned.into(),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(target.into()))),
                ),
            ],
        ),
    };
    let identity = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity { origin: 0 },
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    let report = execute(Program {
        types,
        functions: vec![caller, identity],
    });
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(29)));
}

#[test]
fn indirect_shared_direct_child_result_preserves_ancestry_until_child_destruction() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let callable = types.push(TypeDef::callable(
        "SharedDirectChild",
        CallableInterface::new(
            vec![replace_i64],
            Some(shared_i64),
            SafeReferenceResultContract::SharedDirectChild { origin: 0 },
        ),
    ));
    let target = Place::local(LocalId(0));
    let parent = Place::local(LocalId(1));
    let returned = Place::local(LocalId(2));
    let new_parent = Place::local(LocalId(3));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("returned", shared_i64, false),
                LocalDecl::new("new_parent", replace_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(53)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::FunctionValue(FunctionId(1)),
                        arguments: vec![Operand::Move(parent.into())],
                        destination: Some(returned.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::ReferenceRead {
                            src: ReferenceAccess::new(returned.clone()),
                        },
                        Statement::Drop {
                            place: returned.into(),
                        },
                        Statement::Init {
                            dst: new_parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                        Statement::ReferenceRead {
                            src: ReferenceAccess::new(new_parent.clone()),
                        },
                        Statement::Drop {
                            place: new_parent.into(),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(target.into()))),
                ),
            ],
        ),
    };
    let direct_child = Function {
        name: "direct_child".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("child", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::ReferenceReborrow {
                        permission: ReferencePermission::Shared,
                        src: ReferenceAccess::new(Place::local(LocalId(0))),
                    },
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            )],
        ),
    };

    let report = execute(Program {
        types,
        functions: vec![caller, direct_child],
    });
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(53)));
}
