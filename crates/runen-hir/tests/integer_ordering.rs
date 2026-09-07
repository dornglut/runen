use runen_hir::{
    Diagnostic, DiagnosticKind, IntrinsicType, LiteralValue, ModuleId, SourceUnit, Statement, Type,
    TypedCompilation, Value, ValueKind, build_typed_hir,
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

fn integer_lt(value: &Value) -> (Type, &Value, &Value) {
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::Bool));
    let ValueKind::IntegerLt {
        operand_type,
        left,
        right,
    } = &value.kind
    else {
        panic!("expected fixed-width integer strict-order HIR");
    };
    (*operand_type, left, right)
}

fn unavailable_count(errors: &[Diagnostic]) -> usize {
    errors
        .iter()
        .filter(|error| error.kind == DiagnosticKind::UnavailableBinding)
        .count()
}

#[test]
fn all_fixed_width_integer_types_retain_exact_operand_type() {
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
            "fn ordered(left: {source_name}, right: {source_name}) -> Bool {{ return left < right; }}"
        );
        let hir = build(&source).expect("fixed-width integer strict ordering is accepted");
        let expected = Type::Intrinsic(intrinsic);
        let (operand_type, left, right) = integer_lt(returned(&hir, "ordered"));
        assert_eq!(operand_type, expected);
        assert_eq!(left.ty, expected);
        assert_eq!(right.ty, expected);
    }
}

#[test]
fn exact_and_contextual_selection_is_symmetric_and_materializes_i32_literals() {
    let hir = build(
        "fn right_literal(value: I32) -> Bool { return value < 1; } \
         fn left_literal(value: I32) -> Bool { return 1 < value; }",
    )
    .expect("one exact I32 operand anchors one contextual integer literal");

    let (operand_type, left, right) = integer_lt(returned(&hir, "right_literal"));
    assert_eq!(operand_type, Type::Intrinsic(IntrinsicType::I32));
    assert_eq!(left.ty, operand_type);
    assert!(matches!(
        right.kind,
        ValueKind::Literal(LiteralValue::I32(1))
    ));

    let (operand_type, left, right) = integer_lt(returned(&hir, "left_literal"));
    assert_eq!(operand_type, Type::Intrinsic(IntrinsicType::I32));
    assert!(matches!(
        left.kind,
        ValueKind::Literal(LiteralValue::I32(1))
    ));
    assert_eq!(right.ty, operand_type);
}

#[test]
fn two_contextual_operands_are_unanchored_without_default_numeric_type() {
    let errors = build("fn f() -> Bool { return 1 < 2; }")
        .expect_err("two contextual literals must not select a default integer type");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::IntegerOrderingOperandsUnanchored)
    );
    assert!(
        !errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::EqualityOperandsUnanchored)
    );
}

#[test]
fn conflicting_exact_types_reject_before_producer_validation_or_effects() {
    for right in ["I64", "U32"] {
        let source = format!(
            "record Ticket {{}} \
             fn select(ticket: Ticket, value: I32) -> I32 {{ return value; }} \
             fn sink(ticket: Ticket) {{}} \
             fn f(ticket: Ticket, other: {right}) {{ \
                 let result: Bool = select(ticket, true) < other; \
                 sink(ticket); \
             }}"
        );
        let errors = build(&source).expect_err("conflicting exact evidence must reject ordering");
        assert!(errors.iter().any(|error| matches!(
            error.kind,
            DiagnosticKind::IntegerOrderingOperandTypeConflict {
                left: Type::Intrinsic(IntrinsicType::I32),
                right: Type::Intrinsic(IntrinsicType::I64 | IntrinsicType::U32),
            }
        )));
        assert!(
            !errors
                .iter()
                .any(|error| matches!(error.kind, DiagnosticKind::TypeMismatch { .. })),
            "call arguments must not be validated while exact comparison evidence is collected"
        );
        assert_eq!(unavailable_count(&errors), 0);
    }
}

#[test]
fn contextual_producer_children_are_not_mined_for_ordering_anchors() {
    let errors = build("fn f(x: I32, y: I32) -> Bool { return (x + 1) < (y + 2); }")
        .expect_err("nested exact bindings inside contextual additions are not evidence anchors");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::IntegerOrderingOperandsUnanchored)
    );
}

#[test]
fn exact_non_integer_types_reject_before_operand_producer_effects() {
    for source in [
        "fn f(left: Bool, right: Bool) -> Bool { return left < right; }",
        "fn f(left: F32, right: F32) -> Bool { return left < right; }",
        "fn f(left: I32, right: I32) -> Bool { return &left < &right; }",
        "fn f(left: I32, right: I32) -> Bool { return raw &left < raw &right; }",
    ] {
        let errors = build(source).expect_err("exact non-integer ordering must reject");
        assert!(errors.iter().any(|error| matches!(
            error.kind,
            DiagnosticKind::IntegerOrderingRequiresInteger { .. }
        )));
    }

    let record_errors = build(
        "record Ticket {} \
         record Box {} \
         fn make(ticket: Ticket) -> Box { return Box {}; } \
         fn sink(ticket: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let result: Bool = make(left) < make(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("same exact record type anchors but is not ordering-admissible");
    assert!(record_errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::IntegerOrderingRequiresInteger {
            operand_type: Type::Record(_),
        }
    )));
    assert_eq!(unavailable_count(&record_errors), 0);
}

#[test]
fn valid_grouped_inner_ordering_is_exact_bool_evidence_for_outer_equality() {
    let hir = build(
        "fn equal(a: I32, b: I32, flag: Bool) -> Bool { return (a < b) == flag; } \
         fn different(a: I32, b: I32, flag: Bool) -> Bool { return flag != (a < b); }",
    )
    .expect("admissible grouped strict ordering supplies exact Bool evidence");

    for name in ["equal", "different"] {
        let value = returned(&hir, name);
        assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::Bool));
        let ValueKind::BooleanEquality { left, right, .. } = &value.kind else {
            panic!("expected outer Boolean equality relation");
        };
        assert!(
            matches!(left.kind, ValueKind::IntegerLt { .. })
                || matches!(right.kind, ValueKind::IntegerLt { .. })
        );
    }
}

#[test]
fn invalid_grouped_inner_ordering_cannot_be_repaired_by_outer_bool_anchor() {
    let unanchored = build("fn f(flag: Bool) -> Bool { return (1 < 2) == flag; }")
        .expect_err("outer Bool evidence must not repair unanchored inner ordering");
    assert!(
        unanchored
            .iter()
            .any(|error| error.kind == DiagnosticKind::IntegerOrderingOperandsUnanchored)
    );

    let inadmissible =
        build("fn f(left: F32, right: F32, flag: Bool) -> Bool { return flag == (left < right); }")
            .expect_err("outer Bool evidence must not repair non-integer inner ordering");
    assert!(inadmissible.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::IntegerOrderingRequiresInteger {
            operand_type: Type::Intrinsic(IntrinsicType::F32),
        }
    )));
}

#[test]
fn later_operand_failure_rolls_back_complete_ordering_transaction() {
    let errors = build(
        "record Ticket {} \
         fn produce(ticket: Ticket) -> I32 { return 1; } \
         fn checked(ticket: Ticket, value: I32) -> I32 { return value; } \
         fn sink(ticket: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let result: Bool = produce(left) < checked(right, true); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("right operand validation failure rolls back the complete ordering transaction");
    assert!(errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I32),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    )));
    assert_eq!(unavailable_count(&errors), 0);
}

#[test]
fn strict_ordering_is_bool_in_if_and_while_conditions() {
    let hir = build(
        "fn f(value: I32) { \
             if value < 1 {} \
             while value < 2 { break; } \
         }",
    )
    .expect("anchored integer strict ordering yields Bool for represented control flow");
    let f = function(&hir, "f");
    let Statement::If { condition, .. } = &f.body.statements[0] else {
        panic!("expected if statement");
    };
    assert_eq!(integer_lt(condition).0, Type::Intrinsic(IntrinsicType::I32));
    let Statement::While { condition, .. } = &f.body.statements[1] else {
        panic!("expected while statement");
    };
    assert_eq!(integer_lt(condition).0, Type::Intrinsic(IntrinsicType::I32));
}
