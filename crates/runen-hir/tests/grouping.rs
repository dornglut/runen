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

fn returned<'a>(hir: &'a TypedCompilation, name: &str) -> &'a Value {
    function(hir, name)
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .unwrap_or_else(|| panic!("missing return value for {name}"))
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

fn equality(value: &Value) -> (BooleanEqualityRelation, &Value, &Value) {
    let ValueKind::BooleanEquality {
        relation,
        left,
        right,
    } = &value.kind
    else {
        panic!("expected Boolean equality HIR");
    };
    (*relation, left, right)
}

#[test]
fn grouping_erases_to_existing_hir_value_kinds_for_all_existing_producers() {
    let hir = build(
        r#"
record Flag { ready: Bool }
fn make() -> Bool { return true; }
fn literal() -> Bool { return (true); }
fn binding(flag: Bool) -> Bool { return (flag); }
fn call() -> Bool { return (make()); }
fn field(root: Flag) -> Bool { return (root.ready); }
fn construction() -> Flag { return (Flag { ready: true }); }
"#,
    )
    .expect("grouping is transparent for existing value producers");

    assert!(matches!(
        returned(&hir, "literal").kind,
        ValueKind::Literal(runen_hir::LiteralValue::Bool(true))
    ));
    assert!(matches!(
        returned(&hir, "binding").kind,
        ValueKind::BindingUse {
            ownership: OwnedUse::Duplicate,
            ..
        }
    ));
    assert!(matches!(
        returned(&hir, "call").kind,
        ValueKind::DirectCall { .. }
    ));
    assert!(matches!(
        returned(&hir, "field").kind,
        ValueKind::FieldValueUse {
            ownership: OwnedUse::Duplicate,
            ..
        }
    ));
    assert!(matches!(
        returned(&hir, "construction").kind,
        ValueKind::RecordConstruction { .. }
    ));
}

#[test]
fn grouped_operators_retain_existing_hir_shapes_and_nested_relation_identity() {
    let hir = build(
        r#"
fn negated(a: Bool, b: Bool) -> Bool { return !(a == b); }
fn left(a: Bool, b: Bool, c: Bool) -> Bool { return (a == b) == c; }
fn right(a: Bool, b: Bool, c: Bool) -> Bool { return a == (b != c); }
"#,
    )
    .expect("explicit grouped operator nesting builds existing HIR operators");

    let ValueKind::BooleanNot { operand } = &returned(&hir, "negated").kind else {
        panic!("expected Boolean-not HIR");
    };
    let (relation, _, _) = equality(operand);
    assert_eq!(relation, BooleanEqualityRelation::Equal);

    let (relation, left, _) = equality(returned(&hir, "left"));
    assert_eq!(relation, BooleanEqualityRelation::Equal);
    let (inner, _, _) = equality(left);
    assert_eq!(inner, BooleanEqualityRelation::Equal);

    let (relation, _, right) = equality(returned(&hir, "right"));
    assert_eq!(relation, BooleanEqualityRelation::Equal);
    let (inner, _, _) = equality(right);
    assert_eq!(inner, BooleanEqualityRelation::NotEqual);
}

#[test]
fn grouping_preserves_outer_required_type_and_existing_operator_transactions() {
    let errors = build(
        r#"
record Ticket {}
fn predicate(value: Ticket) -> Bool { return true; }
fn sink(value: Ticket) {}
fn wrong(value: Ticket) {
    let result: I64 = (predicate(value) == true);
    sink(value);
}
"#,
    )
    .expect_err("grouped Bool equality cannot satisfy I64");
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
        "grouping must not cause operand validation before equality outer-type admission"
    );

    let errors = build(
        r#"
record Ticket {}
fn checked(value: Ticket, flag: Bool) -> Bool { return flag; }
fn sink(value: Ticket) {}
fn failed(value: Ticket) {
    let result: Bool = (!checked(value, 1));
    sink(value);
}
"#,
    )
    .expect_err("grouped Boolean-not retains its operand transaction");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "grouping must not interfere with Boolean-not rollback"
    );
}

#[test]
fn grouping_adds_no_ownership_transition_around_consuming_producers() {
    let errors = build(
        r#"
record Ticket {}
fn predicate(value: Ticket) -> Bool { return true; }
fn sink(value: Ticket) {}
fn f(value: Ticket) {
    let result: Bool = (predicate(value));
    sink(value);
}
"#,
    )
    .expect_err("the contained direct call consumes its Ticket argument once");
    assert_eq!(unavailable_count(&errors), 1);

    build(
        "fn f(flag: Bool) { let grouped: Bool = (((flag))); let still_available: Bool = flag; }",
    )
    .expect("grouping adds no consumption to a duplicable binding use");
}

#[test]
fn grouping_flows_through_all_generic_value_consumers_without_special_hir_rules() {
    let hir = build(
        r#"
record Boxed { value: Bool }
fn sink(value: Bool) {}
fn f(flag: Bool) -> Bool {
    let mut local: Bool = (flag);
    local = ((local));
    sink((local));
    let boxed: Boxed = Boxed { value: (local) };
    return (boxed.value);
}
"#,
    )
    .expect("generic Value consumers accept transparent grouping");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    assert!(matches!(initializer.kind, ValueKind::BindingUse { .. }));

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    assert!(matches!(value.kind, ValueKind::BindingUse { .. }));

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    assert!(matches!(arguments[0].kind, ValueKind::BindingUse { .. }));

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    assert!(matches!(fields[0].value.kind, ValueKind::BindingUse { .. }));

    assert!(matches!(
        returned(&hir, "f").kind,
        ValueKind::FieldValueUse { .. }
    ));
}

#[test]
fn grouped_conditions_reuse_existing_condition_values_and_post_condition_ownership() {
    let hir = build("fn control(flag: Bool) { if ((flag)) {} while (flag) { break; } }")
        .expect("grouped conditions erase to existing HIR values");
    let control = function(&hir, "control");
    let Statement::If { condition, .. } = &control.body.statements[0] else {
        panic!("expected if statement");
    };
    assert!(matches!(condition.kind, ValueKind::BindingUse { .. }));
    let Statement::While { condition, .. } = &control.body.statements[1] else {
        panic!("expected while statement");
    };
    assert!(matches!(condition.kind, ValueKind::BindingUse { .. }));

    let errors = build(
        r#"
record Ticket {}
fn predicate(value: Ticket) -> Bool { return true; }
fn sink(value: Ticket) {}
fn f(value: Ticket) { if (predicate(value)) {} sink(value); }
"#,
    )
    .expect_err("contained condition producer consumes its Ticket argument");
    assert_eq!(unavailable_count(&errors), 1);
}
