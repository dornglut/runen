use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Function, FunctionId, LocalDecl, LocalId,
    MirValidationErrorKind, Operand, Place, Program, ReferenceAccess, ReferencePermission,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable,
    Value, validate_program,
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

fn i64_target(name: &str, ty: TypeId) -> Function {
    Function {
        name: name.into(),
        parameters: Vec::new(),
        result: Some(ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(1)))),
            )],
        ),
    }
}

#[test]
fn callable_reference_cycle_and_no_result_none_are_valid() {
    let mut types = TypeTable::new();
    let callable = types.push(TypeDef::callable(
        "RecursiveCallable",
        CallableInterface::new(vec![TypeId(1)], None, SafeReferenceResultContract::None),
    ));
    assert_eq!(callable, TypeId(0));
    let shared_callable = types.push(TypeDef::reference(
        "SharedRecursiveCallable",
        callable,
        ReferencePermission::Shared,
    ));
    assert_eq!(shared_callable, TypeId(1));

    validate_program(Program {
        types,
        functions: Vec::new(),
    })
    .expect("callable/reference signature cycles are semantic edges and no-result + None is valid");
}

#[test]
fn callable_interfaces_reject_unknown_parameter_and_result_types() {
    let unknown_parameter = TypeId(99);
    let mut types = TypeTable::new();
    types.push(TypeDef::callable(
        "UnknownParameter",
        CallableInterface::new(
            vec![unknown_parameter],
            None,
            SafeReferenceResultContract::None,
        ),
    ));
    let error = validate_program(Program {
        types,
        functions: Vec::new(),
    })
    .expect_err("callable parameters must name existing types");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UnknownType(unknown_parameter)
    );

    let unknown_result = TypeId(77);
    let mut types = TypeTable::new();
    types.push(TypeDef::callable(
        "UnknownResult",
        CallableInterface::new(
            Vec::new(),
            Some(unknown_result),
            SafeReferenceResultContract::None,
        ),
    ));
    let error = validate_program(Program {
        types,
        functions: Vec::new(),
    })
    .expect_err("callable results must name existing types");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UnknownType(unknown_result)
    );
}

#[test]
fn callable_interfaces_reject_unsafe_results_and_invalid_reference_contracts() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_i64 = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    types.push(TypeDef::callable(
        "RawResult",
        CallableInterface::new(Vec::new(), Some(raw_i64), SafeReferenceResultContract::None),
    ));
    let error = validate_program(Program {
        types,
        functions: Vec::new(),
    })
    .expect_err("raw-pointer callable results are not result-transfer-safe");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ResultTransferUnsafe(raw_i64)
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    types.push(TypeDef::callable(
        "MissingContract",
        CallableInterface::new(
            Vec::new(),
            Some(shared_i64),
            SafeReferenceResultContract::None,
        ),
    ));
    let error = validate_program(Program {
        types,
        functions: Vec::new(),
    })
    .expect_err("Shared callable results require an accepted reference-result contract");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::MissingSafeReferenceResultContract
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    types.push(TypeDef::callable(
        "BadContractSlot",
        CallableInterface::new(
            Vec::new(),
            Some(shared_i64),
            SafeReferenceResultContract::SharedIdentity { origin: 0 },
        ),
    ));
    let error = validate_program(Program {
        types,
        functions: Vec::new(),
    })
    .expect_err("reference-result contracts must designate an existing parameter slot");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidSafeReferenceResultContractSlot(0)
    );
}

#[test]
fn indirect_calls_require_callable_type_and_exact_result_destination_shape() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let caller = no_result_function(
        "non-callable-caller",
        Vec::new(),
        Vec::new(),
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::IndirectCall {
                    callable: i64_ty,
                    callee: Operand::FunctionValue(FunctionId(1)),
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
    .expect_err("the explicit indirect-call type must be callable");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::CallableTypeRequired(i64_ty)
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "ReturnsI64",
        CallableInterface::new(Vec::new(), Some(i64_ty), SafeReferenceResultContract::None),
    ));
    let caller = no_result_function(
        "missing-destination",
        Vec::new(),
        Vec::new(),
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::FunctionValue(FunctionId(1)),
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
        functions: vec![caller, i64_target("target", i64_ty)],
    })
    .expect_err("result-bearing indirect calls require a destination");
    assert_eq!(error.kind, MirValidationErrorKind::MissingResultDestination);

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "NoResult",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    let caller = no_result_function(
        "unexpected-destination",
        Vec::new(),
        vec![LocalDecl::new("destination", i64_ty, false)],
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
            BasicBlock::new(Vec::new(), Terminator::Return(None)),
        ],
    );
    let error = validate_program(Program {
        types,
        functions: vec![caller, no_result_target("target")],
    })
    .expect_err("no-result indirect calls reject a result destination");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UnexpectedResultDestination
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let callable = types.push(TypeDef::callable(
        "ReturnsI64",
        CallableInterface::new(Vec::new(), Some(i64_ty), SafeReferenceResultContract::None),
    ));
    let caller = no_result_function(
        "wrong-destination",
        Vec::new(),
        vec![LocalDecl::new("destination", bool_ty, false)],
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
            BasicBlock::new(Vec::new(), Terminator::Return(None)),
        ],
    );
    let error = validate_program(Program {
        types,
        functions: vec![caller, i64_target("target", i64_ty)],
    })
    .expect_err("indirect result destinations use the callable interface's exact result type");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: i64_ty }
    );
}

#[test]
fn indirect_call_arguments_use_exact_interface_types() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "TakesI64",
        CallableInterface::new(vec![i64_ty], None, SafeReferenceResultContract::None),
    ));
    let caller = no_result_function(
        "caller",
        Vec::new(),
        Vec::new(),
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::FunctionValue(FunctionId(1)),
                    arguments: vec![Operand::Constant(Value::Bool(true))],
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
        vec![LocalDecl::new("parameter", i64_ty, false)],
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    );
    let error = validate_program(Program {
        types,
        functions: vec![caller, target],
    })
    .expect_err("indirect arguments require exact callable-interface types");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: i64_ty }
    );
}

#[test]
fn indirect_arguments_are_left_to_right_before_final_reference_admission() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let callable = types.push(TypeDef::callable(
        "ConsumeThenBorrow",
        CallableInterface::new(
            vec![i64_ty, replace_i64],
            None,
            SafeReferenceResultContract::None,
        ),
    ));
    let target_place = Place::local(LocalId(0));
    let reference = Place::local(LocalId(1));

    let caller = no_result_function(
        "caller",
        Vec::new(),
        vec![
            LocalDecl::new("target", i64_ty, true),
            LocalDecl::new("reference", replace_i64, false),
        ],
        vec![
            BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target_place.clone(),
                        src: Operand::Constant(Value::I64(7)),
                    },
                    Statement::Init {
                        dst: reference.clone(),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::ExclusiveReplace,
                            place: target_place,
                        },
                    },
                ],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::FunctionValue(FunctionId(1)),
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
    );
    let callee = no_result_function(
        "consume-then-borrow",
        vec![LocalId(0), LocalId(1)],
        vec![
            LocalDecl::new("value", i64_ty, false),
            LocalDecl::new("reference", replace_i64, false),
        ],
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    );

    let error = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect_err("earlier indirect-argument effects must be visible to final call-entry admission");
    assert_eq!(error.kind, MirValidationErrorKind::ReferenceTargetNotLive);
}

#[test]
fn indirect_static_validation_checks_arguments_before_normal_continuation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "TakesI64",
        CallableInterface::new(vec![i64_ty], None, SafeReferenceResultContract::None),
    ));
    let caller = no_result_function(
        "caller",
        Vec::new(),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::IndirectCall {
                callable,
                callee: Operand::FunctionValue(FunctionId(1)),
                arguments: vec![Operand::Constant(Value::Bool(true))],
                destination: None,
                target: BasicBlockId(99),
            },
        )],
    );
    let target = no_result_function(
        "target",
        vec![LocalId(0)],
        vec![LocalDecl::new("parameter", i64_ty, false)],
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    );

    let error = validate_program(Program {
        types,
        functions: vec![caller, target],
    })
    .expect_err("argument validity precedes normal-continuation validity for indirect calls");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: i64_ty }
    );
}
