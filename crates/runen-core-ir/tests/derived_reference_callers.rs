use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, Program, ReferenceAccess, ReferencePermission,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_program,
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

fn identity_callee(shared_ty: runen_core_ir::TypeId) -> Function {
    Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_ty),
        safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity { origin: 0 },
        body: body(
            vec![LocalDecl::new("reference", shared_ty, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    }
}

#[test]
fn direct_child_call_keeps_carrierless_parent_conflict_while_result_lives() {
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

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("result", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(1)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: Some(Place::local(LocalId(2))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: target.clone().into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, direct_child_callee(replace_i64, shared_i64)],
    })
    .expect_err("returned child keeps the exclusive parent authority active carrierlessly");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflictWithReferenceAuthority { place: target }
    );
}

#[test]
fn direct_child_call_does_not_recreate_a_parent_carrier() {
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
    let parent = Place::local(LocalId(1));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("result", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::I64(2)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: Place::local(LocalId(0)),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(parent.clone().into())],
                        destination: Some(Place::local(LocalId(2))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: parent.clone().into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, direct_child_callee(replace_i64, shared_i64)],
    })
    .expect_err("direct-child summary must not synthesize a replacement parent carrier");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(ref place) if *place == parent
    ));
}

#[test]
fn dropping_direct_child_result_releases_the_only_child_branch_and_parent() {
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

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("result", shared_i64, false),
                LocalDecl::new("new_parent", replace_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(3)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: Some(Place::local(LocalId(2))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::Drop {
                            place: Place::local(LocalId(2)).into(),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(3)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target,
                            },
                        },
                    ],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, direct_child_callee(replace_i64, shared_i64)],
    })
    .expect("dropping the one summarized child ends eligible carrierless ancestors");
}

#[test]
fn identity_forwarding_of_a_returned_direct_child_preserves_ancestry() {
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

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("child", shared_i64, false),
                LocalDecl::new("forwarded", shared_i64, false),
                LocalDecl::new("new_parent", replace_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(4)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target.clone(),
                            },
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                        destination: Some(Place::local(LocalId(2))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(2),
                        arguments: vec![Operand::Move(Place::local(LocalId(2)).into())],
                        destination: Some(Place::local(LocalId(3))),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::Drop {
                            place: Place::local(LocalId(3)).into(),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(4)),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::ExclusiveReplace,
                                place: target,
                            },
                        },
                    ],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![
            caller,
            direct_child_callee(replace_i64, shared_i64),
            identity_callee(shared_i64),
        ],
    })
    .expect("SharedIdentity forwards the already-derived child without adding ancestry");
}

#[test]
fn nested_direct_child_forwarding_is_valid_but_second_derivation_is_not() {
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

    let forward = Function {
        name: "forward".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("returned", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
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
        types: types.clone(),
        functions: vec![forward, direct_child_callee(replace_i64, shared_i64)],
    })
    .expect("nested direct-child summary remains a direct child of the enclosing origin");

    let derive_again = Function {
        name: "derive_again".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("returned", shared_i64, false),
                LocalDecl::new("grandchild", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(1))),
                        },
                    }],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
                ),
            ],
        ),
    };
    let error = validate_program(Program {
        types,
        functions: vec![derive_again, direct_child_callee(replace_i64, shared_i64)],
    })
    .expect_err("second derivation creates a grandchild, not the required direct child");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::SharedDirectChildResultMismatch
    );
}

#[test]
fn direct_child_contracts_remain_independently_validatable_under_recursion() {
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

    let recursive = Function {
        name: "recursive".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![
                LocalDecl::new("parent", replace_i64, false),
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
    validate_program(Program {
        types: types.clone(),
        functions: vec![recursive],
    })
    .expect("direct recursion validates from the callable descriptor without body expansion");

    let left = Function {
        name: "left".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![
                LocalDecl::new("parent", replace_i64, false),
                LocalDecl::new("result", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
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
    let right = Function {
        name: "right".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![
                LocalDecl::new("parent", replace_i64, false),
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
    validate_program(Program {
        types,
        functions: vec![left, right],
    })
    .expect("mutual recursion validates from independent callable descriptors");
}

#[test]
fn fault_and_divergence_require_no_synthesized_direct_child_result() {
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
    let diverge = Function {
        name: "diverge".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![LocalDecl::new("parent", replace_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Goto(BasicBlockId(0)),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![fault, diverge],
    })
    .expect("fault and divergence have no normal Return and synthesize no result authority");
}
