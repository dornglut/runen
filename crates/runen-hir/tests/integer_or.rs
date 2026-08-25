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

fn integer_or(value: &Value, ty: IntrinsicType) -> (&Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::IntegerOr { left, right } = &value.kind else {
        panic!("expected IntegerOr HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (left, right)
}

#[test]
fn all_eight_fixed_width_integer_types_retain_explicit_or_hir() {
    let hir = build(
        "fn i8_or(a: I8, b: I8) -> I8 { return a | b; } \
         fn i16_or(a: I16, b: I16) -> I16 { return a | b; } \
         fn i32_or(a: I32, b: I32) -> I32 { return a | b; } \
         fn i64_or(a: I64, b: I64) -> I64 { return a | b; } \
         fn u8_or(a: U8, b: U8) -> U8 { return a | b; } \
         fn u16_or(a: U16, b: U16) -> U16 { return a | b; } \
         fn u32_or(a: U32, b: U32) -> U32 { return a | b; } \
         fn u64_or(a: U64, b: U64) -> U64 { return a | b; }",
    )
    .expect("all fixed-width integer OR values are valid");

    for (name, ty) in [
        ("i8_or", IntrinsicType::I8),
        ("i16_or", IntrinsicType::I16),
        ("i32_or", IntrinsicType::I32),
        ("i64_or", IntrinsicType::I64),
        ("u8_or", IntrinsicType::U8),
        ("u16_or", IntrinsicType::U16),
        ("u32_or", IntrinsicType::U32),
        ("u64_or", IntrinsicType::U64),
    ] {
        let value = function(&hir, name)
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("integer-OR return value");
        let (left, right) = integer_or(value, ty);
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
fn surrounding_exact_type_selects_both_or_operands_without_defaulting() {
    let hir = build(
        "fn i8_value() { let value: I8 = 1 | 2; } \
         fn u64_value() { let value: U64 = 1 | 2; }",
    )
    .expect("surrounding exact integer type selects both OR operands");

    for (name, ty) in [
        ("i8_value", IntrinsicType::I8),
        ("u64_value", IntrinsicType::U64),
    ] {
        let Statement::Local { initializer, .. } = &function(&hir, name).body.statements[0] else {
            panic!("expected OR local");
        };
        let (left, right) = integer_or(initializer, ty);
        assert!(matches!(left.kind, ValueKind::Literal(_)));
        assert!(matches!(right.kind, ValueKind::Literal(_)));
    }
}

#[test]
fn non_integer_outer_requirement_rejects_before_operand_validation_or_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let wrong: Bool = take(value) | true; \
             sink(value); \
         }",
    )
    .expect_err("integer OR cannot satisfy Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerOrRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "outer required-type rejection must occur before operand ownership validation"
    );
}

#[test]
fn failed_left_operand_commits_no_speculative_consumption() {
    let errors = build(
        "record Ticket {} \
         fn malformed(value: Ticket, number: I8) -> I8 { return number; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let result: I8 = malformed(value, true) | 2; \
             sink(value); \
         }",
    )
    .expect_err("left OR operand validation must fail after a speculative first argument");

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
        "failed left OR operand must not commit speculative consumption"
    );
}

#[test]
fn failed_right_operand_rolls_back_successful_left_operand_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 2; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let result: I8 = take(value) | true; \
             sink(value); \
         }",
    )
    .expect_err("right OR operand must have exact I8 type");

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
        "failed OR transaction must not commit successful left consumption"
    );
}

#[test]
fn successful_or_operands_commit_in_source_order_and_or_adds_no_extra_transition() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 2; } \
         fn sink(value: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let result: I8 = take(left) | take(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("both successful OR call operands consume their Ticket arguments");
    assert_eq!(unavailable_count(&errors), 2);

    build(
        "fn f(left: I8, right: I8) { \
             let result: I8 = left | right; \
             let still_left: I8 = left; \
             let still_right: I8 = right; \
         }",
    )
    .expect("IntegerOr adds no ownership transition beyond its operands");
}

#[test]
fn conditional_bool_requirement_rejects_or_without_touching_operand_ownership() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             if take(value) | 1 {} \
             sink(value); \
         }",
    )
    .expect_err("conditional exact-Bool requirement rejects integer OR");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerOrRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(
        unavailable_count(&errors),
        0,
        "condition rejection must occur before the left call can consume its argument"
    );
}

#[test]
fn or_composes_through_existing_generic_value_consumers() {
    let hir = build(
        "record Boxed { value: I8 } \
         fn sink(value: I8) {} \
         fn f(a: I8, b: I8) -> I8 { \
             let mut local: I8 = a | b; \
             local = a | b; \
             sink(a | b); \
             let boxed: Boxed = Boxed { value: a | b }; \
             return boxed.value | b; \
         }",
    )
    .expect("IntegerOr composes through generic Value consumers");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    integer_or(initializer, IntrinsicType::I8);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    integer_or(value, IntrinsicType::I8);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    integer_or(&arguments[0], IntrinsicType::I8);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    integer_or(&fields[0].value, IntrinsicType::I8);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    integer_or(returned, IntrinsicType::I8);
}
