use runen_hir::{
    Diagnostic, DiagnosticKind, IntrinsicType, ModuleId, NumericContract, SourceUnit, Type,
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

fn float_div(value: &Value, ty: IntrinsicType) -> (NumericContract, &Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::FloatDiv {
        contract,
        left,
        right,
    } = &value.kind
    else {
        panic!("expected FloatDiv HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (*contract, left, right)
}

fn contract(value: &Value) -> NumericContract {
    match &value.kind {
        ValueKind::FloatAdd { contract, .. }
        | ValueKind::FloatSub { contract, .. }
        | ValueKind::FloatMul { contract, .. }
        | ValueKind::FloatDiv { contract, .. } => *contract,
        _ => panic!("expected governed floating operation"),
    }
}

fn unavailable_count(errors: &[Diagnostic]) -> usize {
    errors
        .iter()
        .filter(|error| error.kind == DiagnosticKind::UnavailableBinding)
        .count()
}

#[test]
fn all_formats_retain_distinct_standard_and_selected_fast_float_div_hir() {
    let hir = build(
        "fn f16_standard(a: F16, b: F16) -> F16 { return a / b; } \
         fn f32_standard(a: F32, b: F32) -> F32 { return a / b; } \
         fn f64_standard(a: F64, b: F64) -> F64 { return a / b; } \
         fn f16_fast(a: F16, b: F16) -> F16 { return @fast(a / b); } \
         fn f32_fast(a: F32, b: F32) -> F32 { return @fast((a / b)); } \
         fn f64_fast(a: F64, b: F64) -> F64 { return @ fast (((a / b))); }",
    )
    .expect("same-format floating divisions are valid");

    for (name, ty, expected) in [
        ("f16_standard", IntrinsicType::F16, NumericContract::Standard),
        ("f32_standard", IntrinsicType::F32, NumericContract::Standard),
        ("f64_standard", IntrinsicType::F64, NumericContract::Standard),
        ("f16_fast", IntrinsicType::F16, NumericContract::Fast),
        ("f32_fast", IntrinsicType::F32, NumericContract::Fast),
        ("f64_fast", IntrinsicType::F64, NumericContract::Fast),
    ] {
        assert_eq!(float_div(returned_value(&hir, name), ty).0, expected);
    }
}

#[test]
fn division_rejects_nonfloating_requirements_before_operand_consumption() {
    for (ty, literal) in [("Bool", "true"), ("I32", "1")] {
        let source = format!(
            "record Ticket {{}} \
             fn take(value: Ticket) -> {ty} {{ return {literal}; }} \
             fn sink(value: Ticket) {{}} \
             fn f(value: Ticket) {{ \
                 let wrong: {ty} = take(value) / {literal}; \
                 sink(value); \
             }}"
        );
        let errors = build(&source).expect_err("division requires a floating source type");
        let required = if ty == "Bool" {
            Type::Intrinsic(IntrinsicType::Bool)
        } else {
            Type::Intrinsic(IntrinsicType::I32)
        };
        assert!(errors.iter().any(|error| {
            error.kind == DiagnosticKind::DivisionRequiresFloating { required }
        }));
        assert_eq!(unavailable_count(&errors), 0);
    }
}

#[test]
fn failed_right_float_div_validation_rolls_back_left_consumption() {
    for expression in ["take(value) / true", "@fast(take(value) / true)"] {
        let source = format!(
            "record Ticket {{}} \
             fn take(value: Ticket) -> F32 {{ return 1.0; }} \
             fn sink(value: Ticket) {{}} \
             fn f(value: Ticket) {{ \
                 let quotient: F32 = {expression}; \
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
fn selector_stacking_is_rejected_and_nested_division_contracts_are_occurrence_local() {
    let errors = build("fn f(a: F32, b: F32) -> F32 { return @fast(@fast(a / b)); }")
        .expect_err("stacked selectors are invalid");
    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::NumericContractSelectionRequiresGovernedFloatingOperation {
                required: Type::Intrinsic(IntrinsicType::F32),
            }
    }));

    let hir = build(
        "fn fast_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a / (b / c)); } \
         fn fast_child(a: F32, b: F32, c: F32) -> F32 { return a / @fast(b / c); } \
         fn both_fast(a: F32, b: F32, c: F32) -> F32 { return @fast(a / @fast(b / c)); }",
    )
    .expect("nested division contracts are valid");

    let (outer, _, inner) = float_div(returned_value(&hir, "fast_root"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(contract(inner), NumericContract::Standard);

    let (outer, _, inner) = float_div(returned_value(&hir, "fast_child"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Standard);
    assert_eq!(contract(inner), NumericContract::Fast);

    let (outer, _, inner) = float_div(returned_value(&hir, "both_fast"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(contract(inner), NumericContract::Fast);
}

#[test]
fn float_div_contracts_remain_local_across_mul_add_and_sub() {
    let hir = build(
        "fn mul(a: F32, b: F32, c: F32) -> F32 { return @fast(a / (b * c)); } \
         fn add(a: F32, b: F32, c: F32) -> F32 { return a / @fast(b + c); } \
         fn sub(a: F32, b: F32, c: F32) -> F32 { return @fast(a - @fast(b / c)); }",
    )
    .expect("mixed governed floating operations are valid");

    let (outer, _, inner) = float_div(returned_value(&hir, "mul"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(contract(inner), NumericContract::Standard);

    let (outer, _, inner) = float_div(returned_value(&hir, "add"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Standard);
    assert_eq!(contract(inner), NumericContract::Fast);

    let ValueKind::FloatSub {
        contract: outer,
        right,
        ..
    } = &returned_value(&hir, "sub").kind
    else {
        panic!("expected FloatSub root");
    };
    assert_eq!(*outer, NumericContract::Fast);
    assert_eq!(contract(right), NumericContract::Fast);
}

#[test]
fn direct_conditional_selector_root_remains_excluded() {
    let errors = build("fn f(a: F32, b: F32) { if @fast(a / b) {} }")
        .expect_err("numeric selector is not a direct ConditionalValue root");
    assert!(errors.iter().any(|error| {
        matches!(error.kind, DiagnosticKind::SyntaxError(_))
    }));
}
