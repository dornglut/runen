use runen_core_ir::{
    BinaryFloatSign, BinaryFloatValue, ExternalCallableId, FunctionId, ValidatedProgram,
};
use runen_core_lowering::lower;
use runen_core_wasm::{
    ExecutionOutcome, ExternalProviderBinding as WasmExternalProviderBinding,
    ExternalScalarValue as WasmExternalScalarValue, FloatingScalarValue, RealizedProgram,
};
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_reference::{
    ExternalProviderBinding, ExternalScalarValue, Machine, ObservedBinaryFloatValue, ObservedValue,
    TerminalStatus,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("floating-record source must produce accepted HIR");
    lower(&hir)
        .expect("accepted floating-record HIR must lower to validated Core")
        .into_program()
}

#[test]
fn lowered_floating_record_field_observation_agrees_between_core_wasm_and_reference() {
    let lowered = lower_source(
        "record Sample { half: F16, single: F32, double: F64 } \
         external fn classify(F32) -> Bool; \
         fn caller() -> Bool { \
             let sample: Sample = Sample { half: 0.5, single: 1.0, double: -2.0 }; \
             return classify(sample.single); \
         }",
    );
    let interface = lowered.as_program().external_callables[0].interface.clone();
    let expected = BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand: 1_u64 << 23,
        exponent: 0,
    };

    let realized = RealizedProgram::new_with_external_providers(
        &lowered,
        vec![WasmExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            interface.clone(),
            move |arguments| {
                assert_eq!(
                    arguments,
                    &[WasmExternalScalarValue::F32(
                        FloatingScalarValue::Represented(expected)
                    )]
                );
                WasmExternalScalarValue::Bool(true)
            },
        )],
    )
    .expect("lowered floating-record program must realize");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(runen_core_ir::Value::Bool(true)))
    );

    let report = Machine::new_with_external_providers(
        lowered,
        FunctionId(0),
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            interface,
            move |arguments| {
                assert_eq!(
                    arguments,
                    &[ExternalScalarValue::F32(
                        ObservedBinaryFloatValue::Represented(expected)
                    )]
                );
                ExternalScalarValue::Bool(true)
            },
        )],
    )
    .expect("reference floating-record provider environment must admit")
    .execute()
    .expect("reference floating-record program must execute");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::Bool(true)));
}
