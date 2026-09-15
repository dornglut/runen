use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use runen_core_ir::{BinaryFloatSign, BinaryFloatValue, Value};
use runen_core_wasm::{ExecutionOutcome, FloatingScalarValue};
use runen_core_wasm_driver::{
    BuildError, ExternalProviderBinding, ExternalScalarValue, RealizedCompilation,
};
use runen_hir::{FunctionId, ModuleId, SourceUnit, TypedCompilation, build_typed_hir};
use runen_syntax::parse_source;

fn compilation(source: &str) -> TypedCompilation {
    let parsed = parse_source(source.as_bytes()).expect("test source must be valid UTF-8");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted typed HIR")
}

fn function(compilation: &TypedCompilation, name: &str) -> FunctionId {
    compilation
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
        .id
}

fn execute_equal_interface_second(source: &str) -> ExecutionOutcome {
    let compilation = compilation(source);
    let first = function(&compilation, "first");
    let second = function(&compilation, "second");
    let entry = function(&compilation, "entry");
    let realized = RealizedCompilation::new_with_external_providers(
        &compilation,
        vec![
            ExternalProviderBinding::scalar_result(first, |arguments| {
                assert!(arguments.is_empty());
                ExternalScalarValue::I64(11)
            }),
            ExternalProviderBinding::scalar_result(second, |arguments| {
                assert!(arguments.is_empty());
                ExternalScalarValue::I64(22)
            }),
        ],
    )
    .expect("HIR-keyed equal-interface providers must realize");
    realized.execute(entry).expect("entry must execute")
}

#[test]
fn equal_interfaces_remain_hir_identity_keyed_across_declaration_reordering() {
    let first_order =
        "external fn first() -> I64; external fn second() -> I64; fn entry() -> I64 { return second(); }";
    let second_order =
        "external fn second() -> I64; external fn first() -> I64; fn entry() -> I64 { return second(); }";

    assert_eq!(
        execute_equal_interface_second(first_order),
        ExecutionOutcome::Returned(Some(Value::I64(22)))
    );
    assert_eq!(
        execute_equal_interface_second(second_order),
        ExecutionOutcome::Returned(Some(Value::I64(22)))
    );
}

#[test]
fn scalar_result_provider_composes_from_hir_identity_to_execution() {
    let compilation = compilation(
        "external fn transform(I64, Bool) -> U64; \
         fn entry() -> U64 { return transform(7, true); }",
    );
    let transform = function(&compilation, "transform");
    let entry = function(&compilation, "entry");
    let realized = RealizedCompilation::new_with_external_providers(
        &compilation,
        vec![ExternalProviderBinding::scalar_result(
            transform,
            |arguments| {
                assert_eq!(
                    arguments,
                    &[
                        ExternalScalarValue::I64(7),
                        ExternalScalarValue::Bool(true),
                    ]
                );
                ExternalScalarValue::U64(42)
            },
        )],
    )
    .expect("matching scalar provider must realize");

    assert_eq!(
        realized.execute(entry).unwrap(),
        ExecutionOutcome::Returned(Some(Value::U64(42)))
    );
}

#[test]
fn no_result_provider_runs_once_and_runen_execution_continues() {
    let compilation = compilation(
        "external fn sink(I64); \
         fn entry() -> I64 { sink(7); return 9; }",
    );
    let sink = function(&compilation, "sink");
    let entry = function(&compilation, "entry");
    let calls = Arc::new(AtomicUsize::new(0));
    let seen_calls = Arc::clone(&calls);
    let realized = RealizedCompilation::new_with_external_providers(
        &compilation,
        vec![ExternalProviderBinding::no_result(sink, move |arguments| {
            assert_eq!(arguments, &[ExternalScalarValue::I64(7)]);
            seen_calls.fetch_add(1, Ordering::SeqCst);
        })],
    )
    .expect("matching no-result provider must realize");

    assert_eq!(
        realized.execute(entry).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(9)))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn floating_nan_class_round_trips_between_hir_keyed_providers() {
    let compilation = compilation(
        "external fn transform(F32) -> F32; \
         external fn classify(F32) -> Bool; \
         fn entry() -> Bool { return classify(transform(1.0)); }",
    );
    let transform = function(&compilation, "transform");
    let classify = function(&compilation, "classify");
    let entry = function(&compilation, "entry");
    let expected_source_value = BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand: 1_u64 << 23,
        exponent: 0,
    };
    let realized = RealizedCompilation::new_with_external_providers(
        &compilation,
        vec![
            ExternalProviderBinding::scalar_result(transform, move |arguments| {
                assert_eq!(
                    arguments,
                    &[ExternalScalarValue::F32(FloatingScalarValue::Represented(
                        expected_source_value,
                    ))]
                );
                ExternalScalarValue::F32(FloatingScalarValue::NaNClass)
            }),
            ExternalProviderBinding::scalar_result(classify, |arguments| {
                assert_eq!(
                    arguments,
                    &[ExternalScalarValue::F32(FloatingScalarValue::NaNClass)]
                );
                ExternalScalarValue::Bool(true)
            }),
        ],
    )
    .expect("floating provider chain must realize");

    assert_eq!(
        realized.execute(entry).unwrap(),
        ExecutionOutcome::Returned(Some(Value::Bool(true)))
    );
}

#[test]
fn generic_specialization_and_closure_share_one_hir_keyed_external_provider() {
    let compilation = compilation(
        "external fn ext(I64) -> I64; \
         fn generic[T](value: T) -> I64 { return ext(7); } \
         fn entry() -> I64 { \
             let seed: I64 = 0; \
             let call = fn[seed](value: I64) -> I64 { return ext(value) + seed; }; \
             let first: I64 = generic[I64](1); \
             return call(first); \
         }",
    );
    let ext = function(&compilation, "ext");
    let entry = function(&compilation, "entry");
    let seen = Arc::new(Mutex::new(Vec::new()));
    let provider_seen = Arc::clone(&seen);
    let realized = RealizedCompilation::new_with_external_providers(
        &compilation,
        vec![ExternalProviderBinding::scalar_result(ext, move |arguments| {
            let [ExternalScalarValue::I64(value)] = arguments else {
                panic!("external provider must receive exactly one I64 argument");
            };
            provider_seen.lock().unwrap().push(*value);
            ExternalScalarValue::I64(value + 1)
        })],
    )
    .expect("generic and closure external call paths must realize");

    assert_eq!(
        realized.execute(entry).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(9)))
    );
    assert_eq!(*seen.lock().unwrap(), vec![7, 8]);
}

#[test]
fn provider_key_must_name_an_external_in_the_current_compilation() {
    let compilation = compilation("fn ordinary() -> I64 { return 1; }");
    let ordinary = function(&compilation, "ordinary");

    assert!(matches!(
        RealizedCompilation::new_with_external_providers(
            &compilation,
            vec![ExternalProviderBinding::scalar_result(ordinary, |_| {
                ExternalScalarValue::I64(1)
            })],
        ),
        Err(BuildError::ProviderNotExternal(function)) if function == ordinary
    ));
}

#[test]
fn duplicate_provider_is_rejected_by_hir_identity_before_realization() {
    let compilation =
        compilation("external fn ext() -> I64; fn entry() -> I64 { return ext(); }");
    let ext = function(&compilation, "ext");

    assert!(matches!(
        RealizedCompilation::new_with_external_providers(
            &compilation,
            vec![
                ExternalProviderBinding::scalar_result(ext, |_| ExternalScalarValue::I64(1)),
                ExternalProviderBinding::scalar_result(ext, |_| ExternalScalarValue::I64(2)),
            ],
        ),
        Err(BuildError::DuplicateExternalProvider(function)) if function == ext
    ));
}

#[test]
fn every_external_declaration_requires_a_provider_even_when_unused() {
    let compilation =
        compilation("external fn unused() -> I64; fn entry() -> I64 { return 7; }");
    let unused = function(&compilation, "unused");

    assert!(matches!(
        RealizedCompilation::new(&compilation),
        Err(BuildError::MissingExternalProvider(function)) if function == unused
    ));
}

#[test]
fn provider_result_shape_is_checked_in_hir_terms_before_realization() {
    let result_compilation =
        compilation("external fn value() -> I64; fn entry() -> I64 { return 1; }");
    let value = function(&result_compilation, "value");
    assert!(matches!(
        RealizedCompilation::new_with_external_providers(
            &result_compilation,
            vec![ExternalProviderBinding::no_result(value, |_| {})],
        ),
        Err(BuildError::ExternalProviderResultShapeMismatch {
            function,
            declaration_has_result: true,
            provider_has_result: false,
        }) if function == value
    ));

    let no_result_compilation =
        compilation("external fn sink(); fn entry() -> I64 { return 1; }");
    let sink = function(&no_result_compilation, "sink");
    assert!(matches!(
        RealizedCompilation::new_with_external_providers(
            &no_result_compilation,
            vec![ExternalProviderBinding::scalar_result(sink, |_| {
                ExternalScalarValue::I64(1)
            })],
        ),
        Err(BuildError::ExternalProviderResultShapeMismatch {
            function,
            declaration_has_result: false,
            provider_has_result: true,
        }) if function == sink
    ));
}
