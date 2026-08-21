use runen_hir::{
    DiagnosticKind, ImportTarget, IntrinsicType, ModuleId, OwnedUse, SourceUnit, Statement, Type,
    Value, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> runen_hir::TypedCompilation {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR")
}

fn errors(source: &str) -> Vec<runen_hir::Diagnostic> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect_err("test source must be rejected")
}

fn function<'a>(hir: &'a runen_hir::TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .expect("named test function")
}

fn returned_value(function: &runen_hir::Function) -> &Value {
    function
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("result-bearing test function has returned value")
}

fn has_kind(diagnostics: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.kind == kind)
}

#[test]
fn resolves_one_level_and_nested_paths_to_declaration_field_indices() {
    let hir = build(
        "record Inner { pad: U8, value: I8 } \
         record Outer { first: I8, inner: Inner } \
         fn one(root: Outer) -> I8 { return root.first; } \
         fn nested(root: Outer) -> I8 { return root.inner.value; }",
    );

    let one = function(&hir, "one");
    let ValueKind::FieldValueUse { binding, fields } = &returned_value(one).kind else {
        panic!("expected field-value use");
    };
    assert_eq!(*binding, one.parameters[0].binding);
    assert_eq!(fields, &[0]);
    assert_eq!(returned_value(one).ty, Type::Intrinsic(IntrinsicType::I8));

    let nested = function(&hir, "nested");
    let ValueKind::FieldValueUse { binding, fields } = &returned_value(nested).kind else {
        panic!("expected nested field-value use");
    };
    assert_eq!(*binding, nested.parameters[0].binding);
    assert_eq!(fields, &[1, 1]);
}

#[test]
fn field_lookup_is_nominal_and_not_declaration_order_priority() {
    let hir = build(
        "record A { common: I8, other: U8 } \
         record B { other: U8, common: I8 } \
         fn a(root: A) -> I8 { return root.common; } \
         fn b(root: B) -> I8 { return root.common; }",
    );
    let ValueKind::FieldValueUse { fields: a, .. } = &returned_value(function(&hir, "a")).kind
    else {
        panic!("expected A field use");
    };
    let ValueKind::FieldValueUse { fields: b, .. } = &returned_value(function(&hir, "b")).kind
    else {
        panic!("expected B field use");
    };
    assert_eq!(a, &[0]);
    assert_eq!(b, &[1]);
}

#[test]
fn root_lookup_uses_active_binding_precedence_without_category_bypass() {
    let hir = build(
        "record root { value: U8 } record Box { value: I8 } \
         fn f(root: Box) -> I8 { return root.value; }",
    );
    let f = function(&hir, "f");
    let ValueKind::FieldValueUse { binding, fields } = &returned_value(f).kind else {
        panic!("expected field use rooted in parameter");
    };
    assert_eq!(*binding, f.parameters[0].binding);
    assert_eq!(fields, &[0]);
    assert_eq!(returned_value(f).ty, Type::Intrinsic(IntrinsicType::I8));

    let wrong_category = errors("record root { value: I8 } fn g() -> I8 { return root.value; }");
    assert!(has_kind(
        &wrong_category,
        DiagnosticKind::ExpectedValueBinding
    ));
}

#[test]
fn field_access_requires_available_root() {
    let unavailable = errors(
        "record Box { value: I8 } \
         fn take(value: Box) {} \
         fn f(root: Box) -> I8 { take(root); return root.value; }",
    );
    assert!(has_kind(&unavailable, DiagnosticKind::UnavailableBinding));
}

#[test]
fn rejects_non_record_unknown_and_nonduplicable_final_fields() {
    let non_record = errors("fn f(root: I8) -> I8 { return root.value; }");
    assert!(has_kind(
        &non_record,
        DiagnosticKind::ExpectedRecordForFieldAccess
    ));

    let unknown = errors(
        "record Box { value: I8 } record Other { missing: I8 } \
         fn f(root: Box) -> I8 { return root.missing; }",
    );
    assert!(has_kind(&unknown, DiagnosticKind::UnknownRecordField));

    let nonduplicable = errors(
        "record Inner { value: I8 } record Outer { inner: Inner } \
         fn take(value: Outer) {} \
         fn f(root: Outer) { let bad: Inner = root.inner; take(root); }",
    );
    assert!(has_kind(
        &nonduplicable,
        DiagnosticKind::NonDuplicableFieldValue
    ));
    assert!(!has_kind(
        &nonduplicable,
        DiagnosticKind::UnavailableBinding
    ));
}

#[test]
fn nonduplicable_intermediate_records_allow_deeper_duplicable_field() {
    let hir = build(
        "record Inner { value: I8 } record Outer { inner: Inner } \
         fn f(root: Outer) -> I8 { return root.inner.value; }",
    );
    assert!(matches!(
        returned_value(function(&hir, "f")).kind,
        ValueKind::FieldValueUse { .. }
    ));
}

#[test]
fn successful_and_repeated_access_leave_root_available_for_later_whole_binding_use() {
    let hir = build(
        "record Box { value: I8 } \
         fn f(root: Box) -> Box { \
             let first: I8 = root.value; \
             let second: I8 = root.value; \
             return root; \
         }",
    );
    let f = function(&hir, "f");
    assert_eq!(f.body.statements.len(), 2);
    for statement in &f.body.statements {
        let Statement::Local { initializer, .. } = statement else {
            panic!("expected local");
        };
        assert!(matches!(initializer.kind, ValueKind::FieldValueUse { .. }));
    }
    assert!(matches!(
        returned_value(f).kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Consume,
            ..
        }
    ));
}

#[test]
fn direct_access_to_foreign_record_fields_is_rejected() {
    let foreign = parse("export record Foreign { value: I8 }");
    let local = parse("import ext; fn f(root: ext::Foreign) -> I8 { return root.value; }");
    let ext = ImportTarget::new("ext", ModuleId::new(1)).expect("valid alias");
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &foreign, &[]),
        SourceUnit::new(ModuleId::new(2), &local, &[ext]),
    ])
    .expect_err("foreign direct field access must be rejected");

    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));
}

#[test]
fn nested_path_may_reach_foreign_record_but_cannot_select_inside_it() {
    let foreign = parse("export record Foreign { value: I8 }");
    let local = parse(
        "import ext; record Local { foreign: ext::Foreign } \
         fn f(root: Local) -> I8 { return root.foreign.value; }",
    );
    let ext = ImportTarget::new("ext", ModuleId::new(1)).expect("valid alias");
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &foreign, &[]),
        SourceUnit::new(ModuleId::new(2), &local, &[ext]),
    ])
    .expect_err("selector into foreign record must be rejected");

    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));
}

#[test]
fn field_values_require_exact_consumer_types() {
    let diagnostics = errors(
        "record Box { value: I8 } record Holder { value: U8 } \
         fn sink(value: U8) {} \
         fn bad_local(root: Box) { let value: U8 = root.value; } \
         fn bad_assignment(root: Box) { let mut target: U8 = 0; target = root.value; } \
         fn bad_call(root: Box) { sink(root.value); } \
         fn bad_return(root: Box) -> U8 { return root.value; } \
         fn bad_constructor(root: Box) -> Holder { return Holder { value: root.value }; }",
    );
    assert!(
        diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
            .count()
            >= 5
    );
}
