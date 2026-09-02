use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, Function, LocalDecl, LocalId, MirValidationErrorKind,
    Operand, Place, Program, ReferenceAccess, ReferencePermission, SafeReferenceResultContract,
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

fn expect_function_error(types: TypeTable, function: Function, expected: MirValidationErrorKind) {
    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("fixture must be rejected by the selected direct-child contract rule");
    assert_eq!(error.kind, expected);
}

#[test]
fn shared_direct_child_accepts_exclusive_and_replace_origins() {
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
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));

    let exclusive = Function {
        name: "exclusive_child".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
            origin: 0,
        },
        body: body(
            vec![
                LocalDecl::new("parent", exclusive_i64, false),
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

    let replace = Function {
        name: "replace_child".into(),
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

    validate_program(Program {
        types,
        functions: vec![exclusive, replace],
    })
    .expect("Exclusive and ExclusiveReplace origins may return one direct Shared child");
}

#[test]
fn shared_direct_child_declaration_rejects_invalid_origins() {
    let mut types = TypeTable::new();
    let i32_ty = types.push(TypeDef::scalar("I32", ScalarType::I32));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let exclusive_i32 = types.push(TypeDef::reference(
        "ExclusiveI32",
        i32_ty,
        ReferencePermission::Exclusive,
    ));

    expect_function_error(
        types.clone(),
        Function {
            name: "shared_origin".into(),
            parameters: vec![LocalId(0)],
            result: Some(shared_i64),
            safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
                origin: 0,
            },
            body: body(
                vec![LocalDecl::new("origin", shared_i64, false)],
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        },
        MirValidationErrorKind::SharedDirectChildOriginPermissionMismatch(shared_i64),
    );

    expect_function_error(
        types.clone(),
        Function {
            name: "ordinary_origin".into(),
            parameters: vec![LocalId(0)],
            result: Some(shared_i64),
            safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
                origin: 0,
            },
            body: body(
                vec![LocalDecl::new("origin", i64_ty, false)],
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        },
        MirValidationErrorKind::SharedDirectChildOriginPermissionMismatch(i64_ty),
    );

    expect_function_error(
        types.clone(),
        Function {
            name: "wrong_referent".into(),
            parameters: vec![LocalId(0)],
            result: Some(shared_i64),
            safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
                origin: 0,
            },
            body: body(
                vec![LocalDecl::new("origin", exclusive_i32, false)],
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        },
        MirValidationErrorKind::SharedDirectChildOriginReferentMismatch {
            expected: i64_ty,
            found: i32_ty,
        },
    );

    expect_function_error(
        types,
        Function {
            name: "missing_slot".into(),
            parameters: vec![LocalId(0)],
            result: Some(shared_i64),
            safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
                origin: 1,
            },
            body: body(
                vec![LocalDecl::new("origin", exclusive_i32, false)],
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        },
        MirValidationErrorKind::InvalidSafeReferenceResultContractSlot(1),
    );
}

#[test]
fn shared_direct_child_contract_requires_shared_result_permission() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let exclusive_i64 = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));

    for result in [exclusive_i64, replace_i64] {
        expect_function_error(
            types.clone(),
            Function {
                name: "non_shared_result".into(),
                parameters: vec![LocalId(0)],
                result: Some(result),
                safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild {
                    origin: 0,
                },
                body: body(
                    vec![LocalDecl::new("origin", exclusive_i64, false)],
                    vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
                ),
            },
            MirValidationErrorKind::SafeReferenceResultContractRequiresSharedResult(result),
        );
    }
}

#[test]
fn shared_direct_child_return_rejects_fresh_root_escape() {
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

    let function = Function {
        name: "fresh_root_escape".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild { origin: 0 },
        body: body(
            vec![
                LocalDecl::new("origin", exclusive_i64, false),
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("fresh", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Constant(Value::I64(7)),
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
        function,
        MirValidationErrorKind::SharedDirectChildResultMismatch,
    );
}

#[test]
fn shared_direct_child_return_rejects_same_target_wrong_parent() {
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

    let function = Function {
        name: "wrong_direct_parent".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild { origin: 0 },
        body: body(
            vec![
                LocalDecl::new("origin", exclusive_i64, false),
                LocalDecl::new("sibling", shared_i64, false),
                LocalDecl::new("result", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(0))),
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
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            )],
        ),
    };

    expect_function_error(
        types,
        function,
        MirValidationErrorKind::SharedDirectChildResultMismatch,
    );
}

#[test]
fn shared_direct_child_return_rejects_grandchild_even_when_parent_is_copied() {
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
        name: "copied_parent_grandchild".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild { origin: 0 },
        body: body(
            vec![
                LocalDecl::new("origin", replace_i64, false),
                LocalDecl::new("child", shared_i64, false),
                LocalDecl::new("child_copy", shared_i64, false),
                LocalDecl::new("grandchild", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(0))),
                        },
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::Copy(Place::local(LocalId(1)).into()),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(3)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(2))),
                        },
                    },
                ],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(3)).into()))),
            )],
        ),
    };

    expect_function_error(
        types,
        function,
        MirValidationErrorKind::SharedDirectChildResultMismatch,
    );
}

#[test]
fn projected_subregion_cannot_be_admitted_as_direct_child_result() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let exclusive_pair = types.push(TypeDef::reference(
        "ExclusivePair",
        pair_ty,
        ReferencePermission::Exclusive,
    ));

    let function = Function {
        name: "projected_child".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild { origin: 0 },
        body: body(
            vec![
                LocalDecl::new("origin", exclusive_pair, false),
                LocalDecl::new("child", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::ReferenceReborrow {
                        permission: ReferencePermission::Shared,
                        src: ReferenceAccess::new(Place::local(LocalId(0))).field(0),
                    },
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            )],
        ),
    };

    expect_function_error(
        types,
        function,
        MirValidationErrorKind::SharedDirectChildOriginReferentMismatch {
            expected: i64_ty,
            found: pair_ty,
        },
    );
}

#[test]
fn exclusive_replace_direct_child_allows_move_restore_before_return() {
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
        name: "move_restore_then_child".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild { origin: 0 },
        body: body(
            vec![
                LocalDecl::new("origin", replace_i64, false),
                LocalDecl::new("held", i64_ty, false),
                LocalDecl::new("child", shared_i64, false),
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
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(0))),
                        },
                    },
                ],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("ExclusiveReplace origin may restore its external referent before returning a child");
}

#[test]
fn direct_child_normal_return_rejects_any_unavailable_external_referent() {
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
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));

    let function = Function {
        name: "unavailable_external".into(),
        parameters: vec![LocalId(0), LocalId(1)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedDirectChild { origin: 0 },
        body: body(
            vec![
                LocalDecl::new("origin", exclusive_i64, false),
                LocalDecl::new("other", replace_i64, false),
                LocalDecl::new("held", i64_ty, false),
                LocalDecl::new("child", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::ReferenceMove(ReferenceAccess::new(Place::local(LocalId(1)))),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(3)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(0))),
                        },
                    },
                ],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(3)).into()))),
            )],
        ),
    };

    expect_function_error(
        types,
        function,
        MirValidationErrorKind::ExternalReferentNotLive,
    );
}
