use runen_hir::{
    DiagnosticKind, IntrinsicType, LiteralValue, ModuleId, OwnedUse, RecordPatternScrutinee,
    RecordPatternTransientCleanup, SourceUnit, Statement, Type, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<runen_hir::TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn function<'a>(hir: &'a runen_hir::TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

type SelectionView<'a> = (
    &'a RecordPatternScrutinee,
    &'a [runen_hir::RecordPatternLiteralTest],
    &'a [runen_hir::RecordPatternBinding],
    Option<&'a RecordPatternTransientCleanup>,
    &'a runen_hir::Block,
    Option<&'a runen_hir::Block>,
);

fn selection(statement: &Statement) -> SelectionView<'_> {
    let Statement::RefutableRecordSelection {
        scrutinee,
        tests,
        bindings,
        mismatch_cleanup,
        success_block,
        mismatch_block,
        ..
    } = statement
    else {
        panic!("expected refutable record selection");
    };
    (
        scrutinee,
        tests,
        bindings,
        mismatch_cleanup.as_ref(),
        success_block,
        mismatch_block.as_deref(),
    )
}

#[test]
fn retains_independent_test_and_binding_orders_with_exact_materialized_values() {
    let hir = build(
        "record Mixed { flag: Bool, left: I8, count: I8, right: U8 } \
         fn sink_i8(value: I8) {} fn sink_u8(value: U8) {} \
         fn f(root: Mixed) { \
             if let Mixed { flag: false, left: a, count: -1, right: b } = (root) { \
                 sink_i8(a); sink_u8(b); \
             } else {} \
         }",
    )
    .expect("bounded refutable selection must build");
    let f = function(&hir, "f");
    let (scrutinee, tests, bindings, mismatch_cleanup, _, _) = selection(&f.body.statements[0]);

    assert_eq!(
        scrutinee,
        &RecordPatternScrutinee::DirectRoot(f.parameters[0].binding)
    );
    assert!(mismatch_cleanup.is_none());
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].fields, [0]);
    assert_eq!(tests[0].ty, Type::Intrinsic(IntrinsicType::Bool));
    assert_eq!(tests[0].value, LiteralValue::Bool(false));
    assert_eq!(tests[1].fields, [2]);
    assert_eq!(tests[1].ty, Type::Intrinsic(IntrinsicType::I8));
    assert_eq!(tests[1].value, LiteralValue::I8(-1));

    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].fields, [1]);
    assert_eq!(bindings[0].name, "a");
    assert_eq!(bindings[0].ownership, OwnedUse::Duplicate);
    assert_eq!(bindings[1].fields, [3]);
    assert_eq!(bindings[1].name, "b");
    assert_eq!(bindings[1].ownership, OwnedUse::Duplicate);
}

#[test]
fn nested_tests_rest_and_bindings_retain_complete_depth_first_paths() {
    let hir = build(
        "record Leaf { flag: Bool, value: I16, spare: U8 } \
         record Outer { head: I8, leaf: Leaf, tail: U8 } \
         fn sink(value: U8) {} \
         fn f(root: Outer) { \
             if let Outer { \
                 leaf: Leaf { value: -7, flag: true, spare: kept }, \
                 tail: tail, \
                 .., \
             } = (root) { sink(kept); sink(tail); } \
         }",
    )
    .expect("nested refutable pattern with rest must build");
    let (scrutinee, tests, bindings, _, _, mismatch) =
        selection(&function(&hir, "f").body.statements[0]);
    assert!(matches!(scrutinee, RecordPatternScrutinee::DirectRoot(_)));
    assert!(mismatch.is_none());
    assert_eq!(
        tests
            .iter()
            .map(|test| test.fields.clone())
            .collect::<Vec<_>>(),
        [vec![1, 1], vec![1, 0]]
    );
    assert_eq!(tests[0].value, LiteralValue::I16(-7));
    assert_eq!(tests[1].value, LiteralValue::Bool(true));
    assert_eq!(
        bindings
            .iter()
            .map(|binding| binding.fields.clone())
            .collect::<Vec<_>>(),
        [vec![1, 2], vec![2]]
    );
}

#[test]
fn producer_retains_success_frontier_and_distinct_complete_root_mismatch_cleanup() {
    let hir = build(
        "record Ticket {} \
         record Mixed { flag: Bool, ticket: Ticket, count: I8 } \
         fn make() -> Mixed { return Mixed { flag: true, ticket: Ticket {}, count: 3 }; } \
         fn f() { \
             if let Mixed { flag: true, ticket: moved, count: copied } = (make()) {} else {} \
         }",
    )
    .expect("producer-backed refutable selection must build");
    let (scrutinee, tests, bindings, mismatch_cleanup, _, mismatch_block) =
        selection(&function(&hir, "f").body.statements[0]);
    assert_eq!(tests.len(), 1);
    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].ownership, OwnedUse::Consume);
    assert_eq!(bindings[1].ownership, OwnedUse::Duplicate);
    assert!(mismatch_block.is_some());

    let RecordPatternScrutinee::Producer { value, cleanup } = scrutinee else {
        panic!("expected producer-backed scrutinee");
    };
    assert!(matches!(value.kind, ValueKind::DirectCall { .. }));
    assert_eq!(cleanup.paths, [vec![2], vec![0]]);
    assert_eq!(
        mismatch_cleanup,
        Some(&RecordPatternTransientCleanup {
            paths: vec![Vec::new()],
        })
    );
}

#[test]
fn test_only_producer_keeps_complete_root_on_success_and_mismatch() {
    let hir = build(
        "record Flag { ready: Bool } \
         fn make() -> Flag { return Flag { ready: true }; } \
         fn f() { if let Flag { ready: true } = (make()) {} }",
    )
    .expect("one test and zero bindings is a valid refutable pattern");
    let (scrutinee, tests, bindings, mismatch_cleanup, _, _) =
        selection(&function(&hir, "f").body.statements[0]);
    assert_eq!(tests.len(), 1);
    assert!(bindings.is_empty());
    assert!(matches!(
        scrutinee,
        RecordPatternScrutinee::Producer {
            cleanup: RecordPatternTransientCleanup { paths },
            ..
        } if paths == &[Vec::<usize>::new()]
    ));
    assert_eq!(
        mismatch_cleanup,
        Some(&RecordPatternTransientCleanup {
            paths: vec![Vec::new()],
        })
    );
}

#[test]
fn zero_literal_pattern_is_semantically_rejected() {
    let errors = build(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { if let Pair { left: a, right: b } = (root) {} }",
    )
    .expect_err("refutable pattern must contain at least one literal test");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::RefutableRecordPatternRequiresLiteralTest
    ));
}

#[test]
fn literal_tests_require_the_exact_resolved_field_type() {
    let bool_on_integer =
        build("record R { value: I8 } fn f(root: R) { if let R { value: true } = (root) {} }")
            .expect_err("Bool literal may test only Bool field");
    assert!(has_diagnostic(
        &bool_on_integer,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I8),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));

    let integer_on_bool =
        build("record R { value: Bool } fn f(root: R) { if let R { value: 1 } = (root) {} }")
            .expect_err("integer literal may test only fixed-width integer field");
    assert!(has_diagnostic(
        &integer_on_bool,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
}

#[test]
fn out_of_range_test_rejects_before_producer_argument_consumption_can_commit() {
    let errors = build(
        "record Ticket {} record R { value: I8 } \
         fn make(ticket: Ticket) -> R { return R { value: 1 }; } \
         fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { \
             if let R { value: 128 } = (make(ticket)) {} \
             sink(ticket); \
         }",
    )
    .expect_err("out-of-range test must reject statically");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralOutOfRange {
            required: Type::Intrinsic(IntrinsicType::I8),
        }
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "invalid pattern must reject before producer argument ownership is committed"
    );
}

#[test]
fn success_bindings_are_success_scoped_and_mismatch_may_reuse_the_same_key() {
    let hir = build(
        "record R { flag: Bool, value: I8 } \
         fn sink(value: I8) {} \
         fn f(root: R) { \
             if let R { flag: true, value: selected } = (root) { \
                 sink(selected); \
             } else { \
                 let selected: I8 = 9; sink(selected); \
             } \
         }",
    )
    .expect("sibling success/mismatch scopes may independently use one key");
    let (_, _, bindings, _, success, mismatch) = selection(&function(&hir, "f").body.statements[0]);
    assert_eq!(bindings.len(), 1);
    assert!(matches!(success.statements[0], Statement::Call { .. }));
    let mismatch = mismatch.expect("explicit mismatch block");
    let Statement::Local {
        binding: mismatch_binding,
        ..
    } = &mismatch.statements[0]
    else {
        panic!("expected mismatch-local binding");
    };
    assert_ne!(*mismatch_binding, bindings[0].binding);

    let escaping = build(
        "record R { flag: Bool, value: I8 } \
         fn sink(value: I8) {} \
         fn f(root: R) { \
             if let R { flag: true, value: selected } = (root) {} \
             sink(selected); \
         }",
    )
    .expect_err("success binding must not escape its child scope");
    assert!(has_diagnostic(&escaping, DiagnosticKind::UnresolvedName));
}

#[test]
fn mismatch_cannot_resolve_success_binding() {
    let errors = build(
        "record R { flag: Bool, value: I8 } \
         fn sink(value: I8) {} \
         fn f(root: R) { \
             if let R { flag: true, value: selected } = (root) {} else { sink(selected); } \
         }",
    )
    .expect_err("mismatch scope must not contain success bindings");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
}

#[test]
fn direct_root_nonduplicable_success_vs_unchanged_mismatch_fails_exact_join() {
    let errors = build(
        "record Ticket {} record R { flag: Bool, ticket: Ticket } \
         fn f(root: R) { \
             if let R { flag: true, ticket: moved } = (root) {} \
         }",
    )
    .expect_err("normal success consumption cannot join unchanged mismatch state");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ConditionalOwnershipMismatch
    ));
}

#[test]
fn returning_success_is_not_compared_with_sole_normal_mismatch_outcome() {
    build(
        "record Ticket {} record R { flag: Bool, ticket: Ticket } \
         fn f(root: R) -> Ticket { \
             if let R { flag: true, ticket: moved } = (root) { return moved; } \
             return root.ticket; \
         }",
    )
    .expect("only the mismatch outcome continues normally");
}
