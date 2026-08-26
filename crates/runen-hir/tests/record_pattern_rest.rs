use runen_hir::{
    DiagnosticKind, ImportTarget, ModuleId, OwnedUse, RecordPatternScrutinee,
    RecordPatternTransientCleanup, SourceUnit, Statement, ValueKind, build_typed_hir,
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
fn rest_authorizes_only_missing_fields_while_no_rest_remains_exhaustive() {
    let hir = build(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: value, .. } = root; }",
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
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].fields, vec![0]);
    assert_eq!(bindings[0].ownership, OwnedUse::Duplicate);

    let diagnostics = errors(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: value } = root; }",
    );
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::MissingRecordPatternField
    ));
}

#[test]
fn direct_root_rest_preserves_omitted_ownership_and_rest_only_is_noop() {
    let hir = build(
        "record Token {} \
         record Pair { selected: Token, omitted: Token } \
         fn take_token(value: Token) {} \
         fn take_pair(value: Pair) {} \
         fn selected(root: Pair) { \
             let Pair { selected: moved, .. } = root; \
             take_token(root.omitted); \
         } \
         fn rest_only(root: Pair) { \
             let Pair { .. } = root; \
             take_pair(root); \
         }",
    );

    let selected = function(&hir, "selected");
    let Statement::RecordDestructure {
        scrutinee,
        bindings,
        ..
    } = &selected.body.statements[0]
    else {
        panic!("expected selected-field pattern");
    };
    assert_eq!(
        scrutinee,
        &RecordPatternScrutinee::DirectRoot(selected.parameters[0].binding)
    );
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].fields, vec![0]);
    assert_eq!(bindings[0].ownership, OwnedUse::Consume);

    let rest_only = function(&hir, "rest_only");
    let Statement::RecordDestructure {
        scrutinee,
        bindings,
        ..
    } = &rest_only.body.statements[0]
    else {
        panic!("expected rest-only pattern");
    };
    assert_eq!(
        scrutinee,
        &RecordPatternScrutinee::DirectRoot(rest_only.parameters[0].binding)
    );
    assert!(bindings.is_empty());
    assert!(matches!(
        rest_only.body.statements[1],
        Statement::Call { .. }
    ));
}

#[test]
fn nested_direct_root_rest_only_touches_explicit_leaf_paths() {
    let hir = build(
        "record Token {} \
         record Inner { moved: Token, kept: Token } \
         record Outer { inner: Inner, tail: Token } \
         fn take(value: Token) {} \
         fn f(root: Outer) { \
             let Outer { inner: Inner { moved: item, .. }, .. } = root; \
             take(root.inner.kept); \
             take(root.tail); \
         }",
    );
    let f = function(&hir, "f");
    let Statement::RecordDestructure { bindings, .. } = &f.body.statements[0] else {
        panic!("expected nested record destructuring");
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].fields, vec![0, 0]);
    assert_eq!(bindings[0].ownership, OwnedUse::Consume);
}

#[test]
fn producer_rest_uses_existing_remaining_frontier_and_rest_only_keeps_root() {
    let hir = build(
        "record Token {} \
         record Pair { copied: I8, moved: Token, omitted: Token } \
         fn partial() { \
             let Pair { copied: copy, moved: moved, .. } = \
                 Pair { copied: 1, moved: Token {}, omitted: Token {} }; \
         } \
         fn rest_only() { \
             let Pair { .. } = Pair { copied: 2, moved: Token {}, omitted: Token {} }; \
         }",
    );

    let Statement::RecordDestructure {
        scrutinee,
        bindings,
        ..
    } = &function(&hir, "partial").body.statements[0]
    else {
        panic!("expected producer-backed partial pattern");
    };
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].fields, vec![0]);
    assert_eq!(bindings[0].ownership, OwnedUse::Duplicate);
    assert_eq!(bindings[1].fields, vec![1]);
    assert_eq!(bindings[1].ownership, OwnedUse::Consume);
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            value: runen_hir::Value {
                kind: ValueKind::RecordConstruction { .. },
                ..
            },
            cleanup: RecordPatternTransientCleanup { paths },
        } if paths == &[vec![2], vec![0]]
    ));

    let Statement::RecordDestructure {
        scrutinee,
        bindings,
        ..
    } = &function(&hir, "rest_only").body.statements[0]
    else {
        panic!("expected producer-backed rest-only pattern");
    };
    assert!(bindings.is_empty());
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup { paths },
            ..
        } if paths == &[Vec::<usize>::new()]
    ));
}

#[test]
fn nested_producer_rest_cleanup_is_existing_maximal_frontier() {
    let hir = build(
        "record Token {} \
         record Inner { moved: Token, omitted: Token } \
         record Outer { inner: Inner, tail: Token } \
         fn f() { \
             let Outer { inner: Inner { moved: item, .. }, .. } = \
                 Outer { inner: Inner { moved: Token {}, omitted: Token {} }, tail: Token {} }; \
         }",
    );
    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "f").body.statements[0]
    else {
        panic!("expected nested producer-backed pattern");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup { paths },
            ..
        } if paths == &[vec![1], vec![0, 1]]
    ));
}

#[test]
fn zero_field_rest_and_post_cleanup_binding_scope_are_preserved() {
    let hir = build(
        "record Empty {} \
         record Pair { left: I8, right: U8 } \
         fn make() -> Pair { return Pair { left: 1, right: 2 }; } \
         fn direct(empty: Empty) { let Empty { .. } = empty; } \
         fn producer() { let Empty { .. } = Empty {}; } \
         fn scoped() -> I8 { \
             let Pair { left: value, .. } = make(); \
             return value; \
         }",
    );

    let Statement::RecordDestructure { bindings, .. } = &function(&hir, "direct").body.statements[0]
    else {
        panic!("expected direct zero-field rest pattern");
    };
    assert!(bindings.is_empty());

    let Statement::RecordDestructure {
        scrutinee,
        bindings,
        ..
    } = &function(&hir, "producer").body.statements[0]
    else {
        panic!("expected producer zero-field rest pattern");
    };
    assert!(bindings.is_empty());
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup { paths },
            ..
        } if paths == &[Vec::<usize>::new()]
    ));

    let scoped = function(&hir, "scoped");
    let Statement::RecordDestructure { bindings, .. } = &scoped.body.statements[0] else {
        panic!("expected scoped producer pattern");
    };
    let returned = scoped
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("returned pattern binding");
    assert!(matches!(
        returned.kind,
        ValueKind::BindingUse { binding, .. } if binding == bindings[0].binding
    ));
}

#[test]
fn qualified_foreign_rest_may_omit_private_field_but_not_select_it() {
    let dependency = parse("export record Foreign { export visible: I8, hidden: I8 }");
    let accepted = parse(
        "import dep; fn f(root: dep::Foreign) { let dep::Foreign { visible: value, .. } = root; }",
    );
    assert!(dependency.errors().is_empty(), "{:?}", dependency.errors());
    assert!(accepted.errors().is_empty(), "{:?}", accepted.errors());
    let dep_module = ModuleId::new(1);
    let main_module = ModuleId::new(2);
    let dep_import = ImportTarget::new("dep", dep_module).expect("accepted import alias");
    let imports = [dep_import];
    let hir = build_typed_hir(&[
        SourceUnit::new(dep_module, &dependency, &[]),
        SourceUnit::new(main_module, &accepted, &imports),
    ])
    .expect("rest may omit a private foreign field");
    let f = function(&hir, "f");
    let Statement::RecordDestructure { bindings, .. } = &f.body.statements[0] else {
        panic!("expected qualified foreign pattern");
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].fields, vec![0]);

    let rejected = parse(
        "import dep; fn f(root: dep::Foreign) { let dep::Foreign { hidden: value, .. } = root; }",
    );
    assert!(rejected.errors().is_empty(), "{:?}", rejected.errors());
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(dep_module, &dependency, &[]),
        SourceUnit::new(main_module, &rejected, &imports),
    ])
    .expect_err("explicit private foreign field must remain inaccessible");
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));
}
