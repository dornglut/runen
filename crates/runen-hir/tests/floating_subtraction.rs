use runen_hir::{
    Diagnostic, DiagnosticKind, IntrinsicType, LiteralValue, ModuleId, NumericContract, OwnedUse,
    SourceUnit, Type, TypedCompilation, Value, ValueKind, build_typed_hir,
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

fn float_sub_with_contract(value: &Value, ty: IntrinsicType) -> (NumericContract, &Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::FloatSub {
        contract,
        left,
        right,
    } = &value.kind
    else {
        panic!("expected FloatSub HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (*contract, left, right)
}

fn float_add_with_contract(value: &Value) -> (NumericContract, &Value, &Value) {
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
fn all_three_floating_formats_retain_distinct_standard_float_sub_hir() {
    let hir = build(
        "fn f16_sub(a: F16, b: F16) -> F16 { return a - b; } \
         fn f32_sub(a: F32, b: F32) -> F32 { return a - b; } \
         fn f64_sub(a: F64, b: F64) -> F64 { return a - b; }",
    )
    .expect("same-format floating subtractions are valid");

    for (name, ty) in [
        ("f16_sub", IntrinsicType::F16),
        ("f32_sub", IntrinsicType::F32),
        ("f64_sub", IntrinsicType::F64),
    ] {
        let (contract, left, right) = float_sub_with_contract(returned_value(&hir, name), ty);
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
fn fast_selector_selects_float_sub_in_all_three_formats_through_grouping_only() {
    let hir = build(
        "fn f16_sub(a: F16, b: F16) -> F16 { return @fast(a - b); } \
         fn f32_sub(a: F32, b: F32) -> F32 { return @fast((a - b)); } \
         fn f64_sub(a: F64, b: F64) -> F64 { return @ fast (((a - b))); }",
    )
    .expect("selected floating subtractions are valid");

    for (name, ty) in [
        ("f16_sub", IntrinsicType::F16),
        ("f32_sub", IntrinsicType::F32),
        ("f64_sub", IntrinsicType::F64),
    ] {
        assert_eq!(
            float_sub_with_contract(returned_value(&hir, name), ty).0,
            NumericContract::Fast
        );
    }
}

#[test]
fn selector_stacking_remains_opaque_for_float_sub() {
    let errors = build("fn f(a: F32, b: F32) -> F32 { return @fast(@fast(a - b)); }")
        .expect_err("stacked selectors are invalid");
    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::NumericContractSelectionRequiresFloatingAddOrSub {
                required: Type::Intrinsic(IntrinsicType::F32),
            }
    }));
}

#[test]
fn non_numeric_outer_requirement_rejects_before_subtraction_operand_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> F32 { return 1.0; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: Bool = take(value) - true; \
             sink(value); \
         }",
    )
    .expect_err("subtraction cannot satisfy Bool");

    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::SubtractionRequiresIntegerOrFloating {
                required: Type::Intrinsic(IntrinsicType::Bool),
            }
    }));
    assert_eq!(unavailable_count(&errors), 0);
}

#[test]
fn selected_non_floating_requirement_rejects_before_operand_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I32 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: I32 = @fast(take(value) - 1); \
             sink(value); \
         }",
    )
    .expect_err("selector requires floating subtraction");
    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::NumericContractSelectionRequiresFloatingAddOrSub {
                required: Type::Intrinsic(IntrinsicType::I32),
            }
    }));
    assert_eq!(unavailable_count(&errors), 0);
}

#[test]
fn failed_float_sub_right_operand_rolls_back_successful_left_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> F32 { return 1.0; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let difference: F32 = take(value) - true; \
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
    assert_eq!(unavailable_count(&errors), 0);

    let selected = build(
        "record Ticket {} \
         fn take(value: Ticket) -> F32 { return 1.0; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let difference: F32 = @fast(take(value) - true); \
             sink(value); \
         }",
    )
    .expect_err("selected right operand must have exact F32 type");
    assert_eq!(unavailable_count(&selected), 0);
}

#[test]
fn signed_floating_literal_is_an_operand_but_parenthesized_prefix_minus_is_not_float_negation() {
    let hir = build("fn f(a: F32) -> F32 { return a - -1.0; }")
        .expect("signed decimal floating literal remains a FloatSub operand");
    let (_, _, right) = float_sub_with_contract(returned_value(&hir, "f"), IntrinsicType::F32);
    assert!(matches!(
        right.kind,
        ValueKind::Literal(LiteralValue::F32(_))
    ));

    let errors = build("fn bad() -> F32 { return -(1.0); }")
        .expect_err("prefix parenthesized minus remains integer negation syntax");
    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::IntegerNegationRequiresInteger {
                required: Type::Intrinsic(IntrinsicType::F32),
            }
    }));
}

#[test]
fn float_sub_and_float_add_contracts_are_occurrence_local_in_both_nesting_directions() {
    let hir = build(
        "fn sub_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a - (b + c)); } \
         fn add_child(a: F32, b: F32, c: F32) -> F32 { return a - @fast(b + c); } \
         fn add_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a + (b - c)); } \
         fn sub_child(a: F32, b: F32, c: F32) -> F32 { return a + @fast(b - c); } \
         fn sub_sub_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a - (b - c)); } \
         fn sub_sub_child(a: F32, b: F32, c: F32) -> F32 { return a - @fast(b - c); }",
    )
    .expect("mixed operation-local contracts are valid");

    let (outer, _, inner) =
        float_sub_with_contract(returned_value(&hir, "sub_root"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(float_add_with_contract(inner).0, NumericContract::Standard);

    let (outer, _, inner) =
        float_sub_with_contract(returned_value(&hir, "add_child"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Standard);
    assert_eq!(float_add_with_contract(inner).0, NumericContract::Fast);

    let (outer, _, inner) = float_add_with_contract(returned_value(&hir, "add_root"));
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(
        float_sub_with_contract(inner, IntrinsicType::F32).0,
        NumericContract::Standard
    );

    let (outer, _, inner) = float_add_with_contract(returned_value(&hir, "sub_child"));
    assert_eq!(outer, NumericContract::Standard);
    assert_eq!(
        float_sub_with_contract(inner, IntrinsicType::F32).0,
        NumericContract::Fast
    );

    let (outer, _, inner) =
        float_sub_with_contract(returned_value(&hir, "sub_sub_root"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Fast);
    assert_eq!(
        float_sub_with_contract(inner, IntrinsicType::F32).0,
        NumericContract::Standard
    );

    let (outer, _, inner) =
        float_sub_with_contract(returned_value(&hir, "sub_sub_child"), IntrinsicType::F32);
    assert_eq!(outer, NumericContract::Standard);
    assert_eq!(
        float_sub_with_contract(inner, IntrinsicType::F32).0,
        NumericContract::Fast
    );
}
