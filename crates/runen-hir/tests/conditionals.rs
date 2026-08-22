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

fn conditional(
    statement: &Statement,
) -> (
    &runen_hir::Value,
    &runen_hir::Block,
    Option<&runen_hir::Block>,
) {
    let Statement::If {
        condition,
        then_block,
        else_block,
        ..
    } = statement
    else {
        panic!("expected conditional statement");
    };
    (condition, then_block, else_block.as_ref())
}

#[test]
fn retains_exact_bool_condition_and_explicit_arm_blocks() {
    let hir = build("fn f(flag: Bool) { if flag {} else {} }").expect("valid conditional");
    let f = function(&hir, "f");
    let (condition, then_block, else_block) = conditional(&f.body.statements[0]);

    assert_eq!(condition.ty, Type::Intrinsic(IntrinsicType::Bool));
    let ValueKind::BindingUse { ownership, .. } = condition.kind else {
        panic!("expected retained binding condition");
    };
    assert_eq!(ownership, OwnedUse::Duplicate);
    assert!(then_block.statements.is_empty());
    assert!(then_block.normal_cleanup.is_empty());
    assert!(then_block.has_normal_continuation);
    assert!(
        else_block
            .expect("explicit else block")
            .normal_cleanup
            .is_empty()
    );
    assert!(else_block.expect("explicit else block").has_normal_continuation);
}

#[test]
fn integer_condition_is_rejected_through_exact_bool_required_type() {
    let errors = build("fn f() { if 1 {} }").expect_err("integer condition must fail Bool typing");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
}

#[test]
fn direct_call_and_field_conditions_retain_existing_value_forms() {
    let hir = build(
        "record State { ready: Bool } \
         fn ready() -> Bool { return true; } \
         fn call_condition() { if ready() {} } \
         fn field_condition(state: State) { if state.ready {} }",
    )
    .expect("represented Bool producers are valid conditions");

    let (call, _, _) = conditional(&function(&hir, "call_condition").body.statements[0]);
    assert!(matches!(call.kind, ValueKind::DirectCall { .. }));

    let (field, _, _) = conditional(&function(&hir, "field_condition").body.statements[0]);
    assert!(matches!(
        field.kind,
        ValueKind::FieldValueUse {
            ownership: OwnedUse::Duplicate,
            ..
        }
    ));
}

#[test]
fn condition_producer_ownership_is_applied_before_arm_validation() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             if predicate(value) { sink(value); } else {} \
         }",
    )
    .expect_err("condition consumption must be visible to both arm validators");

    assert!(has_diagnostic(&errors, DiagnosticKind::UnavailableBinding));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::ConditionalOwnershipMismatch),
        "an invalid arm must not be followed by a synthetic join mismatch"
    );
}

#[test]
fn failed_condition_does_not_commit_partial_condition_ownership() {
    let errors = build(
        "record Ticket {} \
         fn wrong(value: Ticket) -> I64 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             if wrong(value) {} \
             sink(value); \
         }",
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
        "failed condition validation must not leak speculative argument consumption"
    );
}

#[test]
fn literal_true_does_not_prune_invalid_false_outcome() {
    let errors = build("fn f() { if true {} else { missing(); } }")
        .expect_err("literal true must not exempt the false arm from validation");
    assert!(has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
}

#[test]
fn sibling_arm_locals_reuse_keys_with_distinct_binding_identities() {
    let hir = build("fn f(flag: Bool) { if flag { let x: I64 = 1; } else { let x: I64 = 2; } }")
        .expect("sibling arm scopes may reuse one local key");
    let f = function(&hir, "f");
    let (_, then_block, else_block) = conditional(&f.body.statements[0]);
    let Statement::Local {
        binding: then_binding,
        ..
    } = &then_block.statements[0]
    else {
        panic!("expected then local");
    };
    let Statement::Local {
        binding: else_binding,
        ..
    } = &else_block.expect("else block").statements[0]
    else {
        panic!("expected else local");
    };
    assert_ne!(then_binding, else_binding);
}

#[test]
fn arm_local_shadowing_and_escape_remain_rejected() {
    let shadowing = build("fn f(flag: Bool, x: I64) { if flag { let x: I64 = 1; } }")
        .expect_err("arm may not shadow an active enclosing binding");
    assert!(has_diagnostic(&shadowing, DiagnosticKind::LocalShadowing));

    let escaped = build("fn f(flag: Bool) { if flag { let child: I64 = 1; } let x: I64 = child; }")
        .expect_err("arm-local binding must end before normal continuation");
    assert!(has_diagnostic(&escaped, DiagnosticKind::UnresolvedName));
}

#[test]
fn arm_normal_cleanup_is_retained_before_join() {
    let hir = build(
        "record Ticket {} \
         fn make() -> Ticket { return Ticket {}; } \
         fn f(flag: Bool) { if flag { let ticket: Ticket = make(); } else {} }",
    )
    .expect("arm-local ownership is cleaned before the ownership join");
    let f = function(&hir, "f");
    let (_, then_block, _) = conditional(&f.body.statements[0]);
    let Statement::Local { binding, .. } = &then_block.statements[0] else {
        panic!("expected arm local");
    };
    assert_eq!(then_block.normal_cleanup.len(), 1);
    assert_eq!(then_block.normal_cleanup[0].binding, *binding);
    assert!(then_block.normal_cleanup[0].fields.is_empty());
}

#[test]
fn equal_complete_consumption_joins_but_one_outcome_consumption_rejects() {
    build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { if flag { sink(value); } else { sink(value); } }",
    )
    .expect("both outcomes consuming the same complete binding must join");

    let errors = build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { if flag { sink(value); } else {} }",
    )
    .expect_err("different complete ownership outcomes must reject");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ConditionalOwnershipMismatch
    ));
}

#[test]
fn equal_nested_partial_states_join_and_different_paths_reject() {
    build(
        "record Left {} record Right {} record Pair { left: Left, right: Right } \
         fn take_left(value: Left) {} \
         fn take_right(value: Right) {} \
         fn f(flag: Bool, pair: Pair) { \
             if flag { take_left(pair.left); } else { take_left(pair.left); } \
             take_right(pair.right); \
         }",
    )
    .expect("equal nested consumed-path sets must join and retain disjoint ownership");

    let errors = build(
        "record Left {} record Right {} record Pair { left: Left, right: Right } \
         fn take_left(value: Left) {} \
         fn take_right(value: Right) {} \
         fn f(flag: Bool, pair: Pair) { \
             if flag { take_left(pair.left); } else { take_right(pair.right); } \
         }",
    )
    .expect_err("different nested consumed paths must reject");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ConditionalOwnershipMismatch
    ));
}

#[test]
fn duplicable_use_differences_do_not_change_join_state() {
    build(
        "fn sink(value: I64) {} \
         fn f(flag: Bool, value: I64) { if flag { sink(value); } else {} sink(value); }",
    )
    .expect("duplicable-only arm differences leave ownership state equal");
}

#[test]
fn whole_binding_assignment_can_reconverge_ownership_histories() {
    build(
        "record Ticket {} \
         fn make() -> Ticket { return Ticket {}; } \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool) { \
             let mut value: Ticket = make(); \
             if flag { sink(value); value = make(); } else { value = make(); } \
             sink(value); \
         }",
    )
    .expect("explicit whole-binding replacement may restore equal complete ownership");
}

#[test]
fn branch_dependent_runtime_values_are_valid_when_ownership_state_is_equal() {
    build(
        "fn sink(value: I64) {} \
         fn f(flag: Bool) { \
             let mut value: I64 = 0; \
             if flag { value = 1; } else { value = 2; } \
             sink(value); \
         }",
    )
    .expect("runtime value equality is not a source ownership join requirement");
}

#[test]
fn omitted_else_uses_unchanged_post_condition_state() {
    build("fn f(flag: Bool, value: I64) { if flag { let copy: I64 = value; } }")
        .expect("duplicable then-arm use matches omitted false outcome state");

    let errors = build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { if flag { sink(value); } }",
    )
    .expect_err("consuming only the then outcome cannot match omitted else state");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ConditionalOwnershipMismatch
    ));
}

#[test]
fn rejected_join_does_not_commit_speculative_arm_ownership() {
    let errors = build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { \
             if flag { sink(value); } else {} \
             sink(value); \
         }",
    )
    .expect_err("join mismatch is invalid");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::ConditionalOwnershipMismatch
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "a rejected conditional must not leak speculative arm consumption into later validation"
    );
}

#[test]
fn invalid_arm_suppresses_secondary_join_mismatch_and_does_not_commit() {
    let errors = build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(flag: Bool, value: Ticket) { \
             if flag { sink(value); missing(); } else {} \
             sink(value); \
         }",
    )
    .expect_err("invalid arm must fail source validation");

    assert!(has_diagnostic(&errors, DiagnosticKind::UnresolvedName));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::ConditionalOwnershipMismatch),
        "join comparison requires two valid normal outcomes"
    );
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "invalid arm validation must not leak speculative ownership into later validation"
    );
}

#[test]
fn nested_conditionals_commit_inner_definite_state_before_outer_continuation() {
    build(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(a: Bool, b: Bool, value: Ticket) { \
             if a { \
                 if b { sink(value); } else { sink(value); } \
             } else { \
                 sink(value); \
             } \
         }",
    )
    .expect("nested conditional joins compose recursively");
}

#[test]
fn post_join_field_use_observes_the_committed_single_structural_state() {
    let errors = build(
        "record Left {} record Right {} record Pair { left: Left, right: Right } \
         fn take_left(value: Left) {} \
         fn take_right(value: Right) {} \
         fn f(flag: Bool, pair: Pair) { \
             if flag { take_left(pair.left); } else { take_left(pair.left); } \
             take_right(pair.right); \
             take_left(pair.left); \
         }",
    )
    .expect_err("joined consumed path must remain unavailable afterward");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::UnavailableFieldValue
    ));
}

#[test]
fn returning_outcome_is_not_compared_with_sole_normal_outcome() {
    build(
        "record Ticket {} \
         fn f(flag: Bool, value: Ticket) -> Ticket { \
             if flag { return value; } else {} \
             return value; \
         }",
    )
    .expect("returning ownership need not equal the sole normal outcome");
}

#[test]
fn returning_then_with_omitted_else_commits_unchanged_false_state() {
    build(
        "record Ticket {} \
         fn f(flag: Bool, value: Ticket) -> Ticket { \
             if flag { return value; } \
             return value; \
         }",
    )
    .expect("omitted else is the sole unchanged normal outcome when then returns");
}

#[test]
fn two_returning_arms_remove_normal_continuation_and_satisfy_result_obligation() {
    let hir = build(
        "fn f(flag: Bool, value: I64) -> I64 { \
             if flag { return value; } else { return value; } \
         }",
    )
    .expect("two returning arms terminate every represented static path");
    let f = function(&hir, "f");
    let (_, then_block, else_block) = conditional(&f.body.statements[0]);

    assert!(!then_block.has_normal_continuation);
    assert!(!else_block.expect("explicit else").has_normal_continuation);
    assert!(!f.body.has_normal_continuation);
    assert!(f.body.terminal_return.is_none());
}

#[test]
fn one_remaining_normal_path_without_later_return_still_requires_result() {
    let errors = build("fn f(flag: Bool) -> I64 { if flag { return 1; } else {} }")
        .expect_err("normal root-end path still requires a result return");

    assert!(has_diagnostic(&errors, DiagnosticKind::MissingResultReturn));
}

#[test]
fn semantic_unreachable_after_zero_normal_conditional_is_not_validated() {
    let errors = build("fn f(flag: Bool) { if flag { return; } else { return; } missing(); }")
        .expect_err("sibling after zero-normal conditional must be unreachable");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::UnreachableStatement
    ));
    assert!(
        !has_diagnostic(&errors, DiagnosticKind::UnresolvedName),
        "unreachable sibling must not be semantically validated"
    );
}

#[test]
fn nested_zero_one_two_normal_composition_is_recursive() {
    let hir = build(
        "fn f(a: Bool, b: Bool, value: I64) -> I64 { \
             if a { \
                 if b { return value; } else { return value; } \
             } else { \
                 return value; \
             } \
         }",
    )
    .expect("nested zero-normal conditional composes into the outer arm");

    assert!(!function(&hir, "f").body.has_normal_continuation);
}
