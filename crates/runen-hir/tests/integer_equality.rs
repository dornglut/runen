use runen_hir::{
    BooleanEqualityRelation, Diagnostic, DiagnosticKind, IntrinsicType, LiteralValue, ModuleId,
    SourceUnit, Statement, Type, TypedCompilation, Value, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<Diagnostic>> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
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

fn integer_comparison(value: &Value) -> (bool, Type, &Value, &Value) {
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::Bool));
    match &value.kind {
        ValueKind::IntegerEq {
            operand_type,
            left,
            right,
        } => (true, *operand_type, left, right),
        ValueKind::IntegerNe {
            operand_type,
            left,
            right,
        } => (false, *operand_type, left, right),
        _ => panic!("expected fixed-width integer comparison HIR"),
    }
}

fn unavailable_count(errors: &[Diagnostic]) -> usize {
    errors
        .iter()
        .filter(|error| error.kind == DiagnosticKind::UnavailableBinding)
        .count()
}

#[test]
fn all_fixed_width_integer_types_retain_exact_operand_type_for_eq_and_ne() {
    for (source_name, intrinsic) in [
        ("I8", IntrinsicType::I8),
        ("I16", IntrinsicType::I16),
        ("I32", IntrinsicType::I32),
        ("I64", IntrinsicType::I64),
        ("U8", IntrinsicType::U8),
        ("U16", IntrinsicType::U16),
        ("U32", IntrinsicType::U32),
        ("U64", IntrinsicType::U64),
    ] {
        let source = format!(
            "fn equal(left: {source_name}, right: {source_name}) -> Bool {{ return left == right; }} \
             fn different(left: {source_name}, right: {source_name}) -> Bool {{ return left != right; }}"
        );
        let hir = build(&source).expect("fixed-width integer equality is accepted");
        let expected = Type::Intrinsic(intrinsic);

        let (is_equal, operand_type, left, right) = integer_comparison(returned(&hir, "equal"));
        assert!(is_equal);
        assert_eq!(operand_type, expected);
        assert_eq!(left.ty, expected);
        assert_eq!(right.ty, expected);

        let (is_equal, operand_type, left, right) =
            integer_comparison(returned(&hir, "different"));
        assert!(!is_equal);
        assert_eq!(operand_type, expected);
        assert_eq!(left.ty, expected);
        assert_eq!(right.ty, expected);
    }
}

#[test]
fn exact_and_contextual_selection_is_symmetric_and_materializes_i32_literals() {
    let hir = build(
        "fn right_literal(value: I32) -> Bool { return value == 1; } \
         fn left_literal(value: I32) -> Bool { return 1 != value; }",
    )
    .expect("one exact I32 operand anchors one contextual integer literal");

    let (is_equal, operand_type, left, right) = integer_comparison(returned(&hir, "right_literal"));
    assert!(is_equal);
    assert_eq!(operand_type, Type::Intrinsic(IntrinsicType::I32));
    assert_eq!(left.ty, operand_type);
    assert!(matches!(right.kind, ValueKind::Literal(LiteralValue::I32(1))));

    let (is_equal, operand_type, left, right) = integer_comparison(returned(&hir, "left_literal"));
    assert!(!is_equal);
    assert_eq!(operand_type, Type::Intrinsic(IntrinsicType::I32));
    assert!(matches!(left.kind, ValueKind::Literal(LiteralValue::I32(1))));
    assert_eq!(right.ty, operand_type);
}

#[test]
fn conflicting_exact_integer_types_reject_before_producer_effects() {
    for right in ["I64", "U32"] {
        let source = format!(
            "record Ticket {{}} \
             fn select(ticket: Ticket, value: I32) -> I32 {{ return value; }} \
             fn sink(ticket: Ticket) {{}} \
             fn f(ticket: Ticket, other: {right}) {{ \
                 let result: Bool = select(ticket, true) == other; \
                 sink(ticket); \
             }}"
        );
        let errors = build(&source).expect_err("conflicting exact evidence must reject comparison");
        assert!(errors.iter().any(|error| matches!(
            error.kind,
            DiagnosticKind::EqualityOperandTypeConflict {
                left: Type::Intrinsic(IntrinsicType::I32),
                right: Type::Intrinsic(IntrinsicType::I64 | IntrinsicType::U32),
            }
        )));
        assert!(
            !errors.iter().any(|error| matches!(
                error.kind,
                DiagnosticKind::TypeMismatch { .. }
            )),
            "call arguments must not be validated while exact result evidence is collected"
        );
        assert_eq!(unavailable_count(&errors), 0);
    }
}

#[test]
fn two_contextual_operands_are_unanchored_without_default_numeric_type() {
    let errors = build("fn f() -> Bool { return 1 == 2; }")
        .expect_err("two contextual literals must not select a default integer type");
    assert!(errors
        .iter()
        .any(|error| error.kind == DiagnosticKind::EqualityOperandsUnanchored));
}

#[test]
fn direct_call_and_field_evidence_are_static_and_do_not_validate_receiver_arguments() {
    let call_errors = build(
        "record Ticket {} \
         fn select(ticket: Ticket, value: I32) -> I32 { return value; } \
         fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket, other: I64) { \
             let result: Bool = select(ticket, true) == other; \
             sink(ticket); \
         }",
    )
    .expect_err("I32 call result conflicts with exact I64 operand");
    assert!(call_errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::EqualityOperandTypeConflict {
            left: Type::Intrinsic(IntrinsicType::I32),
            right: Type::Intrinsic(IntrinsicType::I64),
        }
    )));
    assert!(!call_errors
        .iter()
        .any(|error| matches!(error.kind, DiagnosticKind::TypeMismatch { .. })));
    assert_eq!(unavailable_count(&call_errors), 0);

    let field_errors = build(
        "record Ticket {} \
         record Box { value: I32 } \
         fn make(ticket: Ticket, value: I32) -> Box { return Box { value: value }; } \
         fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket, other: I64) { \
             let result: Bool = make(ticket, true).value == other; \
             sink(ticket); \
         }",
    )
    .expect_err("statically resolved I32 field conflicts with exact I64 operand");
    assert!(field_errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::EqualityOperandTypeConflict {
            left: Type::Intrinsic(IntrinsicType::I32),
            right: Type::Intrinsic(IntrinsicType::I64),
        }
    )));
    assert!(!field_errors
        .iter()
        .any(|error| matches!(error.kind, DiagnosticKind::TypeMismatch { .. })));
    assert_eq!(unavailable_count(&field_errors), 0);
}

#[test]
fn grouping_is_transparent_but_contextual_producer_children_are_not_mined() {
    let hir = build(
        "fn anchored(x: I32, y: I32) -> Bool { return (x + 1) == (y); } \
         fn grouped(x: I32) -> Bool { return ((x)) == 1; }",
    )
    .expect("exact top-level evidence anchors grouped contextual producers");

    let (_, operand_type, left, right) = integer_comparison(returned(&hir, "anchored"));
    assert_eq!(operand_type, Type::Intrinsic(IntrinsicType::I32));
    assert!(matches!(left.kind, ValueKind::IntegerAdd { .. }));
    assert_eq!(right.ty, operand_type);

    let (_, operand_type, left, right) = integer_comparison(returned(&hir, "grouped"));
    assert_eq!(operand_type, Type::Intrinsic(IntrinsicType::I32));
    assert_eq!(left.ty, operand_type);
    assert!(matches!(right.kind, ValueKind::Literal(LiteralValue::I32(1))));

    let errors = build("fn unanchored(x: I32, y: I32) -> Bool { return (x + 1) == (y + 2); }")
        .expect_err("nested exact bindings inside contextual additions are not evidence anchors");
    assert!(errors
        .iter()
        .any(|error| error.kind == DiagnosticKind::EqualityOperandsUnanchored));
}

#[test]
fn invalid_evidence_and_later_operand_failure_commit_no_comparison_state() {
    let invalid = build(
        "record Ticket {} \
         fn produce(ticket: Ticket) -> I32 { return 1; } \
         fn sink(ticket: Ticket) {} \
         fn f(ticket: Ticket) { \
             let result: Bool = produce(ticket) == missing; \
             sink(ticket); \
         }",
    )
    .expect_err("invalid right evidence rejects before the left producer is validated");
    assert!(invalid
        .iter()
        .any(|error| error.kind == DiagnosticKind::UnresolvedName));
    assert_eq!(unavailable_count(&invalid), 0);

    let later_failure = build(
        "record Ticket {} \
         fn produce(ticket: Ticket) -> I32 { return 1; } \
         fn checked(ticket: Ticket, value: I32) -> I32 { return value; } \
         fn sink(ticket: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let result: Bool = produce(left) == checked(right, true); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("right operand validation failure rolls back the complete comparison transaction");
    assert!(later_failure.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I32),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    )));
    assert_eq!(unavailable_count(&later_failure), 0);
}

#[test]
fn record_safe_reference_and_raw_pointer_evidence_can_anchor_but_are_operation_invalid() {
    let record_errors = build(
        "record Ticket {} \
         record Box {} \
         fn make(ticket: Ticket) -> Box { return Box {}; } \
         fn sink(ticket: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let result: Bool = make(left) == make(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("same exact record type anchors comparison but is not equality-admissible");
    assert!(record_errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::EqualityRequiresBooleanOrInteger {
            operand_type: Type::Record(_),
        }
    )));
    assert_eq!(unavailable_count(&record_errors), 0);

    let reference_errors = build("fn f(left: I32, right: I32) -> Bool { return &left == &right; }")
        .expect_err("same Shared-reference type anchors but is not equality-admissible");
    assert!(reference_errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::EqualityRequiresBooleanOrInteger {
            operand_type: Type::SafeReference { .. },
        }
    )));

    let raw_errors = build(
        "fn f(left: I32, right: I32) -> Bool { return raw &left == raw &right; }",
    )
    .expect_err("same raw-pointer type anchors but is not equality-admissible");
    assert!(raw_errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::EqualityRequiresBooleanOrInteger {
            operand_type: Type::RawPointer(_),
        }
    )));
}

#[test]
fn integer_comparisons_are_bool_conditions_and_condition_context_never_anchors_literals() {
    let hir = build(
        "fn f(value: I32) { \
             if value == 1 {} \
             while value != 2 { break; } \
         }",
    )
    .expect("anchored integer comparison yields Bool for represented control flow");
    let f = function(&hir, "f");
    let Statement::If { condition, .. } = &f.body.statements[0] else {
        panic!("expected if statement");
    };
    assert_eq!(
        integer_comparison(condition).1,
        Type::Intrinsic(IntrinsicType::I32)
    );
    let Statement::While { condition, .. } = &f.body.statements[1] else {
        panic!("expected while statement");
    };
    assert_eq!(
        integer_comparison(condition).1,
        Type::Intrinsic(IntrinsicType::I32)
    );

    let errors = build("fn f() { if 1 == 2 {} }")
        .expect_err("condition Bool requirement does not select an integer operand type");
    assert!(errors
        .iter()
        .any(|error| error.kind == DiagnosticKind::EqualityOperandsUnanchored));
}

#[test]
fn boolean_equality_hir_remains_the_existing_relation() {
    let hir = build("fn f(left: Bool, right: Bool) -> Bool { return left != right; }")
        .expect("Boolean inequality remains valid");
    let value = returned(&hir, "f");
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::Bool));
    let ValueKind::BooleanEquality {
        relation,
        left,
        right,
    } = &value.kind
    else {
        panic!("expected existing Boolean-equality HIR relation");
    };
    assert_eq!(*relation, BooleanEqualityRelation::NotEqual);
    assert_eq!(left.ty, Type::Intrinsic(IntrinsicType::Bool));
    assert_eq!(right.ty, Type::Intrinsic(IntrinsicType::Bool));
}
