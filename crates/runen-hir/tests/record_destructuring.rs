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
                    fields: ref path,
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
fn foreign_record_may_be_bound_whole_but_not_recursively_opened() {
    let foreign = parse("export record Foreign { value: I8 }");
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

    let opened = parse(
        "import dep; record Outer { foreign: dep::Foreign } \
         fn f(root: Outer) { let Outer { foreign: Foreign { value: item } } = root; }",
    );
    assert!(opened.errors().is_empty());
    let diagnostics = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(2), &foreign, &[]),
        SourceUnit::new(ModuleId::new(1), &opened, &imports),
    ])
    .expect_err("foreign record cannot be recursively opened");
    assert!(has_kind(&diagnostics, DiagnosticKind::UnresolvedName));
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
