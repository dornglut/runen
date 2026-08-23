use runen_hir::{
    CleanupPath, DiagnosticKind, IntrinsicType, ModuleId, OwnedUse, SourceUnit, Statement, Type,
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

fn transfer_cleanup(statement: &Statement) -> &[CleanupPath] {
    match statement {
        Statement::Break { cleanup, .. } | Statement::Continue { cleanup, .. } => cleanup,
        _ => panic!("expected loop transfer statement"),
    }
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
fn transfer_syntax_outside_loop_is_source_invalid() {
    let break_errors = build("fn f() { break; }").expect_err("break requires an enclosing while");
    assert!(has_diagnostic(
        &break_errors,
        DiagnosticKind::BreakOutsideLoop
    ));

    let continue_errors =
        build("fn f() { continue; }").expect_err("continue requires an enclosing while");
    assert!(has_diagnostic(
        &continue_errors,
        DiagnosticKind::ContinueOutsideLoop
    ));
}

#[test]
fn admitted_transfer_makes_later_same_block_statement_unreachable() {
    let break_errors = build("fn sink() {} fn f(flag: Bool) { while flag { break; sink(); } }")
        .expect_err("later sibling after break is unreachable");
    assert!(has_diagnostic(
        &break_errors,
        DiagnosticKind::UnreachableStatement
    ));

    let continue_errors =
        build("fn sink() {} fn f(flag: Bool) { while flag { continue; sink(); } }")
            .expect_err("later sibling after continue is unreachable");
    assert!(has_diagnostic(
        &continue_errors,
        DiagnosticKind::UnreachableStatement
    ));
}

#[test]
fn direct_break_and_continue_are_admitted_against_nearest_loop_state() {
    build("fn f(flag: Bool) { while flag { break; } }").expect("break matches loop C");
    build("fn f(flag: Bool) { while flag { continue; } }").expect("continue matches loop H");
}

#[test]
fn transfer_cleanup_is_innermost_first_and_reverse_declaration_order_per_scope() {
    let hir = build(
        "record Box { value: I64 } \
         fn f(flag: Bool) { \
             while flag { \
                 let outer_a: Box = Box { value: 1 }; \
                 let outer_b: Box = Box { value: 2 }; \
                 { \
                     let inner_a: Box = Box { value: 3 }; \
                     let inner_b: Box = Box { value: 4 }; \
                     break; \
                 } \
             } \
         }",
    )
    .expect("nested transfer cleanup is valid");
    let f = function(&hir, "f");
    let (_, body) = while_statement(&f.body.statements[0]);
    let Statement::Local {
        binding: outer_a, ..
    } = &body.statements[0]
    else {
        panic!("outer_a");
    };
    let Statement::Local {
        binding: outer_b, ..
    } = &body.statements[1]
    else {
        panic!("outer_b");
    };
    let Statement::Block(inner) = &body.statements[2] else {
        panic!("nested block");
    };
    let Statement::Local {
        binding: inner_a, ..
    } = &inner.statements[0]
    else {
        panic!("inner_a");
    };
    let Statement::Local {
        binding: inner_b, ..
    } = &inner.statements[1]
    else {
        panic!("inner_b");
    };
    let cleanup = transfer_cleanup(&inner.statements[2]);
    assert_eq!(cleanup.len(), 4);
    assert_eq!(
        cleanup.iter().map(|path| path.binding).collect::<Vec<_>>(),
        vec![*inner_b, *inner_a, *outer_b, *outer_a]
    );
    assert!(cleanup.iter().all(|path| path.fields.is_empty()));
}

#[test]
fn transfer_cleanup_uses_only_remaining_child_ownership() {
    let hir = build(
        "record Left { value: I64 } \
         record Right { value: I64 } \
         record Pair { left: Left, right: Right } \
         fn take(value: Left) {} \
         fn f(flag: Bool) { \
             while flag { \
                 let child: Pair = Pair { \
                     left: Left { value: 1 }, \
                     right: Right { value: 2 } \
                 }; \
                 take(child.left); \
                 break; \
             } \
         }",
    )
    .expect("partial body-local ownership can be cleaned on transfer");
    let f = function(&hir, "f");
    let (_, body) = while_statement(&f.body.statements[0]);
    let Statement::Local { binding: child, .. } = &body.statements[0] else {
        panic!("child local");
    };
    let cleanup = transfer_cleanup(&body.statements[2]);
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].binding, *child);
    assert_eq!(cleanup[0].fields, vec![1]);
}

#[test]
fn completely_consumed_child_is_not_cleaned_twice() {
    let hir = build(
        "record Ticket { value: I64 } \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool) { \
             while flag { \
                 let child: Ticket = Ticket { value: 1 }; \
                 sink(child); \
                 break; \
             } \
         }",
    )
    .expect("consumed child needs no transfer cleanup");
    let f = function(&hir, "f");
    let (_, body) = while_statement(&f.body.statements[0]);
    assert!(transfer_cleanup(&body.statements[2]).is_empty());
}

#[test]
fn zero_leaf_child_cleanup_is_retained_in_hir() {
    let hir = build(
        "record Empty {} \
         fn f(flag: Bool) { while flag { let child: Empty = Empty {}; break; } }",
    )
    .expect("zero-leaf cleanup is structurally represented");
    let f = function(&hir, "f");
    let (_, body) = while_statement(&f.body.statements[0]);
    let Statement::Local { binding, .. } = &body.statements[0] else {
        panic!("child local");
    };
    let cleanup = transfer_cleanup(&body.statements[1]);
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].binding, *binding);
    assert!(cleanup[0].fields.is_empty());
}

#[test]
fn transfer_cleanup_does_not_include_enclosing_target_bindings() {
    let hir = build(
        "record Box { value: I64 } \
         fn f(flag: Bool) { \
             let parent: Box = Box { value: 0 }; \
             while flag { let child: Box = Box { value: 1 }; break; } \
         }",
    )
    .expect("enclosing binding is target state, not transfer cleanup");
    let f = function(&hir, "f");
    let Statement::Local {
        binding: parent, ..
    } = &f.body.statements[0]
    else {
        panic!("parent local");
    };
    let (_, body) = while_statement(&f.body.statements[1]);
    let Statement::Local { binding: child, .. } = &body.statements[0] else {
        panic!("child local");
    };
    let cleanup = transfer_cleanup(&body.statements[1]);
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].binding, *child);
    assert_ne!(cleanup[0].binding, *parent);
}

#[test]
fn continue_requires_exact_loop_head_ownership_and_can_be_explicitly_restored() {
    let errors = build(
        "record Ticket { value: I64 } \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { while flag { sink(value); continue; } }",
    )
    .expect_err("continue may not lose an enclosing head value");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ContinueOwnershipMismatch
    ));

    build(
        "record Ticket { value: I64 } \
         fn make() -> Ticket { return Ticket { value: 1 }; } \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool) { \
             let mut value: Ticket = make(); \
             while flag { sink(value); value = make(); continue; } \
         }",
    )
    .expect("mutable assignment may explicitly restore H before continue");
}

#[test]
fn continue_checks_h_not_post_condition_c() {
    let errors = build(
        "record Ticket { value: I64 } \
         fn make() -> Ticket { return Ticket { value: 1 }; } \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn f() { \
             let mut value: Ticket = make(); \
             while predicate(value) { continue; } \
         }",
    )
    .expect_err("condition consumption makes C differ from H");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ContinueOwnershipMismatch
    ));

    build(
        "record Ticket { value: I64 } \
         fn make() -> Ticket { return Ticket { value: 1 }; } \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn f() { \
             let mut value: Ticket = make(); \
             while predicate(value) { value = make(); continue; } \
         }",
    )
    .expect("explicit restoration of H admits continue");
}

#[test]
fn break_requires_exact_post_condition_c() {
    build(
        "record Ticket { value: I64 } \
         fn make() -> Ticket { return Ticket { value: 1 }; } \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn f() { \
             let mut value: Ticket = make(); \
             while predicate(value) { break; } \
         }",
    )
    .expect("immediate break matches consumed post-condition C");

    let errors = build(
        "record Ticket { value: I64 } \
         fn make() -> Ticket { return Ticket { value: 1 }; } \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn f() { \
             let mut value: Ticket = make(); \
             while predicate(value) { value = make(); break; } \
         }",
    )
    .expect_err("restoring H does not manufacture break admission against C");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::BreakOwnershipMismatch
    ));
}

#[test]
fn nested_partial_target_mismatch_is_rejected_for_each_transfer_kind() {
    let continue_errors = build(
        "record Left { value: I64 } record Right { value: I64 } \
         record Pair { left: Left, right: Right } \
         fn take(value: Left) {} \
         fn f(flag: Bool, pair: Pair) { while flag { take(pair.left); continue; } }",
    )
    .expect_err("partial loss cannot continue to H");
    assert!(has_diagnostic(
        &continue_errors,
        DiagnosticKind::ContinueOwnershipMismatch
    ));

    let break_errors = build(
        "record Left { value: I64 } record Right { value: I64 } \
         record Pair { left: Left, right: Right } \
         fn take(value: Left) {} \
         fn f(flag: Bool, pair: Pair) { while flag { take(pair.left); break; } }",
    )
    .expect_err("partial loss cannot break to C");
    assert!(has_diagnostic(
        &break_errors,
        DiagnosticKind::BreakOwnershipMismatch
    ));
}

#[test]
fn runtime_value_changes_with_equal_structural_state_remain_valid() {
    build("fn f(flag: Bool) { let mut n: I64 = 0; while flag { n = 1; continue; } }")
        .expect("continue compares structural ownership, not runtime value");
    build("fn f(flag: Bool) { let mut n: I64 = 0; while flag { n = 1; break; } }")
        .expect("break compares structural ownership, not runtime value");
}

#[test]
fn conditional_transfer_composition_uses_only_local_normal_fallthrough() {
    build("fn f(a: Bool, b: Bool) { while a { if b { continue; } else {} } }")
        .expect("normal arm is the sole local successor");
    build("fn f(a: Bool, b: Bool) { while a { if b { break; } else { continue; } } }")
        .expect("two transfer arms have no local successor");
    build("fn f(a: Bool, b: Bool) { while a { if b { break; } } }")
        .expect("omitted else remains a normal false outcome");
}

#[test]
fn inner_loop_transfer_cleanup_stops_at_inner_body_scope() {
    let hir = build(
        "record Box { value: I64 } \
         fn f(a: Bool, b: Bool) { \
             while a { \
                 let outer: Box = Box { value: 1 }; \
                 while b { let inner: Box = Box { value: 2 }; break; } \
             } \
         }",
    )
    .expect("inner transfer targets only the inner loop");
    let f = function(&hir, "f");
    let (_, outer_body) = while_statement(&f.body.statements[0]);
    let Statement::Local { binding: outer, .. } = &outer_body.statements[0] else {
        panic!("outer local");
    };
    let (_, inner_body) = while_statement(&outer_body.statements[1]);
    let Statement::Local { binding: inner, .. } = &inner_body.statements[0] else {
        panic!("inner local");
    };
    let cleanup = transfer_cleanup(&inner_body.statements[1]);
    assert_eq!(cleanup.len(), 1);
    assert_eq!(cleanup[0].binding, *inner);
    assert_ne!(cleanup[0].binding, *outer);
}

#[test]
fn integer_condition_is_rejected_through_exact_bool_required_type() {
    let errors =
        build("fn f() { while 1 {} }").expect_err("integer condition must fail Bool typing");
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
    let errors =
        build("fn f(flag: Bool) { while flag { let child: I64 = 1; } let x: I64 = child; }")
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
