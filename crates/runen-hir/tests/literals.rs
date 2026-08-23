use runen_hir::{
    BinaryFloatSign, BinaryFloatValue, DiagnosticKind, IntrinsicType, LiteralValue, ModuleId,
    SourceUnit, Statement, Type, ValueKind, build_typed_hir,
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

fn normal(sign: BinaryFloatSign, significand: u64, exponent: i16) -> BinaryFloatValue {
    BinaryFloatValue::Normal {
        sign,
        significand,
        exponent,
    }
}

fn subnormal(sign: BinaryFloatSign, significand: u64) -> BinaryFloatValue {
    BinaryFloatValue::Subnormal { sign, significand }
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
        ("I64", "-9223372036854775808", LiteralValue::I64(i64::MIN)),
        ("I64", "9223372036854775807", LiteralValue::I64(i64::MAX)),
        ("U8", "255", LiteralValue::U8(u8::MAX)),
        ("U16", "65535", LiteralValue::U16(u16::MAX)),
        ("U32", "4294967295", LiteralValue::U32(u32::MAX)),
        ("U64", "18446744073709551615", LiteralValue::U64(u64::MAX)),
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
fn every_fixed_width_integer_rejects_values_immediately_outside_its_domain() {
    let cases = [
        ("I8", "128"),
        ("I8", "-129"),
        ("I16", "32768"),
        ("I16", "-32769"),
        ("I32", "2147483648"),
        ("I32", "-2147483649"),
        ("I64", "9223372036854775808"),
        ("I64", "-9223372036854775809"),
        ("U8", "256"),
        ("U8", "-1"),
        ("U16", "65536"),
        ("U16", "-1"),
        ("U32", "4294967296"),
        ("U32", "-1"),
        ("U64", "18446744073709551616"),
        ("U64", "-1"),
    ];

    for (source_type, spelling) in cases {
        let source = format!("fn bad() -> {source_type} {{ return {spelling}; }}");
        let diagnostics = errors(&source);
        assert!(
            diagnostics.iter().any(|diagnostic| matches!(
                diagnostic.kind,
                DiagnosticKind::IntegerLiteralOutOfRange { .. }
            )),
            "missing out-of-range diagnostic for {source_type} literal {spelling}"
        );
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

    let too_wide = errors(
        "fn bad() -> U64 { return 999999999999999999999999999999999999999999999999999999999999; }",
    );
    assert!(too_wide.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::IntegerLiteralOutOfRange { .. }
    )));
}

#[test]
fn floating_spelling_materializes_to_each_exact_required_format() {
    assert_eq!(
        returned_literal("F16", "1.0"),
        LiteralValue::F16(normal(BinaryFloatSign::Positive, 1 << 10, 0))
    );
    assert_eq!(
        returned_literal("F32", "1.0"),
        LiteralValue::F32(normal(BinaryFloatSign::Positive, 1 << 23, 0))
    );
    assert_eq!(
        returned_literal("F64", "1.0"),
        LiteralValue::F64(normal(BinaryFloatSign::Positive, 1 << 52, 0))
    );
}

#[test]
fn floating_literal_rejects_nonfloating_required_types_without_conversion() {
    for (source_type, required) in [
        ("Bool", Type::Intrinsic(IntrinsicType::Bool)),
        ("I32", Type::Intrinsic(IntrinsicType::I32)),
    ] {
        let source = format!("fn bad() -> {source_type} {{ return 1.0; }}");
        let diagnostics = errors(&source);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == DiagnosticKind::FloatingLiteralRequiresFloating { required }
        }));
    }

    let diagnostics = errors("record Ticket {} fn bad() -> Ticket { return 1.0; }");
    assert!(diagnostics.iter().any(|diagnostic| matches!(
        diagnostic.kind,
        DiagnosticKind::FloatingLiteralRequiresFloating {
            required: Type::Record(_)
        }
    )));

    let integer_looking = errors("fn bad() -> F32 { return 1; }");
    assert!(integer_looking.iter().any(|diagnostic| {
        diagnostic.kind
            == DiagnosticKind::IntegerLiteralRequiresInteger {
                required: Type::Intrinsic(IntrinsicType::F32),
            }
    }));
}

#[test]
fn floating_zero_preserves_source_sign_and_redundant_zero_digits() {
    for spelling in ["0.0", "00.000"] {
        assert_eq!(
            returned_literal("F16", spelling),
            LiteralValue::F16(BinaryFloatValue::Zero(BinaryFloatSign::Positive))
        );
    }
    for spelling in ["-0.0", "-00.000"] {
        assert_eq!(
            returned_literal("F16", spelling),
            LiteralValue::F16(BinaryFloatValue::Zero(BinaryFloatSign::Negative))
        );
    }
}

#[test]
fn f16_subnormal_and_zero_boundary_round_exactly() {
    assert_eq!(
        returned_literal("F16", "0.000000059604644775390625"),
        LiteralValue::F16(subnormal(BinaryFloatSign::Positive, 1))
    );
    assert_eq!(
        returned_literal("F16", "0.0000000298023223876953125"),
        LiteralValue::F16(BinaryFloatValue::Zero(BinaryFloatSign::Positive))
    );
    assert_eq!(
        returned_literal("F16", "-0.0000000298023223876953125"),
        LiteralValue::F16(BinaryFloatValue::Zero(BinaryFloatSign::Negative))
    );
    assert_eq!(
        returned_literal("F16", "0.00000002980232238769531249"),
        LiteralValue::F16(BinaryFloatValue::Zero(BinaryFloatSign::Positive))
    );
    assert_eq!(
        returned_literal("F16", "0.00000002980232238769531251"),
        LiteralValue::F16(subnormal(BinaryFloatSign::Positive, 1))
    );
}

#[test]
fn f16_interior_ties_and_subnormal_normal_carry_use_nearest_even() {
    assert_eq!(
        returned_literal("F16", "1.00048828125"),
        LiteralValue::F16(normal(BinaryFloatSign::Positive, 1024, 0))
    );
    assert_eq!(
        returned_literal("F16", "1.00146484375"),
        LiteralValue::F16(normal(BinaryFloatSign::Positive, 1026, 0))
    );
    assert_eq!(
        returned_literal("F16", "0.0000610053539276123046875"),
        LiteralValue::F16(normal(BinaryFloatSign::Positive, 1024, -14))
    );
}

#[test]
fn f16_upper_boundary_rounds_finite_text_to_finite_or_infinity_exactly() {
    assert_eq!(
        returned_literal("F16", "65504.0"),
        LiteralValue::F16(normal(BinaryFloatSign::Positive, 2047, 15))
    );
    assert_eq!(
        returned_literal("F16", "65519.999"),
        LiteralValue::F16(normal(BinaryFloatSign::Positive, 2047, 15))
    );
    assert_eq!(
        returned_literal("F16", "65520.0"),
        LiteralValue::F16(BinaryFloatValue::Infinity(BinaryFloatSign::Positive))
    );
    assert_eq!(
        returned_literal("F16", "-65520.0"),
        LiteralValue::F16(BinaryFloatValue::Infinity(BinaryFloatSign::Negative))
    );
    assert_eq!(
        returned_literal("F16", "999999.0"),
        LiteralValue::F16(BinaryFloatValue::Infinity(BinaryFloatSign::Positive))
    );
}

#[test]
fn tiny_huge_and_long_decimal_inputs_do_not_use_host_numeric_ranges() {
    let tiny_f32 = format!("0.{}1", "0".repeat(44));
    assert_eq!(
        returned_literal("F32", &tiny_f32),
        LiteralValue::F32(subnormal(BinaryFloatSign::Positive, 1))
    );

    let tiny_f64 = format!("0.{}1", "0".repeat(323));
    assert_eq!(
        returned_literal("F64", &tiny_f64),
        LiteralValue::F64(BinaryFloatValue::Zero(BinaryFloatSign::Positive))
    );

    let huge = format!("{}.0", "9".repeat(2000));
    assert_eq!(
        returned_literal("F32", &huge),
        LiteralValue::F32(BinaryFloatValue::Infinity(BinaryFloatSign::Positive))
    );
    assert_eq!(
        returned_literal("F64", &huge),
        LiteralValue::F64(BinaryFloatValue::Infinity(BinaryFloatSign::Positive))
    );

    let long_one = format!("{}1.{}", "0".repeat(2000), "0".repeat(2000));
    assert_eq!(
        returned_literal("F64", &long_one),
        LiteralValue::F64(normal(BinaryFloatSign::Positive, 1 << 52, 0))
    );
}

#[test]
fn floating_conditional_is_rejected_by_existing_exact_bool_requirement() {
    for source in ["fn bad() { if 1.0 {} }", "fn bad() { while -2.0 {} }"] {
        let diagnostics = errors(source);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.kind
                == DiagnosticKind::FloatingLiteralRequiresFloating {
                    required: Type::Intrinsic(IntrinsicType::Bool),
                }
        }));
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
fn no_result_return_rejects_literal_without_manufacturing_a_numeric_context() {
    for source in ["fn bad() { return 1; }", "fn bad() { return 1.0; }"] {
        let diagnostics = errors(source);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.kind == DiagnosticKind::UnexpectedResultValue)
        );
        assert!(!diagnostics.iter().any(|diagnostic| matches!(
            diagnostic.kind,
            DiagnosticKind::IntegerLiteralRequiresInteger { .. }
                | DiagnosticKind::IntegerLiteralOutOfRange { .. }
                | DiagnosticKind::FloatingLiteralRequiresFloating { .. }
        )));
    }
}

#[test]
fn every_value_consumer_supplies_its_required_type_without_consuming_unrelated_bindings() {
    let hir = build(
        "record Ticket {} \
         record Sample { value: F32 } \
         fn id(value: U64) -> U64 { return value; } \
         fn id_float(value: F32) -> F32 { return value; } \
         fn test(ticket: Ticket) -> Ticket { \
             let mut number: U64 = 1; \
             number = 2; \
             let result: U64 = id(18446744073709551615); \
             let mut floating: F32 = 1.0; \
             floating = 2.0; \
             let float_result: F32 = id_float(3.0); \
             let sample: Sample = Sample { value: 4.0 }; \
             return ticket; \
         } \
         fn direct_return() -> I16 { return -32768; } \
         fn float_return() -> F32 { return 5.0; }",
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

    let Statement::Local { initializer, .. } = &test.body.statements[3] else {
        panic!("expected floating local initializer");
    };
    assert_eq!(initializer.ty, Type::Intrinsic(IntrinsicType::F32));
    assert_eq!(
        initializer.kind,
        ValueKind::Literal(LiteralValue::F32(normal(
            BinaryFloatSign::Positive,
            1 << 23,
            0,
        )))
    );

    let Statement::Assignment { value, .. } = &test.body.statements[4] else {
        panic!("expected floating literal assignment");
    };
    assert_eq!(value.ty, Type::Intrinsic(IntrinsicType::F32));

    let Statement::Local { initializer, .. } = &test.body.statements[5] else {
        panic!("expected floating result-bearing call initializer");
    };
    let ValueKind::DirectCall { arguments, .. } = &initializer.kind else {
        panic!("expected floating direct call value");
    };
    assert_eq!(arguments[0].ty, Type::Intrinsic(IntrinsicType::F32));
    assert!(matches!(
        arguments[0].kind,
        ValueKind::Literal(LiteralValue::F32(_))
    ));

    let Statement::Local { initializer, .. } = &test.body.statements[6] else {
        panic!("expected floating record initializer");
    };
    let Type::Record(record) = initializer.ty else {
        panic!("expected Sample record type");
    };
    assert_eq!(hir.record(record).name, "Sample");
    let ValueKind::RecordConstruction { fields, .. } = &initializer.kind else {
        panic!("expected record construction initializer");
    };
    assert_eq!(fields.len(), 1);
    assert!(matches!(
        fields[0].value.kind,
        ValueKind::Literal(LiteralValue::F32(_))
    ));

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

    let float_return = hir
        .functions
        .iter()
        .find(|function| function.name == "float_return")
        .and_then(|function| function.body.terminal_return.as_ref())
        .and_then(|returned| returned.value.as_ref())
        .expect("direct floating literal return");
    assert_eq!(float_return.ty, Type::Intrinsic(IntrinsicType::F32));
    assert!(matches!(
        float_return.kind,
        ValueKind::Literal(LiteralValue::F32(_))
    ));
}
