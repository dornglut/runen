use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, Function, FunctionId, LocalDecl, LocalId,
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
        shared_reference_result_origin: None,
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
        "safe-reference-containing results require an accepted origin contract"
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
            shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
                        arguments: vec![Operand::Move(child.into()), Operand::Move(parent.into())],
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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

    validate_program(Program {
        types,
        functions: vec![caller, middle, inner],
    })
    .expect("nested normal calls restore borrowed authority before each caller continuation");
}

fn expect_function_error(
    types: TypeTable,
    function: Function,
    expected: MirValidationErrorKind,
) {
    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("fixture must be rejected by the selected Core rule");
    assert_eq!(error.kind, expected);
}

#[test]
fn shared_reference_result_contract_declaration_matrix_is_exact() {
    let mut types = TypeTable::new();
    let i32_ty = types.push(TypeDef::scalar("I32", ScalarType::I32));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i32 = types.push(TypeDef::reference(
        "SharedI32",
        i32_ty,
        ReferencePermission::Shared,
    ));
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
    let aggregate_i64 = types.push(TypeDef::structure(
        "AggregateSharedI64",
        vec![Field::new("reference", shared_i64)],
    ));

    expect_function_error(
        types.clone(),
        Function {
            name: "origin_without_result".into(),
            parameters: Vec::new(),
            result: None,
            shared_reference_result_origin: Some(0),
            body: body(Vec::new(), vec![BasicBlock::new(Vec::new(), Terminator::Return(None))]),
        },
        MirValidationErrorKind::UnexpectedSharedReferenceResultOrigin,
    );

    expect_function_error(
        types.clone(),
        Function {
            name: "origin_on_ordinary_result".into(),
            parameters: Vec::new(),
            result: Some(i64_ty),
            shared_reference_result_origin: Some(0),
            body: body(Vec::new(), vec![BasicBlock::new(Vec::new(), Terminator::Return(None))]),
        },
        MirValidationErrorKind::UnexpectedSharedReferenceResultOrigin,
    );

    expect_function_error(
        types.clone(),
        Function {
            name: "missing_origin".into(),
            parameters: vec![LocalId(0)],
            result: Some(shared_i64),
            shared_reference_result_origin: None,
            body: body(
                vec![LocalDecl::new("reference", shared_i64, false)],
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        },
        MirValidationErrorKind::MissingSharedReferenceResultOrigin,
    );

    expect_function_error(
        types.clone(),
        Function {
            name: "invalid_origin_slot".into(),
            parameters: vec![LocalId(0)],
            result: Some(shared_i64),
            shared_reference_result_origin: Some(1),
            body: body(
                vec![LocalDecl::new("reference", shared_i64, false)],
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        },
        MirValidationErrorKind::InvalidSharedReferenceResultOriginSlot(1),
    );

    expect_function_error(
        types.clone(),
        Function {
            name: "exclusive_result".into(),
            parameters: vec![LocalId(0)],
            result: Some(exclusive_i64),
            shared_reference_result_origin: Some(0),
            body: body(
                vec![LocalDecl::new("reference", exclusive_i64, false)],
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        },
        MirValidationErrorKind::SharedReferenceResultRequiresShared(exclusive_i64),
    );

    expect_function_error(
        types.clone(),
        Function {
            name: "exclusive_origin".into(),
            parameters: vec![LocalId(0)],
            result: Some(shared_i64),
            shared_reference_result_origin: Some(0),
            body: body(
                vec![LocalDecl::new("reference", exclusive_i64, false)],
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        },
        MirValidationErrorKind::SharedReferenceResultOriginRequiresShared(exclusive_i64),
    );

    expect_function_error(
        types.clone(),
        Function {
            name: "origin_type_mismatch".into(),
            parameters: vec![LocalId(0)],
            result: Some(shared_i64),
            shared_reference_result_origin: Some(0),
            body: body(
                vec![LocalDecl::new("reference", shared_i32, false)],
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            ),
        },
        MirValidationErrorKind::SharedReferenceResultOriginTypeMismatch {
            expected: shared_i64,
            found: shared_i32,
        },
    );

    expect_function_error(
        types,
        Function {
            name: "aggregate_reference_result".into(),
            parameters: vec![LocalId(0)],
            result: Some(aggregate_i64),
            shared_reference_result_origin: Some(0),
            body: body(
                vec![LocalDecl::new("reference", shared_i64, false)],
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        },
        MirValidationErrorKind::ResultTransferUnsafe(aggregate_i64),
    );
}

#[test]
fn shared_reference_result_preserves_origin_through_move_copy_and_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let by_move = Function {
        name: "by_move".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };
    let by_copy = Function {
        name: "by_copy".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Copy(Place::local(LocalId(0)).into()))),
            )],
        ),
    };
    let through_storage = Function {
        name: "through_storage".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![
                LocalDecl::new("reference", shared_i64, false),
                LocalDecl::new("stored", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::Move(Place::local(LocalId(0)).into()),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![by_move, by_copy, through_storage],
    })
    .expect("Move, Shared Copy, and storage transport preserve the designated authority identity");
}

#[test]
fn shared_reference_result_forwards_through_nested_and_recursive_contract_calls() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let identity = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };
    let forward = Function {
        name: "forward".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![
                LocalDecl::new("reference", shared_i64, false),
                LocalDecl::new("result", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(0),
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
    let recursive = Function {
        name: "recursive".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
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

    validate_program(Program {
        types,
        functions: vec![identity, forward, recursive],
    })
    .expect("contract summaries forward the original authority without callee-body expansion");
}

#[test]
fn caller_created_shared_child_result_keeps_parent_delegated_until_result_drop() {
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

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        shared_reference_result_origin: None,
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
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::I64(13)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: Place::local(LocalId(0)),
                            },
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(2)),
                            src: Operand::ReferenceReborrow {
                                permission: ReferencePermission::Shared,
                                src: ReferenceAccess::new(Place::local(LocalId(1))),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(2)).into())],
                        destination: Some(Place::local(LocalId(3))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::ReferenceRead {
                            src: ReferenceAccess::new(Place::local(LocalId(3))),
                        },
                        Statement::Drop {
                            place: Place::local(LocalId(3)).into(),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(4)),
                            src: Operand::ReferenceMove(ReferenceAccess::new(Place::local(
                                LocalId(1),
                            ))),
                        },
                        Statement::ReferenceAssign {
                            dst: ReferenceAccess::new(Place::local(LocalId(1))),
                            src: Operand::Move(Place::local(LocalId(4)).into()),
                        },
                    ],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let identity = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, identity],
    })
    .expect("the returned child remains active until its result carrier is destroyed");
}

#[test]
fn shared_reference_result_rejects_fresh_reborrow_other_and_reused_authorities() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let fresh_root = Function {
        name: "fresh_root".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![
                LocalDecl::new("origin", shared_i64, false),
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("fresh", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Constant(Value::I64(17)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(1)),
                        },
                    },
                ],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            )],
        ),
    };
    expect_function_error(
        types.clone(),
        fresh_root,
        MirValidationErrorKind::SharedReferenceResultOriginMismatch,
    );

    let reborrow = Function {
        name: "reborrow".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![
                LocalDecl::new("origin", shared_i64, false),
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
    expect_function_error(
        types.clone(),
        reborrow,
        MirValidationErrorKind::SharedReferenceResultOriginMismatch,
    );

    let other_parameter = Function {
        name: "other_parameter".into(),
        parameters: vec![LocalId(0), LocalId(1)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![
                LocalDecl::new("origin", shared_i64, false),
                LocalDecl::new("other", shared_i64, false),
            ],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            )],
        ),
    };
    expect_function_error(
        types.clone(),
        other_parameter,
        MirValidationErrorKind::SharedReferenceResultOriginMismatch,
    );

    let reused_slot = Function {
        name: "reused_slot".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![
                LocalDecl::new("origin", shared_i64, false),
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("fresh", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Drop {
                        place: Place::local(LocalId(0)).into(),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Constant(Value::I64(19)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(1)),
                        },
                    },
                ],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            )],
        ),
    };
    expect_function_error(
        types,
        reused_slot,
        MirValidationErrorKind::SharedReferenceResultOriginMismatch,
    );
}

#[test]
fn shared_reference_result_preserves_external_referent_fully_live_return_postcondition() {
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

    let function = Function {
        name: "incomplete_external_referent".into(),
        parameters: vec![LocalId(0), LocalId(1)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![
                LocalDecl::new("origin", shared_i64, false),
                LocalDecl::new("replace", replace_i64, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(2)),
                    src: Operand::ReferenceMove(ReferenceAccess::new(Place::local(LocalId(1)))),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    expect_function_error(
        types,
        function,
        MirValidationErrorKind::ExternalReferentNotLive,
    );
}

#[test]
fn shared_reference_result_destination_is_admitted_before_argument_effects() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
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
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::I64(23)),
                        },
                        Statement::Init {
                            dst: reference.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: Place::local(LocalId(0)),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(reference.clone().into())],
                        destination: Some(reference.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let identity = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        shared_reference_result_origin: Some(0),
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, identity],
    })
    .expect_err("result destination admission must precede argument evaluation for contracts too");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::CallResultRequiresVacant(reference)
    );
}
