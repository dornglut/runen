use runen_hir::{
    AssignmentMutability, DiagnosticKind, ImportTarget, IntrinsicType, ModuleId, OwnedUse,
    RecordPatternScrutinee, RecordPatternTransientCleanup, SourceUnit, Statement, Type, ValueKind,
    build_typed_hir,
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
fn retains_resolved_pattern_facts_in_source_order() {
    let hir = build(
        "record Token {} \
         record Mixed { first: I8, token: Token, last: U8 } \
         fn f(root: Mixed) { \
             let Mixed { last: z, token: moved, first: a } = root; \
         }",
    );
    let f = function(&hir, "f");
    let Statement::RecordDestructure {
        record,
        scrutinee,
        bindings,
        ..
    } = &f.body.statements[0]
    else {
        panic!("expected record destructuring statement");
    };

    assert_eq!(*record, hir.records[1].id);
    assert_eq!(
        scrutinee,
        &RecordPatternScrutinee::DirectRoot(f.parameters[0].binding)
    );
    assert_eq!(bindings.len(), 3);
    assert_eq!(
        bindings
            .iter()
            .map(|binding| binding.fields.clone())
            .collect::<Vec<_>>(),
        [vec![2], vec![1], vec![0]]
    );
    assert_eq!(
        bindings
            .iter()
            .map(|binding| binding.name.as_str())
            .collect::<Vec<_>>(),
        ["z", "moved", "a"]
    );
    assert_eq!(bindings[0].ty, Type::Intrinsic(IntrinsicType::U8));
    assert_eq!(bindings[1].ty, Type::Record(hir.records[0].id));
    assert_eq!(bindings[2].ty, Type::Intrinsic(IntrinsicType::I8));
    assert_eq!(bindings[0].ownership, OwnedUse::Duplicate);
    assert_eq!(bindings[1].ownership, OwnedUse::Consume);
    assert_eq!(bindings[2].ownership, OwnedUse::Duplicate);
}

#[test]
fn recursive_pattern_retains_full_leaf_paths_in_depth_first_source_order() {
    let hir = build(
        "record Token {} \
         record Leaf { value: I8, token: Token } \
         record Inner { leaf: Leaf, count: U8 } \
         record Outer { tail: I8, inner: Inner } \
         fn f(root: Outer) { \
             let Outer { \
                 inner: Inner { \
                     count: count, \
                     leaf: Leaf { token: moved, value: value }, \
                 }, \
                 tail: tail, \
             } = root; \
         }",
    );
    let f = function(&hir, "f");
    let Statement::RecordDestructure { bindings, .. } = &f.body.statements[0] else {
        panic!("expected recursive record destructuring");
    };
    assert_eq!(
        bindings
            .iter()
            .map(|binding| binding.fields.clone())
            .collect::<Vec<_>>(),
        [vec![1, 1], vec![1, 0, 1], vec![1, 0, 0], vec![0]]
    );
    assert_eq!(
        bindings
            .iter()
            .map(|binding| binding.name.as_str())
            .collect::<Vec<_>>(),
        ["count", "moved", "value", "tail"]
    );
    assert_eq!(bindings[1].ownership, OwnedUse::Consume);
    assert!(
        [0, 2, 3]
            .into_iter()
            .all(|index| bindings[index].ownership == OwnedUse::Duplicate)
    );
}

#[test]
fn producer_backed_scrutinees_retain_typed_value_and_canonical_cleanup_paths() {
    let hir = build(
        "record Token {} \
         record Mixed { first: I8, token: Token, last: U8 } \
         record Left {} record Right {} record Owned { left: Left, right: Right } \
         record Empty {} record Outer { mixed: Mixed } \
         fn make() -> Mixed { return Mixed { first: 1, token: Token {}, last: 2 }; } \
         fn call() { let Mixed { token: moved, last: z, first: a } = make(); } \
         fn construct() { let Mixed { first: a, token: moved, last: z } = Mixed { first: 1, token: Token {}, last: 2 }; } \
         fn field(root: Outer) { let Mixed { first: a, token: moved, last: z } = root.mixed; } \
         fn owned() { let Owned { left: l, right: r } = Owned { left: Left {}, right: Right {} }; } \
         fn empty() { let Empty {} = Empty {}; }",
    );

    for name in ["call", "construct", "field"] {
        let Statement::RecordDestructure { scrutinee, .. } =
            &function(&hir, name).body.statements[0]
        else {
            panic!("expected producer-backed pattern");
        };
        let RecordPatternScrutinee::Producer { cleanup, .. } = scrutinee else {
            panic!("expected producer-backed scrutinee");
        };
        assert_eq!(
            cleanup,
            &RecordPatternTransientCleanup {
                paths: vec![vec![2], vec![0]],
            }
        );
    }

    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "call").body.statements[0]
    else {
        panic!("expected call-backed pattern");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            value: runen_hir::Value {
                kind: ValueKind::DirectCall { .. },
                ..
            },
            ..
        }
    ));

    let Statement::RecordDestructure { scrutinee, .. } =
        &function(&hir, "construct").body.statements[0]
    else {
        panic!("expected construction-backed pattern");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            value: runen_hir::Value {
                kind: ValueKind::RecordConstruction { .. },
                ..
            },
            ..
        }
    ));

    let Statement::RecordDestructure { scrutinee, .. } =
        &function(&hir, "field").body.statements[0]
    else {
        panic!("expected field-backed pattern");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            value: runen_hir::Value {
                kind: ValueKind::FieldValueUse {
                    fields: path,
                    ownership: OwnedUse::Consume,
                    ..
                },
                ..
            },
            ..
        } if path == &[0]
    ));

    let Statement::RecordDestructure { scrutinee, .. } =
        &function(&hir, "owned").body.statements[0]
    else {
        panic!("expected all-consumed pattern");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup { paths },
            ..
        } if paths.is_empty()
    ));

    let Statement::RecordDestructure {
        scrutinee,
        bindings,
        ..
    } = &function(&hir, "empty").body.statements[0]
    else {
        panic!("expected zero-field producer-backed pattern");
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
fn recursive_producer_cleanup_is_maximal_recursive_frontier_in_reverse_structural_order() {
    let hir = build(
        "record Token { value: I8 } \
         record Inner { a: I8, token: Token, b: U8 } \
         record Outer { head: I8, inner: Inner, tail: U8 } \
         fn f() { \
             let Outer { \
                 inner: Inner { token: moved, a: a, b: b }, \
                 head: head, \
                 tail: tail, \
             } = Outer { \
                 head: 1, \
                 inner: Inner { a: 2, token: Token { value: 3 }, b: 4 }, \
                 tail: 5, \
             }; \
         }",
    );
    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "f").body.statements[0]
    else {
        panic!("expected recursive producer pattern");
    };
    let RecordPatternScrutinee::Producer { cleanup, .. } = scrutinee else {
        panic!("expected producer scrutinee");
    };
    assert_eq!(cleanup.paths, [vec![2], vec![1, 2], vec![1, 0], vec![0]]);
}

#[test]
fn recursive_producer_no_consume_keeps_complete_root_and_all_transferred_is_empty() {
    let all_dup = build(
        "record Inner { a: I8, b: U8 } record Outer { inner: Inner, tail: I8 } \
         fn f() { \
             let Outer { inner: Inner { b: b, a: a }, tail: tail } = \
                 Outer { inner: Inner { a: 1, b: 2 }, tail: 3 }; \
         }",
    );
    let Statement::RecordDestructure { scrutinee, .. } =
        &function(&all_dup, "f").body.statements[0]
    else {
        panic!("expected producer pattern");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup { paths },
            ..
        } if paths == &[Vec::<usize>::new()]
    ));

    let all_moved = build(
        "record A {} record B {} record C {} \
         record Inner { b: B, c: C } record Outer { a: A, inner: Inner } \
         fn f() { \
             let Outer { inner: Inner { c: c, b: b }, a: a } = \
                 Outer { a: A {}, inner: Inner { b: B {}, c: C {} } }; \
         }",
    );
    let Statement::RecordDestructure { scrutinee, .. } =
        &function(&all_moved, "f").body.statements[0]
    else {
        panic!("expected producer pattern");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup { paths },
            ..
        } if paths.is_empty()
    ));
}

#[test]
fn producer_lookup_occurs_before_pattern_bindings_enter_scope() {
    let hir = build(
        "record Pair { left: I8, right: U8 } \
         fn make() -> Pair { return Pair { left: 1, right: 2 }; } \
         fn f() -> I8 { \
             let Pair { left: make, right: other } = make(); \
             return make; \
         }",
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
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            value: runen_hir::Value {
                kind: ValueKind::DirectCall { .. },
                ..
            },
            ..
        }
    ));
    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("returned value");
    assert!(matches!(
        returned.kind,
        ValueKind::BindingUse { binding, .. } if binding == bindings[0].binding
    ));
}

#[test]
fn invalid_recursive_structure_does_not_validate_or_consume_producer() {
    let diagnostics = errors(
        "record Token {} record Inner { token: Token, count: I8 } record Outer { inner: Inner } \
         fn make(token: Token) -> Outer { return Outer { inner: Inner { token: token, count: 1 } }; } \
         fn take(value: Token) {} \
         fn f(root: Inner) { \
             let Outer { inner: Inner { missing: bad, count: copied } } = make(root.token); \
             take(root.token); \
         }",
    );
    assert!(has_kind(&diagnostics, DiagnosticKind::UnknownRecordField));
    assert!(!has_kind(
        &diagnostics,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn invalid_producer_rolls_back_tentative_ownership_transitions() {
    let diagnostics = errors(
        "record Token {} record Pair { token: Token, count: I8 } \
         fn make(token: Token, count: I8) -> Pair { return Pair { token: token, count: count }; } \
         fn take(value: Token) {} \
         fn f(root: Pair) { \
             let Pair { token: moved, count: copied } = make(root.token, missing); \
             take(root.token); \
         }",
    );
    assert!(has_kind(&diagnostics, DiagnosticKind::UnresolvedName));
    assert!(!has_kind(
        &diagnostics,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn successful_field_producer_commits_source_binding_transition() {
    let diagnostics = errors(
        "record Token {} record Pair { token: Token, count: I8 } record Outer { pair: Pair } \
         fn take(value: Pair) {} \
         fn f(root: Outer) { \
             let Pair { token: moved, count: copied } = root.pair; \
             take(root.pair); \
         }",
    );
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn nested_head_must_match_selected_nominal_type_exactly() {
    let diagnostics = errors(
        "record A { value: I8 } record B { value: I8 } record Outer { inner: A } \
         fn f(root: Outer) { let Outer { inner: B { value: extracted } } = root; }",
    );
    assert!(diagnostics.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Record(_),
            found: Type::Record(_),
        }
    )));
}

#[test]
fn recursive_unknown_duplicate_missing_and_binding_failures_are_atomic() {
    for (source, required) in [
        (
            "record Token {} record Inner { token: Token, count: I8 } record Outer { inner: Inner } fn take(value: Token) {} \
             fn f(root: Outer) { let Outer { inner: Inner { missing: bad, count: copied } } = root; take(root.inner.token); }",
            DiagnosticKind::UnknownRecordField,
        ),
        (
            "record Token {} record Inner { token: Token, count: I8 } record Outer { inner: Inner } fn take(value: Token) {} \
             fn f(root: Outer) { let Outer { inner: Inner { token: one, token: two, count: copied } } = root; take(root.inner.token); }",
            DiagnosticKind::DuplicateRecordPatternField,
        ),
        (
            "record Token {} record Inner { token: Token, count: I8 } record Outer { inner: Inner } fn take(value: Token) {} \
             fn f(root: Outer) { let Outer { inner: Inner { count: copied } } = root; take(root.inner.token); }",
            DiagnosticKind::MissingRecordPatternField,
        ),
        (
            "record Token {} record Inner { left: Token, right: Token } record Outer { inner: Inner } fn take(value: Token) {} \
             fn f(root: Outer) { let Outer { inner: Inner { left: item, right: item } } = root; take(root.inner.left); }",
            DiagnosticKind::DuplicatePatternBinding,
        ),
    ] {
        let diagnostics = errors(source);
        assert!(has_kind(&diagnostics, required), "{diagnostics:?}");
        assert!(!has_kind(
            &diagnostics,
            DiagnosticKind::UnavailableFieldValue
        ));
    }

    let shadow = errors(
        "record Token {} record Inner { left: Token, right: Token } record Outer { inner: Inner } fn take(value: Token) {} \
         fn f(root: Outer, item: I8) { let Outer { inner: Inner { left: item, right: other } } = root; take(root.inner.left); }",
    );
    assert!(has_kind(&shadow, DiagnosticKind::LocalShadowing));
    assert!(!has_kind(&shadow, DiagnosticKind::UnavailableFieldValue));
}

#[test]
fn partially_available_intermediate_can_recurse_to_disjoint_leaf_and_zero_field_node() {
    let hir = build(
        "record Token {} record Inner { left: Token, right: Token } record Outer { inner: Inner, tail: I8 } \
         fn take(value: Token) {} \
         fn f(root: Outer) { \
             take(root.inner.left); \
             let Outer { \
                 inner: Inner { left: Token {}, right: moved_right }, \
                 tail: copied_tail, \
             } = root; \
         }",
    );
    let Statement::RecordDestructure { bindings, .. } = &function(&hir, "f").body.statements[1]
    else {
        panic!("expected recursive pattern");
    };
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].fields, vec![0, 1]);
    assert_eq!(bindings[0].ownership, OwnedUse::Consume);
    assert_eq!(bindings[1].fields, vec![1]);
}

#[test]
fn unavailable_nested_leaf_rejects_before_any_new_pattern_transition() {
    let diagnostics = errors(
        "record Token {} record Inner { left: Token, right: Token } record Outer { inner: Inner, tail: I8 } \
         fn take(value: Token) {} \
         fn f(root: Outer) { \
             take(root.inner.right); \
             let Outer { inner: Inner { left: moved_left, right: moved_right }, tail: copied } = root; \
             take(root.inner.left); \
         }",
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind == DiagnosticKind::UnavailableFieldValue)
            .count(),
        1,
        "{diagnostics:?}"
    );
}

#[test]
fn zero_leaf_nonduplicable_leaf_consumption_is_retained_at_full_nested_path() {
    let hir = build(
        "record Empty {} record Inner { empty: Empty } record Outer { inner: Inner } \
         fn f(root: Outer) { let Outer { inner: Inner { empty: moved } } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } = &function(&hir, "f").body.statements[0]
    else {
        panic!("expected recursive pattern");
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].fields, vec![0, 0]);
    assert_eq!(bindings[0].ownership, OwnedUse::Consume);
}

#[test]
fn foreign_record_may_be_bound_whole_and_only_qualified_head_opens_it() {
    let foreign = parse("export record Foreign { export value: I8 }");
    let whole = parse(
        "import dep; record Outer { foreign: dep::Foreign } \
         fn f(root: Outer) { let Outer { foreign: whole } = root; }",
    );
    assert!(foreign.errors().is_empty());
    assert!(whole.errors().is_empty());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &foreign, &[]),
        SourceUnit::new(ModuleId::new(1), &whole, &imports),
    ])
    .expect("foreign record field may be bound whole");

    let unqualified = parse(
        "import dep; record Outer { foreign: dep::Foreign } \
         fn f(root: Outer) { let Outer { foreign: Foreign { value: item } } = root; }",
    );
    assert!(unqualified.errors().is_empty());
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &foreign, &[]),
        SourceUnit::new(ModuleId::new(1), &unqualified, &imports),
    ])
    .expect_err("unqualified nested head remains same-module lookup");
    assert!(has_kind(&diagnostics, DiagnosticKind::UnresolvedName));

    let qualified = parse(
        "import dep; record Outer { foreign: dep::Foreign } \
         fn f(root: Outer) -> I8 { \
             let Outer { foreign: dep::Foreign { value: item } } = root; \
             return item; \
         }",
    );
    assert!(qualified.errors().is_empty());
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &foreign, &[]),
        SourceUnit::new(ModuleId::new(1), &qualified, &imports),
    ])
    .expect("qualified nested head opens exported foreign record");
}

#[test]
fn qualified_top_head_reuses_existing_hir_for_all_scrutinee_categories() {
    let foreign = parse(
        "export record Foreign { export value: I8 } \
         export fn make() -> Foreign { return Foreign { value: 1 }; }",
    );
    let caller = parse(
        "import dep; \
         record Holder { foreign: dep::Foreign } \
         fn direct(root: dep::Foreign, dep: I8, Foreign: I8) -> I8 { \
             let dep::Foreign { value: item } = root; return item; \
         } \
         fn call() -> I8 { \
             let dep::Foreign { value: item } = dep::make(); return item; \
         } \
         fn construct() -> I8 { \
             let dep::Foreign { value: item } = dep::Foreign { value: 2 }; return item; \
         } \
         fn field(root: Holder) -> I8 { \
             let dep::Foreign { value: item } = root.foreign; return item; \
         }",
    );
    assert!(foreign.errors().is_empty(), "{:?}", foreign.errors());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &foreign, &[]),
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
    ])
    .expect("qualified patterns compose with all existing scrutinee categories");
    let foreign_id = hir
        .records
        .iter()
        .find(|record| record.name == "Foreign")
        .expect("foreign record")
        .id;

    for name in ["direct", "call", "construct", "field"] {
        let Statement::RecordDestructure {
            record,
            bindings,
            ..
        } = &function(&hir, name).body.statements[0]
        else {
            panic!("expected record destructuring in {name}");
        };
        assert_eq!(*record, foreign_id);
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].fields, vec![0]);
        assert_eq!(bindings[0].ty, Type::Intrinsic(IntrinsicType::I8));
        assert_eq!(bindings[0].ownership, OwnedUse::Duplicate);
    }

    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "direct").body.statements[0]
    else {
        panic!("direct pattern");
    };
    assert!(matches!(scrutinee, RecordPatternScrutinee::DirectRoot(_)));

    for (name, expected) in [
        ("call", "call"),
        ("construct", "construction"),
        ("field", "field"),
    ] {
        let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, name).body.statements[0]
        else {
            panic!("producer pattern");
        };
        let RecordPatternScrutinee::Producer { value, .. } = scrutinee else {
            panic!("expected producer scrutinee");
        };
        assert!(
            matches!(
                (&value.kind, expected),
                (ValueKind::DirectCall { .. }, "call")
                    | (ValueKind::RecordConstruction { .. }, "construction")
                    | (ValueKind::FieldValueUse { .. }, "field")
            ),
            "unexpected producer kind for {name}: {:?}",
            value.kind
        );
    }
}

#[test]
fn qualified_pattern_head_lookup_preserves_existing_diagnostic_partition() {
    let target = parse(
        "record Private { export value: I8 } \
         export fn Wrong() {}",
    );
    let caller = parse(
        "import dep; record Local { value: I8 } \
         fn private(root: Local) { let dep::Private { value: item } = root; } \
         fn wrong(root: Local, dep: I8) { let dep::Wrong {} = root; } \
         fn missing_alias(root: Local) { let nope::Missing {} = root; } \
         fn missing_member(root: Local) { let dep::Missing {} = root; }",
    );
    assert!(target.errors().is_empty());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &target, &[]),
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
    ])
    .expect_err("qualified head lookup failures must reject");

    assert!(has_kind(&diagnostics, DiagnosticKind::InaccessibleBinding));
    assert!(has_kind(&diagnostics, DiagnosticKind::ExpectedRecordType));
    assert!(has_kind(&diagnostics, DiagnosticKind::UnresolvedName));
}

#[test]
fn qualified_top_and_nested_heads_require_exact_nominal_identity() {
    let target = parse(
        "export record A { export value: I8 } \
         export record B { export value: I8 }",
    );
    let caller = parse(
        "import dep; \
         record Outer { inner: dep::A } \
         fn top(root: dep::B) { let dep::A { value: item } = root; } \
         fn nested(root: Outer) { \
             let Outer { inner: dep::B { value: item } } = root; \
         }",
    );
    assert!(target.errors().is_empty());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &target, &[]),
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
    ])
    .expect_err("structurally equal foreign records remain nominally distinct");
    assert!(
        diagnostics
            .iter()
            .filter(|diagnostic| matches!(diagnostic.kind, DiagnosticKind::TypeMismatch { .. }))
            .count()
            >= 2,
        "{diagnostics:?}"
    );
}

#[test]
fn qualified_foreign_pattern_fields_use_direct_accessibility_after_identity_resolution() {
    let mixed = parse("export record Foreign { private: I8, export public: I8 }");
    let caller = parse(
        "import dep; fn f(root: dep::Foreign) { \
             let dep::Foreign { private: hidden, public: shown } = root; \
         }",
    );
    assert!(mixed.errors().is_empty());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &mixed, &[]),
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
    ])
    .expect_err("private foreign field must remain inaccessible");
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));

    let omitted = parse(
        "import dep; fn f(root: dep::Foreign) { \
             let dep::Foreign { public: shown } = root; \
         }",
    );
    assert!(omitted.errors().is_empty());
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &mixed, &[]),
        SourceUnit::new(ModuleId::new(1), &omitted, &imports),
    ])
    .expect_err("omitting private foreign field remains non-exhaustive");
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::MissingRecordPatternField
    ));
    assert!(!has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));

    let public_only = parse("export record PublicOnly { export public: I8 }");
    let unknown = parse(
        "import dep; fn f(root: dep::PublicOnly) { \
             let dep::PublicOnly { missing: bad, public: shown } = root; \
         }",
    );
    assert!(public_only.errors().is_empty());
    assert!(unknown.errors().is_empty());
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &public_only, &[]),
        SourceUnit::new(ModuleId::new(1), &unknown, &imports),
    ])
    .expect_err("unknown foreign field must reject");
    assert!(has_kind(&diagnostics, DiagnosticKind::UnknownRecordField));
    assert!(!has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));

    let empty = parse("export record Empty {}");
    let empty_caller = parse("import dep; fn f(root: dep::Empty) { let dep::Empty {} = root; }");
    assert!(empty.errors().is_empty());
    assert!(empty_caller.errors().is_empty());
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &empty, &[]),
        SourceUnit::new(ModuleId::new(1), &empty_caller, &imports),
    ])
    .expect("zero-field exported foreign pattern requires no field access");
}

#[test]
fn recursive_qualified_patterns_recompute_accessibility_at_each_module_transition() {
    let foreign = parse("export record Foreign { export value: I8 }");
    let local_to_foreign = parse(
        "import dep; record Outer { foreign: dep::Foreign } \
         fn f(root: Outer) -> I8 { \
             let Outer { foreign: dep::Foreign { value: item } } = root; \
             return item; \
         }",
    );
    assert!(foreign.errors().is_empty());
    assert!(local_to_foreign.errors().is_empty());
    let dep_import = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &foreign, &[]),
        SourceUnit::new(ModuleId::new(1), &local_to_foreign, &dep_import),
    ])
    .expect("local to foreign nested pattern recomputes foreign accessibility");

    let app = parse(
        "import dep; export record Local { private: I8 } \
         fn f(root: dep::Foreign) -> I8 { \
             let dep::Foreign { local: Local { private: item } } = root; \
             return item; \
         }",
    );
    let dep = parse("import app; export record Foreign { export local: app::Local }");
    assert!(app.errors().is_empty(), "{:?}", app.errors());
    assert!(dep.errors().is_empty(), "{:?}", dep.errors());
    let app_imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    let foreign_imports = [ImportTarget::new("app", ModuleId::new(1)).expect("valid alias")];
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &app, &app_imports),
        SourceUnit::new(ModuleId::new(2), &dep, &foreign_imports),
    ])
    .expect("foreign to caller-module nested pattern resumes same-module private access");

    let third = parse("export record Third { export value: I8 }");
    let middle = parse(
        "import third; export record Foreign { export third: third::Third }",
    );
    let caller = parse(
        "import dep; import third; fn f(root: dep::Foreign) -> I8 { \
             let dep::Foreign { third: third::Third { value: item } } = root; \
             return item; \
         }",
    );
    assert!(third.errors().is_empty());
    assert!(middle.errors().is_empty());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let middle_imports = [ImportTarget::new("third", ModuleId::new(3)).expect("valid alias")];
    let caller_imports = [
        ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias"),
        ImportTarget::new("third", ModuleId::new(3)).expect("valid alias"),
    ];
    build_typed_hir(&[
        SourceUnit::new(ModuleId::new(3), &third, &[]),
        SourceUnit::new(ModuleId::new(2), &middle, &middle_imports),
        SourceUnit::new(ModuleId::new(1), &caller, &caller_imports),
    ])
    .expect("foreign to third-module nested pattern performs independent qualified lookup");
}

#[test]
fn invalid_qualified_pattern_prevalidation_does_not_commit_producer_ownership() {
    let target = parse(
        "export record Token {} \
         export record Foreign { private: I8, export token: Token } \
         export fn make(token: Token) -> Foreign { \
             return Foreign { private: 1, token: token }; \
         }",
    );
    let caller = parse(
        "import dep; fn take(value: dep::Token) {} \
         fn f(token: dep::Token) { \
             let dep::Foreign { private: bad, token: moved } = dep::make(token); \
             take(token); \
         }",
    );
    assert!(target.errors().is_empty());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &target, &[]),
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
    ])
    .expect_err("inaccessible pattern field must reject before producer ownership commits");
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::InaccessibleRecordField
    ));
    assert!(!has_kind(&diagnostics, DiagnosticKind::UnavailableBinding));
}

#[test]
fn valid_qualified_producer_pattern_commits_existing_producer_ownership() {
    let target = parse(
        "export record Token {} \
         export record Foreign { export value: I8 } \
         export fn make(token: Token) -> Foreign { return Foreign { value: 1 }; }",
    );
    let caller = parse(
        "import dep; fn take(value: dep::Token) {} \
         fn f(token: dep::Token) { \
             let dep::Foreign { value: copied } = dep::make(token); \
             take(token); \
         }",
    );
    assert!(target.errors().is_empty());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid alias")];
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &target, &[]),
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
    ])
    .expect_err("successful producer validation must commit argument ownership");
    assert!(has_kind(&diagnostics, DiagnosticKind::UnavailableBinding));
}

#[test]
fn all_duplicable_pattern_leaves_root_available_for_later_whole_use() {
    let hir = build(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) -> Pair { \
             let Pair { right: r, left: l } = root; \
             return root; \
         }",
    );
    let f = function(&hir, "f");
    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("returned value");
    assert!(matches!(
        returned.kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Consume,
            ..
        }
    ));
}

#[test]
fn mixed_pattern_consumes_exact_field_but_preserves_disjoint_field_access() {
    let hir = build(
        "record Token {} record Holder { token: Token, count: I8 } \
         fn f(root: Holder) -> I8 { \
             let Holder { token: moved, count: copied } = root; \
             return root.count; \
         }",
    );
    let f = function(&hir, "f");
    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("returned value");
    assert!(matches!(
        returned.kind,
        ValueKind::FieldValueUse {
            fields: ref path,
            ownership: OwnedUse::Duplicate,
            ..
        } if path == &[1]
    ));
}

#[test]
fn mutable_partial_root_can_be_whole_replaced_after_pattern() {
    build(
        "record Token {} record Holder { token: Token, count: I8 } \
         fn take(value: Holder) {} \
         fn f() { \
             let mut root: Holder = Holder { token: Token {}, count: 1 }; \
             let Holder { token: moved, count: copied } = root; \
             root = Holder { token: Token {}, count: 2 }; \
             take(root); \
         }",
    );
}

#[test]
fn pattern_head_and_root_use_their_distinct_lookup_relations() {
    let hir = build(
        "record Pair { value: I8 } \
         fn f(root: Pair) -> I8 { \
             let Pair: I8 = 7; \
             let Pair { value: extracted } = root; \
             return extracted; \
         }",
    );
    let f = function(&hir, "f");
    assert!(matches!(
        f.body.statements[1],
        Statement::RecordDestructure { .. }
    ));

    let wrong_root_category = errors(
        "record Pair { value: I8 } record root {} \
         fn f() { let Pair { value: extracted } = root; }",
    );
    assert!(has_kind(
        &wrong_root_category,
        DiagnosticKind::ExpectedValueBinding
    ));
}

#[test]
fn root_type_must_match_selected_nominal_record_exactly() {
    let diagnostics = errors(
        "record A { value: I8 } record B { value: I8 } \
         fn f(root: B) { let A { value: extracted } = root; }",
    );
    assert!(diagnostics.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Record(_),
            found: Type::Record(_),
        }
    )));
}

#[test]
fn zero_field_pattern_is_noop_and_zero_leaf_field_still_records_consumption() {
    let hir = build(
        "record Empty {} record Holder { empty: Empty } \
         fn empty(root: Empty) { let Empty {} = root; } \
         fn field(root: Holder) { let Holder { empty: moved } = root; }",
    );

    let Statement::RecordDestructure { bindings, .. } = &function(&hir, "empty").body.statements[0]
    else {
        panic!("expected empty record destructuring");
    };
    assert!(bindings.is_empty());

    let Statement::RecordDestructure { bindings, .. } = &function(&hir, "field").body.statements[0]
    else {
        panic!("expected field record destructuring");
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].ownership, OwnedUse::Consume);
    assert_eq!(bindings[0].fields, vec![0]);
    assert_eq!(bindings[0].ty, Type::Record(hir.records[0].id));
}

#[test]
fn nested_block_cleanup_uses_reverse_depth_first_pattern_binding_order() {
    let hir = build(
        "record Inner { left: I8, right: U8 } record Outer { head: I8, inner: Inner } \
         fn f(root: Outer) { \
             { let Outer { inner: Inner { right: second, left: first }, head: head } = root; } \
         }",
    );
    let f = function(&hir, "f");
    let Statement::Block(block) = &f.body.statements[0] else {
        panic!("expected nested block");
    };
    let Statement::RecordDestructure { bindings, .. } = &block.statements[0] else {
        panic!("expected pattern statement");
    };
    assert_eq!(
        bindings
            .iter()
            .map(|binding| binding.name.as_str())
            .collect::<Vec<_>>(),
        ["second", "first", "head"]
    );
    assert_eq!(block.normal_cleanup.len(), 3);
    assert_eq!(block.normal_cleanup[0].binding, bindings[2].binding);
    assert_eq!(block.normal_cleanup[1].binding, bindings[1].binding);
    assert_eq!(block.normal_cleanup[2].binding, bindings[0].binding);
    assert!(
        block
            .normal_cleanup
            .iter()
            .all(|cleanup| cleanup.fields.is_empty())
    );
}

#[test]
fn pattern_bindings_are_immutable_and_enter_normal_local_lookup_after_declaration() {
    let hir = build(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) -> I8 { \
             let Pair { left: extracted, right: other } = root; \
             return extracted; \
         }",
    );
    let f = function(&hir, "f");
    let Statement::RecordDestructure { bindings, .. } = &f.body.statements[0] else {
        panic!("expected pattern statement");
    };
    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("returned value");
    assert!(matches!(
        returned.kind,
        ValueKind::BindingUse { binding, .. } if binding == bindings[0].binding
    ));

    let diagnostics = errors(
        "record Pair { left: I8, right: U8 } \
         fn bad(root: Pair) { \
             let Pair { left: extracted, right: other } = root; \
             extracted = 4; \
         }",
    );
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::ImmutableAssignmentTarget
    ));

    let ordinary = build("fn local() { let mut value: I8 = 1; value = 2; }");
    let Statement::Local { mutability, .. } = &function(&ordinary, "local").body.statements[0]
    else {
        panic!("expected ordinary local");
    };
    assert_eq!(*mutability, AssignmentMutability::Mutable);
}
