use runen_hir::{
    AssignmentMutability, DiagnosticKind, ModuleId, OwnedUse, SourceUnit, Statement, ValueKind,
    build_typed_hir,
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

    let Statement::Assignment { target, value, .. } = &function.body.statements[1] else {
        panic!("expected resolved assignment");
    };
    assert_eq!(*target, binding);
    let ValueKind::BindingUse { ownership, .. } = value.kind else {
        panic!("expected binding-use assignment RHS");
    };
    assert_eq!(ownership, OwnedUse::Duplicate);
}

#[test]
fn immutable_locals_and_parameters_reject_assignment_independent_of_availability() {
    for source in [
        "fn f(input: I64) { let x: I64 = input; x = input; }",
        "fn f(input: I64) { input = input; }",
        "record Ticket {} fn sink(v: Ticket) {} fn f(x: Ticket, replacement: Ticket) { sink(x); x = replacement; }",
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
        has_diagnostic(&errors, |kind| kind == DiagnosticKind::BorrowedAssignmentTarget),
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
        !has_diagnostic(&errors, |kind| kind == DiagnosticKind::BorrowedAssignmentTarget),
        "speculative RHS validation must preserve existing diagnostic priority: {errors:?}"
    );
}
