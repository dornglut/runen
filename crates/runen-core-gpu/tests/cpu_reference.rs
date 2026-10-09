//! Independent, always-on Core-Wasm observations for the same U32 functions
//! selected by the source-derived GPU integration. This is not GPU evidence.

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use runen_core_wasm::{ExecutionOutcome, ExecutionValue};
use runen_core_wasm_driver::{ExternalProviderBinding, ExternalScalarValue, RealizedCompilation};
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::parse_source;

fn assert_runen_u32_source_result(source: &str, constant: u32) {
    let parsed = parse_source(source.as_bytes()).expect("source must be valid UTF-8");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());

    let compilation = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("source must have accepted typed HIR");
    let external = compilation
        .functions
        .iter()
        .find(|f| f.name == "input" && f.is_external())
        .expect("source input declaration")
        .id;
    let entry = compilation
        .functions
        .iter()
        .find(|f| f.name == "cpu_entry" && !f.is_external())
        .expect("source CPU entry")
        .id;

    let provided = Arc::new(AtomicU32::new(0));
    let provider_value = Arc::clone(&provided);
    let realized = RealizedCompilation::new_with_external_providers(
        &compilation,
        vec![ExternalProviderBinding::scalar_result(
            external,
            move |arguments| {
                assert!(arguments.is_empty());
                ExternalScalarValue::U32(provider_value.load(Ordering::Relaxed))
            },
        )],
    )
    .expect("source-defined function and host scalar provider must realize");

    let mut inputs = vec![0_u32, 1, 65_536, u32::MAX, 2, 3, 65_535];
    for value in 0_u32..72 {
        inputs.push(value.wrapping_mul(1_103_515_245).wrapping_add(12_345));
    }
    for value in inputs {
        provided.store(value, Ordering::Relaxed);
        let expected = value.wrapping_mul(value).wrapping_add(constant);
        assert_eq!(
            realized.execute(entry).expect("Core-Wasm execution"),
            ExecutionOutcome::Returned(Some(ExecutionValue::U32(expected))),
            "Runen CPU result mismatch for source input {value}"
        );
    }
}

#[test]
fn source_derived_u32_transform_has_independent_cpu_observations() {
    assert_runen_u32_source_result(
        "external fn input() -> U32; \
         fn transform(value: U32) -> U32 { return value * value + 2; } \
         fn cpu_entry() -> U32 { return transform(input()); }",
        2,
    );
    assert_runen_u32_source_result(
        "external fn input() -> U32; \
         fn transform(value: U32) -> U32 { return value * value + 7; } \
         fn cpu_entry() -> U32 { return transform(input()); }",
        7,
    );
}
