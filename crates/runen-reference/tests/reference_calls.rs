use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, ReferenceAccess, ReferencePermission, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_program,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
