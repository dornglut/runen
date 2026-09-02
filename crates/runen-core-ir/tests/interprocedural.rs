use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Function, FunctionId, LoanDecl, LoanId, LocalDecl,
    LocalId, MirLocation, MirValidationErrorKind, Operand, Place, Program,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_program,
};

fn scalar_program_types() -> (TypeTable, runen_core_ir::TypeId) {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    (types, i64_ty)
}

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

#[test]
fn validates_no_result_call_and_parameter_is_live_on_entry() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("arg", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(7)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "callee".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(
                vec![Statement::Read {
                    src: Place::local(LocalId(0)).into(),
                }],
                Terminator::Return(None),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("valid parameter transfer and no-result call");
}

#[test]
fn result_call_initializes_destination_on_normal_successor() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("arg", i64_ty, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(9)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(1)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("normal result transfer initializes destination");
}

#[test]
fn call_arguments_reject_exact_arity_and_type_mismatches() {
    let (types, i64_ty) = scalar_program_types();
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
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "take_one".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };
    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("call arity must match exactly");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ArgumentCount {
            expected: 1,
            found: 0,
        }
    );

    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("argument", bool_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::Bool(true)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Copy(Place::local(LocalId(0)).into())],
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "take_i64".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };
    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("call argument types must match exactly");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: i64_ty }
    );
}

#[test]
fn call_arguments_apply_move_effects_left_to_right() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("value", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![
                            Operand::Move(Place::local(LocalId(0)).into()),
                            Operand::Move(Place::local(LocalId(0)).into()),
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
        name: "take_two".into(),
        parameters: vec![LocalId(0), LocalId(1)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("left", i64_ty, false),
                LocalDecl::new("right", i64_ty, false),
            ],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("second move observes first argument consumption");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(ref place)
            if *place == Place::local(LocalId(0))
    ));
}

#[test]
fn call_arguments_apply_copy_then_move_effects_left_to_right() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("value", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(1)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![
                            Operand::Copy(Place::local(LocalId(0)).into()),
                            Operand::Move(Place::local(LocalId(0)).into()),
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
        name: "take_two".into(),
        parameters: vec![LocalId(0), LocalId(1)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("left", i64_ty, false),
                LocalDecl::new("right", i64_ty, false),
            ],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("copy leaves the source live for the following move");
}

#[test]
fn result_destination_must_be_vacant() {
    let (types, i64_ty) = scalar_program_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(5)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "produce".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(8)))),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("call result initialization cannot replace a Live destination");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::CallResultRequiresVacant(ref place)
            if *place == Place::local(LocalId(0))
    ));
}

#[test]
fn result_destination_vacancy_is_checked_before_argument_moves() {
    let (types, i64_ty) = scalar_program_types();
    let result = Place::local(LocalId(0));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: result.clone(),
                        src: Operand::Constant(Value::I64(5)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(result.clone().into())],
                        destination: Some(result.clone()),
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
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
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
    .expect_err("argument Move cannot manufacture vacancy for its own result destination");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::CallResultRequiresVacant(result)
    );
}

#[test]
fn cyclic_result_call_reuses_destination_after_prior_lifetime_ends() {
    let (types, i64_ty) = scalar_program_types();
    let result = Place::local(LocalId(0));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(result.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Drop {
                        place: result.into(),
                    }],
                    Terminator::Goto(BasicBlockId(0)),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "produce".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(8)))),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("cyclic call may reuse its result destination after the prior lifetime ends");
}

#[test]
fn result_destination_requires_init_like_exclusive_authority() {
    let (types, i64_ty) = scalar_program_types();
    let result = Place::local(LocalId(0));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("result", i64_ty, false)],
            loans: vec![LoanDecl::new("shared", i64_ty)],
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: result.clone(),
                            src: Operand::Constant(Value::I64(5)),
                        },
                        Statement::Borrow {
                            loan: LoanId(0),
                            kind: BorrowKind::Shared,
                            src: result.clone().into(),
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(result.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        },
    };
    let callee = Function {
        name: "produce".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(8)))),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("call result initialization requires exclusive direct authority");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: result,
            loan: LoanId(0),
        }
    );
}

#[test]
fn raw_pointer_containing_signatures_are_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("Ptr", i64_ty));
    let function = Function {
        name: "pointer_parameter".into(),
        parameters: vec![LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("pointer", pointer_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("raw-pointer values do not cross activations in this slice");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ParameterTransferUnsafe(pointer_ty)
    );
    assert_eq!(error.location, MirLocation::Function(FunctionId(0)));
}

#[test]
fn raw_pointer_containing_results_are_rejected() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("Ptr", i64_ty));
    let function = Function {
        name: "pointer_result".into(),
        parameters: Vec::new(),
        result: Some(pointer_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("raw-pointer results do not cross activations in this slice");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ResultTransferUnsafe(pointer_ty)
    );
    assert_eq!(error.location, MirLocation::Function(FunctionId(0)));
}

#[test]
fn direct_and_mutual_recursive_call_graphs_are_valid() {
    let types = TypeTable::new();
    let left = Function {
        name: "left".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let right = Function {
        name: "right".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(0),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![left, right],
    })
    .expect("call-graph cycles are not a validation error");
}

#[test]
fn result_return_requires_an_owned_value() {
    let (types, i64_ty) = scalar_program_types();
    let function = Function {
        name: "bad_return".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("result-bearing function must return a value");
    assert_eq!(error.kind, MirValidationErrorKind::MissingReturnValue);
}

#[test]
fn no_result_return_rejects_an_owned_value() {
    let (types, _) = scalar_program_types();
    let function = Function {
        name: "bad_return".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(1)))),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("no-result function must not return a value");
    assert_eq!(error.kind, MirValidationErrorKind::UnexpectedReturnValue);
}

#[test]
fn parameter_local_designations_are_unique() {
    let (types, i64_ty) = scalar_program_types();
    let function = Function {
        name: "duplicate_parameter".into(),
        parameters: vec![LocalId(0), LocalId(0)],
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("parameter", i64_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("one local cannot realize two parameter slots");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DuplicateParameter(LocalId(0))
    );
}

#[test]
fn body_points_include_function_identity() {
    let types = TypeTable::new();
    let first = Function {
        name: "first".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };
    let second = Function {
        name: "second".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Goto(BasicBlockId(9)),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![first, second],
    })
    .expect_err("invalid block in second function");
    assert!(matches!(
        error.location,
        MirLocation::Point(ref point)
            if point.function == FunctionId(1) && point.block == BasicBlockId(0)
    ));
}

#[test]
fn ordinary_source_scalar_tags_are_copyable_without_value_carriers() {
    let tags = [
        ScalarType::Bool,
        ScalarType::I8,
        ScalarType::I16,
        ScalarType::I32,
        ScalarType::I64,
        ScalarType::U8,
        ScalarType::U16,
        ScalarType::U32,
        ScalarType::U64,
        ScalarType::F16,
        ScalarType::F32,
        ScalarType::F64,
    ];
    let mut types = TypeTable::new();
    for (index, tag) in tags.into_iter().enumerate() {
        let ty = types.push(TypeDef::scalar(format!("scalar-{index}"), tag));
        assert!(types.is_copy(ty));
        assert!(types.is_parameter_transfer_safe(ty));
        assert!(types.is_result_transfer_safe(ty));
    }
}

fn branch(condition: Operand, true_target: u32, false_target: u32) -> Terminator {
    Terminator::Branch {
        condition,
        true_target: BasicBlockId(true_target),
        false_target: BasicBlockId(false_target),
    }
}

#[test]
fn branch_validates_targets_and_bool_valued_operand_shape() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));

    let valid = Function {
        name: "valid".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    branch(Operand::Constant(Value::Bool(true)), 1, 1),
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    validate_program(Program {
        types: types.clone(),
        functions: vec![valid],
    })
    .expect("Bool constant with equal valid targets is branch-admissible without a Bool TypeId");

    let invalid_target = Function {
        name: "invalid_target".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                branch(Operand::Constant(Value::Bool(true)), 0, 9),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![invalid_target],
    })
    .expect_err("both Branch targets must exist");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidTargetBlock(BasicBlockId(9))
    );

    let non_bool = Function {
        name: "non_bool".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                branch(Operand::Constant(Value::I64(1)), 0, 0),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![non_bool],
    })
    .expect_err("non-Bool constant is not branch-admissible");
    assert_eq!(error.kind, MirValidationErrorKind::BranchConditionNotBool);

    let bad_access = Function {
        name: "bad_access".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("value", i64_ty, false)],
            vec![BasicBlock::new(
                Vec::new(),
                branch(Operand::AddressOf(Place::local(LocalId(9)).into()), 0, 0),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![bad_access],
    })
    .expect_err("malformed AddressOf access fails before Branch Bool admission");
    assert_eq!(error.kind, MirValidationErrorKind::InvalidLocal(LocalId(9)));
}

#[test]
fn branch_move_copy_and_raw_move_use_existing_operand_contracts() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let tracked_ty = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let bool_ptr = types.push(TypeDef::raw_pointer("BoolPtr", bool_ty));
    let i64_ptr = types.push(TypeDef::raw_pointer("I64Ptr", i64_ty));

    let copy_bool = Function {
        name: "copy_bool".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("condition", bool_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::Bool(true)),
                    }],
                    branch(Operand::Copy(Place::local(LocalId(0)).into()), 1, 2),
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(0)).into(),
                    }],
                    Terminator::Return(None),
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
    validate_program(Program {
        types: types.clone(),
        functions: vec![copy_bool],
    })
    .expect("Bool Copy leaves the source live on both validation successors");

    let move_non_bool = Function {
        name: "move_non_bool".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("value", i64_ty, false)],
            vec![BasicBlock::new(
                Vec::new(),
                branch(Operand::Move(Place::local(LocalId(0)).into()), 0, 0),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![move_non_bool],
    })
    .expect_err("non-Bool Move is not branch-admissible");
    assert_eq!(error.kind, MirValidationErrorKind::BranchConditionNotBool);

    let copy_noncopy = Function {
        name: "copy_noncopy".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("value", tracked_ty, false)],
            vec![BasicBlock::new(
                Vec::new(),
                branch(Operand::Copy(Place::local(LocalId(0)).into()), 0, 0),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![copy_noncopy],
    })
    .expect_err("existing CopyOfNonCopy diagnostic precedes Branch admission");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::CopyOfNonCopy(tracked_ty)
    );

    let raw_bool = Function {
        name: "raw_bool".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("condition", bool_ty, false),
                LocalDecl::new("pointer", bool_ptr, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::Bool(true)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::AddressOf(Place::local(LocalId(0)).into()),
                        },
                    ],
                    branch(Operand::RawMove(Place::local(LocalId(1)).into()), 1, 1),
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    validate_program(Program {
        types: types.clone(),
        functions: vec![raw_bool],
    })
    .expect("RawMove through Bool pointee is branch-admissible");

    let raw_i64 = Function {
        name: "raw_i64".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("pointer", i64_ptr, false)],
            vec![BasicBlock::new(
                Vec::new(),
                branch(Operand::RawMove(Place::local(LocalId(0)).into()), 0, 0),
            )],
        ),
    };
    let error = validate_program(Program {
        types: types.clone(),
        functions: vec![raw_i64],
    })
    .expect_err("RawMove through non-Bool pointee is not branch-admissible");
    assert_eq!(error.kind, MirValidationErrorKind::BranchConditionNotBool);
}

#[test]
fn branch_move_effect_is_propagated_to_both_validation_edges() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let condition = Place::local(LocalId(0));
    let function = Function {
        name: "move_condition".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("condition", bool_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: condition.clone(),
                        src: Operand::Constant(Value::Bool(true)),
                    }],
                    branch(Operand::Move(condition.clone().into()), 1, 2),
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: condition.clone().into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("false validation edge observes the Branch Move consumption");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(ref place) if *place == condition
    ));
}

#[test]
fn constant_branches_validate_both_cfg_edges_without_value_pruning() {
    for condition in [true, false] {
        let (types, i64_ty) = scalar_program_types();
        let unread = Place::local(LocalId(0));
        let invalid_target = if condition { 2 } else { 1 };
        let function = Function {
            name: "constant_branch".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: body(
                vec![LocalDecl::new("uninitialized", i64_ty, false)],
                vec![
                    BasicBlock::new(
                        Vec::new(),
                        branch(Operand::Constant(Value::Bool(condition)), 1, 2),
                    ),
                    BasicBlock::new(
                        if invalid_target == 1 {
                            vec![Statement::Read {
                                src: unread.clone().into(),
                            }]
                        } else {
                            Vec::new()
                        },
                        Terminator::Return(None),
                    ),
                    BasicBlock::new(
                        if invalid_target == 2 {
                            vec![Statement::Read {
                                src: unread.clone().into(),
                            }]
                        } else {
                            Vec::new()
                        },
                        Terminator::Return(None),
                    ),
                ],
            ),
        };

        let error = validate_program(Program {
            types,
            functions: vec![function],
        })
        .expect_err("constant Branch does not prune its opposite validation edge");
        assert!(matches!(
            error.kind,
            MirValidationErrorKind::UseOfUninitialized(ref place) if *place == unread
        ));
    }
}

#[test]
fn branch_join_keeps_distinct_partial_initialization_states() {
    let (types, i64_ty) = scalar_program_types();
    let value = Place::local(LocalId(0));
    let function = Function {
        name: "partial_join".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("value", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    branch(Operand::Constant(Value::Bool(true)), 1, 2),
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: value.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    }],
                    Terminator::Goto(BasicBlockId(3)),
                ),
                BasicBlock::new(Vec::new(), Terminator::Goto(BasicBlockId(3))),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: value.clone().into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("join operation must be valid under every incoming initialization state");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(ref place) if *place == value
    ));
}

#[test]
fn branch_join_keeps_distinct_active_loan_states() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let value = Place::local(LocalId(0));
    let function = Function {
        name: "loan_join".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("value", i64_ty, true)],
            loans: vec![LoanDecl::new("shared", i64_ty)],
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: value.clone(),
                        src: Operand::Constant(Value::I64(1)),
                    }],
                    branch(Operand::Constant(Value::Bool(true)), 1, 2),
                ),
                BasicBlock::new(
                    vec![Statement::Borrow {
                        loan: LoanId(0),
                        kind: BorrowKind::Shared,
                        src: value.clone().into(),
                    }],
                    Terminator::Goto(BasicBlockId(3)),
                ),
                BasicBlock::new(Vec::new(), Terminator::Goto(BasicBlockId(3))),
                BasicBlock::new(
                    vec![Statement::Assign {
                        dst: value.clone().into(),
                        src: Operand::Constant(Value::I64(2)),
                    }],
                    Terminator::Return(None),
                ),
            ],
        },
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("join retains the incoming active-loan distinction");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::DirectAccessConflict {
            place: value,
            loan: LoanId(0),
        }
    );
}

#[test]
fn branch_join_keeps_raw_pointer_targets_and_no_continuation_is_path_local() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let ptr_ty = types.push(TypeDef::raw_pointer("Ptr", i64_ty));
    let a = Place::local(LocalId(0));
    let b = Place::local(LocalId(1));
    let pa = Place::local(LocalId(2));
    let pb = Place::local(LocalId(3));
    let selected = Place::local(LocalId(4));
    let marker = Place::local(LocalId(5));

    let function = Function {
        name: "pointer_join".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![
                LocalDecl::new("a", i64_ty, false),
                LocalDecl::new("b", i64_ty, false),
                LocalDecl::new("pa", ptr_ty, false),
                LocalDecl::new("pb", ptr_ty, false),
                LocalDecl::new("selected", ptr_ty, false),
                LocalDecl::new("marker", i64_ty, false),
            ],
            loans: vec![LoanDecl::new("exclusive_a", i64_ty)],
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: a.clone(),
                            src: Operand::Constant(Value::I64(1)),
                        },
                        Statement::Init {
                            dst: b.clone(),
                            src: Operand::Constant(Value::I64(2)),
                        },
                        Statement::Init {
                            dst: pa.clone(),
                            src: Operand::AddressOf(a.clone().into()),
                        },
                        Statement::Init {
                            dst: pb.clone(),
                            src: Operand::AddressOf(b.clone().into()),
                        },
                        Statement::Borrow {
                            loan: LoanId(0),
                            kind: BorrowKind::Exclusive,
                            src: a.clone().into(),
                        },
                    ],
                    branch(Operand::Constant(Value::Bool(true)), 1, 2),
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: selected.clone(),
                        src: Operand::Move(pa.into()),
                    }],
                    Terminator::Goto(BasicBlockId(3)),
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: selected.clone(),
                        src: Operand::Move(pb.into()),
                    }],
                    Terminator::Goto(BasicBlockId(3)),
                ),
                BasicBlock::new(
                    vec![
                        Statement::RawRead {
                            pointer: selected.into(),
                        },
                        Statement::Read {
                            src: marker.clone().into(),
                        },
                    ],
                    Terminator::Return(None),
                ),
            ],
        },
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("the b-target path continues past RawRead and validates the marker read");
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(ref place) if *place == marker
    ));
}

#[test]
fn branch_condition_ub_creates_no_successor_work_item() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let bool_ptr = types.push(TypeDef::raw_pointer("BoolPtr", bool_ty));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let condition = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let marker = Place::local(LocalId(2));
    let function = Function {
        name: "ub_condition".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("condition", bool_ty, false),
                LocalDecl::new("pointer", bool_ptr, false),
                LocalDecl::new("marker", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: condition.clone(),
                            src: Operand::Constant(Value::Bool(true)),
                        },
                        Statement::Init {
                            dst: pointer.clone(),
                            src: Operand::AddressOf(condition.clone().into()),
                        },
                        Statement::Drop {
                            place: condition.into(),
                        },
                    ],
                    branch(Operand::RawMove(pointer.into()), 1, 2),
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: marker.clone().into(),
                    }],
                    Terminator::Return(None),
                ),
                BasicBlock::new(
                    vec![Statement::Read { src: marker.into() }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("unsafe no-defined-continuation Branch condition reaches neither successor");
}

#[test]
fn disconnected_invalid_branch_is_still_statically_rejected() {
    let types = TypeTable::new();
    let function = Function {
        name: "disconnected".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
                BasicBlock::new(
                    Vec::new(),
                    branch(Operand::Constant(Value::Bool(true)), 9, 9),
                ),
            ],
        ),
    };

    let error = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect_err("static validation covers disconnected Branch blocks");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidTargetBlock(BasicBlockId(9))
    );
}

#[test]
fn branch_and_goto_cycles_deduplicate_complete_validation_states() {
    let types = TypeTable::new();
    let function = Function {
        name: "cycle".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    branch(Operand::Constant(Value::Bool(true)), 0, 1),
                ),
                BasicBlock::new(Vec::new(), Terminator::Goto(BasicBlockId(0))),
            ],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("repeated identical complete CFG states terminate validation exploration");
}
