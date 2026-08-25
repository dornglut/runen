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

fn integer_complement(value: &Value, ty: IntrinsicType) -> &Value {
    let expected = Type::Intrinsic(ty);
    assert_eq!(value.ty, expected);
    let ValueKind::IntegerComplement { operand } = &value.kind else {
        panic!("expected IntegerComplement HIR value");
    };
    assert_eq!(operand.ty, expected);
    operand
}

#[test]
fn all_eight_fixed_width_integer_types_retain_explicit_complement_hir() {
    let hir = build(
        "fn i8_not(a: I8) -> I8 { return ~a; } \
         fn i16_not(a: I16) -> I16 { return ~a; } \
         fn i32_not(a: I32) -> I32 { return ~a; } \
         fn i64_not(a: I64) -> I64 { return ~a; } \
         fn u8_not(a: U8) -> U8 { return ~a; } \
         fn u16_not(a: U16) -> U16 { return ~a; } \
         fn u32_not(a: U32) -> U32 { return ~a; } \
         fn u64_not(a: U64) -> U64 { return ~a; }",
    )
    .expect("all fixed-width integer complements are valid");

    for (name, ty) in [
        ("i8_not", IntrinsicType::I8),
        ("i16_not", IntrinsicType::I16),
        ("i32_not", IntrinsicType::I32),
        ("i64_not", IntrinsicType::I64),
        ("u8_not", IntrinsicType::U8),
        ("u16_not", IntrinsicType::U16),
        ("u32_not", IntrinsicType::U32),
        ("u64_not", IntrinsicType::U64),
    ] {
        let value = function(&hir, name)
            .body
            .terminal_return
            .as_ref()
            .and_then(|returned| returned.value.as_ref())
            .expect("integer-complement return value");
        let operand = integer_complement(value, ty);
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
             let wrong: Bool = ~take(value); \
             sink(value); \
         }",
    )
    .expect_err("integer complement cannot satisfy Bool");

    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::IntegerComplementRequiresInteger {
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
             let complemented: I8 = ~malformed(value, true); \
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
fn successful_operand_effects_commit_once_and_complement_adds_no_binding_transition() {
    let errors = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { \
             let complemented: I8 = ~take(value); \
             sink(value); \
         }",
    )
    .expect_err("successful operand call consumes its Ticket argument");
    assert_eq!(unavailable_count(&errors), 1);

    build(
        "fn f(value: I8) { \
             let complemented: I8 = ~value; \
             let still_available: I8 = value; \
         }",
    )
    .expect("integer complement itself adds no ownership transition");
}

#[test]
fn floating_and_conditional_bool_requirements_reject_before_operand_effects() {
    let floating = build("fn f() -> F32 { return ~(1.0); }")
        .expect_err("integer complement does not admit floating result type");
    assert!(has_diagnostic(
        &floating,
        DiagnosticKind::IntegerComplementRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::F32),
        }
    ));

    let conditional = build(
        "record Ticket {} \
         fn take(value: Ticket) -> I8 { return 1; } \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { if ~take(value) {} sink(value); }",
    )
    .expect_err("integer complement cannot satisfy the exact Bool condition type");
    assert!(has_diagnostic(
        &conditional,
        DiagnosticKind::IntegerComplementRequiresInteger {
            required: Type::Intrinsic(IntrinsicType::Bool),
        }
    ));
    assert_eq!(unavailable_count(&conditional), 0);
}

#[test]
fn nested_grouped_and_mixed_prefixes_retain_explicit_source_operation_tree() {
    let hir = build(
        "fn nested(value: I8) -> I8 { return ~~value; } \
         fn signed() -> I8 { return ~-1; } \
         fn grouped(value: I8) -> I8 { return ~(value + 1); } \
         fn multiplied(value: I8, other: I8) -> I8 { return ~value * other; } \
         fn negated(value: I8) -> I8 { return -~value; }",
    )
    .expect("nested and mixed integer complement is valid");

    let nested = function(&hir, "nested")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("nested return");
    let inner = integer_complement(nested, IntrinsicType::I8);
    integer_complement(inner, IntrinsicType::I8);

    let signed = function(&hir, "signed")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("signed return");
    let operand = integer_complement(signed, IntrinsicType::I8);
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
    let operand = integer_complement(grouped, IntrinsicType::I8);
    assert!(matches!(operand.kind, ValueKind::IntegerAdd { .. }));

    let multiplied = function(&hir, "multiplied")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("multiplied return");
    let ValueKind::IntegerMul { left, .. } = &multiplied.kind else {
        panic!("expected multiplication outside the tighter prefix complement");
    };
    integer_complement(left, IntrinsicType::I8);

    let negated = function(&hir, "negated")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("negated return");
    let ValueKind::IntegerNeg { operand } = &negated.kind else {
        panic!("expected integer negation outside the nested complement");
    };
    integer_complement(operand, IntrinsicType::I8);
}

#[test]
fn integer_complement_flows_through_existing_generic_value_consumers() {
    let hir = build(
        "record Boxed { value: I8 } \
         fn sink(value: I8) {} \
         fn f(value: I8) -> I8 { \
             let mut local: I8 = ~value; \
             local = ~local; \
             sink(~local); \
             let boxed: Boxed = Boxed { value: ~local }; \
             return ~boxed.value; \
         }",
    )
    .expect("IntegerComplement composes through generic Value consumers");

    let f = function(&hir, "f");
    let Statement::Local { initializer, .. } = &f.body.statements[0] else {
        panic!("expected local declaration");
    };
    integer_complement(initializer, IntrinsicType::I8);

    let Statement::Assignment { value, .. } = &f.body.statements[1] else {
        panic!("expected assignment");
    };
    integer_complement(value, IntrinsicType::I8);

    let Statement::Call { arguments, .. } = &f.body.statements[2] else {
        panic!("expected call statement");
    };
    integer_complement(&arguments[0], IntrinsicType::I8);

    let Statement::Local { initializer, .. } = &f.body.statements[3] else {
        panic!("expected record-construction local");
    };
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction");
    };
    integer_complement(&fields[0].value, IntrinsicType::I8);

    let returned = f
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("return value");
    integer_complement(returned, IntrinsicType::I8);
}