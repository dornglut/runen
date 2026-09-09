use runen_core_ir::{
    BinaryFloatSign, BinaryFloatValue, Function, Operand, Statement, ValidatedProgram, Value,
};
use runen_core_lowering::lower;
use runen_hir::{ImportTarget, ModuleId, SourceUnit, build_typed_hir};
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

fn lower_qualified(dependency: &str, caller: &str) -> ValidatedProgram {
    let dependency = parse(dependency);
    let caller = parse(caller);
    assert!(dependency.errors().is_empty(), "{:?}", dependency.errors());
    assert!(caller.errors().is_empty(), "{:?}", caller.errors());
    let imports = [ImportTarget::new("dep", ModuleId::new(2)).expect("valid import alias")];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("qualified constant source must produce accepted HIR");
    lower(&hir).expect("accepted qualified constant HIR must lower to validated Core")
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
fn every_admitted_constant_scalar_lowers_through_existing_core_constants() {
    let lowered = lower_source(
        r#"
const BOOL_VALUE: Bool = true;
const I8_VALUE: I8 = -128;
const I16_VALUE: I16 = -32768;
const I32_VALUE: I32 = -2147483648;
const I64_VALUE: I64 = -9223372036854775808;
const U8_VALUE: U8 = 255;
const U16_VALUE: U16 = 65535;
const U32_VALUE: U32 = 4294967295;
const U64_VALUE: U64 = 18446744073709551615;
const F16_VALUE: F16 = 1.0;
const F32_VALUE: F32 = 1.0;
const F64_VALUE: F64 = 1.0;
fn bool_value() -> Bool { return BOOL_VALUE; }
fn i8_value() -> I8 { return I8_VALUE; }
fn i16_value() -> I16 { return I16_VALUE; }
fn i32_value() -> I32 { return I32_VALUE; }
fn i64_value() -> I64 { return I64_VALUE; }
fn u8_value() -> U8 { return U8_VALUE; }
fn u16_value() -> U16 { return U16_VALUE; }
fn u32_value() -> U32 { return U32_VALUE; }
fn u64_value() -> U64 { return U64_VALUE; }
fn f16_value() -> F16 { return F16_VALUE; }
fn f32_value() -> F32 { return F32_VALUE; }
fn f64_value() -> F64 { return F64_VALUE; }
"#,
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
fn floating_constant_semantic_classes_survive_existing_literal_lowering_exactly() {
    let lowered = lower_source(
        r#"
const NEGATIVE_ZERO: F16 = -0.0;
const MINIMUM_SUBNORMAL: F16 = 0.000000059604644775390625;
const NORMAL_VALUE: F16 = 1.0;
const NEGATIVE_INFINITY: F16 = -65520.0;
fn negative_zero() -> F16 { return NEGATIVE_ZERO; }
fn minimum_subnormal() -> F16 { return MINIMUM_SUBNORMAL; }
fn normal_value() -> F16 { return NORMAL_VALUE; }
fn negative_infinity() -> F16 { return NEGATIVE_INFINITY; }
"#,
    );
    let program = lowered.as_program();

    assert_eq!(
        constants(function(program, "negative_zero")),
        vec![&Value::F16(BinaryFloatValue::Zero(
            BinaryFloatSign::Negative
        ))]
    );
    assert_eq!(
        constants(function(program, "minimum_subnormal")),
        vec![&Value::F16(BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1,
        })]
    );
    assert_eq!(
        constants(function(program, "normal_value")),
        vec![&Value::F16(normal(BinaryFloatSign::Positive, 1 << 10, 0))]
    );
    assert_eq!(
        constants(function(program, "negative_infinity")),
        vec![&Value::F16(BinaryFloatValue::Infinity(
            BinaryFloatSign::Negative
        ))]
    );
}

#[test]
fn qualified_constant_erases_before_core_and_creates_no_runtime_initializer_function() {
    let lowered = lower_qualified(
        "export const ANSWER: U64 = 18446744073709551615;",
        "import dep; fn entry() -> U64 { return dep::ANSWER; }",
    );
    let program = lowered.as_program();

    assert_eq!(
        program.functions.len(),
        1,
        "constant declaration must not lower to a runtime initializer or global function"
    );
    assert_eq!(
        constants(function(program, "entry")),
        vec![&Value::U64(u64::MAX)]
    );
}
