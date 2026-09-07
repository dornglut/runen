use runen_hir::{
    DiagnosticKind, ImportTarget, IntrinsicType, LiteralValue, ModuleId, OwnedUse,
    RecordPatternScrutinee, RecordPatternTestKind, RecordPatternTransientCleanup, SourceUnit,
    Statement, Type, ValueKind, build_typed_hir,
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
    &'a [runen_hir::RecordPatternTest],
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
             if let Mixed { flag: false, left: a, count: < -1, right: b } = (root) { \
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
    assert_eq!(tests[0].kind, RecordPatternTestKind::Equality);
    assert_eq!(tests[0].ty, Type::Intrinsic(IntrinsicType::Bool));
    assert_eq!(tests[0].value, LiteralValue::Bool(false));
    assert_eq!(tests[1].fields, [2]);
    assert_eq!(tests[1].kind, RecordPatternTestKind::StrictUpperBound);
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
                 leaf: Leaf { value: < -7, flag: true, spare: kept }, \
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
    assert_eq!(tests[0].kind, RecordPatternTestKind::StrictUpperBound);
    assert_eq!(tests[0].value, LiteralValue::I16(-7));
    assert_eq!(tests[1].kind, RecordPatternTestKind::Equality);
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
fn strict_upper_bounds_retain_exact_signed_and_unsigned_types_and_values() {
    let hir = build(
        "record R { signed: I8, unsigned: U8 } \
         fn f(root: R) { if let R { signed: < -1, unsigned: < 200 } = (root) {} }",
    )
    .expect("signed and unsigned strict upper bounds must build");
    let (_, tests, bindings, _, _, _) = selection(&function(&hir, "f").body.statements[0]);
    assert!(bindings.is_empty());
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].kind, RecordPatternTestKind::StrictUpperBound);
    assert_eq!(tests[0].fields, [0]);
    assert_eq!(tests[0].ty, Type::Intrinsic(IntrinsicType::I8));
    assert_eq!(tests[0].value, LiteralValue::I8(-1));
    assert_eq!(tests[1].kind, RecordPatternTestKind::StrictUpperBound);
    assert_eq!(tests[1].fields, [1]);
    assert_eq!(tests[1].ty, Type::Intrinsic(IntrinsicType::U8));
    assert_eq!(tests[1].value, LiteralValue::U8(200));
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
    assert_eq!(tests[0].kind, RecordPatternTestKind::Equality);
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
    assert_eq!(tests[0].kind, RecordPatternTestKind::Equality);
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
fn ordering_only_pattern_satisfies_refutability_requirement() {
    let hir = build(
        "record R { value: I8 } fn f(root: R) { if let R { value: < 3 } = (root) {} }",
    )
    .expect("one strict-upper-bound test is refutable without equality");
    let (_, tests, bindings, _, _, _) = selection(&function(&hir, "f").body.statements[0]);
    assert_eq!(tests.len(), 1);
    assert_eq!(tests[0].kind, RecordPatternTestKind::StrictUpperBound);
    assert_eq!(tests[0].value, LiteralValue::I8(3));
    assert!(bindings.is_empty());
}

#[test]
fn zero_test_pattern_is_semantically_rejected() {
    let errors = build(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { if let Pair { left: a, right: b } = (root) {} }",
    )
    .expect_err("refutable pattern must contain at least one refutable test");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::RefutableRecordPatternRequiresRefutableTest
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
fn strict_upper_bounds_require_fixed_width_integer_fields() {
    for (source, operand_type) in [
        (
            "record R { value: Bool } fn f(root: R) { if let R { value: < 1 } = (root) {} }",
            Type::Intrinsic(IntrinsicType::Bool),
        ),
        (
            "record R { value: F32 } fn f(root: R) { if let R { value: < 1 } = (root) {} }",
            Type::Intrinsic(IntrinsicType::F32),
        ),
    ] {
        let errors = build(source).expect_err("strict upper bound requires fixed-width integer field");
        assert!(has_diagnostic(
            &errors,
            DiagnosticKind::IntegerOrderingRequiresInteger { operand_type }
        ));
    }

    let record_errors = build(
        "record Inner {} record R { value: Inner } \
         fn f(root: R) { if let R { value: < 1 } = (root) {} }",
    )
    .expect_err("record field may not carry strict upper-bound test");
    assert!(record_errors.iter().any(|error| {
        matches!(
            error.kind,
            DiagnosticKind::IntegerOrderingRequiresInteger {
                operand_type: Type::Record(_)
            }
        )
    }));
}

#[test]
fn out_of_range_strict_bound_rejects_before_producer_argument_consumption_can_commit() {
    let errors = build(
        "record Ticket {} record R { value: I8 } \
         fn make(ticket: Ticket) -> R { return R { value: 1 }; } \
         fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { \
             if let R { value: < 128 } = (make(ticket)) {} \
             sink(ticket); \
         }",
    )
    .expect_err("out-of-range strict bound must reject statically");
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

#[test]
fn qualified_foreign_and_nested_heads_preserve_field_accessibility() {
    let dependency = parse(
        "export record Inner { export flag: Bool, hidden: Bool } \
         export record Outer { export inner: Inner }",
    );
    let accepted = parse(
        "import dep; fn f(root: dep::Outer) { \
             if let dep::Outer { inner: dep::Inner { flag: true, .. } } = (root) {} \
         }",
    );
    assert!(dependency.errors().is_empty(), "{:?}", dependency.errors());
    assert!(accepted.errors().is_empty(), "{:?}", accepted.errors());
    let dep_module = ModuleId::new(1);
    let main_module = ModuleId::new(2);
    let imports = [ImportTarget::new("dep", dep_module).expect("accepted import alias")];
    build_typed_hir(&[
        SourceUnit::new(dep_module, &dependency, &[]),
        SourceUnit::new(main_module, &accepted, &imports),
    ])
    .expect("qualified foreign and nested visible test fields must remain accessible");

    let rejected = parse(
        "import dep; fn f(root: dep::Outer) { \
             if let dep::Outer { inner: dep::Inner { hidden: true, .. } } = (root) {} \
         }",
    );
    assert!(rejected.errors().is_empty(), "{:?}", rejected.errors());
    let errors = build_typed_hir(&[
        SourceUnit::new(dep_module, &dependency, &[]),
        SourceUnit::new(main_module, &rejected, &imports),
    ])
    .expect_err("qualified nested private test fields must remain inaccessible");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::InaccessibleRecordField
    ));
}

#[test]
fn direct_root_test_requires_the_selected_path_to_be_fully_available() {
    let errors = build(
        "record Ticket {} \
         record Inner { flag: Bool, ticket: Ticket } \
         record Outer { inner: Inner } \
         fn take(value: Inner) {} \
         fn f(root: Outer) { \
             take(root.inner); \
             if let Outer { inner: Inner { flag: true, .. } } = (root) {} \
         }",
    )
    .expect_err("a consumed ancestor makes the selected literal-test path unavailable");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn direct_root_strict_upper_bound_test_requires_the_selected_path_to_be_fully_available() {
    let errors = build(
        "record Ticket {} \
         record Inner { value: I8, ticket: Ticket } \
         record Outer { inner: Inner } \
         fn take(value: Inner) {} \
         fn f(root: Outer) { \
             take(root.inner); \
             if let Outer { inner: Inner { value: < 3, .. } } = (root) {} \
         }",
    )
    .expect_err("a consumed ancestor makes the selected strict-bound test path unavailable");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn direct_root_tests_require_shared_safe_authority_compatibility() {
    let errors = build(
        "record R { flag: Bool } \
         fn f(seed: R) { \
             let mut root: R = seed; \
             let replacement: &mut R = &mut root; \
             if let R { flag: true } = (root) {} \
         }",
    )
    .expect_err("literal testing may not bypass overlapping replacement authority");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ReferencePermissionUnavailable
    ));
}

#[test]
fn direct_root_strict_upper_bound_tests_require_shared_safe_authority_compatibility() {
    let errors = build(
        "record R { value: I8 } \
         fn f(seed: R) { \
             let mut root: R = seed; \
             let replacement: &mut R = &mut root; \
             if let R { value: < 3 } = (root) {} \
         }",
    )
    .expect_err("strict-bound testing may not bypass overlapping replacement authority");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ReferencePermissionUnavailable
    ));
}

#[test]
fn nonduplicable_success_binding_retains_exclusive_authority_requirement() {
    let errors = build(
        "record Ticket { value: I64 } record R { flag: Bool, ticket: Ticket } \
         fn f(root: R) { \
             let shared: &I64 = &root.ticket.value; \
             if let R { flag: true, ticket: moved } = (root) { fault; } else { fault; } \
         }",
    )
    .expect_err("non-duplicable binding transfer requires Exclusive-compatible access");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ReferencePermissionUnavailable
    ));
}

#[test]
fn omitted_mismatch_is_the_sole_normal_outcome_when_success_faults() {
    let hir = build(
        "record R { flag: Bool, value: I8 } \
         fn f(root: R) { \
             if let R { flag: true, .. } = (root) { fault; } \
             let observed: I8 = root.value; \
         }",
    )
    .expect("omitted mismatch must carry the unchanged normal state without a synthetic scope");
    let f = function(&hir, "f");
    let (_, _, bindings, _, success, mismatch) = selection(&f.body.statements[0]);
    assert!(bindings.is_empty());
    assert!(!success.has_normal_continuation);
    assert!(mismatch.is_none());
    assert!(matches!(f.body.statements[1], Statement::Local { .. }));
}

#[test]
fn restoring_success_consumption_allows_exact_two_normal_outcome_join() {
    build(
        "record Ticket {} record R { flag: Bool, ticket: Ticket } \
         fn take(value: R) {} \
         fn f(seed: R) { \
             let mut root: R = seed; \
             if let R { flag: true, ticket: moved } = (root) { root.ticket = moved; } \
             take(root); \
         }",
    )
    .expect("success may restore its consumed path so both normal outcomes become exactly equal");
}

#[test]
fn exact_pointer_origin_and_external_referent_join_rules_are_reused() {
    let pointer_errors = build(
        "record Flag { flag: Bool } \
         fn f(root: Flag, a: I64, b: I64) { \
             let mut p: raw I64 = raw &a; \
             if let Flag { flag: true } = (root) { p = raw &b; } else {} \
         }",
    )
    .expect_err("two normal selection outcomes require exact equal raw-pointer origins");
    assert!(has_diagnostic(
        &pointer_errors,
        DiagnosticKind::ConditionalPointerOriginMismatch
    ));

    build(
        "record Flag { flag: Bool } \
         fn f(root: Flag, a: I64, b: I64) { \
             let mut p: raw I64 = raw &a; \
             if let Flag { flag: true } = (root) { p = raw &b; p = raw &a; } else {} \
         }",
    )
    .expect("restoring the raw-pointer origin must permit the two-normal selection join");

    let reference_errors = build(
        "record Ticket { value: I64 } record Flag { flag: Bool } \
         fn f(root: Flag, r: &mut Ticket) { \
             if let Flag { flag: true } = (root) { let moved: Ticket = *r; } else {} \
         }",
    )
    .expect_err("two normal selection outcomes require exact equal external-referent state");
    assert!(has_diagnostic(
        &reference_errors,
        DiagnosticKind::ConditionalReferenceStateMismatch
    ));

    build(
        "record Ticket { value: I64 } record Flag { flag: Bool } \
         fn f(root: Flag, r: &mut Ticket) { \
             if let Flag { flag: true } = (root) { \
                 let moved: Ticket = *r; *r = moved; \
             } else {} \
         }",
    )
    .expect("restoring the replacement referent must permit the two-normal selection join");
}
