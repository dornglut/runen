use runen_hir::{
    CallTarget, DiagnosticKind, ImportTarget, ModuleId, SafeReferenceResultContract, SourceUnit,
    Type, TypedCompilation, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

#[test]
fn qualified_exported_function_forms_a_value_but_private_member_does_not() {
    let dependency = parse(
        "export fn exported(value: I64) -> I64 { return value; } \
         fn private(value: I64) -> I64 { return value; }",
    );
    let good = parse(
        "import dep; fn use() -> I64 { let f: fn(I64) -> I64 = dep::exported; return f(3); }",
    );
    assert!(dependency.errors().is_empty(), "{:?}", dependency.errors());
    assert!(good.errors().is_empty(), "{:?}", good.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid import alias")];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &good, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("exported non-generic function is an accessible function value");
    let use_fn = function(&hir, "use");
    let initializer = match &use_fn.body.statements[0] {
        runen_hir::Statement::Local { initializer, .. } => initializer,
        other => panic!("expected function-valued local, found {other:?}"),
    };
    assert!(matches!(initializer.kind, ValueKind::FunctionValue { .. }));
    let returned = use_fn
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("use returns one indirect call");
    assert!(matches!(
        returned.kind,
        ValueKind::Call {
            target: CallTarget::Indirect { .. },
            ..
        }
    ));

    let bad =
        parse("import dep; fn use() -> I64 { let f: fn(I64) -> I64 = dep::private; return f(3); }");
    assert!(bad.errors().is_empty(), "{:?}", bad.errors());
    let errors = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &bad, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect_err("private imported function must not form a value");
    assert!(has_diagnostic(&errors, DiagnosticKind::InaccessibleBinding));
}

#[test]
fn function_value_formation_requires_exact_structural_type() {
    let errors = build(
        "fn target(value: I64) -> I64 { return value; } \
         fn bad() { let f: fn(Bool) -> I64 = target; }",
    )
    .expect_err("function value must match the required function type exactly");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Function(_),
            found: Type::Function(_),
        }
    )));
}

#[test]
fn generic_parameter_inside_function_type_never_falls_back_to_same_named_record() {
    let errors = build("record T {} fn bad[T](f: fn(T) -> I64) {}")
        .expect_err("function-type components are concrete even beside a same-named nominal");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::InvalidGenericTypeParameterPosition
    ));
}

#[test]
fn exported_function_type_recursively_rejects_private_nominal_components() {
    let errors = build(
        "record Private {} \
         export fn use(f: fn(Private) -> I64) {}",
    )
    .expect_err("exported callable signature cannot expose a private record transitively");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::PrivateTypeInExportedSignature
    ));
}

#[test]
fn rejected_indirect_arity_does_not_consume_ordinary_argument_state() {
    let errors = build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn use(f: fn(Ticket), ticket: Ticket) { f(); sink(ticket); }",
    )
    .expect_err("first indirect call has wrong arity");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::ArgumentCount {
            expected: 1,
            found: 0,
        }
    )));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "failed indirect static admission must not mutate ordinary argument ownership"
    );
}

#[test]
fn indirect_shared_identity_result_preserves_the_caller_origin_contract() {
    let hir = build(
        "fn identity(reference: &I64) -> &I64 { return reference; } \
         fn use(f: fn(&I64) -> &I64, reference: &I64) -> &I64 { return f(reference); }",
    )
    .expect("indirect shared-identity return is valid");
    let use_fn = function(&hir, "use");
    assert_eq!(
        use_fn.safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 1 }
    );
    let returned = use_fn
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("use returns indirect result");
    assert!(matches!(
        returned.kind,
        ValueKind::Call {
            target: CallTarget::Indirect { .. },
            ..
        }
    ));
}

#[test]
fn indirect_shared_direct_child_result_preserves_parent_ancestry() {
    let hir = build(
        "fn child(reference: &mut I64) -> &I64 { return &*reference; } \
         fn use(f: fn(&mut I64) -> &I64, reference: &mut I64) -> &I64 { return f(reference); }",
    )
    .expect("indirect shared direct-child return is valid");
    let use_fn = function(&hir, "use");
    assert_eq!(
        use_fn.safe_reference_result_contract,
        SafeReferenceResultContract::SharedDirectChild { origin: 1 }
    );
    let returned = use_fn
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("use returns indirect result");
    assert!(matches!(
        returned.kind,
        ValueKind::Call {
            target: CallTarget::Indirect { .. },
            ..
        }
    ));
}

#[test]
fn recursive_and_mutual_indirect_calls_validate_without_target_set_analysis() {
    let recursive = build(
        "fn recurse(value: I64) -> I64 { \
             let f: fn(I64) -> I64 = recurse; \
             return f(value); \
         }",
    )
    .expect("self-indirect recursion is statically valid without target expansion");
    let returned = function(&recursive, "recurse")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("recursive function returns one indirect call");
    assert!(matches!(
        returned.kind,
        ValueKind::Call {
            target: CallTarget::Indirect { .. },
            ..
        }
    ));

    let mutual = build(
        "fn left(value: I64) -> I64 { \
             let f: fn(I64) -> I64 = right; \
             return f(value); \
         } \
         fn right(value: I64) -> I64 { \
             let f: fn(I64) -> I64 = left; \
             return f(value); \
         }",
    )
    .expect("mutual indirect recursion is statically valid without target expansion");
    for name in ["left", "right"] {
        let returned = function(&mutual, name)
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("mutually recursive function returns one indirect call");
        assert!(matches!(
            returned.kind,
            ValueKind::Call {
                target: CallTarget::Indirect { .. },
                ..
            }
        ));
    }
}
