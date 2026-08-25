use runen_hir::{
    DiagnosticKind, IntrinsicType, ModuleId, SourceUnit, Statement, Type, TypedCompilation,
    Value, ValueKind, build_typed_hir,
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

fn boolean_and(value: &Value) -> (&Value, &Value) {
    let ValueKind::BooleanAnd { left, right } = &value.kind else {
        panic!("expected Boolean conjunction HIR value");
    };
    (left, right)
}

#[test]
fn retains_explicit_exact_bool_conjunction_hir() {
    let hir = build("fn f(a: Bool, b: Bool) -> Bool { return a && b; }")
        .expect("Boolean conjunction must build");
    let returned = function(&hir, "f")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    let (left, right) = boolean_and(returned);

    let bool_ty = Type::Intrinsic(IntrinsicType::Bool);
    assert_eq!(returned.ty, bool_ty);
    assert_eq!(left.ty, bool_ty);
    assert_eq!(right.ty, bool_ty);
    assert!(matches!(left.kind, ValueKind::BindingUse { .. }));
    assert!(matches!(right.kind, ValueKind::BindingUse { .. }));
}

#[test]
fn outer_non_bool_requirement_rejects_before_operand_ownership_can_commit() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: I64 = predicate(value) && true; \
             sink(value); \
         }",
    )
    .expect_err("conjunction result is intrinsically Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I64),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "outer type failure must prevent speculative left consumption from escaping"
    );
}

#[test]
fn literal_false_still_requires_full_rhs_static_validation() {
    let errors = build("fn f() -> Bool { return false && missing(); }")
        .expect_err("known false does not prune RHS validation");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
}

#[test]
fn successful_conjunction_commits_exact_post_left_ownership_state() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { \
             let result: Bool = predicate(value) && flag; \
             sink(value); \
         }",
    )
    .expect_err("left producer consumption is the successful conjunction post-state");

    assert!(has_diagnostic(&errors, DiagnosticKind::UnavailableBinding));
    assert!(
        !has_diagnostic(
            &errors,
            DiagnosticKind::BooleanConjunctionOwnershipMismatch
        ),
        "RHS adds no ownership transition, so L and R are equal"
    );
}

#[test]
fn rhs_only_consumption_rejects_exact_state_mismatch_even_for_false_left() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn f(value: Ticket) { let result: Bool = false && predicate(value); }",
    )
    .expect_err("RHS-only consumption differs from skipped-RHS state");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::BooleanConjunctionOwnershipMismatch
    ));
}

#[test]
fn rejected_rhs_ownership_mismatch_rolls_back_the_whole_operator_transaction() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let result: Bool = false && predicate(value); \
             sink(value); \
         }",
    )
    .expect_err("conjunction mismatch rejects but must not consume enclosing ownership");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::BooleanConjunctionOwnershipMismatch
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "failed conjunction must leave the original binding environment available"
    );
}

#[test]
fn right_type_failure_rolls_back_successful_left_consumption() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn integer() -> I64 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let result: Bool = predicate(value) && integer(); \
             sink(value); \
         }",
    )
    .expect_err("non-Bool RHS rejects the conjunction transaction");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::Bool),
            found: Type::Intrinsic(IntrinsicType::I64),
        }
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "RHS failure must roll back the already validated left producer"
    );
}

#[test]
fn grouped_nested_conjunctions_retain_independent_explicit_hir_nodes() {
    let hir = build("fn f(a: Bool, b: Bool, c: Bool) -> Bool { return a && (b && c); }")
        .expect("grouped nesting is represented");
    let returned = function(&hir, "f")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    let (_, right) = boolean_and(returned);
    let (nested_left, nested_right) = boolean_and(right);
    assert_eq!(nested_left.ty, Type::Intrinsic(IntrinsicType::Bool));
    assert_eq!(nested_right.ty, Type::Intrinsic(IntrinsicType::Bool));
}

#[test]
fn conjunction_is_accepted_by_existing_value_consumers_and_conditions() {
    let hir = build(
        "fn sink(value: Bool) {} \
         fn f(a: Bool, b: Bool) -> Bool { \
             let mut local: Bool = a && b; \
             local = a && b; \
             sink(a && b); \
             if a && b {} \
             while a && b { break; } \
             return a && b; \
         }",
    )
    .expect("conjunction remains an ordinary typed value producer");
    let f = function(&hir, "f");

    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    assert!(matches!(initializer.kind, ValueKind::BooleanAnd { .. }));

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    assert!(matches!(value.kind, ValueKind::BooleanAnd { .. }));

    let Statement::If { condition, .. } = &f.body.statements[3] else {
        panic!("expected if statement");
    };
    assert!(matches!(condition.kind, ValueKind::BooleanAnd { .. }));

    let Statement::While { condition, .. } = &f.body.statements[4] else {
        panic!("expected while statement");
    };
    assert!(matches!(condition.kind, ValueKind::BooleanAnd { .. }));

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    assert!(matches!(returned.kind, ValueKind::BooleanAnd { .. }));
}

#[test]
fn equality_stays_nested_inside_conjunction_hir() {
    let hir = build("fn f(a: Bool, b: Bool, c: Bool) -> Bool { return a == b && c; }")
        .expect("equality is tighter than conjunction");
    let returned = function(&hir, "f")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    let (left, _) = boolean_and(returned);
    assert!(matches!(left.kind, ValueKind::BooleanEquality { .. }));
}
