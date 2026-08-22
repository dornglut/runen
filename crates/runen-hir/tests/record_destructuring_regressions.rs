use runen_hir::{
    DiagnosticKind, IntrinsicType, ModuleId, OwnedUse, RecordPatternScrutinee, SourceUnit,
    Statement, Type, ValueKind, build_typed_hir,
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

fn has_kind(diagnostics: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic.kind == kind)
}

#[test]
fn one_level_pattern_binding_identities_remain_distinct() {
    let hir = build(
        "record Token {} record Mixed { first: I8, token: Token, last: U8 } \
         fn f(root: Mixed) { let Mixed { last: z, token: moved, first: a } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } = &function(&hir, "f").body.statements[0]
    else {
        panic!("expected record destructuring statement");
    };
    assert_eq!(bindings.len(), 3);
    assert_ne!(bindings[0].binding, bindings[1].binding);
    assert_ne!(bindings[1].binding, bindings[2].binding);
    assert_ne!(bindings[0].binding, bindings[2].binding);
}

#[test]
fn one_level_producer_result_contract_remains_unchanged() {
    let mismatch = errors(
        "record A { value: I8 } record B { value: I8 } \
         fn make() -> B { return B { value: 1 }; } \
         fn f() { let A { value: extracted } = make(); }",
    );
    assert!(mismatch.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Record(_),
            found: Type::Record(_),
        }
    )));

    let no_result = errors(
        "record A { value: I8 } fn make() {} \
         fn f() { let A { value: extracted } = make(); }",
    );
    assert!(has_kind(
        &no_result,
        DiagnosticKind::NoResultCallUsedAsValue
    ));
}

#[test]
fn one_level_partial_consumption_still_blocks_whole_root_use() {
    let diagnostics = errors(
        "record Token {} record Holder { token: Token, count: I8 } \
         fn take(value: Holder) {} \
         fn bad(root: Holder) { \
             let Holder { token: moved, count: copied } = root; \
             take(root); \
         }",
    );
    assert!(has_kind(&diagnostics, DiagnosticKind::UnavailableBinding));
}

#[test]
fn immutable_root_can_transfer_one_level_field_ownership_but_remains_nonassignable() {
    let diagnostics = errors(
        "record Token {} record Holder { token: Token, count: I8 } \
         fn f(root: Holder) { \
             let Holder { token: moved, count: copied } = root; \
             root = Holder { token: Token {}, count: 2 }; \
         }",
    );
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::ImmutableAssignmentTarget
    ));
    assert!(!has_kind(
        &diagnostics,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn all_nonduplicable_one_level_fields_remain_independent_consumes() {
    let hir = build(
        "record Left {} record Right {} record Pair { left: Left, right: Right } \
         fn f(root: Pair) { let Pair { right: moved_right, left: moved_left } = root; }",
    );
    let f = function(&hir, "f");
    let Statement::RecordDestructure {
        scrutinee,
        bindings,
        ..
    } = &f.body.statements[0]
    else {
        panic!("expected record destructuring statement");
    };
    assert_eq!(
        scrutinee,
        &RecordPatternScrutinee::DirectRoot(f.parameters[0].binding)
    );
    assert_eq!(
        bindings
            .iter()
            .map(|binding| binding.fields.clone())
            .collect::<Vec<_>>(),
        [vec![1], vec![0]]
    );
    assert!(
        bindings
            .iter()
            .all(|binding| binding.ownership == OwnedUse::Consume)
    );
}

#[test]
fn one_level_pattern_bindings_obey_child_scope_lookup() {
    let hir = build(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { \
             { \
                 let Pair { left: extracted, right: other } = root; \
                 { let copied: I8 = extracted; } \
             } \
         }",
    );
    let f = function(&hir, "f");
    let Statement::Block(outer) = &f.body.statements[0] else {
        panic!("expected outer child block");
    };
    let Statement::RecordDestructure { bindings, .. } = &outer.statements[0] else {
        panic!("expected pattern statement");
    };
    let Statement::Block(inner) = &outer.statements[1] else {
        panic!("expected nested child block");
    };
    let Statement::Local { initializer, .. } = &inner.statements[0] else {
        panic!("expected nested local");
    };
    assert!(matches!(
        initializer.kind,
        ValueKind::BindingUse { binding, .. } if binding == bindings[0].binding
    ));

    let diagnostics = errors(
        "record Pair { left: I8, right: U8 } \
         fn bad(root: Pair) { \
             { let Pair { left: extracted, right: other } = root; } \
             let copied: I8 = extracted; \
         }",
    );
    assert!(has_kind(&diagnostics, DiagnosticKind::UnresolvedName));
}

#[test]
fn one_level_disjoint_field_access_remains_available_after_partial_pattern() {
    let hir = build(
        "record Token {} record Holder { token: Token, count: I8 } \
         fn f(root: Holder) -> I8 { \
             let Holder { token: moved, count: copied } = root; \
             return root.count; \
         }",
    );
    let returned = function(&hir, "f")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("returned value");
    assert!(matches!(
        &returned.kind,
        ValueKind::FieldValueUse {
            fields: path,
            ownership: OwnedUse::Duplicate,
            ..
        } if path == &[1]
    ));
}

#[test]
fn one_level_leaf_types_remain_exact() {
    let hir = build(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: left, right: right } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } = &function(&hir, "f").body.statements[0]
    else {
        panic!("expected record destructuring statement");
    };
    assert_eq!(bindings[0].ty, Type::Intrinsic(IntrinsicType::I8));
    assert_eq!(bindings[1].ty, Type::Intrinsic(IntrinsicType::U8));
}
