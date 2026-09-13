from pathlib import Path


def append_exact(path: str, marker: str, addition: str) -> None:
    p = Path(path)
    text = p.read_text()
    if marker in text:
        raise SystemExit(f"{path}: phase18 marker already present")
    p.write_text(text + "\n" + addition)


append_exact(
    "crates/runen-hir/tests/external_callables.rs",
    "external_lookup_obeys_local_shadowing_and_qualified_accessibility",
    r'''
#[test]
fn external_lookup_obeys_local_shadowing_and_qualified_accessibility() {
    let hir = build(
        "external fn target(I64) -> I64; \
         fn local(value: I64) -> I64 { return value; } \
         fn use() -> I64 { \
             let target: fn(I64) -> I64 = local; \
             return target(2); \
         }",
    )
    .expect("active local callable must shadow same-module external function");
    let use_fn = hir
        .functions
        .iter()
        .find(|function| function.name == "use")
        .expect("use function exists");
    let returned = use_fn
        .runen_body()
        .expect("use is Runen-origin")
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("use returns one call");
    let ValueKind::Call { target, .. } = &returned.kind else {
        panic!("return must retain one call");
    };
    assert!(matches!(target, runen_hir::CallTarget::Indirect { .. }));

    let dependency = parse(
        "export external fn exposed(I64) -> I64; external fn hidden(I64) -> I64;",
    );
    let importer = parse("import dep; fn use() -> I64 { return dep::exposed(3); }");
    let imports = [runen_hir::ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    let qualified = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &importer, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("exported external function is a qualified direct-call target");
    let exposed = qualified
        .functions
        .iter()
        .find(|function| function.name == "exposed")
        .expect("external target exists");
    assert!(exposed.is_external());
    let caller = qualified
        .functions
        .iter()
        .find(|function| function.name == "use")
        .expect("importing caller exists");
    let returned = caller
        .runen_body()
        .expect("caller is Runen-origin")
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("caller returns qualified call");
    let ValueKind::Call {
        target: runen_hir::CallTarget::Direct { function, .. },
        ..
    } = &returned.kind
    else {
        panic!("qualified external call must remain direct");
    };
    assert_eq!(*function, exposed.id);

    let private_importer = parse("import dep; fn use() -> I64 { return dep::hidden(3); }");
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &private_importer, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect_err("module-private external function is inaccessible cross-module");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::InaccessibleBinding),
        "{errors:?}"
    );
}

#[test]
fn generic_and_closure_bodies_direct_call_external_without_capturing_it() {
    let hir = build(
        "external fn ext(I64); \
         fn generic[T](value: T) { ext(1); } \
         fn root(seed: I64) { \
             let call = fn[seed](value: I64) { ext(value); let keep: I64 = seed; }; \
             call(2); \
         }",
    )
    .expect("generic and closure bodies may direct-call one external function");
    let ext = hir
        .functions
        .iter()
        .find(|function| function.name == "ext")
        .expect("external declaration exists");
    let generic = hir
        .functions
        .iter()
        .find(|function| function.name == "generic")
        .expect("generic function exists");
    let Statement::Call { target, .. } = &generic
        .runen_body()
        .expect("generic function is Runen-origin")
        .statements[0]
    else {
        panic!("generic body begins with external call");
    };
    assert!(matches!(
        target,
        runen_hir::CallTarget::Direct { function, type_arguments }
            if *function == ext.id && type_arguments.is_empty()
    ));

    assert_eq!(hir.closures.len(), 1);
    let closure = &hir.closures[0];
    assert_eq!(closure.captures.len(), 1);
    assert_eq!(closure.captures[0].name, "seed");
    let Statement::Call { target, .. } = &closure.body.statements[0] else {
        panic!("closure body begins with external call");
    };
    assert!(matches!(
        target,
        runen_hir::CallTarget::Direct { function, type_arguments }
            if *function == ext.id && type_arguments.is_empty()
    ));
}
''',
)

append_exact(
    "crates/runen-core-ir/tests/external_callables.rs",
    "external_interfaces_reject_every_non_admitted_boundary_category",
    r'''
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
    function.body.blocks.push(BasicBlock::new(Vec::new(), Terminator::Return(None)));
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
    assert_eq!(error.kind, MirValidationErrorKind::UnexpectedResultDestination);
}
''',
)

append_exact(
    "crates/runen-core-lowering/tests/external_callables.rs",
    "qualified_exported_external_call_lowers_to_same_structural_external_category",
    r'''
#[test]
fn qualified_exported_external_call_lowers_to_same_structural_external_category() {
    let dependency = parse("export external fn ext(I64) -> I64;");
    let caller = parse("import dep; fn caller() -> I64 { return dep::ext(9); }");
    let imports = [runen_hir::ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("qualified exported external call is accepted HIR");
    let lowered = lower(&hir).expect("qualified external call lowers to Core");
    let program = lowered.as_program();
    assert_eq!(program.external_callables.len(), 1);
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "caller");
    assert_eq!(external_calls(program), vec![ExternalCallableId(0)]);
    let interface = &program.external_callables[0].interface;
    assert_eq!(interface.parameters.len(), 1);
    assert!(interface.result.is_some());
}
''',
)

append_exact(
    "crates/runen-reference/tests/external_callables.rs",
    "nested_runen_caller_preserves_bool_round_trip_argument_order_and_single_invocation",
    r'''
#[test]
fn nested_runen_caller_preserves_bool_round_trip_argument_order_and_single_invocation() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let external = interface(vec![bool_ty, i64_ty], Some(bool_ty));

    let main = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(bool_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("result", bool_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        },
    };
    let wrapper = Function {
        name: "wrapper".into(),
        parameters: Vec::new(),
        result: Some(bool_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("result", bool_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: vec![
                            Operand::Constant(Value::Bool(true)),
                            Operand::Constant(Value::I64(7)),
                        ],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        },
    };
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: vec![ExternalCallableDecl::new(external.clone())],
        functions: vec![main, wrapper],
    })
    .expect("nested external-call fixture is valid");

    let seen = Rc::new(RefCell::new(Vec::new()));
    let provider_seen = Rc::clone(&seen);
    let machine = Machine::new_with_external_providers(
        program,
        FunctionId(0),
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            external,
            move |arguments| {
                provider_seen.borrow_mut().push(arguments.to_vec());
                ExternalScalarValue::Bool(false)
            },
        )],
    )
    .expect("matching provider is admitted");
    let report = machine.execute().expect("nested external call returns normally");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::Bool(false)));
    assert_eq!(
        *seen.borrow(),
        vec![vec![ExternalScalarValue::Bool(true), ExternalScalarValue::I64(7)]]
    );
    assert!(
        report
            .verification_events
            .iter()
            .any(|event| event.activation.0 == 2),
        "ordinary nested Runen callee must create its own activation"
    );
    assert!(
        report
            .verification_events
            .iter()
            .all(|event| event.activation.0 <= 2),
        "external provider invocation must not create an additional Core activation"
    );
}
''',
)

print("staged #705 exact minimum conformance completion")
