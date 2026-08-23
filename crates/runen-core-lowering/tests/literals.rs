use runen_core_ir::{
    BinaryFloatSign, BinaryFloatValue, Function, Operand, Statement, Terminator, ValidatedProgram,
    Value,
};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR");
    lower(&hir).expect("accepted HIR must lower to validated Core")
}

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a Function {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn constants(function: &Function) -> Vec<&Value> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| match statement {
            Statement::Init {
                src: Operand::Constant(value),
                ..
            } => Some(value),
            _ => None,
        })
        .collect()
}

fn normal(sign: BinaryFloatSign, significand: u64, exponent: i16) -> BinaryFloatValue {
    BinaryFloatValue::Normal {
        sign,
        significand,
        exponent,
    }
}

#[test]
fn every_typed_literal_variant_lowers_to_the_exact_core_value_variant() {
    let lowered = lower_source(
        "fn bool_value() -> Bool { return true; } \
         fn i8_value() -> I8 { return -128; } \
         fn i16_value() -> I16 { return -32768; } \
         fn i32_value() -> I32 { return -2147483648; } \
         fn i64_value() -> I64 { return -9223372036854775808; } \
         fn u8_value() -> U8 { return 255; } \
         fn u16_value() -> U16 { return 65535; } \
         fn u32_value() -> U32 { return 4294967295; } \
         fn u64_value() -> U64 { return 18446744073709551615; } \
         fn f16_value() -> F16 { return 1.0; } \
         fn f32_value() -> F32 { return 1.0; } \
         fn f64_value() -> F64 { return 1.0; }",
    );
    let program = lowered.as_program();
    let cases = [
        ("bool_value", Value::Bool(true)),
        ("i8_value", Value::I8(i8::MIN)),
        ("i16_value", Value::I16(i16::MIN)),
        ("i32_value", Value::I32(i32::MIN)),
        ("i64_value", Value::I64(i64::MIN)),
        ("u8_value", Value::U8(u8::MAX)),
        ("u16_value", Value::U16(u16::MAX)),
        ("u32_value", Value::U32(u32::MAX)),
        ("u64_value", Value::U64(u64::MAX)),
        (
            "f16_value",
            Value::F16(normal(BinaryFloatSign::Positive, 1 << 10, 0)),
        ),
        (
            "f32_value",
            Value::F32(normal(BinaryFloatSign::Positive, 1 << 23, 0)),
        ),
        (
            "f64_value",
            Value::F64(normal(BinaryFloatSign::Positive, 1 << 52, 0)),
        ),
    ];

    for (name, expected) in cases {
        assert_eq!(constants(function(program, name)), vec![&expected]);
    }
}

#[test]
fn u64_max_and_i64_min_survive_source_hir_and_core_validation_without_narrowing() {
    let lowered = lower_source(
        "fn widest() -> U64 { return 18446744073709551615; } \
         fn lowest() -> I64 { return -9223372036854775808; }",
    );
    let program = lowered.as_program();

    assert_eq!(
        constants(function(program, "widest")),
        vec![&Value::U64(u64::MAX)]
    );
    assert_eq!(
        constants(function(program, "lowest")),
        vec![&Value::I64(i64::MIN)]
    );
}

#[test]
fn floating_semantic_classes_survive_source_hir_and_core_validation_exactly() {
    let lowered = lower_source(
        "fn classes() -> F16 { \
             let negative_zero: F16 = -0.0; \
             let minimum_subnormal: F16 = 0.000000059604644775390625; \
             let normal_value: F16 = 1.0; \
             return -65520.0; \
         }",
    );
    let program = lowered.as_program();

    assert_eq!(
        constants(function(program, "classes")),
        vec![
            &Value::F16(BinaryFloatValue::Zero(BinaryFloatSign::Negative)),
            &Value::F16(BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Positive,
                significand: 1,
            }),
            &Value::F16(normal(BinaryFloatSign::Positive, 1 << 10, 0)),
            &Value::F16(BinaryFloatValue::Infinity(BinaryFloatSign::Negative)),
        ]
    );
}

#[test]
fn composed_literal_consumers_preserve_existing_lowering_order_and_validate() {
    let lowered = lower_source(
        "fn id(value: U64) -> U64 { return value; } \
         fn entry() -> U64 { \
             let mut local: U64 = 1; \
             local = 2; \
             let result: U64 = id(3); \
             return 4; \
         }",
    );
    let program = lowered.as_program();
    let entry = function(program, "entry");

    assert_eq!(
        constants(entry),
        vec![
            &Value::U64(1),
            &Value::U64(2),
            &Value::U64(3),
            &Value::U64(4),
        ]
    );

    let calls = entry
        .body
        .blocks
        .iter()
        .filter_map(|block| match &block.terminator {
            Terminator::Call {
                arguments,
                destination,
                ..
            } => Some((arguments, destination)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0.len(), 1);
    assert!(
        calls[0].1.is_some(),
        "result-bearing call keeps its destination"
    );

    let last_block = entry.body.blocks.last().expect("entry has a final block");
    assert!(matches!(
        last_block.terminator,
        Terminator::Return(Some(Operand::Move(_)))
    ));
}

#[test]
fn composed_floating_consumers_preserve_existing_lowering_order_and_validate() {
    let lowered = lower_source(
        "record Sample { value: F32 } \
         fn id(value: F32) -> F32 { return value; } \
         fn entry() -> F32 { \
             let mut local: F32 = 1.0; \
             local = 2.0; \
             let result: F32 = id(3.0); \
             let sample: Sample = Sample { value: 4.0 }; \
             return 5.0; \
         }",
    );
    let program = lowered.as_program();
    let entry = function(program, "entry");

    assert_eq!(
        constants(entry),
        vec![
            &Value::F32(normal(BinaryFloatSign::Positive, 1 << 23, 0)),
            &Value::F32(normal(BinaryFloatSign::Positive, 1 << 23, 1)),
            &Value::F32(normal(BinaryFloatSign::Positive, 12_582_912, 1)),
            &Value::F32(normal(BinaryFloatSign::Positive, 1 << 23, 2)),
            &Value::F32(normal(BinaryFloatSign::Positive, 10_485_760, 2)),
        ]
    );

    let calls = entry
        .body
        .blocks
        .iter()
        .filter_map(|block| match &block.terminator {
            Terminator::Call {
                arguments,
                destination,
                ..
            } => Some((arguments, destination)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0.len(), 1);
    assert!(calls[0].1.is_some());

    let last_block = entry.body.blocks.last().expect("entry has a final block");
    assert!(matches!(
        last_block.terminator,
        Terminator::Return(Some(Operand::Move(_)))
    ));
}
