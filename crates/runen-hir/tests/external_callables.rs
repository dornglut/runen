use runen_hir::{
    DiagnosticKind, FunctionExecution, IntrinsicType, ModuleId, SourceUnit, Statement, Type,
    TypedCompilation, ValueKind, build_typed_hir,
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

#[test]
fn external_declarations_share_function_identity_namespace_but_retain_bodyless_execution_origin() {
    let hir = build(
        "external fn sink(I64); \
         export external fn transform(Bool, F32) -> U64; \
         fn caller() -> U64 { sink(7); return transform(true, 1.0); }",
    )
    .expect("external scalar declarations and direct calls are valid");
    assert_eq!(hir.functions.len(), 3);
    let sink = &hir.functions[0];
    let transform = &hir.functions[1];
    let caller = &hir.functions[2];

    assert_eq!(sink.name, "sink");
    assert_eq!(sink.type_parameters, []);
    assert_eq!(sink.result, None);
    assert_eq!(
        sink.parameter_types(),
        vec![Type::Intrinsic(IntrinsicType::I64)]
    );
    assert!(sink.is_external());
    let FunctionExecution::External { parameters } = &transform.execution else {
        panic!("transform must retain external execution origin");
    };
    assert_eq!(parameters, &[IntrinsicType::Bool, IntrinsicType::F32]);
    assert_eq!(transform.result, Some(Type::Intrinsic(IntrinsicType::U64)));

    let body = caller.runen_body().expect("caller is a Runen body");
    let Statement::Call { target, .. } = &body.statements[0] else {
        panic!("first caller statement must be direct call");
    };
    let runen_hir::CallTarget::Direct {
        function,
        type_arguments,
    } = target
    else {
        panic!("external sink remains a direct function target");
    };
    assert_eq!(*function, sink.id);
    assert!(type_arguments.is_empty());
    let returned = body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("caller returns transform result");
    let ValueKind::Call { target, .. } = &returned.kind else {
        panic!("return value is external call");
    };
    let runen_hir::CallTarget::Direct { function, .. } = target else {
        panic!("transform remains a direct function target");
    };
    assert_eq!(*function, transform.id);
}

#[test]
fn external_function_cannot_form_a_function_value() {
    let errors = build("external fn sink(I64); fn bad() { let f: fn(I64) = sink; f(1); }")
        .expect_err("external functions are direct-call-only");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::ExternalFunctionValue),
        "{errors:?}"
    );
}

#[test]
fn external_and_runen_functions_share_one_module_binding_category() {
    let errors = build("external fn same(I64); fn same(value: I64) {}")
        .expect_err("same-category duplicate binding must be rejected");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::DuplicateModuleBinding),
        "{errors:?}"
    );
}

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

    let dependency =
        parse("export external fn exposed(I64) -> I64; external fn hidden(I64) -> I64;");
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
