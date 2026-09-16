//! Consumer-level contract tests for direct semantic execution-result observation.
//! The tests intentionally use the selected function's result, not a host callback
//! which classifies an internal value and changes the function's result type.

use runen_core_ir::{BinaryFloatSign, BinaryFloatValue};
use runen_core_wasm::{ExecutionOutcome, ExecutionValue, FloatingScalarValue};
use runen_core_wasm_driver::{ExternalProviderBinding, ExternalScalarValue, RealizedCompilation};
use runen_hir::{FunctionId, ModuleId, SourceUnit, TypedCompilation, build_typed_hir};
use runen_syntax::parse_source;

fn compilation(source: &str) -> TypedCompilation {
    let parsed = parse_source(source.as_bytes()).expect("source must be valid UTF-8");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("source must produce accepted typed HIR")
}

fn function(compilation: &TypedCompilation, name: &str) -> FunctionId {
    compilation
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
        .id
}

fn represented_one(precision: u32) -> FloatingScalarValue {
    FloatingScalarValue::Represented(BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand: 1_u64 << (precision - 1),
        exponent: 0,
    })
}

#[test]
fn direct_f16_f32_f64_results_are_observed_by_the_selected_hir_function() {
    for (ty, precision) in [("F16", 11), ("F32", 24), ("F64", 53)] {
        let source = format!("fn other() -> I64 {{ return 9; }} fn selected() -> {ty} {{ return 1.0; }}");
        let compilation = compilation(&source);
        let selected = function(&compilation, "selected");
        let realized = RealizedCompilation::new(&compilation).expect("realization must succeed");
        let value = match ty {
            "F16" => ExecutionValue::F16(represented_one(precision)),
            "F32" => ExecutionValue::F32(represented_one(precision)),
            "F64" => ExecutionValue::F64(represented_one(precision)),
            _ => unreachable!(),
        };
        assert_eq!(realized.execute(selected).unwrap(), ExecutionOutcome::Returned(Some(value)));
    }
}

#[test]
fn selected_result_preserves_nested_declaration_order_and_empty_records() {
    let compilation = compilation(
        "record Empty {} \
         record Inner { value: F32 } \
         record Outer { empty: Empty, inner: Inner, count: I64 } \
         fn selected() -> Outer { \
             return Outer { empty: Empty {}, inner: Inner { value: 1.0 }, count: 7 }; \
         }",
    );
    let selected = function(&compilation, "selected");
    let realized = RealizedCompilation::new(&compilation).expect("nested records must realize");
    assert_eq!(
        realized.execute(selected).unwrap(),
        ExecutionOutcome::Returned(Some(ExecutionValue::Struct(vec![
            ExecutionValue::Struct(vec![]),
            ExecutionValue::Struct(vec![ExecutionValue::F32(represented_one(24))]),
            ExecutionValue::I64(7),
        ]))),
    );
}

#[test]
fn provider_nan_class_is_a_direct_result_and_a_nested_result_not_a_fault() {
    let compilation = compilation(
        "record Sample { value: F32, count: I64 } \
         external fn input() -> F32; \
         fn scalar() -> F32 { return input(); } \
         fn record_result() -> Sample { return Sample { value: input(), count: 7 }; }",
    );
    let input = function(&compilation, "input");
    let scalar = function(&compilation, "scalar");
    let record_result = function(&compilation, "record_result");
    let realized = RealizedCompilation::new_with_external_providers(
        &compilation,
        vec![ExternalProviderBinding::scalar_result(input, |arguments| {
            assert!(arguments.is_empty());
            ExternalScalarValue::F32(FloatingScalarValue::NaNClass)
        })],
    )
    .expect("semantic NaN provider must realize");
    assert_eq!(
        realized.execute(scalar).unwrap(),
        ExecutionOutcome::Returned(Some(ExecutionValue::F32(FloatingScalarValue::NaNClass))),
    );
    assert_eq!(
        realized.execute(record_result).unwrap(),
        ExecutionOutcome::Returned(Some(ExecutionValue::Struct(vec![
            ExecutionValue::F32(FloatingScalarValue::NaNClass),
            ExecutionValue::I64(7),
        ]))),
    );
}
