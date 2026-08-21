use runen_hir::{
    DiagnosticKind, IntrinsicType, LiteralValue, ModuleId, SourceUnit, Statement, Type, ValueKind,
    build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> runen_hir::TypedCompilation {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR")
}

fn errors(source: &str) -> Vec<runen_hir::Diagnostic> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect_err("test source must be rejected")
}

fn returned_literal(source_type: &str, spelling: &str) -> LiteralValue {
    let source = format!("fn value() -> {source_type} {{ return {spelling}; }}");
    let hir = build(&source);
    let value = hir.functions[0]
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("result-bearing function has one return value");
    let ValueKind::Literal(literal) = value.kind else {
        panic!("expected typed literal HIR value");
    };
    literal
}

#[test]
fn boolean_and_same_decimal_spelling_materialize_to_exact_required_types() {
    assert_eq!(returned_literal("Bool", "true"), LiteralValue::Bool(true));
    assert_eq!(returned_literal("Bool", "false"), LiteralValue::Bool(false));
    assert_eq!(returned_literal("I8", "1"), LiteralValue::I8(1));
    assert_eq!(returned_literal("U64", "1"), LiteralValue::U64(1));
}

#[test]
fn fixed_width_boundaries_and_zero_spellings_materialize_exactly() {
    let cases = [
        ("I8", "-128", LiteralValue::I8(i8::MIN)),
        ("I8", "127", LiteralValue::I8(i8::MAX)),
        ("I16", "-32768", LiteralValue::I16(i16::MIN)),
        ("I16", "32767", LiteralValue::I16(i16::MAX)),
        ("I32", "-2147483648", LiteralValue::I32(i32::MIN)),
        ("I32", "2147483647", LiteralValue::I32(i32::MAX)),
        (
            "I64",
            "-9223372036854775808",
            LiteralValue::I64(i64::MIN),
        ),
        (
            "I64",
            "9223372036854775807",
            LiteralValue::I64(i64::MAX),
        ),
        ("U8", "255", LiteralValue::U8(u8::MAX)),
        ("U16", "65535", LiteralValue::U16(u16::MAX)),
        ("U32", "4294967295", LiteralValue::U32(u32::MAX)),
        (
            "U64",
            "18446744073709551615",
            LiteralValue::U64(u64::MAX),
        ),
        ("I8", "00", LiteralValue::I8(0)),
        ("I8", "-00", LiteralValue::I8(0)),
        ("U8", "0", LiteralValue::U8(0)),
        ("U8", "-00", LiteralValue::U8(0)),
    ];

    for (source_type, spelling, expected) in cases {
        assert_eq!(returned_literal(source_type, spelling), expected);
    }
}

#[test]
fn integer_literal_diagnostics_distinguish_context_kind_from_range_failure() {
    let wrong_kind = errors("fn bad() -> Bool { return 1; }");
    assert!(wrong_kind.iter().any(|diagnostic| {
        diagnostic.kind
            == DiagnosticKind::IntegerLiteralRequiresInteger {
                required: Type::Intrinsic(IntrinsicType::Bool),
            }
    }));

    let floating = errors("fn bad() -> F32 { return 1; }");
    assert!(floating.iter().any(|diagnostic| {
        diagnostic.kind
            == DiagnosticKind::IntegerLiteralRequiresInteger {
                required: Type::Intrinsic(IntrinsicType::F32),
            }
    }));

    let record = errors("record Ticket {} fn bad() -> Ticket { return 1; }");
    assert!(record.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::IntegerLiteralRequiresInteger {
            required: Type::Record(_)
        }
    )));

    for source in [
        "fn bad() -> I8 { return 128; }",
        "fn bad() -> I8 { return -129; }",
        "fn bad() -> U8 { return 256; }",
        "fn bad() -> U8 { return -1; }",
        "fn bad() -> U64 { return 999999999999999999999999999999999999999999999999999999999999; }",
    ] {
        let diagnostics = errors(source);
        assert!(diagnostics.iter().any(|diagnostic| matches!(
            diagnostic.kind,
            DiagnosticKind::IntegerLiteralOutOfRange { .. }
        )));
    }
}

#[test]
fn boolean_literal_uses_existing_exact_type_mismatch_boundary() {
    let diagnostics = errors("fn bad() -> I8 { return true; }");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.kind
            == DiagnosticKind::TypeMismatch {
                expected: Type::Intrinsic(IntrinsicType::I8),
                found: Type::Intrinsic(IntrinsicType::Bool),
            }
    }));
}

#[test]
fn no_result_return_rejects_literal_without_manufacturing_an_integer_context() {
    let diagnostics = errors("fn bad() { return 1; }");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnexpectedResultValue)
    );
    assert!(!diagnostics.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::IntegerLiteralRequiresInteger { .. }
            | DiagnosticKind::IntegerLiteralOutOfRange { .. }
    )));
}

#[test]
fn every_value_consumer_supplies_its_required_type_without_consuming_unrelated_bindings() {
    let hir = build(
        "record Ticket {} \
         fn id(value: U64) -> U64 { return value; } \
         fn test(ticket: Ticket) -> Ticket { \
             let mut number: U64 = 1; \
             number = 2; \
             let result: U64 = id(18446744073709551615); \
             return ticket; \
         } \
         fn direct_return() -> I16 { return -32768; }",
    );

    let test = hir
        .functions
        .iter()
        .find(|function| function.name == "test")
        .expect("test function");

    let Statement::Local { initializer, .. } = &test.body.statements[0] else {
        panic!("expected literal local initializer");
    };
    assert_eq!(initializer.ty, Type::Intrinsic(IntrinsicType::U64));
    assert_eq!(initializer.kind, ValueKind::Literal(LiteralValue::U64(1)));

    let Statement::Assignment { value, .. } = &test.body.statements[1] else {
        panic!("expected literal assignment");
    };
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::U64));
    assert_eq!(value.kind, ValueKind::Literal(LiteralValue::U64(2)));

    let Statement::Local { initializer, .. } = &test.body.statements[2] else {
        panic!("expected result-bearing call initializer");
    };
    let ValueKind::DirectCall { arguments, .. } = &initializer.kind else {
        panic!("expected direct call value");
    };
    assert_eq!(arguments.len(), 1);
    assert_eq!(arguments[0].ty, Type::Intrinsic(IntrinsicType::U64));
    assert_eq!(
        arguments[0].kind,
        ValueKind::Literal(LiteralValue::U64(u64::MAX))
    );

    let returned = test
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("ticket return remains available after unrelated literals");
    assert!(matches!(returned.kind, ValueKind::BindingUse { .. }));

    let direct_return = hir
        .functions
        .iter()
        .find(|function| function.name == "direct_return")
        .and_then(|function| function.body.terminal_return.as_ref())
        .and_then(|returned| returned.value.as_ref())
        .expect("direct literal return");
    assert_eq!(direct_return.ty, Type::Intrinsic(IntrinsicType::I16));
    assert_eq!(
        direct_return.kind,
        ValueKind::Literal(LiteralValue::I16(i16::MIN))
    );
}
