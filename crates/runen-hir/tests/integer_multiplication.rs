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

fn integer_mul(value: &Value, ty: IntrinsicType) -> (&Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::IntegerMul { left, right } = &value.kind else {
        panic!("expected IntegerMul HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (left, right)
}

#[test]
fn all_eight_fixed_width_integer_types_retain_explicit_multiplication_hir() {
    let hir = build(
        "fn i8_mul(a: I8, b: I8) -> I8 { return a * b; } \
         fn i16_mul(a: I16, b: I16) -> I16 { return a * b; } \
         fn i32_mul(a: I32, b: I32) -> I32 { return a * b; } \
         fn i64_mul(a: I64, b: I64) -> I64 { return a * b; } \
         fn u8_mul(a: U8, b: U8) -> U8 { return a * b; } \
         fn u16_mul(a: U16, b: U16) -> U16 { return a * b; } \
         fn u32_mul(a: U32, b: U32) -> U32 { return a * b; } \
         fn u64_mul(a: U64, b: U64) -> U64 { return a * b; }",
    )
    .expect("all fixed-width integer multiplications are valid");

    for (name, ty) in [
        ("i8_mul", IntrinsicType::I8),
        ("i16_mul", IntrinsicType::I16),
        ("i32_mul", IntrinsicType::I32),
        ("i64_mul", IntrinsicType::I64),
        ("u8_mul", IntrinsicType::U8),
        ("u16_mul", IntrinsicType::U16),
        ("u32_mul", IntrinsicType::U32),
        ("u64_mul", IntrinsicType::U64),
    ] {
        let value = function(&hir, name)
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("integer-mul return value");
        let (left, right) = integer_mul(value, ty);
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
fn non_numeric_outer_requirement_rejects_before_operand_validation_or_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: Bool = take(value) * true; \
             sink(value); \
         }",
    )
    .expect_err("multiplication cannot satisfy Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::MultiplicationRequiresIntegerOrFloating {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(unavailable_count(&errors), 0);
}

#[test]
fn failed_left_operand_commits_no_speculative_consumption() {
    let errors = build(
        "record Ticket {} \
         fn malformed(value: Ticket, number: I8) -> I8 { return number; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let product: I8 = malformed(value, true) * 2; \
             sink(value); \
         }",
    )
    .expect_err("left operand validation must fail after a speculative first argument");

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
        "failed left operand must not commit its speculative consumption"
    );
}

#[test]
fn failed_right_operand_rolls_back_left_operand_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 2; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let product: I8 = take(value) * true; \
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
        "failed multiplication transaction must not commit left consumption"
    );
}

#[test]
fn successful_operand_effects_commit_and_multiplication_adds_no_binding_transition() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 2; } \
         fn sink(value: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let product: I8 = take(left) * take(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("both successful call operands consume their Ticket arguments");
    assert_eq!(unavailable_count(&errors), 2);

    build(
        "fn f(left: I8, right: I8) { \
             let product: I8 = left * right; \
             let still_left: I8 = left; \
             let still_right: I8 = right; \
         }",
    )
    .expect("IntegerMul adds no ownership transition beyond its operands");
}

#[test]
fn operand_literal_materialization_precedes_arithmetic_and_is_not_folded() {
    let errors = build("fn bad() { let product: I8 = 128 * 0; }")
        .expect_err("out-of-range I8 operand remains invalid even when product would be zero");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerLiteralOutOfRange {
            required: Type::Intrinsic(IntrinsicType::I8),
        }
    ));
}

#[test]
fn grouping_and_mixed_tiers_retain_explicit_operation_tree() {
    let hir = build(
        "fn tighter(a: I8, b: I8, c: I8) -> I8 { return a + b * c; } \
         fn override_left(a: I8, b: I8, c: I8) -> I8 { return (a + b) * c; } \
         fn repeat(a: I8, b: I8, c: I8) -> I8 { return a * (b * c); } \
         fn sub_right(a: I8, b: I8, c: I8) -> I8 { return a * (b - c); }",
    )
    .expect("grouping and mixed tiers retain explicit arithmetic HIR");

    let tighter = function(&hir, "tighter")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("tighter return");
    let ValueKind::IntegerAdd { right, .. } = &tighter.kind else {
        panic!("expected outer addition");
    };
    integer_mul(right, IntrinsicType::I8);

    let override_left = function(&hir, "override_left")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("override return");
    let (left, _) = integer_mul(override_left, IntrinsicType::I8);
    assert!(matches!(left.kind, ValueKind::IntegerAdd { .. }));

    let repeat = function(&hir, "repeat")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("repeat return");
    let (_, right) = integer_mul(repeat, IntrinsicType::I8);
    integer_mul(right, IntrinsicType::I8);

    let sub_right = function(&hir, "sub_right")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("sub-right return");
    let (_, right) = integer_mul(sub_right, IntrinsicType::I8);
    assert!(matches!(right.kind, ValueKind::IntegerSub { .. }));
}

#[test]
fn integer_multiplication_flows_through_existing_generic_value_consumers() {
    let hir = build(
        "record Boxed { value: I8 } \
         fn sink(value: I8) {} \
         fn f(a: I8, b: I8) -> I8 { \
             let mut local: I8 = a * b; \
             local = a * b; \
             sink(a * b); \
             let boxed: Boxed = Boxed { value: a * b }; \
             return boxed.value * b; \
         }",
    )
    .expect("IntegerMul composes through generic Value consumers");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    integer_mul(initializer, IntrinsicType::I8);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    integer_mul(value, IntrinsicType::I8);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    integer_mul(&arguments[0], IntrinsicType::I8);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    integer_mul(&fields[0].value, IntrinsicType::I8);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    integer_mul(returned, IntrinsicType::I8);
}
