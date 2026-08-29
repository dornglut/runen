use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, Program, ReferenceAccess, ReferencePermission,
    ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
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
fn shared_reference_parameter_reads_suspended_caller_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::I64(7)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: Place::local(LocalId(0)),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(0)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    let callee = Function {
        name: "read".into(),
        parameters: vec![LocalId(0)],
        result: None,
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

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("a transferred Shared carrier may read its fully-Live external referent");
}

#[test]
fn exclusive_replace_parameter_may_move_then_restore_before_return() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::reference(
        "ExclusiveReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("reference", reference_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::I64(9)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: Place::local(LocalId(0)),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(0)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    let callee = Function {
        name: "round_trip".into(),
        parameters: vec![LocalId(0)],
        result: None,
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

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("normal Return is valid after an ExclusiveReplace referent is restored fully Live");
}

#[test]
fn reference_result_type_remains_outside_the_transfer_boundary() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let function = Function {
        name: "invalid_reference_result".into(),
        parameters: Vec::new(),
        result: Some(shared_i64),
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    assert!(
        validate_program(Program {
            types,
            functions: vec![function],
        })
        .is_err(),
        "safe-reference-containing results require a later accepted origin contract"
    );
}

#[test]
fn reference_parameter_referent_may_not_contain_raw_or_nested_reference_leaves() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_i64 = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let shared_raw = types.push(TypeDef::reference(
        "SharedRawI64",
        raw_i64,
        ReferencePermission::Shared,
    ));
    let shared_nested = types.push(TypeDef::reference(
        "SharedNestedI64",
        shared_i64,
        ReferencePermission::Shared,
    ));

    for invalid_parameter_type in [shared_raw, shared_nested] {
        let function = Function {
            name: "invalid_reference_parameter".into(),
            parameters: vec![LocalId(0)],
            result: None,
            body: body(
                vec![LocalDecl::new("parameter", invalid_parameter_type, false)],
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        };

        assert!(
            validate_program(Program {
                types: types.clone(),
                functions: vec![function],
            })
            .is_err(),
            "transferred reference referents must be raw-free and reference-free"
        );
    }
}

#[test]
fn call_result_destination_is_admitted_before_argument_effects() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![LocalDecl::new("value", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: value.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(value.clone().into())],
                        destination: Some(value.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(i64_ty),
        body: body(
            vec![LocalDecl::new("value", i64_ty, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("a call result destination must be vacant before any argument can vacate it");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::CallResultRequiresVacant(value)
    );
}

#[test]
fn call_arguments_are_left_to_right_before_final_reference_admission() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let target = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("reference", replace_i64, false),
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
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target,
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![
                            Operand::ReferenceMove(ReferenceAccess::new(reference.clone())),
                            Operand::Move(reference.into()),
                        ],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "consume_then_borrow".into(),
        parameters: vec![LocalId(0), LocalId(1)],
        result: None,
        body: body(
            vec![
                LocalDecl::new("value", i64_ty, false),
                LocalDecl::new("reference", replace_i64, false),
            ],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("earlier argument effects must be visible to final borrowed-call admission");
    assert_eq!(error.kind, MirValidationErrorKind::ReferenceTargetNotLive);
}

#[test]
fn call_retains_all_arguments_before_checking_full_reference_authority() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let exclusive_i64 = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));
    let target = Place::local(LocalId(0));
    let parent = Place::local(LocalId(1));
    let child = Place::local(LocalId(2));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("parent", exclusive_i64, false),
                LocalDecl::new("child", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(7)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Exclusive,
                                place: target,
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
                        arguments: vec![
                            Operand::Move(child.into()),
                            Operand::Move(parent.into()),
                        ],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "borrow_both".into(),
        parameters: vec![LocalId(0), LocalId(1)],
        result: None,
        body: body(
            vec![
                LocalDecl::new("child", shared_i64, false),
                LocalDecl::new("parent", exclusive_i64, false),
            ],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("held child authority must keep the overlapping parent incomplete at admission");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferenceAuthorityIncomplete
    );
}

#[test]
fn reference_parameter_return_requires_live_external_referent_but_fault_does_not() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));

    let invalid_return = Function {
        name: "invalid_return".into(),
        parameters: vec![LocalId(0)],
        result: None,
        body: body(
            vec![
                LocalDecl::new("reference", replace_i64, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::ReferenceMove(ReferenceAccess::new(Place::local(LocalId(0)))),
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![invalid_return],
    })
    .expect_err("normal Return requires every external referent domain fully Live");
    assert_eq!(error.kind, MirValidationErrorKind::ExternalReferentNotLive);

    let fault = Function {
        name: "fault".into(),
        parameters: vec![LocalId(0)],
        result: None,
        body: body(
            vec![
                LocalDecl::new("reference", replace_i64, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::ReferenceMove(ReferenceAccess::new(Place::local(LocalId(0)))),
                }],
                Terminator::Fault(runen_core_ir::Fault::new("boom")),
            )],
        ),
    };
    validate_program(Program {
        types,
        functions: vec![fault],
    })
    .expect("explicit Fault has cleanup but no normal-return external-liveness postcondition");
}

#[test]
fn temporary_child_borrowed_call_restores_parent_authority() {
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
        result: None,
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
                            src: Operand::Constant(Value::I64(9)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target,
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
                            dst: ReferenceAccess::new(parent),
                            src: Operand::Move(held.into()),
                        },
                    ],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "read".into(),
        parameters: vec![LocalId(0)],
        result: None,
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

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("normal call cleanup ends the temporary child and restores parent authority");
}

#[test]
fn nested_borrowed_call_restores_authority_through_each_activation() {
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
        result: None,
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
                            src: Operand::Constant(Value::I64(11)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target,
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
                            dst: ReferenceAccess::new(parent),
                            src: Operand::Move(held.into()),
                        },
                    ],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let middle = Function {
        name: "middle".into(),
        parameters: vec![LocalId(0)],
        result: None,
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

    validate_program(Program {
        types,
        functions: vec![caller, middle, inner],
    })
    .expect("nested normal calls restore borrowed authority before each caller continuation");
}
