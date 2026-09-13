use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, ExternalCallableDecl, ExternalCallableId,
    Function, LocalDecl, LocalId, MirValidationErrorKind, Operand, Place, Program,
    SafeReferenceResultContract, ScalarType, Terminator, TypeDef, TypeId, TypeTable,
    validate_program,
};

fn body(locals: Vec<LocalDecl>, terminator: Terminator) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(Vec::new(), terminator)],
    }
}

fn root(result: Option<TypeId>, body: Body) -> Function {
    Function {
        name: "root".into(),
        parameters: Vec::new(),
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body,
    }
}

#[test]
fn external_identity_and_scalar_interface_are_distinct_from_functions() {
    let first = ExternalCallableId(0);
    let second = ExternalCallableId(1);
    assert_ne!(first, second);

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let interface = CallableInterface {
        parameters: vec![i64_ty],
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
    };
    let function = root(
        Some(i64_ty),
        body(
            vec![LocalDecl::new("result", i64_ty, false)],
            Terminator::ExternalCall {
                external: first,
                arguments: vec![Operand::Constant(runen_core_ir::Value::I64(4))],
                destination: Some(Place::local(LocalId(0))),
                target: BasicBlockId(1),
            },
        ),
    );
    let mut function = function;
    function.body.blocks.push(BasicBlock::new(
        Vec::new(),
        Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
    ));
    validate_program(Program {
        external_callables: vec![
            ExternalCallableDecl::new(interface.clone()),
            ExternalCallableDecl::new(interface),
        ],
        types,
        persistent: Vec::new(),
        functions: vec![function],
    })
    .expect("scalar external call is valid");
}

#[test]
fn external_declarations_reject_non_scalar_and_reference_contract_interfaces() {
    let mut types = TypeTable::new();
    let record_ty = types.push(TypeDef::structure("Record", vec![]));
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: vec![record_ty],
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
        })],
        types,
        persistent: Vec::new(),
        functions: Vec::new(),
    })
    .expect_err("aggregate external interface is invalid");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidExternalCallableType {
            external: ExternalCallableId(0),
            ty: record_ty,
        }
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: vec![i64_ty],
            result: Some(i64_ty),
            safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity {
                origin: 0,
            },
        })],
        types,
        persistent: Vec::new(),
        functions: Vec::new(),
    })
    .expect_err("external interface contract must be None");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ExternalCallableRequiresNoReferenceResultContract(
            ExternalCallableId(0)
        )
    );
}

#[test]
fn external_call_checks_target_arity_and_destination_shape() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let interface = CallableInterface {
        parameters: vec![i64_ty],
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
    };
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(interface)],
        types,
        persistent: Vec::new(),
        functions: vec![root(
            None,
            body(
                Vec::new(),
                Terminator::ExternalCall {
                    external: ExternalCallableId(0),
                    arguments: Vec::new(),
                    destination: None,
                    target: BasicBlockId(0),
                },
            ),
        )],
    })
    .expect_err("result-bearing external call requires destination");
    assert_eq!(error.kind, MirValidationErrorKind::MissingResultDestination);
}

fn assert_invalid_external_parameter(types: TypeTable, ty: TypeId) {
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: vec![ty],
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
        })],
        types,
        persistent: Vec::new(),
        functions: Vec::new(),
    })
    .expect_err("non-admitted external parameter type must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidExternalCallableType {
            external: ExternalCallableId(0),
            ty,
        }
    );
}

#[test]
fn external_interfaces_reject_every_non_admitted_boundary_category() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_ty = types.push(TypeDef::scalar("Raw", ScalarType::RawPointer(i64_ty)));
    assert_invalid_external_parameter(types, raw_ty);

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::scalar(
        "SharedI64",
        ScalarType::Reference {
            referent: i64_ty,
            permission: runen_core_ir::ReferencePermission::Shared,
        },
    ));
    assert_invalid_external_parameter(types, reference_ty);

    let mut types = TypeTable::new();
    let callable_ty = types.push(TypeDef::scalar(
        "Callable",
        ScalarType::Callable(CallableInterface {
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
        }),
    ));
    assert_invalid_external_parameter(types, callable_ty);

    let mut types = TypeTable::new();
    let record_ty = types.push(TypeDef::structure("Record", vec![]));
    assert_invalid_external_parameter(types, record_ty);

    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    assert_invalid_external_parameter(types, tracked_ty);
}

#[test]
fn external_call_validation_covers_target_arity_operand_and_destination_edges() {
    let types = TypeTable::new();
    let error = validate_program(Program {
        external_callables: Vec::new(),
        types,
        persistent: Vec::new(),
        functions: vec![root(
            None,
            body(
                Vec::new(),
                Terminator::ExternalCall {
                    external: ExternalCallableId(0),
                    arguments: Vec::new(),
                    destination: None,
                    target: BasicBlockId(0),
                },
            ),
        )],
    })
    .expect_err("unknown external target must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidExternalCallable(ExternalCallableId(0))
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: vec![i64_ty],
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
        })],
        types,
        persistent: Vec::new(),
        functions: vec![root(
            None,
            body(
                Vec::new(),
                Terminator::ExternalCall {
                    external: ExternalCallableId(0),
                    arguments: Vec::new(),
                    destination: None,
                    target: BasicBlockId(0),
                },
            ),
        )],
    })
    .expect_err("external call arity must match exactly");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ArgumentCount {
            expected: 1,
            found: 0,
        }
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let _bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: vec![i64_ty],
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
        })],
        types,
        persistent: Vec::new(),
        functions: vec![root(
            None,
            body(
                Vec::new(),
                Terminator::ExternalCall {
                    external: ExternalCallableId(0),
                    arguments: vec![Operand::Constant(runen_core_ir::Value::Bool(false))],
                    destination: None,
                    target: BasicBlockId(0),
                },
            ),
        )],
    })
    .expect_err("external operand type must match exactly");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: i64_ty }
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let mut function = root(
        None,
        body(
            vec![LocalDecl::new("wrong_result", bool_ty, false)],
            Terminator::ExternalCall {
                external: ExternalCallableId(0),
                arguments: Vec::new(),
                destination: Some(Place::local(LocalId(0))),
                target: BasicBlockId(1),
            },
        ),
    );
    function
        .body
        .blocks
        .push(BasicBlock::new(Vec::new(), Terminator::Return(None)));
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: Vec::new(),
            result: Some(i64_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
        })],
        types,
        persistent: Vec::new(),
        functions: vec![function],
    })
    .expect_err("external result destination type must match exactly");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { expected: i64_ty }
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
        })],
        types,
        persistent: Vec::new(),
        functions: vec![root(
            None,
            body(
                vec![LocalDecl::new("unexpected", i64_ty, false)],
                Terminator::ExternalCall {
                    external: ExternalCallableId(0),
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(0))),
                    target: BasicBlockId(0),
                },
            ),
        )],
    })
    .expect_err("no-result external call must not have a destination");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UnexpectedResultDestination
    );
}
