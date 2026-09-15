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
use runen_syntax::parse_source;

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse_source(source.as_bytes()).expect("source fixture must be valid UTF-8");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("floating arithmetic source must produce accepted HIR");
    lower(&hir).expect("accepted floating arithmetic HIR must lower to validated Core")
}

fn normal(significand: u64, exponent: i16) -> BinaryFloatValue {
    BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand,
        exponent,
    }
}

#[test]
fn each_source_arithmetic_family_lowers_once_and_executes_through_both_engines() {
    let one = normal(1_u64 << 23, 0);
    let two = normal(1_u64 << 23, 1);
    let three = normal(3_u64 << 22, 1);
    let four = normal(1_u64 << 23, 2);
    let cases = [
        (
            "external fn classify(F32) -> Bool; fn caller() -> Bool { return classify(1.0 + 2.0); }",
            three,
        ),
        (
            "external fn classify(F32) -> Bool; fn caller() -> Bool { return classify(2.0 - 1.0); }",
            one,
        ),
        (
            "external fn classify(F32) -> Bool; fn caller() -> Bool { return classify(2.0 * 2.0); }",
            four,
        ),
        (
            "external fn classify(F32) -> Bool; fn caller() -> Bool { return classify(@fast(4.0 / 2.0)); }",
            two,
        ),
    ];

    for (source, expected) in cases {
        let lowered = lower_source(source);
        let interface = lowered.as_program().external_callables[0].interface.clone();

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
        .expect("the once-lowered source program must realize through Core Wasm");
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
        .expect("the same once-lowered source program must admit in the reference machine")
        .execute()
        .expect("the same once-lowered source program must execute in the reference machine");
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(ObservedValue::Bool(true)));
    }
}

#[test]
fn each_f16_source_arithmetic_family_lowers_once_and_executes_through_both_engines() {
    let one = normal(1_u64 << 10, 0);
    let two = normal(1_u64 << 10, 1);
    let three = normal(3_u64 << 9, 1);
    let four = normal(1_u64 << 10, 2);
    let cases = [
        (
            "external fn classify(F16) -> Bool; fn caller() -> Bool { return classify(1.0 + 2.0); }",
            three,
        ),
        (
            "external fn classify(F16) -> Bool; fn caller() -> Bool { return classify(2.0 - 1.0); }",
            one,
        ),
        (
            "external fn classify(F16) -> Bool; fn caller() -> Bool { return classify(2.0 * 2.0); }",
            four,
        ),
        (
            "external fn classify(F16) -> Bool; fn caller() -> Bool { return classify(@fast(4.0 / 2.0)); }",
            two,
        ),
    ];

    for (source, expected) in cases {
        let lowered = lower_source(source);
        let interface = lowered.as_program().external_callables[0].interface.clone();

        let realized = RealizedProgram::new_with_external_providers(
            &lowered,
            vec![WasmExternalProviderBinding::scalar_result(
                ExternalCallableId(0),
                interface.clone(),
                move |arguments| {
                    assert_eq!(
                        arguments,
                        &[WasmExternalScalarValue::F16(
                            FloatingScalarValue::Represented(expected)
                        )]
                    );
                    WasmExternalScalarValue::Bool(true)
                },
            )],
        )
        .expect("the once-lowered F16 source program must realize through Core Wasm");
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
                        &[ExternalScalarValue::F16(
                            ObservedBinaryFloatValue::Represented(expected)
                        )]
                    );
                    ExternalScalarValue::Bool(true)
                },
            )],
        )
        .expect("the same once-lowered F16 source program must admit in the reference machine")
        .execute()
        .expect("the same once-lowered F16 source program must execute in the reference machine");
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(ObservedValue::Bool(true)));
    }
}
