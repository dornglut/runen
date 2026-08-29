use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, Function, FunctionId, LoanDecl, LoanId,
    LocalDecl, LocalId, MirValidationErrorKind, Operand, Place, Program, ReferenceAccess,
    ReferencePermission, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable, Value,
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

#[test]
fn reference_type_identity_is_exact_referent_and_permission() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
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
    let shared_bool = types.push(TypeDef::reference(
        "SharedBool",
        bool_ty,
        ReferencePermission::Shared,
    ));

    assert_eq!(
        types.reference(shared_i64),
        Some((i64_ty, ReferencePermission::Shared))
    );
    assert_eq!(
        types.reference(exclusive_i64),
        Some((i64_ty, ReferencePermission::Exclusive))
    );
    assert_eq!(
        types.reference(shared_bool),
        Some((bool_ty, ReferencePermission::Shared))
    );
    assert_eq!(
        types.reference_type_id(i64_ty, ReferencePermission::Shared),
        Some(shared_i64)
    );
    assert_eq!(
        types.reference_type_id(i64_ty, ReferencePermission::Exclusive),
        Some(exclusive_i64)
    );
    assert_eq!(
        types.reference_type_id(bool_ty, ReferencePermission::Shared),
        Some(shared_bool)
    );
}

#[test]
fn duplicate_reference_type_pair_is_rejected_by_program_validation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    types.push(TypeDef::reference(
        "SharedI64A",
        i64_ty,
        ReferencePermission::Shared,
    ));
    types.push(TypeDef::reference(
        "SharedI64B",
        i64_ty,
        ReferencePermission::Shared,
    ));

    assert!(
        validate_program(Program {
            types,
            functions: Vec::new(),
        })
        .is_err(),
        "one semantic reference type pair must map to one exact TypeId"
    );
}

#[test]
fn reference_referent_edge_is_not_structural_recursion() {
    let mut types = TypeTable::new();
    let node = types.push(TypeDef::structure(
        "Node",
        vec![Field::new("next", TypeId(1))],
    ));
    let shared_node = types.push(TypeDef::reference(
        "SharedNode",
        node,
        ReferencePermission::Shared,
    ));

    assert_eq!(shared_node, TypeId(1));
    validate_program(Program {
        types,
        functions: Vec::new(),
    })
    .expect("a type cycle through a safe-reference referent edge is not structural recursion");
}

#[test]
fn reference_copyability_follows_permission_and_structural_recursion() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let exclusive = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));
    let exclusive_replace = types.push(TypeDef::reference(
        "ExclusiveReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let shared_pair = types.push(TypeDef::structure(
        "SharedPair",
        vec![Field::new("left", shared), Field::new("right", shared)],
    ));
    let exclusive_pair = types.push(TypeDef::structure(
        "ExclusivePair",
        vec![Field::new("left", shared), Field::new("right", exclusive)],
    ));

    assert!(types.is_copy(shared));
    assert!(!types.is_copy(exclusive));
    assert!(!types.is_copy(exclusive_replace));
    assert!(types.is_copy(shared_pair));
    assert!(!types.is_copy(exclusive_pair));
}

#[test]
fn reference_values_cannot_be_fabricated_as_core_constants() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    assert!(!types.value_matches(reference_ty, &Value::I64(0)));
}

#[test]
fn parameter_and_result_transfer_rules_are_distinct() {
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
    let nested_shared = types.push(TypeDef::reference(
        "NestedSharedI64",
        shared_i64,
        ReferencePermission::Shared,
    ));

    assert!(types.is_parameter_transfer_safe(i64_ty));
    assert!(types.is_result_transfer_safe(i64_ty));

    assert!(types.is_parameter_transfer_safe(shared_i64));
    assert!(!types.is_result_transfer_safe(shared_i64));

    assert!(!types.is_parameter_transfer_safe(raw_i64));
    assert!(!types.is_result_transfer_safe(raw_i64));

    assert!(!types.is_parameter_transfer_safe(shared_raw));
    assert!(!types.is_parameter_transfer_safe(nested_shared));
}

#[test]
fn root_reference_formation_requires_live_exact_referent_and_replace_permission() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
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

    let wrong_type = Function {
        name: "wrong_type".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", bool_ty, false),
                LocalDecl::new("reference", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::Bool(true)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(0)),
                        },
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![wrong_type],
    })
    .expect_err("reference root must use the exact referent type identity");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch {
            expected: shared_i64,
        }
    );

    let uninitialized = Function {
        name: "uninitialized".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::ReferenceRoot {
                        permission: ReferencePermission::Shared,
                        place: Place::local(LocalId(0)),
                    },
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![uninitialized],
    })
    .expect_err("non-zero-leaf reference root must be fully Live");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(ref place)
            if *place == Place::local(LocalId(0))
    ));

    let immutable_replace = Function {
        name: "immutable_replace".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", replace_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::ExclusiveReplace,
                            place: Place::local(LocalId(0)),
                        },
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types,
        functions: vec![immutable_replace],
    })
    .expect_err("ExclusiveReplace root requires ordinary direct assignment permission");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::AssignToImmutable(LocalId(0))
    );
}

#[test]
fn reference_root_and_explicit_borrow_share_one_conflict_domain() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let exclusive_ref = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));

    let reference_after_loan = Function {
        name: "reference_after_loan".into(),
        parameters: Vec::new(),
        result: None,
        body: Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", exclusive_ref, false),
            ],
            loans: vec![LoanDecl::new("shared", i64_ty)],
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Borrow {
                        loan: LoanId(0),
                        kind: BorrowKind::Shared,
                        src: Place::local(LocalId(0)).into(),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Exclusive,
                            place: Place::local(LocalId(0)),
                        },
                    },
                ],
                Terminator::Return(None),
            )],
        },
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![reference_after_loan],
    })
    .expect_err("exclusive reference root conflicts with overlapping shared explicit loan");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferenceFormationConflictsWithLoan {
            place: Place::local(LocalId(0)),
            loan: LoanId(0),
        }
    );

    let loan_after_reference = Function {
        name: "loan_after_reference".into(),
        parameters: Vec::new(),
        result: None,
        body: Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", exclusive_ref, false),
            ],
            loans: vec![LoanDecl::new("shared", i64_ty)],
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Exclusive,
                            place: Place::local(LocalId(0)),
                        },
                    },
                    Statement::Borrow {
                        loan: LoanId(0),
                        kind: BorrowKind::Shared,
                        src: Place::local(LocalId(0)).into(),
                    },
                ],
                Terminator::Return(None),
            )],
        },
    };
    let error = validate_program(Program {
        types,
        functions: vec![loan_after_reference],
    })
    .expect_err("root explicit borrow conflicts with overlapping exclusive reference authority");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::BorrowConflictWithReferenceAuthority {
            place: Place::local(LocalId(0)),
        }
    );
}

#[test]
fn direct_access_observes_active_reference_authority() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let exclusive_ref = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));
    let target = Place::local(LocalId(0));
    let function = Function {
        name: "direct_conflict".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", exclusive_ref, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Exclusive,
                            place: target.clone(),
                        },
                    },
                    Statement::Read {
                        src: target.clone().into(),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("direct read cannot bypass an overlapping exclusive reference authority");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflictWithReferenceAuthority { place: target }
    );
}

#[test]
fn reference_permission_matrix_keeps_move_drop_assign_and_interior_assign_distinct() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let exclusive = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));
    let replace = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));

    let shared_move = Function {
        name: "shared_move".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(2)),
                    src: Operand::ReferenceMove(ReferenceAccess::new(Place::local(LocalId(1)))),
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![shared_move],
    })
    .expect_err("Shared reference does not authorize Move");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferencePermissionRequired(ReferencePermission::Exclusive)
    );

    let shared_drop = Function {
        name: "shared_drop".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::ReferenceDrop {
                    place: ReferenceAccess::new(Place::local(LocalId(1))),
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![shared_drop],
    })
    .expect_err("Shared reference does not authorize Drop");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferencePermissionRequired(ReferencePermission::Exclusive)
    );

    let exclusive_assign = Function {
        name: "exclusive_assign".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", exclusive, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::ReferenceAssign {
                    dst: ReferenceAccess::new(Place::local(LocalId(1))),
                    src: Operand::Constant(Value::I64(2)),
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![exclusive_assign],
    })
    .expect_err("Exclusive reference does not imply ordinary replacement permission");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferencePermissionRequired(ReferencePermission::ExclusiveReplace)
    );

    let replace_assign = Function {
        name: "replace_assign".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("reference", replace, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::ExclusiveReplace,
                            place: Place::local(LocalId(0)),
                        },
                    },
                    Statement::ReferenceAssign {
                        dst: ReferenceAccess::new(Place::local(LocalId(1))),
                        src: Operand::Constant(Value::I64(2)),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };
    validate_program(Program {
        types: types.clone(),
        functions: vec![replace_assign],
    })
    .expect("ExclusiveReplace authorizes ordinary replacement through the reference");

    let interior_i64 =
        types.push(TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability());
    let shared_interior = types.push(TypeDef::reference(
        "SharedInteriorI64",
        interior_i64,
        ReferencePermission::Shared,
    ));
    let shared_interior_assign = Function {
        name: "shared_interior_assign".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", interior_i64, false),
                LocalDecl::new("reference", shared_interior, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(0)),
                        },
                    },
                    Statement::ReferenceInteriorAssign {
                        dst: ReferenceAccess::new(Place::local(LocalId(1))),
                        src: Operand::Constant(Value::I64(2)),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };
    validate_program(Program {
        types: types.clone(),
        functions: vec![shared_interior_assign],
    })
    .expect("Shared reference may InteriorAssign only through independently marked storage");

    let shared_plain_interior_assign = Function {
        name: "shared_plain_interior_assign".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("reference", shared, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::ReferenceInteriorAssign {
                    dst: ReferenceAccess::new(Place::local(LocalId(1))),
                    src: Operand::Constant(Value::I64(2)),
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types,
        functions: vec![shared_plain_interior_assign],
    })
    .expect_err("reference permission never substitutes for interior mutability");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InteriorMutationRequiresMarkedReferenceRegion
    );
}

#[test]
fn reborrow_permission_never_strengthens_and_delegation_is_structural() {
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

    let strengthen_shared = Function {
        name: "strengthen_shared".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("parent", shared_i64, false),
                LocalDecl::new("child", exclusive_i64, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(2)),
                    src: Operand::ReferenceReborrow {
                        permission: ReferencePermission::Exclusive,
                        src: ReferenceAccess::new(Place::local(LocalId(1))),
                    },
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![strengthen_shared],
    })
    .expect_err("Shared parent cannot create Exclusive child");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferencePermissionRequired(ReferencePermission::Exclusive)
    );

    let strengthen_exclusive = Function {
        name: "strengthen_exclusive".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("parent", exclusive_i64, false),
                LocalDecl::new("child", replace_i64, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(2)),
                    src: Operand::ReferenceReborrow {
                        permission: ReferencePermission::ExclusiveReplace,
                        src: ReferenceAccess::new(Place::local(LocalId(1))),
                    },
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![strengthen_exclusive],
    })
    .expect_err("Exclusive parent cannot create ExclusiveReplace child");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferencePermissionRequired(ReferencePermission::ExclusiveReplace)
    );

    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let exclusive_pair = types.push(TypeDef::reference(
        "ExclusivePair",
        pair_ty,
        ReferencePermission::Exclusive,
    ));
    let shared_field = shared_i64;

    let disjoint = Function {
        name: "disjoint".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", pair_ty, false),
                LocalDecl::new("parent", exclusive_pair, false),
                LocalDecl::new("child", shared_field, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)).field(0),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(0)).field(1),
                        src: Operand::Constant(Value::I64(2)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Exclusive,
                            place: Place::local(LocalId(0)),
                        },
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(1))).field(0),
                        },
                    },
                    Statement::ReferenceRead {
                        src: ReferenceAccess::new(Place::local(LocalId(1))).field(0),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(3)),
                        src: Operand::ReferenceMove(
                            ReferenceAccess::new(Place::local(LocalId(1))).field(1),
                        ),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };
    validate_program(Program {
        types: types.clone(),
        functions: vec![disjoint],
    })
    .expect("Shared child leaves parent shared access on overlap and exclusive access disjointly");

    let overlapping_move = Function {
        name: "overlapping_move".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", pair_ty, false),
                LocalDecl::new("parent", exclusive_pair, false),
                LocalDecl::new("child", shared_field, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)).field(0),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(0)).field(1),
                        src: Operand::Constant(Value::I64(2)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Exclusive,
                            place: Place::local(LocalId(0)),
                        },
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(Place::local(LocalId(1))).field(0),
                        },
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(3)),
                        src: Operand::ReferenceMove(
                            ReferenceAccess::new(Place::local(LocalId(1))).field(0),
                        ),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types,
        functions: vec![overlapping_move],
    })
    .expect_err("overlapping Shared child suspends parent exclusive access");
    assert_eq!(error.kind, MirValidationErrorKind::ReferenceAccessDelegated);
}

#[test]
fn cleanup_enforces_storage_extent_and_accepts_zero_leaf_reference_targets() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let bad_order = Function {
        name: "bad_cleanup_order".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("reference", shared_i64, false),
                LocalDecl::new("target", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(1)),
                        },
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![bad_order],
    })
    .expect_err("target storage cannot end before its surviving reference carrier");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ReferenceOutlivesStorage(LocalId(1))
    );

    let empty_ty = types.push(TypeDef::structure("Empty", Vec::new()));
    let shared_empty = types.push(TypeDef::reference(
        "SharedEmpty",
        empty_ty,
        ReferencePermission::Shared,
    ));
    let zero_leaf = Function {
        name: "zero_leaf".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", empty_ty, false),
                LocalDecl::new("reference", shared_empty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: Place::local(LocalId(0)),
                        },
                    },
                    Statement::ReferenceRead {
                        src: ReferenceAccess::new(Place::local(LocalId(1))),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };
    validate_program(Program {
        types,
        functions: vec![zero_leaf],
    })
    .expect("zero-leaf target retains structural storage and authority semantics");
}

#[test]
fn reborrow_drop_loop_has_finite_canonical_authority_state() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let parent = Place::local(LocalId(1));
    let child = Place::local(LocalId(2));
    let function = Function {
        name: "reborrow_loop".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("parent", shared_i64, false),
                LocalDecl::new("child", shared_i64, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: target.clone(),
                            src: Operand::Constant(Value::I64(1)),
                        },
                        Statement::Init {
                            dst: parent.clone(),
                            src: Operand::ReferenceRoot {
                                permission: ReferencePermission::Shared,
                                place: target,
                            },
                        },
                    ],
                    Terminator::Goto(BasicBlockId(1)),
                ),
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: child.clone(),
                            src: Operand::ReferenceReborrow {
                                permission: ReferencePermission::Shared,
                                src: ReferenceAccess::new(parent),
                            },
                        },
                        Statement::Drop {
                            place: child.into(),
                        },
                    ],
                    Terminator::Goto(BasicBlockId(1)),
                ),
            ],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("repeated create/end reborrow state must deduplicate finitely");
}
