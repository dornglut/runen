use runen_hir::{
    DiagnosticKind, IntrinsicType, ModuleId, OwnedUse, SourceUnit, Statement, Type,
    TypedCompilation, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

fn while_statement(statement: &Statement) -> (&runen_hir::Value, &runen_hir::Block) {
    let Statement::While {
        condition, body, ..
    } = statement
    else {
        panic!("expected while statement");
    };
    (condition, body)
}

#[test]
fn retains_exact_bool_condition_body_and_normal_cleanup() {
    let hir = build(
        "record Ticket {} \
         fn make() -> Ticket { return Ticket {}; } \
         fn f(flag: Bool) { while flag { let ticket: Ticket = make(); } }",
    )
    .expect("valid bounded while");
    let f = function(&hir, "f");
    assert!(f.body.has_normal_continuation);
    let (condition, body) = while_statement(&f.body.statements[0]);

    assert_eq!(condition.ty, Type::Intrinsic(IntrinsicType::Bool));
    let ValueKind::BindingUse { ownership, .. } = condition.kind else {
        panic!("expected retained binding condition");
    };
    assert_eq!(ownership, OwnedUse::Duplicate);
    assert!(body.has_normal_continuation);
    let Statement::Local { binding, .. } = &body.statements[0] else {
        panic!("expected body local");
    };
    assert_eq!(body.normal_cleanup.len(), 1);
    assert_eq!(body.normal_cleanup[0].binding, *binding);
    assert!(body.normal_cleanup[0].fields.is_empty());
}

#[test]
fn integer_condition_is_rejected_through_exact_bool_required_type() {
    let errors = build("fn f() { while 1 {} }").expect_err("integer condition must fail Bool typing");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
}

#[test]
fn failed_condition_does_not_commit_speculative_argument_consumption() {
    let errors = build(
        "record Ticket {} \
         fn wrong(value: Ticket) -> I64 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { while wrong(value) {} sink(value); }",
    )
    .expect_err("non-Bool condition result must reject");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::Bool),
            found: Type::Intrinsic(IntrinsicType::I64),
        }
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "failed condition validation must not leak speculative consumption"
    );
}

#[test]
fn condition_consumption_is_visible_to_body_validation() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { while predicate(value) { sink(value); } }",
    )
    .expect_err("condition consumption must precede body validation");

    assert!(has_diagnostic(&errors, DiagnosticKind::UnavailableBinding));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::LoopOwnershipMismatch),
        "invalid body must not produce a secondary backedge mismatch"
    );
}

#[test]
fn normal_body_must_restore_complete_loop_head_ownership() {
    let errors = build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { while flag { sink(value); } }",
    )
    .expect_err("normal backedge cannot lose an enclosing owned value");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::LoopOwnershipMismatch
    ));
}

#[test]
fn normal_body_must_restore_exact_nested_consumed_path_state() {
    let errors = build(
        "record Left {} record Right {} record Pair { left: Left, right: Right } \
         fn take_left(value: Left) {} \
         fn f(flag: Bool, pair: Pair) { while flag { take_left(pair.left); } }",
    )
    .expect_err("normal backedge cannot change nested ownership state");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::LoopOwnershipMismatch
    ));
}

#[test]
fn mutable_assignment_can_restore_head_ownership_after_condition_consumption() {
    build(
        "record Ticket {} \
         fn make() -> Ticket { return Ticket {}; } \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn f() { \
             let mut value: Ticket = make(); \
             while predicate(value) { value = make(); } \
         }",
    )
    .expect("explicit mutable assignment may restore the next-iteration head state");
}

#[test]
fn mutable_runtime_value_changes_do_not_change_structural_loop_invariant() {
    build("fn f(flag: Bool) { let mut value: I64 = 0; while flag { value = 1; } }")
        .expect("loop invariant compares ownership, not runtime value equality");
}

#[test]
fn no_normal_body_requires_no_backedge_ownership_equality() {
    build(
        "record Ticket {} \
         fn f(flag: Bool, value: Ticket) -> Ticket { \
             while flag { return value; } \
             return value; \
         }",
    )
    .expect("returning body contributes no backedge state");

    build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { \
             while flag { sink(value); fault; } \
             sink(value); \
         }",
    )
    .expect("faulting body contributes no backedge state and false exit retains head ownership");
}

#[test]
fn false_exit_commits_post_condition_state_not_body_restoration() {
    let errors = build(
        "record Ticket {} \
         fn make() -> Ticket { return Ticket {}; } \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f() { \
             let mut value: Ticket = make(); \
             while predicate(value) { value = make(); } \
             sink(value); \
         }",
    )
    .expect_err("false exit has evaluated and consumed the condition argument");

    assert!(has_diagnostic(&errors, DiagnosticKind::UnavailableBinding));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::LoopOwnershipMismatch),
        "body restoration must satisfy the backedge even though it does not apply to false exit"
    );
}

#[test]
fn literal_true_still_has_static_normal_continuation_for_result_obligation() {
    let errors = build("fn f() -> I64 { while true {} }")
        .expect_err("literal true is not constant-pruned into a non-continuing source statement");
    assert!(has_diagnostic(&errors, DiagnosticKind::MissingResultReturn));
}

#[test]
fn body_locals_end_at_each_body_scope_and_do_not_escape() {
    let errors = build("fn f(flag: Bool) { while flag { let child: I64 = 1; } let x: I64 = child; }")
        .expect_err("while body local must not escape the child block");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
}

#[test]
fn nested_while_and_if_compose_without_a_generic_state_join() {
    build(
        "fn f(a: Bool, b: Bool) { \
             while a { if b { while a {} } else {} } \
         }",
    )
    .expect("nested bounded control flow composes through existing exact ownership state");
}
