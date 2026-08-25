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

fn integer_xor(value: &Value, ty: IntrinsicType) -> (&Value, &Value) {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::IntegerXor { left, right } = &value.kind else {
        panic!("expected IntegerXor HIR value");
    };
    assert_eq!(left.ty, expected);
    assert_eq!(right.ty, expected);
    (left, right)
}

#[test]
fn all_eight_fixed_width_integer_types_retain_explicit_xor_hir() {
    let hir = build(
        "fn i8_xor(a: I8, b: I8) -> I8 { return a ^ b; } \
         fn i16_xor(a: I16, b: I16) -> I16 { return a ^ b; } \
         fn i32_xor(a: I32, b: I32) -> I32 { return a ^ b; } \
         fn i64_xor(a: I64, b: I64) -> I64 { return a ^ b; } \
         fn u8_xor(a: U8, b: U8) -> U8 { return a ^ b; } \
         fn u16_xor(a: U16, b: U16) -> U16 { return a ^ b; } \
         fn u32_xor(a: U32, b: U32) -> U32 { return a ^ b; } \
         fn u64_xor(a: U64, b: U64) -> U64 { return a ^ b; }",
    )
    .expect("all fixed-width integer XOR values are valid");

    for (name, ty) in [
        ("i8_xor", IntrinsicType::I8),
        ("i16_xor", IntrinsicType::I16),
        ("i32_xor", IntrinsicType::I32),
        ("i64_xor", IntrinsicType::I64),
        ("u8_xor", IntrinsicType::U8),
        ("u16_xor", IntrinsicType::U16),
        ("u32_xor", IntrinsicType::U32),
        ("u64_xor", IntrinsicType::U64),
    ] {
        let value = function(&hir, name)
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("integer-XOR return value");
        let (left, right) = integer_xor(value, ty);
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
fn surrounding_exact_type_selects_both_xor_operands_without_defaulting() {
    let hir = build(
        "fn i8_value() { let value: I8 = 1 ^ 2; } \
         fn u64_value() { let value: U64 = 1 ^ 2; }",
    )
    .expect("surrounding exact integer type selects both XOR operands");

    for (name, ty) in [
        ("i8_value", IntrinsicType::I8),
        ("u64_value", IntrinsicType::U64),
    ] {
        let Statement::Local { initializer, .. } = &function(&hir, name).body.statements[0] else {
            panic!("expected XOR local");
        };
        let (left, right) = integer_xor(initializer, ty);
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
             let wrong: Bool = take(value) ^ true; \
             sink(value); \
         }",
    )
    .expect_err("integer XOR cannot satisfy Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerXorRequiresInteger {
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
             let result: I8 = malformed(value, true) ^ 2; \
             sink(value); \
         }",
    )
    .expect_err("left XOR operand validation must fail after a speculative first argument");

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
        "failed left XOR operand must not commit speculative consumption"
    );
}

#[test]
fn failed_right_operand_rolls_back_successful_left_operand_consumption() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 2; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let result: I8 = take(value) ^ true; \
             sink(value); \
         }",
    )
    .expect_err("right XOR operand must have exact I8 type");

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
        "failed XOR transaction must not commit successful left consumption"
    );
}

#[test]
fn successful_xor_operands_commit_in_source_order_and_xor_adds_no_extra_transition() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 2; } \
         fn sink(value: Ticket) {} \
         fn f(left: Ticket, right: Ticket) { \
             let result: I8 = take(left) ^ take(right); \
             sink(left); \
             sink(right); \
         }",
    )
    .expect_err("both successful XOR call operands consume their Ticket arguments");
    assert_eq!(unavailable_count(&errors), 2);

    build(
        "fn f(left: I8, right: I8) { \
             let result: I8 = left ^ right; \
             let still_left: I8 = left; \
             let still_right: I8 = right; \
         }",
    )
    .expect("IntegerXor adds no ownership transition beyond its operands");
}

#[test]
fn conditional_bool_requirement_rejects_xor_without_touching_operand_ownership() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             if take(value) ^ 1 {} \
             sink(value); \
         }",
    )
    .expect_err("conditional exact-Bool requirement rejects integer XOR");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerXorRequiresInteger {
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
fn xor_composes_through_existing_generic_value_consumers() {
    let hir = build(
        "record Boxed { value: I8 } \
         fn sink(value: I8) {} \
         fn f(a: I8, b: I8) -> I8 { \
             let mut local: I8 = a ^ b; \
             local = a ^ b; \
             sink(a ^ b); \
             let boxed: Boxed = Boxed { value: a ^ b }; \
             return boxed.value ^ b; \
         }",
    )
    .expect("IntegerXor composes through generic Value consumers");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    integer_xor(initializer, IntrinsicType::I8);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    integer_xor(value, IntrinsicType::I8);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    integer_xor(&arguments[0], IntrinsicType::I8);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    integer_xor(&fields[0].value, IntrinsicType::I8);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    integer_xor(returned, IntrinsicType::I8);
}
