use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, ReferenceAccess, ReferencePermission, SafeReferenceResultContract, ScalarType,
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

fn direct_child_callee(
    parent_ty: runen_core_ir::TypeId,
    child_ty: runen_core_ir::TypeId,
) -> Function {
    Function {
        name: "direct_child".into(),
        parameters: vec![LocalId(0)],
        result: Some(child_ty),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![
                LocalDecl::new("parent", parent_ty, false),
                LocalDecl::new("child", child_ty, false),
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
    }
}

#[test]
fn direct_child_runtime_survives_cleanup_and_releases_ancestry_after_last_descendant() {
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
    let returned = Place::local(LocalId(2));
    let grandchild = Place::local(LocalId(3));
    let new_parent = Place::local(LocalId(4));

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
                LocalDecl::new("grandchild", shared_i64, false),
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
                    Terminator::Call {
                        function: FunctionId(1),
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
                        Statement::Init {
                            dst: grandchild.clone(),
                            src: Operand::ReferenceReborrow {
                                permission: ReferencePermission::Shared,
                                src: ReferenceAccess::new(returned.clone()),
                            },
                        },
                        Statement::Drop {
                            place: returned.into(),
                        },
                        Statement::ReferenceRead {
                            src: ReferenceAccess::new(grandchild.clone()),
                        },
                        Statement::Drop {
                            place: grandchild.into(),
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

    let validated = validate_program(Program {
        types,
        functions: vec![caller, direct_child_callee(replace_i64, shared_i64)],
    })
    .expect("direct-child caller lifecycle is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("direct child and its descendant remain defined across callee cleanup");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(53)));
}

#[test]
fn direct_child_fault_runtime_initializes_no_result_destination() {
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
    let destination = Place::local(LocalId(2));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("destination", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(59)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target,
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(parent.into())],
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
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![LocalDecl::new("parent", replace_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Fault(Fault::new("boom")),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, fault],
    })
    .expect("a direct-child contract may fault without a normal result");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined fault returns no derived child carrier");

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
