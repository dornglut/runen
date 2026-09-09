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
fn private_constants_materialize_once_and_uses_erase_to_exact_literals() {
    let hir = build(
        r#"
const ENABLED: Bool = true;
const COUNT: I64 = -42;
const BYTE_MAX: U8 = 255;
const RATIO: F32 = -0.5;
fn f() -> I64 {
    let first: I64 = COUNT;
    let second: I64 = COUNT;
    if ENABLED {}
    return second;
}
"#,
    )
    .expect("intrinsic scalar constants are valid");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("first statement must be the first local");
    };
    assert_eq!(initializer.ty, Type::Intrinsic(IntrinsicType::I64));
    assert_eq!(initializer.kind, ValueKind::Literal(LiteralValue::I64(-42)));

    let Statement::Local { initializer, .. } = &f.body.statements[1] else {
        panic!("second statement must be the second local");
    };
    assert_eq!(initializer.kind, ValueKind::Literal(LiteralValue::I64(-42)));

    let Statement::If { condition, .. } = &f.body.statements[2] else {
        panic!("third statement must be the conditional");
    };
    assert_eq!(condition.kind, ValueKind::Literal(LiteralValue::Bool(true)));
}

#[test]
fn local_binding_precedence_is_final_over_same_module_constant() {
    let hir = build("const VALUE: I64 = 1; fn f(VALUE: I64) -> I64 { return VALUE; }")
        .expect("parameter lookup must retain precedence over module constants");
    let returned = returned_value(&hir, "f");
    assert_eq!(returned.ty, Type::Intrinsic(IntrinsicType::I64));
    assert!(
        matches!(returned.kind, ValueKind::BindingUse { .. }),
        "local-first lookup must not erase the selected parameter to a constant literal"
    );
}

#[test]
fn local_selection_remains_final_when_wrong_typed_or_unavailable() {
    let wrong_type = build("const VALUE: I64 = 1; fn f(VALUE: Bool) -> I64 { return VALUE; }")
        .expect_err("wrong-typed local must remain selected rather than falling back to constant");
    assert!(has_kind(
        &wrong_type,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I64),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));

    let unavailable = build(
        "const VALUE: I64 = 1; record Ticket {} fn sink(value: Ticket) {} fn f(VALUE: Ticket) -> I64 { sink(VALUE); return VALUE; }",
    )
    .expect_err("unavailable local must remain selected rather than falling back to constant");
    assert!(has_kind(&unavailable, DiagnosticKind::UnavailableBinding));
}

#[test]
fn generic_type_parameter_name_does_not_shadow_constant_in_value_position() {
    let hir = build("const T: I64 = 5; fn generic[T](value: T) -> I64 { return T; }")
        .expect("generic type-parameter lookup remains type-position-only");
    assert_eq!(
        returned_value(&hir, "generic").kind,
        ValueKind::Literal(LiteralValue::I64(5))
    );
}

#[test]
fn contextual_const_spelling_remains_an_ordinary_constant_name() {
    let hir = build("const const: I64 = 3; fn f() -> I64 { return const; }")
        .expect("contextual const spelling remains a user identifier outside introducer position");
    assert_eq!(
        returned_value(&hir, "f").kind,
        ValueKind::Literal(LiteralValue::I64(3))
    );
}

#[test]
fn constant_declaration_order_does_not_affect_same_module_lookup() {
    let hir = build("fn f() -> I64 { return LATE; } const LATE: I64 = 9;")
        .expect("constant declarations are order-independent module declarations");
    assert_eq!(
        returned_value(&hir, "f").kind,
        ValueKind::Literal(LiteralValue::I64(9))
    );
}

#[test]
fn constants_are_ordinary_values_in_existing_receiving_contexts() {
    let hir = build(
        r#"
const ONE: I64 = 1;
const ENABLED: Bool = true;
record Pair { left: I64, right: I64 }
fn take(value: I64) {}
fn generic[T](value: T) -> I64 { return ONE; }
fn f(seed: I64) -> I64 {
    let sum: I64 = ONE + 2;
    take(ONE);
    let pair: Pair = Pair { left: ONE, right: ONE };
    let mut assigned: I64 = seed;
    assigned = ONE;
    if ENABLED {}
    return ONE;
}
"#,
    )
    .expect("constant production composes with represented value contexts and generic bodies");

    let f = function(&hir, "f");
    let Statement::Assignment { value, .. } = &f.body.statements[4] else {
        panic!("fifth statement must be the whole-binding assignment");
    };
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::I64));
    assert_eq!(value.kind, ValueKind::Literal(LiteralValue::I64(1)));
}

#[test]
fn constant_exact_type_anchors_existing_comparison_selection() {
    build("const LIMIT: I64 = 7; fn ok() -> Bool { return LIMIT == 7; }")
        .expect("constant reference supplies exact comparison operand type");

    let errors = build("fn bad() -> Bool { return 1 == 2; }")
        .expect_err("two contextual literals remain unanchored");
    assert!(has_kind(
        &errors,
        DiagnosticKind::EqualityOperandsUnanchored
    ));
}

#[test]
fn constant_type_mismatch_is_reported_in_receiving_context() {
    let errors = build("const ONE: I64 = 1; fn bad() { if ONE {} }")
        .expect_err("numeric constant is not a Boolean condition");
    assert!(has_kind(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::Bool),
            found: Type::Intrinsic(IntrinsicType::I64),
        }
    ));
}

#[test]
fn constant_initializer_uses_existing_exact_literal_materialization() {
    let wrong_family = build("const BAD: Bool = 1; fn f() {}")
        .expect_err("integer literal cannot initialize Bool constant");
    assert!(has_kind(
        &wrong_family,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));

    let out_of_range = build("const BAD: I8 = 128; fn f() {}")
        .expect_err("out-of-range constant initializer must be rejected");
    assert!(has_kind(
        &out_of_range,
        DiagnosticKind::IntegerLiteralOutOfRange {
            required: Type::Intrinsic(IntrinsicType::I8),
        }
    ));
}

#[test]
fn constants_share_the_existing_module_namespace() {
    for source in [
        "const duplicate: I64 = 1; const duplicate: I64 = 2; fn f() {}",
        "const duplicate: I64 = 1; record duplicate {} fn f() {}",
        "const duplicate: I64 = 1; fn duplicate() {}",
        "const duplicate: I64 = 1; trait duplicate; fn f() {}",
    ] {
        let errors = build(source).expect_err("duplicate module key must be rejected");
        assert!(
            has_kind(&errors, DiagnosticKind::DuplicateModuleBinding),
            "missing duplicate-module diagnostic for {source}"
        );
    }
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

#[test]
fn exported_qualified_constant_bypasses_local_lookup_and_erases_to_literal() {
    let hir = build_qualified(
        "export const ANSWER: I64 = 42;",
        "import dep; fn f(ANSWER: I64) -> I64 { return dep::ANSWER; }",
    )
    .expect("qualified exported constant is visible through the explicit import alias");

    let returned = returned_value(&hir, "f");
    assert_eq!(returned.ty, Type::Intrinsic(IntrinsicType::I64));
    assert_eq!(returned.kind, ValueKind::Literal(LiteralValue::I64(42)));
}

#[test]
fn qualified_constant_exact_type_anchors_existing_comparison_selection() {
    build_qualified(
        "export const LIMIT: I64 = 7;",
        "import dep; fn ok() -> Bool { return dep::LIMIT == 7; }",
    )
    .expect("qualified constant reference supplies exact comparison operand type");
}

#[test]
fn qualified_constant_lookup_enforces_accessibility_and_category() {
    let private = build_qualified(
        "const PRIVATE: I64 = 1;",
        "import dep; fn f() -> I64 { return dep::PRIVATE; }",
    )
    .expect_err("module-private constant is inaccessible cross-module");
    assert!(has_kind(&private, DiagnosticKind::InaccessibleBinding));

    let wrong_category = build_qualified(
        "export fn helper() -> I64 { return 1; }",
        "import dep; fn f() -> I64 { return dep::helper; }",
    )
    .expect_err("qualified value lookup requires constant category");
    assert!(has_kind(
        &wrong_category,
        DiagnosticKind::ExpectedValueBinding
    ));
}

#[test]
fn unresolved_unqualified_alias_and_member_names_remain_unresolved() {
    let unqualified = build("fn f() -> I64 { return MISSING; }")
        .expect_err("missing same-module name must remain unresolved");
    assert!(has_kind(&unqualified, DiagnosticKind::UnresolvedName));

    let alias = build_qualified(
        "export const ANSWER: I64 = 42;",
        "fn f() -> I64 { return missing::ANSWER; }",
    )
    .expect_err("missing import alias must remain unresolved");
    assert!(has_kind(&alias, DiagnosticKind::UnresolvedName));

    let member = build_qualified(
        "export const ANSWER: I64 = 42;",
        "import dep; fn f() -> I64 { return dep::MISSING; }",
    )
    .expect_err("missing target member must remain unresolved");
    assert!(has_kind(&member, DiagnosticKind::UnresolvedName));
}

#[test]
fn same_module_wrong_category_remains_final_without_fallback() {
    let errors = build("fn helper() -> I64 { return 1; } fn f() -> I64 { return helper; }")
        .expect_err("ordinary value position does not reinterpret a function declaration");
    assert!(has_kind(&errors, DiagnosticKind::ExpectedValueBinding));
}

#[test]
fn constants_do_not_become_binding_reference_or_raw_pointer_targets() {
    for source in [
        "const VALUE: I64 = 1; fn f() { VALUE = 2; }",
        "const VALUE: I64 = 1; fn f() { let r: &I64 = &VALUE; }",
        "const VALUE: I64 = 1; fn f() { let p: raw I64 = raw &VALUE; }",
    ] {
        let errors =
            build(source).expect_err("constant name must not acquire local binding identity");
        assert!(
            has_kind(&errors, DiagnosticKind::ExpectedValueBinding),
            "missing binding-only rejection for {source}"
        );
    }
}

#[test]
fn constants_do_not_become_field_or_pattern_targets() {
    let field = build("const VALUE: I64 = 1; fn f() -> I64 { return VALUE.field; }")
        .expect_err("constant name must not become a field receiver root");
    assert!(has_kind(&field, DiagnosticKind::ExpectedValueBinding));

    let pattern = build("const VALUE: I64 = 1; fn f(seed: I64) { let VALUE {} = seed; }")
        .expect_err("constant name must not become a record-pattern head");
    assert!(has_kind(&pattern, DiagnosticKind::ExpectedRecordType));
}

#[test]
fn numeric_contract_selection_does_not_accept_constant_as_governed_root() {
    let errors = build("const VALUE: F32 = 1.0; fn f() -> F32 { return @fast(VALUE); }")
        .expect_err("numeric contract selection is not a general constant wrapper");
    assert!(has_kind(
        &errors,
        DiagnosticKind::NumericContractSelectionRequiresGovernedFloatingOperation {
            required: Type::Intrinsic(IntrinsicType::F32),
        }
    ));
}
