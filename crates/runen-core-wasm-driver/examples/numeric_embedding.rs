//! A bounded Rust host consuming the selected Runen function's original result.
//!
//! Run with:
//! cargo +stable run -p runen-core-wasm-driver --example numeric_embedding --locked
//!
//! This is an embedding example, not a `main` convention, entry-argument API,
//! persistent session, source-file loader, or stable binary ABI.

use runen_core_ir::{BinaryFloatSign, BinaryFloatValue};
use runen_core_wasm::{ExecutionOutcome, ExecutionValue, FloatingScalarValue};
use runen_core_wasm_driver::{ExternalProviderBinding, ExternalScalarValue, RealizedCompilation};
use runen_hir::{FunctionId, ModuleId, SourceUnit, TypedCompilation, build_typed_hir};
use runen_syntax::parse_source;

const SOURCE: &str = r#"
    record Sample { value: F32, count: I64 }
    external fn input() -> F32;
    fn scalar() -> F32 { return input() + 2.0; }
    fn sample() -> Sample {
        return Sample { value: input() + 2.0, count: 7 };
    }
"#;

fn selected(compilation: &TypedCompilation, name: &str) -> Result<FunctionId, String> {
    compilation
        .functions
        .iter()
        .find(|function| function.name == name && !function.is_external())
        .map(|function| function.id)
        .ok_or_else(|| format!("ordinary HIR function {name:?} is missing"))
}

fn observed_result(
    realized: &RealizedCompilation,
    function: FunctionId,
) -> Result<ExecutionValue, String> {
    match realized.execute(function) {
        Ok(ExecutionOutcome::Returned(Some(value))) => Ok(value),
        Ok(ExecutionOutcome::Returned(None)) => Err("selected function returned no value".into()),
        Ok(ExecutionOutcome::Faulted(fault)) => Err(format!("Runen fault: {fault:?}")),
        Err(error) => Err(format!("realization or execution failure: {error:?}")),
    }
}

fn main() -> Result<(), String> {
    let parsed = parse_source(SOURCE.as_bytes()).map_err(|error| format!("UTF-8: {error:?}"))?;
    if !parsed.errors().is_empty() {
        return Err(format!("source diagnostics: {:?}", parsed.errors()));
    }
    // Module and function identities belong to this explicitly supplied compilation.
    let compilation = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .map_err(|error| format!("typed HIR: {error:?}"))?;
    let input = compilation
        .functions
        .iter()
        .find(|function| function.name == "input" && function.is_external())
        .map(|function| function.id)
        .ok_or_else(|| "external HIR input declaration is missing".to_owned())?;
    let scalar = selected(&compilation, "scalar")?;
    let sample = selected(&compilation, "sample")?;

    let host_input = BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand: 1_u64 << 23,
        exponent: 0,
    };
    let realized = RealizedCompilation::new_with_external_providers(
        &compilation,
        vec![ExternalProviderBinding::scalar_result(input, move |arguments| {
            assert!(arguments.is_empty(), "input provider must take zero arguments");
            ExternalScalarValue::F32(FloatingScalarValue::Represented(host_input))
        })],
    )
    .map_err(|error| format!("build/realization failure: {error:?}"))?;

    // Input 1.0 comes from the Rust provider; Runen computes 1.0 + 2.0.
    let expected = ExecutionValue::F32(FloatingScalarValue::Represented(
        BinaryFloatValue::Normal {
            sign: BinaryFloatSign::Positive,
            significand: 3_u64 << 22,
            exponent: 1,
        },
    ));
    let actual_scalar = observed_result(&realized, scalar)?;
    if actual_scalar != expected {
        return Err(format!("direct result mismatch: {actual_scalar:?}"));
    }
    let actual_sample = observed_result(&realized, sample)?;
    let expected_sample = ExecutionValue::Struct(vec![expected, ExecutionValue::I64(7)]);
    if actual_sample != expected_sample {
        return Err(format!("record result mismatch: {actual_sample:?}"));
    }
    println!("numeric embedding: direct F32 = 3, Sample {{ value: 3, count: 7 }}");
    Ok(())
}
