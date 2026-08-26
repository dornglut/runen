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

fn returned_value<'a>(hir: &'a TypedCompilation, name: &str) -> &'a Value {
    function(hir, name)
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .unwrap_or_else(|| panic!("missing returned value for {name}"))
}

fn float_add(value: &Value, ty: IntrinsicType) -> (&Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::FloatAdd { left, right } = &value.kind else {
        panic!("expected FloatAdd HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (left, right)
}

fn unavailable_count(errors: &[Diagnostic]) -> usize {
    errors
        .iter()
        .filter(|error| error.kind == DiagnosticKind::UnavailableBinding)
        .count()
}

#[test]
fn all_three_floating_formats_retain_distinct_float_add_hir() {
    let hir = build(
        "fn f16_add(a: F16, b: F16) -> F16 { return a + b; } \
         fn f32_add(a: F32, b: F32) -> F32 { return a + b; } \
         fn f64_add(a: F64, b: F64) -> F64 { return a + b; }",
    )
    .expect("same-format floating additions are valid");

    for (name, ty) in [
        ("f16_add", IntrinsicType::F16),
        ("f32_add", IntrinsicType::F32),
        ("f64_add", IntrinsicType::F64),
    ] {
        let (left, right) = float_add(returned_value(&hir, name), ty);
        assert!(matches!(
            left.kind,
            ValueKind::BindingUse {
                ownership: OwnedUse::Duplicate,
                ..
            }
        ));
        assert!(matches!(
            right.kind,
            ValueKind::BindingUse {
                ownership: OwnedUse::Duplicate,
                ..
            }
        ));
    }
}

#[test]
fn non_numeric_outer_requirements_reject_before_operand_validation_or_consumption() {
    let bool_errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> F32 { return 1.0; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: Bool = take(value) + true; \
             sink(value); \
         }",
    )
    .expect_err("addition cannot satisfy Bool");
    assert!(bool_errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::AdditionRequiresIntegerOrFloating {
                required: Type::Intrinsic(IntrinsicType::Bool),
            }
    }));
    assert_eq!(unavailable_count(&bool_errors), 0);

    let record_errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> F32 { return 1.0; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: Ticket = take(value) + take(value); \
             sink(value); \
         }",
    )
    .expect_err("addition cannot satisfy a record requirement");
    assert!(record_errors.iter().any(|error| matches!(
        error.kind,
        DiagnosticKind::AdditionRequiresIntegerOrFloating {
            required: Type::Record(_)
        }
    )));
    assert_eq!(unavailable_count(&record_errors), 0);
}

#[test]
fn failed_floating_right_operand_rolls_back_successful_left_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> F32 { return 1.0; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let sum: F32 = take(value) + true; \
             sink(value); \
         }",
    )
    .expect_err("right operand must have exact F32 type");

    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::TypeMismatch {
                expected: Type::Intrinsic(IntrinsicType::F32),
                found: Type::Intrinsic(IntrinsicType::Bool),
            }
    }));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "failed two-operand validation must not commit left consumption"
    );
}

#[test]
fn floating_literals_materialize_under_the_exact_receiving_format() {
    let hir = build(
        "fn f16() -> F16 { return 1.0 + 2.0; } \
         fn f32() -> F32 { return 1.0 + 2.0; } \
         fn f64() -> F64 { return 1.0 + 2.0; }",
    )
    .expect("floating literal operands materialize under exact receiving types");

    for (name, ty) in [
        ("f16", IntrinsicType::F16),
        ("f32", IntrinsicType::F32),
        ("f64", IntrinsicType::F64),
    ] {
        let (left, right) = float_add(returned_value(&hir, name), ty);
        match ty {
            IntrinsicType::F16 => {
                assert!(matches!(left.kind, ValueKind::Literal(LiteralValue::F16(_))));
                assert!(matches!(right.kind, ValueKind::Literal(LiteralValue::F16(_))));
            }
            IntrinsicType::F32 => {
                assert!(matches!(left.kind, ValueKind::Literal(LiteralValue::F32(_))));
                assert!(matches!(right.kind, ValueKind::Literal(LiteralValue::F32(_))));
            }
            IntrinsicType::F64 => {
                assert!(matches!(left.kind, ValueKind::Literal(LiteralValue::F64(_))));
                assert!(matches!(right.kind, ValueKind::Literal(LiteralValue::F64(_))));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn integer_literals_are_not_reinterpreted_as_floating_operands() {
    let errors = build("fn f() -> F32 { return 1 + 2.0; }")
        .expect_err("integer literal must not materialize as F32");
    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::IntegerLiteralRequiresInteger {
                required: Type::Intrinsic(IntrinsicType::F32),
            }
    }));
}

#[test]
fn cross_format_bindings_and_calls_remain_exact_type_errors() {
    let binding_errors = build("fn f(a: F16, b: F32) -> F32 { return a + b; }")
        .expect_err("F16 binding must not convert to F32");
    assert!(binding_errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::TypeMismatch {
                expected: Type::Intrinsic(IntrinsicType::F32),
                found: Type::Intrinsic(IntrinsicType::F16),
            }
    }));

    let call_errors = build(
        "fn make() -> F64 { return 1.0; } \
         fn f(value: F32) -> F32 { return value + make(); }",
    )
    .expect_err("F64 call result must not convert to F32");
    assert!(call_errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::TypeMismatch {
                expected: Type::Intrinsic(IntrinsicType::F32),
                found: Type::Intrinsic(IntrinsicType::F64),
            }
    }));
}

#[test]
fn grouping_preserves_one_float_add_hir_node_per_represented_operation() {
    let hir = build(
        "fn left(a: F32, b: F32, c: F32) -> F32 { return (a + b) + c; } \
         fn right(a: F32, b: F32, c: F32) -> F32 { return a + (b + c); }",
    )
    .expect("grouping admits explicit nested floating additions");

    let (left_inner, _) = float_add(returned_value(&hir, "left"), IntrinsicType::F32);
    float_add(left_inner, IntrinsicType::F32);

    let (_, right_inner) = float_add(returned_value(&hir, "right"), IntrinsicType::F32);
    float_add(right_inner, IntrinsicType::F32);
}

#[test]
fn floating_addition_flows_through_existing_generic_value_consumers() {
    let hir = build(
        "record Boxed { value: F32 } \
         fn sink(value: F32) {} \
         fn f(a: F32, b: F32) -> F32 { \
             let mut local: F32 = a + b; \
             local = a + b; \
             sink(a + b); \
             let boxed: Boxed = Boxed { value: a + b }; \
             return boxed.value + b; \
         }",
    )
    .expect("FloatAdd composes through generic Value consumers");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    float_add(initializer, IntrinsicType::F32);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    float_add(value, IntrinsicType::F32);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    float_add(&arguments[0], IntrinsicType::F32);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    float_add(&fields[0].value, IntrinsicType::F32);

    float_add(returned_value(&hir, "f"), IntrinsicType::F32);
}
