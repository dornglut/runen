use runen_hir::{
    AssignmentMutability, DiagnosticKind, ImportTarget, IntrinsicType, ModuleId, OwnedUse,
    SourceUnit, Statement, Type, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<runen_hir::TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn has_diagnostic(
    errors: &[runen_hir::Diagnostic],
    predicate: impl Fn(DiagnosticKind) -> bool,
) -> bool {
    errors.iter().any(|error| predicate(error.kind))
}

fn count_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> usize {
    errors.iter().filter(|error| error.kind == kind).count()
}

#[test]
fn retains_local_mutability_and_resolved_assignment_identity() {
    let hir = build("fn f(input: I64) { let mut x: I64 = input; x = input; }")
        .expect("mutable whole-binding assignment must validate");
    let function = &hir.functions[0];

    let Statement::Local {
        binding,
        mutability,
        ..
    } = function.body.statements[0]
    else {
        panic!("expected mutable local");
    };
    assert_eq!(mutability, AssignmentMutability::Mutable);

    let Statement::Assignment {
        target,
        fields,
        value,
        ..
    } = &function.body.statements[1]
    else {
        panic!("expected resolved assignment");
    };
    assert_eq!(*target, binding);
    assert!(
        fields.is_empty(),
        "whole-binding assignment retains empty path"
    );
    let ValueKind::BindingUse { ownership, .. } = value.kind else {
        panic!("expected binding-use assignment RHS");
    };
    assert_eq!(ownership, OwnedUse::Duplicate);
}

#[test]
fn retains_exact_resolved_field_assignment_path_and_type() {
    let hir = build(
        "record Inner { pad: I8, value: I64 } \
         record Outer { first: I8, inner: Inner } \
         fn f(seed: Outer, replacement: I64) { \
             let mut root: Outer = seed; \
             root.inner.value = replacement; \
         }",
    )
    .expect("nested field assignment must validate");
    let function = hir
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();
    let Statement::Local { binding, .. } = function.body.statements[0] else {
        panic!("expected mutable root local");
    };
    let Statement::Assignment {
        target,
        fields,
        value,
        ..
    } = &function.body.statements[1]
    else {
        panic!("expected field assignment");
    };
    assert_eq!(*target, binding);
    assert_eq!(fields, &[1, 1]);
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::I64));
}

#[test]
fn immutable_locals_and_parameters_reject_assignment_independent_of_availability() {
    for source in [
        "fn f(input: I64) { let x: I64 = input; x = input; }",
        "fn f(input: I64) { input = input; }",
        "record Ticket {} fn sink(v: Ticket) {} fn f(x: Ticket, replacement: Ticket) { sink(x); x = replacement; }",
        "record Box { value: I64 } fn f(seed: Box, replacement: I64) { let x: Box = seed; x.value = replacement; }",
        "record Box { value: I64 } fn f(x: Box, replacement: I64) { x.value = replacement; }",
    ] {
        let errors = build(source).expect_err("immutable assignment must be source-invalid");
        assert!(
            has_diagnostic(&errors, |kind| kind
                == DiagnosticKind::ImmutableAssignmentTarget),
            "missing immutable-target diagnostic for {source}: {errors:?}"
        );
    }
}

#[test]
fn mutable_unavailable_binding_can_be_reinitialized_and_used_again() {
    let source = "record Ticket {} fn sink(v: Ticket) {} fn f(seed: Ticket, replacement: Ticket) { let mut x: Ticket = seed; sink(x); x = replacement; sink(x); }";
    let hir = build(source).expect("assignment must reinitialize a consumed mutable binding");
    let function = hir
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();

    let Statement::Assignment { value, .. } = &function.body.statements[2] else {
        panic!("expected reinitializing assignment");
    };
    let ValueKind::BindingUse { ownership, .. } = value.kind else {
        panic!("expected replacement binding use");
    };
    assert_eq!(ownership, OwnedUse::Consume);
    assert!(matches!(
        function.body.statements[3],
        Statement::Call { .. }
    ));
}

#[test]
fn assignment_requires_exact_source_type() {
    let errors = build("fn f(a: I32, b: I64) { let mut x: I64 = b; x = a; }")
        .expect_err("assignment must not introduce implicit conversion");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch { .. }
    )));

    let errors = build(
        "record Box { value: I64 } fn f(seed: Box, replacement: I32) { let mut x: Box = seed; x.value = replacement; }",
    )
    .expect_err("field assignment must require the selected field's exact type");
    assert!(has_diagnostic(&errors, |kind| matches!(
        kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I64),
            found: Type::Intrinsic(IntrinsicType::I32),
        }
    )));
}

#[test]
fn field_assignment_reuses_field_resolution_diagnostics() {
    let non_record = build(
        "fn f(seed: I64, replacement: I64) { let mut x: I64 = seed; x.value = replacement; }",
    )
    .expect_err("selector on a scalar target must reject");
    assert!(has_diagnostic(&non_record, |kind| kind
        == DiagnosticKind::ExpectedRecordForFieldAccess));

    let unknown = build(
        "record Box { value: I64 } fn f(seed: Box, replacement: I64) { let mut x: Box = seed; x.missing = replacement; }",
    )
    .expect_err("unknown target field must reject");
    assert!(has_diagnostic(&unknown, |kind| kind == DiagnosticKind::UnknownRecordField));

    let dependency = parse("export record Foreign { hidden: I64 }");
    let application = parse(
        "import dep; fn f(seed: dep::Foreign, replacement: I64) { let mut x: dep::Foreign = seed; x.hidden = replacement; }",
    );
    let dependency_module = ModuleId::new(2);
    let imports = [ImportTarget::new("dep", dependency_module).unwrap()];
    let inaccessible = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &application, &imports),
        SourceUnit::new(dependency_module, &dependency, &[]),
    ])
    .expect_err("foreign private assignment field must reject");
    assert!(has_diagnostic(&inaccessible, |kind| kind
        == DiagnosticKind::InaccessibleRecordField));
}

#[test]
fn self_assignment_uses_existing_duplicate_or_consume_semantics() {
    let scalar = build("fn f(input: I64) { let mut x: I64 = input; x = x; x = x; }")
        .expect("duplicable self-assignment must validate");
    let scalar_function = &scalar.functions[0];
    for statement in &scalar_function.body.statements[1..] {
        let Statement::Assignment { value, .. } = statement else {
            panic!("expected assignment");
        };
        let ValueKind::BindingUse { ownership, .. } = value.kind else {
            panic!("expected binding-use RHS");
        };
        assert_eq!(ownership, OwnedUse::Duplicate);
    }

    let record = build(
        "record Ticket {} fn f(input: Ticket) -> Ticket { let mut x: Ticket = input; x = x; return x; }",
    )
    .expect("non-duplicable self-assignment must consume then reinitialize");
    let function = record
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();
    let Statement::Assignment { value, .. } = &function.body.statements[1] else {
        panic!("expected assignment");
    };
    let ValueKind::BindingUse { ownership, .. } = value.kind else {
        panic!("expected binding-use RHS");
    };
    assert_eq!(ownership, OwnedUse::Consume);
}

#[test]
fn call_rhs_ownership_effects_precede_assignment_reavailability() {
    let hir = build(
        "record Ticket {} fn id(v: Ticket) -> Ticket { return v; } fn f(input: Ticket) -> Ticket { let mut x: Ticket = input; x = id(x); return x; }",
    )
    .expect("RHS call may consume target and then return its replacement");
    let function = hir
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();
    let Statement::Assignment { value, .. } = &function.body.statements[1] else {
        panic!("expected assignment");
    };
    let ValueKind::DirectCall { arguments, .. } = &value.kind else {
        panic!("expected result-bearing call RHS");
    };
    let ValueKind::BindingUse { ownership, .. } = arguments[0].kind else {
        panic!("expected target use as call argument");
    };
    assert_eq!(ownership, OwnedUse::Consume);
    assert!(
        function
            .body
            .terminal_return
            .as_ref()
            .unwrap()
            .value
            .is_some()
    );
}

#[test]
fn field_assignment_admits_replacement_reinitialization_and_partial_reconstruction() {
    build(
        "record Leaf {} record Inner { a: Leaf, b: Leaf } record Outer { inner: Inner, spare: Leaf } \
         fn sink(v: Leaf) {} \
         fn replacement(seed: Outer) -> Outer { \
             let mut x: Outer = seed; \
             x.inner = Inner { a: Leaf {}, b: Leaf {} }; \
             return x; \
         } \
         fn exact(seed: Outer) -> Outer { \
             let mut x: Outer = seed; \
             sink(x.inner.a); \
             x.inner.a = Leaf {}; \
             return x; \
         } \
         fn partial(seed: Outer) -> Outer { \
             let mut x: Outer = seed; \
             sink(x.inner.a); \
             x.inner = Inner { a: Leaf {}, b: Leaf {} }; \
             return x; \
         }",
    )
    .expect("field assignment must admit fully available, exact-consumed, and partial targets");
}

#[test]
fn field_assignment_rejects_consumed_strict_ancestor_without_splitting_state() {
    let errors = build(
        "record Leaf {} record Inner { a: Leaf, b: Leaf } record Outer { inner: Inner, spare: Leaf } \
         fn sink(v: Inner) {} \
         fn f(seed: Outer) { \
             let mut x: Outer = seed; \
             sink(x.inner); \
             x.inner.a = Leaf {}; \
         }",
    )
    .expect_err("assignment below a consumed strict ancestor must reject");
    assert_eq!(
        count_diagnostic(&errors, DiagnosticKind::UnavailableFieldValue),
        1,
        "strict-ancestor rejection must use the existing field-unavailable diagnostic: {errors:?}"
    );
}

#[test]
fn failed_post_rhs_structural_admission_does_not_leak_speculative_state() {
    let errors = build(
        "record Leaf {} record Inner { a: Leaf, b: Leaf } record Outer { inner: Inner, spare: Leaf } \
         fn sink_inner(v: Inner) {} \
         fn sink_leaf(v: Leaf) {} \
         fn id(v: Leaf) -> Leaf { return v; } \
         fn f(seed: Outer) { \
             let mut x: Outer = seed; \
             sink_inner(x.inner); \
             x.inner.a = id(x.spare); \
             sink_leaf(x.spare); \
         }",
    )
    .expect_err("post-RHS strict-ancestor rejection must reject the assignment");
    assert_eq!(
        count_diagnostic(&errors, DiagnosticKind::UnavailableFieldValue),
        1,
        "rejected assignment must not leak speculative RHS consumption of the disjoint field: {errors:?}"
    );
}

#[test]
fn field_assignment_rhs_consumption_composes_with_installation_law() {
    build(
        "record Leaf {} record Inner { a: Leaf, b: Leaf } record Outer { inner: Inner, spare: Leaf } \
         fn id(v: Leaf) -> Leaf { return v; } \
         fn rebuild(a: Leaf, b: Leaf) -> Inner { return Inner { a: a, b: b }; } \
         fn exact(seed: Outer) -> Outer { \
             let mut x: Outer = seed; \
             x.inner.a = id(x.inner.a); \
             return x; \
         } \
         fn descendant(seed: Outer) -> Outer { \
             let mut x: Outer = seed; \
             x.inner = rebuild(x.inner.a, Leaf {}); \
             return x; \
         }",
    )
    .expect("RHS exact-target and descendant consumption must compose with installation");

    let disjoint = build(
        "record Leaf {} record Inner { a: Leaf, b: Leaf } record Outer { inner: Inner, spare: Leaf } \
         fn choose(value: Inner, discard: Leaf) -> Inner { return value; } \
         fn sink(v: Leaf) {} \
         fn f(seed: Outer) { \
             let mut x: Outer = seed; \
             x.inner = choose(Inner { a: Leaf {}, b: Leaf {} }, x.spare); \
             sink(x.spare); \
         }",
    )
    .expect_err("disjoint RHS consumption must remain consumed after target installation");
    assert_eq!(
        count_diagnostic(&disjoint, DiagnosticKind::UnavailableFieldValue),
        1,
        "target installation must preserve the disjoint consumed path: {disjoint:?}"
    );
}

#[test]
fn field_assignment_authority_is_exact_target_relative() {
    for source in [
        "record Pair { left: I64, right: I64 } \
         fn f(seed: Pair, replacement: I64) { \
             let mut x: Pair = seed; \
             let r: &I64 = &x.left; \
             x.left = replacement; \
         }",
        "record Inner { left: I64, right: I64 } record Outer { inner: Inner } \
         fn f(seed: Outer, replacement: I64) { \
             let mut x: Outer = seed; \
             let r: &Inner = &x.inner; \
             x.inner.left = replacement; \
         }",
        "record Inner { left: I64, right: I64 } record Outer { inner: Inner } \
         fn f(seed: Outer, replacement: Inner) { \
             let mut x: Outer = seed; \
             let r: &I64 = &x.inner.left; \
             x.inner = replacement; \
         }",
    ] {
        let errors =
            build(source).expect_err("overlapping Shared authority must block replacement");
        assert!(
            has_diagnostic(&errors, |kind| kind
                == DiagnosticKind::BorrowedAssignmentTarget),
            "missing overlapping Shared-authority rejection for {source}: {errors:?}"
        );
    }

    build(
        "record Pair { left: I64, right: I64 } \
         fn f(seed: Pair, replacement: I64) { \
             let mut x: Pair = seed; \
             let r: &I64 = &x.right; \
             x.left = replacement; \
             let observed: I64 = *r; \
         }",
    )
    .expect("disjoint sibling Shared authority must remain compatible with assignment");
}

#[test]
fn field_assignment_entry_authority_cannot_be_cured_by_rhs_consuming_carrier() {
    let errors = build(
        "record Pair { left: I64, right: I64 } \
         fn release(r: &mut Pair) -> I64 { return 1; } \
         fn consume(r: &mut Pair) {} \
         fn f(seed: Pair) { \
             let mut x: Pair = seed; \
             let r: &mut Pair = &mut x; \
             x.left = release(r); \
             consume(r); \
         }",
    )
    .expect_err("entry-state overlapping authority must reject even if RHS consumes its carrier");
    assert!(has_diagnostic(&errors, |kind| kind
        == DiagnosticKind::BorrowedAssignmentTarget));
    assert!(
        !has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnavailableBinding),
        "rejected field assignment must not leak speculative RHS carrier consumption: {errors:?}"
    );
}

#[test]
fn invalid_field_rhs_diagnostic_precedes_entry_authority_admission() {
    let errors = build(
        "record Pair { left: I64, right: I64 } \
         fn f(seed: Pair) { \
             let mut x: Pair = seed; \
             let r: &I64 = &x.left; \
             x.left = missing; \
         }",
    )
    .expect_err("invalid RHS must reject before entry authority diagnostic");
    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnresolvedName));
    assert!(
        !has_diagnostic(&errors, |kind| kind
            == DiagnosticKind::BorrowedAssignmentTarget),
        "RHS diagnostics retain priority over remembered entry authority: {errors:?}"
    );
}

#[test]
fn field_assignment_reuses_existing_branch_and_loop_exact_state_rules() {
    build(
        "record Leaf {} record Inner { a: Leaf, b: Leaf } record Outer { inner: Inner } \
         fn sink(v: Leaf) {} \
         fn branch(cond: Bool, seed: Outer) -> Outer { \
             let mut x: Outer = seed; \
             sink(x.inner.a); \
             if cond { \
                 x.inner = Inner { a: Leaf {}, b: Leaf {} }; \
             } else { \
                 x.inner = Inner { a: Leaf {}, b: Leaf {} }; \
             } \
             return x; \
         } \
         fn looping(cond: Bool, seed: Outer) -> Outer { \
             let mut x: Outer = seed; \
             while cond { \
                 sink(x.inner.a); \
                 x.inner.a = Leaf {}; \
             } \
             return x; \
         }",
    )
    .expect("explicit field restoration may satisfy existing branch and loop state equality");
}

#[test]
fn unavailable_use_remains_invalid_until_successful_reinitialization() {
    let invalid = build(
        "record Ticket {} fn sink(v: Ticket) {} fn f(input: Ticket) { let mut x: Ticket = input; sink(x); sink(x); }",
    )
    .expect_err("ordinary use of unavailable binding must remain invalid");
    assert!(has_diagnostic(&invalid, |kind| kind == DiagnosticKind::UnavailableBinding));

    build(
        "record Ticket {} fn sink(v: Ticket) {} fn f(input: Ticket, replacement: Ticket) { let mut x: Ticket = input; sink(x); x = replacement; sink(x); }",
    )
    .expect("successful reinitialization must restore availability");
}

#[test]
fn assignment_target_lookup_never_bypasses_selected_entity() {
    let wrong_category = build("fn helper() {} fn f(x: I64) { helper = x; }")
        .expect_err("module function is not an assignment target");
    assert!(has_diagnostic(&wrong_category, |kind| kind
        == DiagnosticKind::ExpectedValueBinding));

    let local_blocks_module = build(
        "fn helper(x: I64) -> I64 { return x; } fn f(helper: I64, replacement: I64) { helper = replacement; }",
    )
    .expect_err("selected parameter must block same-module function fallback");
    assert!(has_diagnostic(&local_blocks_module, |kind| kind
        == DiagnosticKind::ImmutableAssignmentTarget));
}

#[test]
fn entry_borrow_cannot_be_cured_by_rhs_consuming_its_authority() {
    let errors = build(
        "fn release(r: &mut I64) -> I64 { return 1; }\
         fn consume(r: &mut I64) {}\
         fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let r: &mut I64 = &mut x;\
             x = release(r);\
             consume(r);\
         }",
    )
    .expect_err("entry-state Exclusive conflict must reject assignment even if RHS consumes it");

    assert!(
        has_diagnostic(&errors, |kind| kind
            == DiagnosticKind::BorrowedAssignmentTarget),
        "missing statement-entry borrowed-target rejection: {errors:?}"
    );
    assert!(
        !has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnavailableBinding),
        "rejected assignment must not leak the speculative RHS consumption of r: {errors:?}"
    );
}

#[test]
fn invalid_rhs_diagnostic_precedes_entry_borrow_admission() {
    let errors = build(
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let r: &I64 = &x;\
             x = missing;\
         }",
    )
    .expect_err("invalid RHS must reject before a borrowed-target diagnostic is emitted");

    assert!(has_diagnostic(&errors, |kind| kind == DiagnosticKind::UnresolvedName));
    assert!(
        !has_diagnostic(&errors, |kind| kind
            == DiagnosticKind::BorrowedAssignmentTarget),
        "speculative RHS validation must preserve existing diagnostic priority: {errors:?}"
    );
}
