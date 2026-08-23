use runen_hir::{
    BooleanEqualityRelation, Diagnostic, DiagnosticKind, IntrinsicType, ModuleId, OwnedUse,
    SourceUnit, Statement, Type, TypedCompilation, Value, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<Diagnostic>> {
    let parsed = parse(source);
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

fn has_diagnostic(errors: &[Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

fn unavailable_count(errors: &[Diagnostic]) -> usize {
    errors
        .iter()
        .filter(|error| error.kind == DiagnosticKind::UnavailableBinding)
        .count()
}

fn boolean_not(value: &Value) -> &Value {
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::Bool));
    let ValueKind::BooleanNot { operand } = &value.kind else {
        panic!("expected Boolean-not HIR value");
    };
    operand
}

fn boolean_equality(value: &Value) -> (BooleanEqualityRelation, &Value, &Value) {
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::Bool));
    let ValueKind::BooleanEquality {
        relation,
        left,
        right,
    } = &value.kind
    else {
        panic!("expected Boolean-equality HIR value");
    };
    assert_eq!(left.ty, Type::Intrinsic(IntrinsicType::Bool));
    assert_eq!(right.ty, Type::Intrinsic(IntrinsicType::Bool));
    (*relation, left, right)
}

#[test]
fn retains_bool_literal_and_nested_boolean_not_structure() {
    let hir = build(
        "fn literal() -> Bool { return !true; } \
         fn nested(flag: Bool) -> Bool { return !!flag; }",
    )
    .expect("Boolean-not values are valid");

    let literal = function(&hir, "literal")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("literal return value");
    let literal_operand = boolean_not(literal);
    assert!(matches!(
        literal_operand.kind,
        ValueKind::Literal(runen_hir::LiteralValue::Bool(true))
    ));

    let nested = function(&hir, "nested")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("nested return value");
    let inner = boolean_not(nested);
    let binding = boolean_not(inner);
    assert!(matches!(
        binding.kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Duplicate,
            ..
        }
    ));
}

#[test]
fn outer_non_bool_requirement_rejects_before_operand_validation_or_consumption() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: I64 = !predicate(value); \
             sink(value); \
         }",
    )
    .expect_err("Boolean-not result cannot satisfy I64");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I64),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "rejected outer result must not validate or consume through its operand"
    );
}

#[test]
fn non_bool_operand_is_rejected_under_exact_bool_requirement() {
    let errors = build("fn f(value: I64) { let negated: Bool = !value; }")
        .expect_err("I64 operand cannot be negated as Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::Bool),
            found: Type::Intrinsic(IntrinsicType::I64),
        }
    ));
}

#[test]
fn failed_operand_validation_rolls_back_partial_argument_consumption() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket, flag: Bool) -> Bool { return flag; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let negated: Bool = !predicate(value, 1); \
             sink(value); \
         }",
    )
    .expect_err("second operand argument has the wrong source type");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "failed operand transaction must not commit the first argument consumption"
    );
}

#[test]
fn successful_operand_effects_commit_once_and_operator_adds_no_extra_transition() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let negated: Bool = !predicate(value); \
             sink(value); \
         }",
    )
    .expect_err("successful operand call consumes its Ticket argument");
    assert_eq!(unavailable_count(&errors), 1);

    build("fn f(flag: Bool) { let negated: Bool = !flag; let still_available: Bool = flag; }")
        .expect("operator itself must not consume a duplicable Bool binding");
}

#[test]
fn direct_call_and_field_operands_retain_existing_value_semantics() {
    let hir = build(
        "record Ticket {} \
         record State { ready: Bool } \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn call(value: Ticket) -> Bool { return !predicate(value); } \
         fn field(state: State) -> Bool { return !state.ready; }",
    )
    .expect("represented Bool producers remain valid Boolean-not operands");

    let call = function(&hir, "call")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("call-backed return value");
    let call_operand = boolean_not(call);
    let ValueKind::DirectCall { arguments, .. } = &call_operand.kind else {
        panic!("expected direct-call operand");
    };
    assert_eq!(arguments.len(), 1);
    assert!(matches!(
        arguments[0].kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Consume,
            ..
        }
    ));

    let field = function(&hir, "field")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("field-backed return value");
    let field_operand = boolean_not(field);
    assert!(matches!(
        field_operand.kind,
        ValueKind::FieldValueUse {
            ownership: OwnedUse::Duplicate,
            ..
        }
    ));
}

#[test]
fn boolean_not_flows_through_all_generic_value_consumers() {
    let hir = build(
        "record Flags { value: Bool } \
         fn sink(value: Bool) {} \
         fn f(flag: Bool) -> Bool { \
             let mut local: Bool = !flag; \
             local = !local; \
             sink(!local); \
             let flags: Flags = Flags { value: !local }; \
             return !flags.value; \
         }",
    )
    .expect("Boolean-not composes through generic Value receiving paths");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    boolean_not(initializer);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    boolean_not(value);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    boolean_not(&arguments[0]);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    boolean_not(&fields[0].value);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    boolean_not(returned);
}

#[test]
fn if_and_while_reuse_the_same_boolean_not_hir_and_commit_operand_state() {
    let hir = build("fn control(flag: Bool) { if !flag {} while !!flag { break; } }")
        .expect("Boolean-not conditions are represented through the generic Value kind");
    let control = function(&hir, "control");
    let Statement::If { condition, .. } = &control.body.statements[0] else {
        panic!("expected if statement");
    };
    boolean_not(condition);
    let Statement::While { condition, .. } = &control.body.statements[1] else {
        panic!("expected while statement");
    };
    boolean_not(boolean_not(condition));

    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { if !predicate(value) {} sink(value); }",
    )
    .expect_err("successful condition operand consumption must become the post-condition state");
    assert_eq!(unavailable_count(&errors), 1);
}

#[test]
fn boolean_equality_retains_relation_identity_and_exact_bool_operands() {
    let hir = build(
        "fn equal(a: Bool, b: Bool) -> Bool { return !a == b; } \
         fn different(a: Bool, b: Bool) -> Bool { return a != !b; }",
    )
    .expect("both Boolean equality relations are represented");

    let equal = function(&hir, "equal")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("equal return value");
    let (relation, left, right) = boolean_equality(equal);
    assert_eq!(relation, BooleanEqualityRelation::Equal);
    boolean_not(left);
    assert!(matches!(right.kind, ValueKind::BindingUse { .. }));

    let different = function(&hir, "different")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("different return value");
    let (relation, left, right) = boolean_equality(different);
    assert_eq!(relation, BooleanEqualityRelation::NotEqual);
    assert!(matches!(left.kind, ValueKind::BindingUse { .. }));
    boolean_not(right);
}

#[test]
fn equality_outer_non_bool_requirement_rejects_before_either_operand_validation() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let wrong: I64 = predicate(left) == predicate(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("Boolean equality result cannot satisfy I64");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I64),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "outer result rejection must occur before either consuming call operand is validated"
    );
}

#[test]
fn equality_rejects_non_bool_left_and_right_under_exact_bool_requirements() {
    let left_errors = build("fn f(value: I64, flag: Bool) { let result: Bool = value == flag; }")
        .expect_err("I64 left operand cannot satisfy exact Bool");
    assert!(has_diagnostic(
        &left_errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::Bool),
            found: Type::Intrinsic(IntrinsicType::I64),
        }
    ));

    let right_errors = build("fn f(flag: Bool, value: I64) { let result: Bool = flag != value; }")
        .expect_err("I64 right operand cannot satisfy exact Bool");
    assert!(has_diagnostic(
        &right_errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::Bool),
            found: Type::Intrinsic(IntrinsicType::I64),
        }
    ));
}

#[test]
fn equality_left_failure_commits_no_partial_ownership() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket, flag: Bool) -> Bool { return flag; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let result: Bool = predicate(value, 1) == false; \
             sink(value); \
         }",
    )
    .expect_err("left call fails after speculatively consuming its first argument");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(unavailable_count(&errors), 0);
}

#[test]
fn equality_right_failure_rolls_back_successful_left_and_partial_right_effects() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn checked(value: Ticket, flag: Bool) -> Bool { return flag; } \
         fn sink(value: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let result: Bool = predicate(left) == checked(right, 1); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("right validation failure must roll back the entire two-operand transaction");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "right failure must roll back both the successful left consumption and partial right consumption"
    );
}

#[test]
fn equality_success_commits_left_then_right_once_and_adds_no_operator_transition() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let result: Bool = predicate(left) != predicate(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("successful equality operands consume both Ticket arguments");
    assert_eq!(unavailable_count(&errors), 2);

    build(
        "fn f(left: Bool, right: Bool) { \
             let compared: Bool = left == right; \
             let left_again: Bool = left; \
             let right_again: Bool = right; \
         }",
    )
    .expect("equality itself adds no transition beyond its duplicable Bool operands");
}

#[test]
fn equality_operands_reuse_call_field_construction_and_prefix_semantics() {
    let hir = build(
        "record Ticket {} \
         record State { ready: Bool } \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn f(value: Ticket, state: State) -> Bool { return predicate(value) == !state.ready; }",
    )
    .expect("call, field, and prefix producers retain their existing equality operand semantics");

    let returned = function(&hir, "f")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    let (_, left, right) = boolean_equality(returned);
    let ValueKind::DirectCall { arguments, .. } = &left.kind else {
        panic!("expected direct-call left operand");
    };
    assert!(matches!(
        arguments[0].kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Consume,
            ..
        }
    ));
    let field = boolean_not(right);
    assert!(matches!(
        field.kind,
        ValueKind::FieldValueUse {
            ownership: OwnedUse::Duplicate,
            ..
        }
    ));

    let construction_errors = build(
        "record Flag { ready: Bool } fn bad(flag: Bool) { let result: Bool = Flag { ready: true } == flag; }",
    )
    .expect_err("ordinary record construction remains syntactically admitted but is not Bool");
    assert!(construction_errors.iter().any(|error| {
        matches!(
            error.kind,
            DiagnosticKind::TypeMismatch {
                expected: Type::Intrinsic(IntrinsicType::Bool),
                found: Type::Record(_),
            }
        )
    }));
}

#[test]
fn boolean_equality_flows_through_all_generic_value_consumers() {
    let hir = build(
        "record Flags { value: Bool } \
         fn sink(value: Bool) {} \
         fn f(a: Bool, b: Bool) -> Bool { \
             let mut local: Bool = a == b; \
             local = a != b; \
             sink(a == b); \
             let flags: Flags = Flags { value: a != b }; \
             if a == b {} \
             while a != b { break; } \
             return flags.value == local; \
         }",
    )
    .expect("Boolean equality composes through every generic Value receiving path");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    boolean_equality(initializer);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    boolean_equality(value);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    boolean_equality(&arguments[0]);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    boolean_equality(&fields[0].value);

    let Statement::If { condition, .. } = &f.body.statements[4] else {
        panic!("expected if statement");
    };
    boolean_equality(condition);

    let Statement::While { condition, .. } = &f.body.statements[5] else {
        panic!("expected while statement");
    };
    boolean_equality(condition);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    boolean_equality(returned);
}

#[test]
fn equality_condition_commits_the_sequential_post_right_ownership_state() {
    let errors = build(
        "record Ticket {} \
         fn predicate(value: Ticket) -> Bool { return true; } \
         fn sink(value: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             if predicate(left) == predicate(right) {} \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("both successful condition operands are consumed before branch validation");
    assert_eq!(unavailable_count(&errors), 2);
}
