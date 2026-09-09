use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Field, Function, FunctionId, LocalDecl,
    LocalId, MirValidationErrorKind, Operand, Place, Program, SafeReferenceResultContract,
    ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable, Value, validate_program,
};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn no_result_function(
    name: &str,
    parameters: Vec<LocalId>,
    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
) -> Function {
    Function {
        name: name.into(),
        parameters,
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(locals, blocks),
    }
}

fn no_result_target(name: &str) -> Function {
    no_result_function(
        name,
        Vec::new(),
        Vec::new(),
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    )
}

#[test]
fn callable_signature_cycles_are_non_structural_but_struct_cycles_remain_invalid() {
    let mut callable_types = TypeTable::new();
    let callable = callable_types.push(TypeDef::callable(
        "recursive-callable",
        CallableInterface::new(
            vec![TypeId(0)],
            Some(TypeId(0)),
            SafeReferenceResultContract::None,
        ),
    ));
    assert_eq!(callable, TypeId(0));
    validate_program(Program {
        types: callable_types,
        functions: Vec::new(),
    })
    .expect("callable signature edges are non-structural and may cycle");

    let mut structural_types = TypeTable::new();
    let recursive = structural_types.push(TypeDef::structure(
        "recursive-struct",
        vec![Field::new("self", TypeId(0))],
    ));
    assert_eq!(recursive, TypeId(0));
    let error = validate_program(Program {
        types: structural_types,
        functions: Vec::new(),
    })
    .expect_err("by-value structural cycles remain invalid");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::RecursiveType(TypeId(0))
    );
}

#[test]
fn callable_interface_admission_reuses_existing_transfer_safety() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_pointer_ty = types.push(TypeDef::raw_pointer("raw-I64", i64_ty));
    types.push(TypeDef::callable(
        "bad-callable",
        CallableInterface::new(
            vec![raw_pointer_ty],
            None,
            SafeReferenceResultContract::None,
        ),
    ));

    let error = validate_program(Program {
        types,
        functions: Vec::new(),
    })
    .expect_err("raw-pointer parameters are not call-transfer-safe");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ParameterTransferUnsafe(raw_pointer_ty)
    );
}

#[test]
fn equal_callable_interfaces_allow_formation_under_distinct_nominal_type_ids() {
    let interface = CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None);
    let mut types = TypeTable::new();
    let first = types.push(TypeDef::callable("First", interface.clone()));
    let second = types.push(TypeDef::callable("Second", interface));
    assert_ne!(first, second);

    let caller = no_result_function(
        "caller",
        Vec::new(),
        vec![
            LocalDecl::new("first", first, false),
            LocalDecl::new("second", second, false),
        ],
        vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(1)),
                },
                Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::FunctionValue(FunctionId(1)),
                },
            ],
            Terminator::Return(None),
        )],
    );

    validate_program(Program {
        types,
        functions: vec![caller, no_result_target("target")],
    })
    .expect("one function entity may form values under distinct equal callable interfaces");
}

#[test]
fn exact_callable_type_identity_is_preserved_through_storage_and_indirect_calls() {
    let interface = CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None);
    let mut types = TypeTable::new();
    let first = types.push(TypeDef::callable("First", interface.clone()));
    let second = types.push(TypeDef::callable("Second", interface));

    let caller = no_result_function(
        "caller",
        Vec::new(),
        vec![LocalDecl::new("callee", first, false)],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(1)),
                }],
                Terminator::IndirectCall {
                    callable: second,
                    callee: Operand::Copy(Place::local(LocalId(0)).into()),
                    arguments: Vec::new(),
                    destination: None,
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(Vec::new(), Terminator::Return(None)),
        ],
    );

    let error = validate_program(Program {
        types,
        functions: vec![caller, no_result_target("target")],
    })
    .expect_err("equal callable interfaces do not convert one nominal type into another");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: second }
    );
}

#[test]
fn function_value_formation_rejects_unknown_non_callable_and_mismatched_targets() {
    let mut types = TypeTable::new();
    let callable = types.push(TypeDef::callable(
        "Callable",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    let unknown_caller = no_result_function(
        "unknown-caller",
        Vec::new(),
        vec![LocalDecl::new("callee", callable, false)],
        vec![BasicBlock::new(
            vec![Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::FunctionValue(FunctionId(99)),
            }],
            Terminator::Return(None),
        )],
    );
    let error = validate_program(Program {
        types,
        functions: vec![unknown_caller],
    })
    .expect_err("function-value formation requires a same-program function entity");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidFunction(FunctionId(99))
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let non_callable_caller = no_result_function(
        "non-callable-caller",
        Vec::new(),
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![BasicBlock::new(
            vec![Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::FunctionValue(FunctionId(1)),
            }],
            Terminator::Return(None),
        )],
    );
    let error = validate_program(Program {
        types,
        functions: vec![non_callable_caller, no_result_target("target")],
    })
    .expect_err("function values cannot inhabit non-callable expected types");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::CallableTypeRequired(i64_ty)
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "NoArgs",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    let mismatch_caller = no_result_function(
        "mismatch-caller",
        Vec::new(),
        vec![LocalDecl::new("callee", callable, false)],
        vec![BasicBlock::new(
            vec![Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::FunctionValue(FunctionId(1)),
            }],
            Terminator::Return(None),
        )],
    );
    let mismatch_target = no_result_function(
        "takes-I64",
        vec![LocalId(0)],
        vec![LocalDecl::new("parameter", i64_ty, false)],
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    );
    let error = validate_program(Program {
        types,
        functions: vec![mismatch_caller, mismatch_target],
    })
    .expect_err("function-value formation requires exact callable-interface equality");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: callable }
    );
}

#[test]
fn indirect_call_validates_result_interface_and_initializes_normal_destination() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "ReturnsI64",
        CallableInterface::new(
            Vec::new(),
            Some(i64_ty),
            SafeReferenceResultContract::None,
        ),
    ));

    let caller = no_result_function(
        "caller",
        Vec::new(),
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
                vec![Statement::Read {
                    src: Place::local(LocalId(0)).into(),
                }],
                Terminator::Return(None),
            ),
        ],
    );
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

    validate_program(Program {
        types,
        functions: vec![caller, target],
    })
    .expect("indirect call consumes the callable interface for result validation");
}

#[test]
fn indirect_call_evaluates_callee_before_arguments() {
    let mut types = TypeTable::new();
    let callable = types.push(TypeDef::callable(
        "RecursiveArg",
        CallableInterface::new(
            vec![TypeId(0)],
            None,
            SafeReferenceResultContract::None,
        ),
    ));
    assert_eq!(callable, TypeId(0));

    let caller = no_result_function(
        "caller",
        Vec::new(),
        vec![LocalDecl::new("callee", callable, false)],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(1)),
                }],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: vec![Operand::Copy(Place::local(LocalId(0)).into())],
                    destination: None,
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(Vec::new(), Terminator::Return(None)),
        ],
    );
    let target = no_result_function(
        "target",
        vec![LocalId(0)],
        vec![LocalDecl::new("parameter", callable, false)],
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    );

    let error = validate_program(Program {
        types,
        functions: vec![caller, target],
    })
    .expect_err("callee Move must commit before the first argument is evaluated");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(Place::local(LocalId(0)))
    );
}

#[test]
fn indirect_result_destination_is_admitted_before_callee_effects() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "ReturnsI64",
        CallableInterface::new(
            Vec::new(),
            Some(i64_ty),
            SafeReferenceResultContract::None,
        ),
    ));

    let caller = no_result_function(
        "caller",
        Vec::new(),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("destination", i64_ty, false),
        ],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::Constant(Value::I64(7)),
                }],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(Vec::new(), Terminator::Return(None)),
        ],
    );

    let error = validate_program(Program {
        types,
        functions: vec![caller],
    })
    .expect_err("occupied result destination must reject before callee Move is evaluated");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::CallResultRequiresVacant(Place::local(LocalId(1)))
    );
}

#[test]
fn recursive_indirect_call_graph_is_language_valid_without_static_expansion() {
    let mut types = TypeTable::new();
    let callable = types.push(TypeDef::callable(
        "NoArgs",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    let recursive = no_result_function(
        "recursive",
        Vec::new(),
        Vec::new(),
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::FunctionValue(FunctionId(0)),
                    arguments: Vec::new(),
                    destination: None,
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(Vec::new(), Terminator::Return(None)),
        ],
    );

    validate_program(Program {
        types,
        functions: vec![recursive],
    })
    .expect("indirect recursion is valid and does not require target-set expansion");
}
