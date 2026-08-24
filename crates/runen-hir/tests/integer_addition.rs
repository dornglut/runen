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
fn all_eight_fixed_width_integer_types_retain_explicit_addition_hir() {
    let hir = build(
        "fn i8_add(a: I8, b: I8) -> I8 { return a + b; } \
         fn i16_add(a: I16, b: I16) -> I16 { return a + b; } \
         fn i32_add(a: I32, b: I32) -> I32 { return a + b; } \
         fn i64_add(a: I64, b: I64) -> I64 { return a + b; } \
         fn u8_add(a: U8, b: U8) -> U8 { return a + b; } \
         fn u16_add(a: U16, b: U16) -> U16 { return a + b; } \
         fn u32_add(a: U32, b: U32) -> U32 { return a + b; } \
         fn u64_add(a: U64, b: U64) -> U64 { return a + b; }",
    )
    .expect("all fixed-width integer additions are valid");

    for (name, ty) in [
        ("i8_add", IntrinsicType::I8),
        ("i16_add", IntrinsicType::I16),
        ("i32_add", IntrinsicType::I32),
        ("i64_add", IntrinsicType::I64),
        ("u8_add", IntrinsicType::U8),
        ("u16_add", IntrinsicType::U16),
        ("u32_add", IntrinsicType::U32),
        ("u64_add", IntrinsicType::U64),
    ] {
        let value = function(&hir, name)
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("integer-add return value");
        let (left, right) = integer_add(value, ty);
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
             let wrong: Bool = take(value) + true; \
             sink(value); \
         }",
    )
    .expect_err("integer addition cannot satisfy Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerAdditionRequiresInteger {
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
             let sum: I8 = take(value) + true; \
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
             let sum: I8 = take(left) + take(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("both successful call operands consume their Ticket arguments");
    assert_eq!(unavailable_count(&errors), 2);

    build(
        "fn f(left: I8, right: I8) { \
             let sum: I8 = left + right; \
             let still_left: I8 = left; \
             let still_right: I8 = right; \
         }",
    )
    .expect("IntegerAdd adds no ownership transition beyond its operands");
}

#[test]
fn operand_literal_materialization_precedes_arithmetic_and_is_not_folded() {
    let errors = build("fn bad() { let sum: I8 = 128 + -1; }")
        .expect_err("out-of-range I8 operand remains invalid before arithmetic");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralOutOfRange {
            required: Type::Intrinsic(IntrinsicType::I8),
        }
    ));
}

#[test]
fn grouping_allows_explicit_nested_addition_without_erasing_hir_operations() {
    let hir = build(
        "fn left(a: I8, b: I8, c: I8) -> I8 { return (a + b) + c; } \
         fn right(a: I8, b: I8, c: I8) -> I8 { return a + (b + c); }",
    )
    .expect("grouping admits explicit nested additions");

    let left = function(&hir, "left")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("left-associated grouped return");
    let (left_inner, _) = integer_add(left, IntrinsicType::I8);
    integer_add(left_inner, IntrinsicType::I8);

    let right = function(&hir, "right")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("right-associated grouped return");
    let (_, right_inner) = integer_add(right, IntrinsicType::I8);
    integer_add(right_inner, IntrinsicType::I8);
}

#[test]
fn integer_addition_flows_through_existing_generic_value_consumers() {
    let hir = build(
        "record Boxed { value: I8 } \
         fn sink(value: I8) {} \
         fn f(a: I8, b: I8) -> I8 { \
             let mut local: I8 = a + b; \
             local = a + b; \
             sink(a + b); \
             let boxed: Boxed = Boxed { value: a + b }; \
             return boxed.value + b; \
         }",
    )
    .expect("IntegerAdd composes through generic Value consumers");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    integer_add(initializer, IntrinsicType::I8);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    integer_add(value, IntrinsicType::I8);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    integer_add(&arguments[0], IntrinsicType::I8);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    integer_add(&fields[0].value, IntrinsicType::I8);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    integer_add(returned, IntrinsicType::I8);
}
