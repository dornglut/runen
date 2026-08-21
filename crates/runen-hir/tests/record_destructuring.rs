use runen_hir::{
    AssignmentMutability, DiagnosticKind, IntrinsicType, ModuleId, OwnedUse,
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
            .map(|binding| binding.field)
            .collect::<Vec<_>>(),
        [2, 1, 0]
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
    assert_ne!(bindings[0].binding, bindings[1].binding);
    assert_ne!(bindings[1].binding, bindings[2].binding);
    assert_ne!(bindings[0].binding, bindings[2].binding);
}

#[test]
fn producer_backed_scrutinees_retain_typed_value_and_cleanup_facts() {
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

    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "call").body.statements[0]
    else {
        panic!("expected call-backed pattern");
    };
    let RecordPatternScrutinee::Producer { value, cleanup } = scrutinee else {
        panic!("expected producer-backed scrutinee");
    };
    assert!(matches!(value.kind, ValueKind::DirectCall { .. }));
    assert_eq!(
        cleanup,
        &RecordPatternTransientCleanup::DirectFields(vec![2, 0])
    );

    let Statement::RecordDestructure { scrutinee, .. } =
        &function(&hir, "construct").body.statements[0]
    else {
        panic!("expected construction-backed pattern");
    };
    let RecordPatternScrutinee::Producer { value, cleanup } = scrutinee else {
        panic!("expected producer-backed scrutinee");
    };
    assert!(matches!(value.kind, ValueKind::RecordConstruction { .. }));
    assert_eq!(
        cleanup,
        &RecordPatternTransientCleanup::DirectFields(vec![2, 0])
    );

    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "field").body.statements[0]
    else {
        panic!("expected field-backed pattern");
    };
    let RecordPatternScrutinee::Producer { value, cleanup } = scrutinee else {
        panic!("expected producer-backed scrutinee");
    };
    assert!(matches!(
        value.kind,
        ValueKind::FieldValueUse {
            fields: ref path,
            ownership: OwnedUse::Consume,
            ..
        } if path == &[0]
    ));
    assert_eq!(
        cleanup,
        &RecordPatternTransientCleanup::DirectFields(vec![2, 0])
    );

    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "owned").body.statements[0]
    else {
        panic!("expected all-consumed pattern");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup::None,
            ..
        }
    ));

    let Statement::RecordDestructure { scrutinee, bindings, .. } =
        &function(&hir, "empty").body.statements[0]
    else {
        panic!("expected zero-field producer-backed pattern");
    };
    assert!(bindings.is_empty());
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup::Complete,
            ..
        }
    ));
}

#[test]
fn all_duplicable_producer_backed_pattern_retains_complete_transient_cleanup() {
    let hir = build(
        "record Pair { left: I8, right: U8 } \
         fn f() { let Pair { right: r, left: l } = Pair { left: 1, right: 2 }; }",
    );
    let Statement::RecordDestructure { scrutinee, .. } = &function(&hir, "f").body.statements[0]
    else {
        panic!("expected record destructuring statement");
    };
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup::Complete,
            ..
        }
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
fn invalid_pattern_structure_does_not_validate_or_consume_producer() {
    let diagnostics = errors(
        "record Token {} record Pair { token: Token, count: I8 } \
         fn make(token: Token, count: I8) -> Pair { return Pair { token: token, count: count }; } \
         fn take(value: Token) {} \
         fn f(root: Pair) { \
             let Pair { missing: bad, count: copied } = make(root.token, 1); \
             take(root.token); \
         }",
    );
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::UnknownRecordField
    ));
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
fn producer_result_must_match_pattern_nominal_type_and_have_a_result() {
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
fn immutable_root_can_transfer_field_ownership_but_remains_nonassignable() {
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
fn structural_pattern_failures_are_atomic_and_do_not_consume_root() {
    for (source, required) in [
        (
            "record Token {} record Holder { token: Token, count: I8 } fn take(value: Token) {} \
             fn f(root: Holder) { let Holder { missing: bad, count: copied } = root; take(root.token); }",
            DiagnosticKind::UnknownRecordField,
        ),
        (
            "record Token {} record Holder { token: Token, count: I8 } fn take(value: Token) {} \
             fn f(root: Holder) { let Holder { token: one, token: two, count: copied } = root; take(root.token); }",
            DiagnosticKind::DuplicateRecordPatternField,
        ),
        (
            "record Token {} record Holder { token: Token, count: I8 } fn take(value: Token) {} \
             fn f(root: Holder) { let Holder { count: copied } = root; take(root.token); }",
            DiagnosticKind::MissingRecordPatternField,
        ),
    ] {
        let diagnostics = errors(source);
        assert!(has_kind(&diagnostics, required), "{diagnostics:?}");
        assert!(!has_kind(
            &diagnostics,
            DiagnosticKind::UnavailableFieldValue
        ));
    }
}

#[test]
fn duplicate_pattern_bindings_and_active_shadowing_are_rejected_atomically() {
    let duplicate = errors(
        "record Token {} record Pair { left: Token, right: Token } fn take(value: Token) {} \
         fn f(root: Pair) { let Pair { left: item, right: item } = root; take(root.left); }",
    );
    assert!(has_kind(
        &duplicate,
        DiagnosticKind::DuplicatePatternBinding
    ));
    assert!(!has_kind(&duplicate, DiagnosticKind::UnavailableFieldValue));

    let shadow = errors(
        "record Token {} record Pair { left: Token, right: Token } fn take(value: Token) {} \
         fn f(root: Pair, item: I8) { let Pair { left: item, right: other } = root; take(root.left); }",
    );
    assert!(has_kind(&shadow, DiagnosticKind::LocalShadowing));
    assert!(!has_kind(&shadow, DiagnosticKind::UnavailableFieldValue));
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
fn partially_consumed_selected_field_rejects_pattern_before_any_new_transition() {
    let diagnostics = errors(
        "record Token {} record Holder { token: Token, count: I8 } fn take(value: Token) {} \
         fn f(root: Holder) { \
             take(root.token); \
             let Holder { count: copied, token: moved } = root; \
         }",
    );
    assert!(has_kind(
        &diagnostics,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn all_nonduplicable_fields_remain_independent_pattern_consumes() {
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
            .map(|binding| binding.field)
            .collect::<Vec<_>>(),
        [1, 0]
    );
    assert!(
        bindings
            .iter()
            .all(|binding| binding.ownership == OwnedUse::Consume)
    );
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
    assert_eq!(bindings[0].ty, Type::Record(hir.records[0].id));
}

#[test]
fn nested_block_cleanup_uses_reverse_pattern_source_binding_order() {
    let hir = build(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { \
             { let Pair { right: second, left: first } = root; } \
         }",
    );
    let f = function(&hir, "f");
    let Statement::Block(block) = &f.body.statements[0] else {
        panic!("expected nested block");
    };
    let Statement::RecordDestructure { bindings, .. } = &block.statements[0] else {
        panic!("expected pattern statement");
    };
    assert_eq!(bindings.len(), 2);
    assert_eq!(block.normal_cleanup.len(), 2);
    assert_eq!(block.normal_cleanup[0].binding, bindings[1].binding);
    assert_eq!(block.normal_cleanup[1].binding, bindings[0].binding);
    assert!(
        block
            .normal_cleanup
            .iter()
            .all(|cleanup| cleanup.fields.is_empty())
    );
}

#[test]
fn nested_pattern_bindings_obey_child_scope_lookup() {
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
