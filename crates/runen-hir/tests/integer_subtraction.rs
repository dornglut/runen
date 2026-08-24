use runen_hir::{
    Diagnostic, DiagnosticKind, IntrinsicType, ModuleId, OwnedUse, SourceUnit, Statement, Type,
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

fn integer_sub(value: &Value, ty: IntrinsicType) -> (&Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::IntegerSub { left, right } = &value.kind else {
        panic!("expected IntegerSub HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (left, right)
}

fn integer_add(value: &Value, ty: IntrinsicType) -> (&Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::IntegerAdd { left, right } = &value.kind else {
        panic!("expected IntegerAdd HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (left, right)
}

#[test]
fn all_eight_fixed_width_integer_types_retain_explicit_subtraction_hir() {
    let hir = build(
        "fn i8_sub(a: I8, b: I8) -> I8 { return a - b; } \
         fn i16_sub(a: I16, b: I16) -> I16 { return a - b; } \
         fn i32_sub(a: I32, b: I32) -> I32 { return a - b; } \
         fn i64_sub(a: I64, b: I64) -> I64 { return a - b; } \
         fn u8_sub(a: U8, b: U8) -> U8 { return a - b; } \
         fn u16_sub(a: U16, b: U16) -> U16 { return a - b; } \
         fn u32_sub(a: U32, b: U32) -> U32 { return a - b; } \
         fn u64_sub(a: U64, b: U64) -> U64 { return a - b; }",
    )
    .expect("all fixed-width integer subtractions are valid");

    for (name, ty) in [
        ("i8_sub", IntrinsicType::I8),
        ("i16_sub", IntrinsicType::I16),
        ("i32_sub", IntrinsicType::I32),
        ("i64_sub", IntrinsicType::I64),
        ("u8_sub", IntrinsicType::U8),
        ("u16_sub", IntrinsicType::U16),
        ("u32_sub", IntrinsicType::U32),
        ("u64_sub", IntrinsicType::U64),
    ] {
        let value = function(&hir, name)
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("integer-sub return value");
        let (left, right) = integer_sub(value, ty);
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
fn non_integer_outer_requirement_rejects_before_operand_validation_or_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: Bool = take(value) - true; \
             sink(value); \
         }",
    )
    .expect_err("integer subtraction cannot satisfy Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerSubtractionRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "outer rejection must not validate or consume through either operand"
    );
}

#[test]
fn failed_right_operand_rolls_back_left_operand_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let difference: I8 = take(value) - true; \
             sink(value); \
         }",
    )
    .expect_err("right operand must have exact I8 type");

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
        "failed two-operand transaction must not commit left consumption"
    );
}

#[test]
fn successful_operand_effects_commit_in_source_order_without_operator_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let difference: I8 = take(left) - take(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("both successful call operands consume their Ticket arguments");
    assert_eq!(unavailable_count(&errors), 2);

    build(
        "fn f(left: I8, right: I8) { \
             let difference: I8 = left - right; \
             let still_left: I8 = left; \
             let still_right: I8 = right; \
         }",
    )
    .expect("IntegerSub adds no ownership transition beyond its operands");
}

#[test]
fn operand_literal_materialization_precedes_arithmetic_and_is_not_folded() {
    let errors = build("fn bad() { let difference: I8 = 128 - -1; }")
        .expect_err("out-of-range I8 operand remains invalid before arithmetic");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralOutOfRange {
            required: Type::Intrinsic(IntrinsicType::I8),
        }
    ));
}

#[test]
fn signed_literal_right_operand_is_retained_as_existing_literal_hir() {
    let hir = build("fn f(a: I8) -> I8 { return a--1; }")
        .expect("adjacent subtraction and signed literal are valid");
    let value = function(&hir, "f")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    let (_, right) = integer_sub(value, IntrinsicType::I8);
    assert!(matches!(right.kind, ValueKind::Literal(runen_hir::LiteralValue::I8(-1))));
}

#[test]
fn grouping_retains_explicit_nested_and_mixed_addition_subtraction_hir() {
    let hir = build(
        "fn left(a: I8, b: I8, c: I8) -> I8 { return (a + b) - c; } \
         fn right(a: I8, b: I8, c: I8) -> I8 { return a - (b - c); } \
         fn mixed(a: I8, b: I8, c: I8) -> I8 { return (a - b) + c; }",
    )
    .expect("grouping admits explicit Add/Sub trees");

    let left = function(&hir, "left")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("left return");
    let (left_inner, _) = integer_sub(left, IntrinsicType::I8);
    integer_add(left_inner, IntrinsicType::I8);

    let right = function(&hir, "right")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("right return");
    let (_, right_inner) = integer_sub(right, IntrinsicType::I8);
    integer_sub(right_inner, IntrinsicType::I8);

    let mixed = function(&hir, "mixed")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("mixed return");
    let (mixed_inner, _) = integer_add(mixed, IntrinsicType::I8);
    integer_sub(mixed_inner, IntrinsicType::I8);
}

#[test]
fn integer_subtraction_flows_through_existing_generic_value_consumers() {
    let hir = build(
        "record Boxed { value: I8 } \
         fn sink(value: I8) {} \
         fn f(a: I8, b: I8) -> I8 { \
             let mut local: I8 = a - b; \
             local = a - b; \
             sink(a - b); \
             let boxed: Boxed = Boxed { value: a - b }; \
             return boxed.value - b; \
         }",
    )
    .expect("IntegerSub composes through generic Value consumers");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    integer_sub(initializer, IntrinsicType::I8);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    integer_sub(value, IntrinsicType::I8);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    integer_sub(&arguments[0], IntrinsicType::I8);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    integer_sub(&fields[0].value, IntrinsicType::I8);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    integer_sub(returned, IntrinsicType::I8);
}
