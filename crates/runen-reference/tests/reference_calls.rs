use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, ReferenceAccess, ReferencePermission, SafeReferenceResultContract, ScalarType,
    Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{
    Machine, ObservedValue, TerminalStatus, VerificationEventKind, VerificationWriteKind,
};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

#[test]
fn transferred_shared_reference_reads_suspended_caller_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(7)),
                        },
                        Statement::Init {
                            dst: reference.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(reference.into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(target.into()))),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "read".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                vec![Statement::ReferenceRead {
                    src: ReferenceAccess::new(Place::local(LocalId(0))),
                }],
                Terminator::Return(None),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("borrowed Shared call is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("cross-frame safe-reference read is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(7)));
}

#[test]
fn exclusive_replace_parameter_moves_and_restores_suspended_caller_target() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::reference(
        "ExclusiveReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let target = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("reference", reference_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(9)),
                        },
                        Statement::Init {
                            dst: reference.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(reference.into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(target.into()))),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "round_trip".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("reference", reference_ty, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceMove(ReferenceAccess::new(Place::local(LocalId(0)))),
                    },
                    Statement::ReferenceAssign {
                        dst: ReferenceAccess::new(Place::local(LocalId(0))),
                        src: Operand::Move(Place::local(LocalId(1)).into()),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("ExclusiveReplace borrowed call restores its referent before Return");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("cross-frame ExclusiveReplace execution is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(9)));
}

#[test]
fn temporary_child_borrowed_call_restores_parent_at_runtime() {
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
    let target = Place::local(LocalId(0));
    let parent = Place::local(LocalId(1));
    let child = Place::local(LocalId(2));
    let held = Place::local(LocalId(3));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("child", shared_i64, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(37)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                        Statement::Init {
                            dst: child.clone(),
                            src: Operand::ReferenceReborrow {
                                permission: ReferencePermission::Shared,
                                src: ReferenceAccess::new(parent.clone()),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(child.into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: held.clone(),
                            src: Operand::ReferenceMove(ReferenceAccess::new(parent.clone())),
                        },
                        Statement::ReferenceAssign {
                            dst: ReferenceAccess::new(parent.clone()),
                            src: Operand::Move(held.into()),
                        },
                        Statement::Drop {
                            place: parent.into(),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(target.into()))),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "read".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                vec![Statement::ReferenceRead {
                    src: ReferenceAccess::new(Place::local(LocalId(0))),
                }],
                Terminator::Return(None),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("temporary child borrowed call is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("callee cleanup ends child authority and restores parent");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(37)));
}

#[test]
fn nested_borrowed_call_resolves_original_target_across_three_frames() {
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
    let target = Place::local(LocalId(0));
    let parent = Place::local(LocalId(1));
    let child = Place::local(LocalId(2));
    let held = Place::local(LocalId(3));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("child", shared_i64, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(41)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                        Statement::Init {
                            dst: child.clone(),
                            src: Operand::ReferenceReborrow {
                                permission: ReferencePermission::Shared,
                                src: ReferenceAccess::new(parent.clone()),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(child.into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: held.clone(),
                            src: Operand::ReferenceMove(ReferenceAccess::new(parent.clone())),
                        },
                        Statement::ReferenceAssign {
                            dst: ReferenceAccess::new(parent.clone()),
                            src: Operand::Move(held.into()),
                        },
                        Statement::Drop {
                            place: parent.into(),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(target.into()))),
                ),
            ],
        ),
    };
    let middle = Function {
        name: "middle".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(2),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let inner = Function {
        name: "inner".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                vec![Statement::ReferenceRead {
                    src: ReferenceAccess::new(Place::local(LocalId(0))),
                }],
                Terminator::Return(None),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, middle, inner],
    })
    .expect("nested borrowed call is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("safe references resolve through suspended ancestor frames");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(41)));
}

#[test]
fn call_fault_cleanup_destroys_callee_carrier_before_caller_storage_ends() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(4)),
                        },
                        Statement::Init {
                            dst: reference.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: target,
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(reference.into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "fault".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Fault(runen_core_ir::Fault::new("boom")),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("borrowed fault future is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined fault cleanup preserves reference storage validity");

    assert_eq!(report.terminal, TerminalStatus::Faulted("boom".into()));
    assert_eq!(report.result, None);
}

#[test]
fn shared_reference_result_round_trip_remains_dereferenceable_in_caller() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
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
                    Terminator::Call {
                        function: FunctionId(1),
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

    let validated = validate_program(Program {
        types,
        functions: vec![caller, identity],
    })
    .expect("identity-preserving Shared result round trip is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("returned Shared carrier remains dereferenceable");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(29)));
}

#[test]
fn shared_copy_return_cleanup_preserves_returned_carrier() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
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
                            src: Operand::Constant(Value::I64(31)),
                        },
                        Statement::Init {
                            dst: reference.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
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
    let copy_identity = Function {
        name: "copy_identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity { origin: 0 },
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Copy(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, copy_identity],
    })
    .expect("Shared Copy result contract is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("callee cleanup destroys only its original parameter carrier");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(31)));
    assert!(report.verification_events.iter().any(|event| {
        matches!(event.kind, VerificationEventKind::Copy(ref place) if *place == Place::local(LocalId(0)))
    }));
}

#[test]
fn returned_shared_child_keeps_parent_delegated_until_result_destruction() {
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
    let target = Place::local(LocalId(0));
    let parent = Place::local(LocalId(1));
    let child = Place::local(LocalId(2));
    let returned = Place::local(LocalId(3));
    let held = Place::local(LocalId(4));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("child", shared_i64, false),
                LocalDecl::new("returned", shared_i64, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(37)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                        Statement::Init {
                            dst: child.clone(),
                            src: Operand::ReferenceReborrow {
                                permission: ReferencePermission::Shared,
                                src: ReferenceAccess::new(parent.clone()),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(child.into())],
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
                            dst: held.clone(),
                            src: Operand::ReferenceMove(ReferenceAccess::new(parent.clone())),
                        },
                        Statement::ReferenceAssign {
                            dst: ReferenceAccess::new(parent.clone()),
                            src: Operand::Move(held.into()),
                        },
                        Statement::Drop {
                            place: parent.into(),
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

    let validated = validate_program(Program {
        types,
        functions: vec![caller, identity],
    })
    .expect("caller-created Shared child can be returned under the identity contract");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("result destruction restores the parent authority at runtime");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(37)));
}

#[test]
fn nested_shared_reference_result_forwarding_preserves_original_target() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
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
                            src: Operand::Constant(Value::I64(41)),
                        },
                        Statement::Init {
                            dst: reference.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
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
    let middle = Function {
        name: "middle".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity { origin: 0 },
        body: body(
            vec![
                LocalDecl::new("reference", shared_i64, false),
                LocalDecl::new("result", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(2),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
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
    };
    let inner = Function {
        name: "inner".into(),
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

    let validated = validate_program(Program {
        types,
        functions: vec![caller, middle, inner],
    })
    .expect("nested identity-preserving result forwarding is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("nested forwarding preserves the original runtime target");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(41)));
}

#[test]
fn contract_bearing_fault_initializes_no_result_destination() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));
    let destination = Place::local(LocalId(2));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared_i64, false),
                LocalDecl::new("destination", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(43)),
                        },
                        Statement::Init {
                            dst: reference.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: target,
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(reference.into())],
                        destination: Some(destination.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let fault = Function {
        name: "fault".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity { origin: 0 },
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Fault(runen_core_ir::Fault::new("boom")),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, fault],
    })
    .expect("a contract-bearing function may fault without a normal result");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined fault returns no result carrier");

    assert_eq!(report.terminal, TerminalStatus::Faulted("boom".into()));
    assert_eq!(report.result, None);
    assert!(!report.verification_events.iter().any(|event| {
        matches!(
            &event.kind,
            VerificationEventKind::Write {
                place,
                kind: VerificationWriteKind::Init,
            } if *place == destination
        )
    }));
}
