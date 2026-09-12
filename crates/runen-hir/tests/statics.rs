use runen_hir::{
    DiagnosticKind, ImportTarget, IntrinsicType, LiteralValue, ModuleId, SourceUnit, Statement,
    Type, TypedCompilation, Value, ValueKind, build_typed_hir,
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

fn build_qualified(
    dependency: &str,
    caller: &str,
) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let dependency = parse(dependency);
    let caller = parse(caller);
    assert!(dependency.errors().is_empty(), "{:?}", dependency.errors());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid import alias")];
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

fn returned_value<'a>(hir: &'a TypedCompilation, name: &str) -> &'a Value {
    function(hir, name)
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .unwrap_or_else(|| panic!("function {name} has no returned value"))
}

fn has_kind(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

#[test]
fn statics_retain_exact_scalar_declaration_identity_and_reads() {
    let hir = build("static VALUE: I64 = -42; fn read() -> I64 { return VALUE; }")
        .expect("execution static is valid");
    assert_eq!(hir.statics.len(), 1);
    let declaration = &hir.statics[0];
    assert_eq!(declaration.name, "VALUE");
    assert_eq!(declaration.ty, IntrinsicType::I64);
    assert_eq!(declaration.initializer, LiteralValue::I64(-42));
    assert_eq!(hir.modules[0].statics, vec![declaration.id]);

    let returned = returned_value(&hir, "read");
    assert_eq!(returned.ty, Type::Intrinsic(IntrinsicType::I64));
    assert_eq!(returned.kind, ValueKind::StaticRead(declaration.id));
}

#[test]
fn static_reads_are_non_consuming_and_anchor_existing_comparison_selection() {
    let hir = build(
        "static LIMIT: I64 = 7; fn ok() -> Bool { let first: I64 = LIMIT; let second: I64 = LIMIT; return LIMIT == 7; }",
    )
    .expect("repeated static reads and comparison anchoring are valid");
    let ok = function(&hir, "ok");
    for statement in &ok.body.statements {
        let Statement::Local { initializer, .. } = statement else {
            panic!("expected only local statements");
        };
        assert!(matches!(initializer.kind, ValueKind::StaticRead(_)));
    }
}

#[test]
fn active_local_selection_is_final_over_same_module_static() {
    let errors = build("static VALUE: I64 = 1; fn f(VALUE: Bool) -> I64 { return VALUE; }")
        .expect_err("wrong-typed local remains selected over a same-name static");
    assert!(has_kind(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I64),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
}

#[test]
fn qualified_exported_static_bypasses_local_lookup_and_private_static_is_inaccessible() {
    let hir = build_qualified(
        "export static ANSWER: I64 = 42;",
        "import dep; fn f(ANSWER: Bool) -> I64 { return dep::ANSWER; }",
    )
    .expect("qualified exported static bypasses local lookup");
    let declaration = &hir.statics[0];
    assert_eq!(
        returned_value(&hir, "f").kind,
        ValueKind::StaticRead(declaration.id)
    );

    let errors = build_qualified(
        "static PRIVATE: I64 = 1;",
        "import dep; fn f() -> I64 { return dep::PRIVATE; }",
    )
    .expect_err("module-private static is inaccessible cross-module");
    assert!(has_kind(&errors, DiagnosticKind::InaccessibleBinding));
}

#[test]
fn statics_share_the_module_namespace_and_wrong_categories_remain_final() {
    for source in [
        "static duplicate: I64 = 1; static duplicate: I64 = 2; fn f() {}",
        "static duplicate: I64 = 1; const duplicate: I64 = 2; fn f() {}",
        "static duplicate: I64 = 1; record duplicate {} fn f() {}",
        "static duplicate: I64 = 1; fn duplicate() {}",
        "static duplicate: I64 = 1; trait duplicate; fn f() {}",
    ] {
        let errors = build(source).expect_err("duplicate module key must be rejected");
        assert!(has_kind(&errors, DiagnosticKind::DuplicateModuleBinding));
    }

    let errors = build_qualified(
        "export record VALUE {}",
        "import dep; fn f() -> I64 { return dep::VALUE; }",
    )
    .expect_err("qualified scalar value lookup is category-final");
    assert!(has_kind(&errors, DiagnosticKind::ExpectedValueBinding));
}

#[test]
fn only_complete_shared_static_roots_are_semantically_admitted() {
    let hir =
        build("static VALUE: I64 = 7; fn read() -> I64 { let root: &I64 = &VALUE; return *root; }")
            .expect("complete Shared static root is valid");
    let read = function(&hir, "read");
    let Statement::Local { initializer, .. } = &read.body.statements[0] else {
        panic!("first statement must bind the static root");
    };
    assert!(matches!(
        initializer.kind,
        ValueKind::StaticReferenceRoot(_)
    ));

    let replacement = build("static VALUE: I64 = 7; fn bad() { let root: &mut I64 = &mut VALUE; }")
        .expect_err("static replacement roots are excluded");
    assert!(has_kind(
        &replacement,
        DiagnosticKind::InvalidReplacementReferenceTarget
    ));

    let raw = build("static VALUE: I64 = 7; fn bad() { let p: raw I64 = raw &VALUE; }")
        .expect_err("static raw roots are excluded");
    assert!(has_kind(&raw, DiagnosticKind::ExpectedValueBinding));

    let assignment = build("static VALUE: I64 = 7; fn bad() { VALUE = 8; }")
        .expect_err("static assignment targets are excluded");
    assert!(has_kind(&assignment, DiagnosticKind::ExpectedValueBinding));

    let field = build("static VALUE: I64 = 7; fn bad() { let root: &I64 = &VALUE.field; }")
        .expect_err("static field roots are excluded");
    assert!(has_kind(
        &field,
        DiagnosticKind::ExpectedRecordForFieldAccess
    ));
}

#[test]
fn static_root_provenance_cannot_define_result_origin_but_survives_shared_identity_calls() {
    let direct = build("static VALUE: I64 = 7; fn bad() -> &I64 { return &VALUE; }")
        .expect_err("fresh static root is not an admitted callable result origin");
    assert!(has_kind(
        &direct,
        DiagnosticKind::MissingSharedReferenceResultOrigin
    ));

    build(
        "static VALUE: I64 = 7; \
         fn identity(value: &I64) -> &I64 { return value; } \
         fn ok() -> I64 { \
             let root: &I64 = &VALUE; \
             let roundtrip: &I64 = identity(root); \
             return *roundtrip; \
         }",
    )
    .expect("SharedIdentity call preserves caller-side static provenance");
}

#[test]
fn generic_and_closure_bodies_resolve_statics_without_capture_identity() {
    let hir = build(
        "static VALUE: I64 = 9; \
         fn generic[T](ignored: T) -> I64 { return VALUE; } \
         fn make(dummy: I64) -> I64 { let c = fn[dummy]() -> I64 { return VALUE; }; return c(); }",
    )
    .expect("generic and closure bodies may read execution statics");
    assert!(matches!(
        returned_value(&hir, "generic").kind,
        ValueKind::StaticRead(_)
    ));
    assert_eq!(hir.closures.len(), 1);
    assert_eq!(hir.closures[0].captures.len(), 1);
    assert_eq!(hir.closures[0].captures[0].name, "dummy");
    assert!(matches!(
        hir.closures[0]
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("closure returns static")
            .kind,
        ValueKind::StaticRead(_)
    ));
}
