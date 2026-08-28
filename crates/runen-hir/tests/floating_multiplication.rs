use runen_hir::{
    Diagnostic, DiagnosticKind, IntrinsicType, ModuleId, NumericContract, OwnedUse, SourceUnit, Type,
    TypedCompilation, Value, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<Diagnostic>> {
    let parsed = parse(source);
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn returned_value<'a>(hir: &'a TypedCompilation, name: &str) -> &'a Value {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.body.terminal_return.as_ref())
        .and_then(|returned| returned.value.as_ref())
        .unwrap_or_else(|| panic!("missing returned value for {name}"))
}

fn float_mul(value: &Value, ty: IntrinsicType) -> (NumericContract, &Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::FloatMul {
        contract,
        left,
        right,
    } = &value.kind
    else {
        panic!("expected FloatMul HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (*contract, left, right)
}

fn float_add(value: &Value) -> (NumericContract, &Value, &Value) {
    let ValueKind::FloatAdd {
        contract,
        left,
        right,
    } = &value.kind
    else {
        panic!("expected FloatAdd HIR value");
    };
    (*contract, left, right)
}

fn unavailable_count(errors: &[Diagnostic]) -> usize {
    errors
        .iter()
        .filter(|error| error.kind == DiagnosticKind::UnavailableBinding)
        .count()
}

#[test]
fn all_three_floating_formats_retain_distinct_standard_float_mul_hir() {
    let hir = build(
        "fn f16_mul(a: F16, b: F16) -> F16 { return a * b; } \
         fn f32_mul(a: F32, b: F32) -> F32 { return a * b; } \
         fn f64_mul(a: F64, b: F64) -> F64 { return a * b; }",
    )
    .expect("same-format floating multiplications are valid");

    for (name, ty) in [
        ("f16_mul", IntrinsicType::F16),
        ("f32_mul", IntrinsicType::F32),
        ("f64_mul", IntrinsicType::F64),
    ] {
        let (contract, left, right) = float_mul(returned_value(&hir, name), ty);
        assert_eq!(contract, NumericContract::Standard);
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
fn fast_selector_selects_float_mul_in_all_three_formats_through_grouping_only() {
    let hir = build(
        "fn f16_mul(a: F16, b: F16) -> F16 { return @fast(a * b); } \
         fn f32_mul(a: F32, b: F32) -> F32 { return @fast((a * b)); } \
         fn f64_mul(a: F64, b: F64) -> F64 { return @ fast (((a * b))); }",
    )
    .expect("selected floating multiplications are valid");

    for (name, ty) in [
        ("f16_mul", IntrinsicType::F16),
        ("f32_mul", IntrinsicType::F32),
        ("f64_mul", IntrinsicType::F64),
    ] {
        assert_eq!(
            float_mul(returned_value(&hir, name), ty).0,
            NumericContract::Fast
        );
    }
}

#[test]
fn selector_stacking_remains_opaque_for_float_mul() {
    let errors = build("fn f(a: F32, b: F32) -> F32 { return @fast(@fast(a * b)); }")
        .expect_err("stacked selectors are invalid");
    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::NumericContractSelectionRequiresGovernedFloatingOperation {
                required: Type::Intrinsic(IntrinsicType::F32),
            }
    }));
}

#[test]
fn non_numeric_and_selected_integer_requirements_reject_before_operand_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> F32 { return 1.0; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: Bool = take(value) * true; \
             sink(value); \
         }",
    )
    .expect_err("multiplication cannot satisfy Bool");
    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::MultiplicationRequiresIntegerOrFloating {
                required: Type::Intrinsic(IntrinsicType::Bool),
            }
    }));
    assert_eq!(unavailable_count(&errors), 0);

    let selected = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I32 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: I32 = @fast(take(value) * 1); \
             sink(value); \
         }",
    )
    .expect_err("selector cannot govern integer multiplication");
    assert!(selected.iter().any(|error| {
        error.kind
            == DiagnosticKind::NumericContractSelectionRequiresGovernedFloatingOperation {
                required: Type::Intrinsic(IntrinsicType::I32),
            }
    }));
    assert_eq!(unavailable_count(&selected), 0);
}

#[test]
fn ordinary_and_selected_failed_right_float_mul_roll_back_left_consumption() {
    for expression in ["take(value) * true", "@fast(take(value) * true)"] {
        let source = format!(
            "record Ticket {{}} \
             fn take(value: Ticket) -> F32 {{ return 1.0; }} \
             fn sink(value: Ticket) {{}} \
             fn f(value: Ticket) {{ \
                 let product: F32 = {expression}; \
                 sink(value); \
             }}"
        );
        let errors = build(&source).expect_err("right operand must have exact F32 type");
        assert!(errors.iter().any(|error| {
            error.kind
                == DiagnosticKind::TypeMismatch {
                    expected: Type::Intrinsic(IntrinsicType::F32),
                    found: Type::Intrinsic(IntrinsicType::Bool),
                }
        }));
        assert_eq!(unavailable_count(&errors), 0);
    }
}

#[test]
fn float_mul_contracts_are_occurrence_local_for_nested_multiplication() {
    let hir = build(
        "fn fast_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a * (b * c)); } \
         fn fast_child(a: F32, b: F32, c: F32) -> F32 { return a * @fast(b * c); } \
         fn both_fast(a: F32, b: F32, c: F32) -> F32 { return @fast(a * @fast(b * c)); }",
    )
    .expect("nested multiplication contracts are valid");

    let (outer, _, inner) = float_mul(returned_value(&hir, "fast_root"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(
        float_mul(inner, IntrinsicType::F32).0,
        NumericContract::Standard
    );

    let (outer, _, inner) = float_mul(returned_value(&hir, "fast_child"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Standard);
    assert_eq!(
        float_mul(inner, IntrinsicType::F32).0,
        NumericContract::Fast
    );

    let (outer, _, inner) = float_mul(returned_value(&hir, "both_fast"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(
        float_mul(inner, IntrinsicType::F32).0,
        NumericContract::Fast
    );
}

#[test]
fn float_mul_and_float_add_contracts_are_occurrence_local_in_both_directions() {
    let hir = build(
        "fn mul_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a * (b + c)); } \
         fn add_child(a: F32, b: F32, c: F32) -> F32 { return a * @fast(b + c); } \
         fn add_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a + (b * c)); } \
         fn mul_child(a: F32, b: F32, c: F32) -> F32 { return a + @fast(b * c); } \
         fn both_fast(a: F32, b: F32, c: F32) -> F32 { return @fast(a + @fast(b * c)); }",
    )
    .expect("mixed FloatMul/FloatAdd contracts are valid");

    let (outer, _, inner) = float_mul(returned_value(&hir, "mul_root"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(float_add(inner).0, NumericContract::Standard);

    let (outer, _, inner) = float_mul(returned_value(&hir, "add_child"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Standard);
    assert_eq!(float_add(inner).0, NumericContract::Fast);

    let (outer, _, inner) = float_add(returned_value(&hir, "add_root"));
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(
        float_mul(inner, IntrinsicType::F32).0,
        NumericContract::Standard
    );

    let (outer, _, inner) = float_add(returned_value(&hir, "mul_child"));
    assert_eq!(outer, NumericContract::Standard);
    assert_eq!(
        float_mul(inner, IntrinsicType::F32).0,
        NumericContract::Fast
    );

    let (outer, _, inner) = float_add(returned_value(&hir, "both_fast"));
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(
        float_mul(inner, IntrinsicType::F32).0,
        NumericContract::Fast
    );
}
