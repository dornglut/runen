use runen_hir::{
    Diagnostic, DiagnosticKind, IntrinsicType, LiteralValue, ModuleId, OwnedUse, SourceUnit,
    Statement, Type, TypedCompilation, Value, ValueKind, build_typed_hir,
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

fn integer_neg(value: &Value, ty: IntrinsicType) -> &Value {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::IntegerNeg { operand } = &value.kind else {
        panic!("expected IntegerNeg HIR value");
    };
    assert_eq!(operand.ty, expected);
    operand
}

#[test]
fn all_eight_fixed_width_integer_types_retain_explicit_negation_hir() {
    let hir = build(
        "fn i8_neg(a: I8) -> I8 { return -a; } \
         fn i16_neg(a: I16) -> I16 { return -a; } \
         fn i32_neg(a: I32) -> I32 { return -a; } \
         fn i64_neg(a: I64) -> I64 { return -a; } \
         fn u8_neg(a: U8) -> U8 { return -a; } \
         fn u16_neg(a: U16) -> U16 { return -a; } \
         fn u32_neg(a: U32) -> U32 { return -a; } \
         fn u64_neg(a: U64) -> U64 { return -a; }",
    )
    .expect("all fixed-width integer negations are valid");

    for (name, ty) in [
        ("i8_neg", IntrinsicType::I8),
        ("i16_neg", IntrinsicType::I16),
        ("i32_neg", IntrinsicType::I32),
        ("i64_neg", IntrinsicType::I64),
        ("u8_neg", IntrinsicType::U8),
        ("u16_neg", IntrinsicType::U16),
        ("u32_neg", IntrinsicType::U32),
        ("u64_neg", IntrinsicType::U64),
    ] {
        let value = function(&hir, name)
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("integer-neg return value");
        let operand = integer_neg(value, ty);
        assert!(matches!(
            operand.kind,
            ValueKind::BindingUse {
                ownership: OwnedUse::Duplicate,
                ..
            }
        ));
    }
}

#[test]
fn non_integer_outer_requirement_rejects_before_operand_validation_or_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: Bool = -take(value); \
             sink(value); \
         }",
    )
    .expect_err("integer negation cannot satisfy Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerNegationRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "outer rejection must happen before the consuming operand is validated"
    );
}

#[test]
fn failed_operand_validation_rolls_back_partial_nested_consumption() {
    let errors = build(
        "record Ticket {} \
         fn malformed(value: Ticket, number: I8) -> I8 { return number; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let negated: I8 = -malformed(value, true); \
             sink(value); \
         }",
    )
    .expect_err("operand call fails after speculatively consuming its first argument");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::TypeMismatch {
            expected: Type::Intrinsic(IntrinsicType::I8),
            found: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "failed operand transaction must not commit partial ownership effects"
    );
}

#[test]
fn successful_operand_effects_commit_once_and_negation_adds_no_binding_transition() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let negated: I8 = -take(value); \
             sink(value); \
         }",
    )
    .expect_err("successful operand call consumes its Ticket argument");
    assert_eq!(unavailable_count(&errors), 1);

    build(
        "fn f(value: I8) { \
             let negated: I8 = -value; \
             let still_available: I8 = value; \
         }",
    )
    .expect("integer negation itself adds no ownership transition");
}

#[test]
fn signed_literal_and_integer_negation_remain_distinct_for_unsigned_requirements() {
    let errors = build("fn bad() -> U8 { return -1; }")
        .expect_err("negative signed literal is not representable as U8");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralOutOfRange {
            required: Type::Intrinsic(IntrinsicType::U8),
        }
    ));
    assert!(!has_diagnostic(
        &errors,
        DiagnosticKind::IntegerNegationRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::U8),
        }
    ));

    let hir = build("fn good() -> U8 { return -(1); }")
        .expect("parenthesized unsigned literal can be integer-negated");
    let value = function(&hir, "good")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("negated U8 return value");
    let operand = integer_neg(value, IntrinsicType::U8);
    assert!(matches!(
        operand.kind,
        ValueKind::Literal(LiteralValue::U8(1))
    ));
}

#[test]
fn floating_negation_is_rejected_at_the_integer_negation_boundary() {
    let errors = build("fn f() -> F32 { return -(1.0); }")
        .expect_err("integer negation does not admit floating result type");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerNegationRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::F32),
        }
    ));
}

#[test]
fn conditional_bool_requirement_rejects_before_integer_operand_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { if -take(value) {} sink(value); }",
    )
    .expect_err("integer negation cannot satisfy the exact Bool condition type");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerNegationRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(unavailable_count(&errors), 0);
}

#[test]
fn nested_and_mixed_arithmetic_retain_explicit_source_operation_tree() {
    let hir = build(
        "fn nested() -> I8 { return --1; } \
         fn grouped(value: I8) -> I8 { return -(value + 1); } \
         fn multiplied(value: I8, other: I8) -> I8 { return -value * other; }",
    )
    .expect("nested and mixed integer negation is valid");

    let nested = function(&hir, "nested")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("nested return");
    let operand = integer_neg(nested, IntrinsicType::I8);
    assert!(matches!(
        operand.kind,
        ValueKind::Literal(LiteralValue::I8(-1))
    ));

    let grouped = function(&hir, "grouped")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("grouped return");
    let operand = integer_neg(grouped, IntrinsicType::I8);
    assert!(matches!(operand.kind, ValueKind::IntegerAdd { .. }));

    let multiplied = function(&hir, "multiplied")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("multiplied return");
    let ValueKind::IntegerMul { left, .. } = &multiplied.kind else {
        panic!("expected multiplication outside the tighter prefix negation");
    };
    integer_neg(left, IntrinsicType::I8);
}

#[test]
fn integer_negation_flows_through_existing_generic_value_consumers() {
    let hir = build(
        "record Boxed { value: I8 } \
         fn sink(value: I8) {} \
         fn f(value: I8) -> I8 { \
             let mut local: I8 = -value; \
             local = -local; \
             sink(-local); \
             let boxed: Boxed = Boxed { value: -local }; \
             return -boxed.value; \
         }",
    )
    .expect("IntegerNeg composes through generic Value consumers");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    integer_neg(initializer, IntrinsicType::I8);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    integer_neg(value, IntrinsicType::I8);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    integer_neg(&arguments[0], IntrinsicType::I8);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    integer_neg(&fields[0].value, IntrinsicType::I8);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    integer_neg(returned, IntrinsicType::I8);
}
