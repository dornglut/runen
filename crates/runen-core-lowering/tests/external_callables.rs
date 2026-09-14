use runen_core_ir::{
    BinaryFloatSign, BinaryFloatValue, ExternalCallableId, FunctionId, ScalarType, Terminator,
    TypeKind, ValidatedProgram,
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
        .expect("external-callable source must produce accepted HIR");
    lower(&hir).expect("accepted external-callable HIR must lower to validated Core")
}

fn external_calls(program: &runen_core_ir::Program) -> Vec<ExternalCallableId> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.body.blocks)
        .filter_map(|block| match block.terminator {
            Terminator::ExternalCall { external, .. } => Some(external),
            _ => None,
        })
        .collect()
}

#[test]
fn external_declaration_lowers_once_without_becoming_a_core_function() {
    let lowered = lower_source(
        "external fn transform(I64, Bool) -> U64; \
         fn caller() -> U64 { return transform(7, true); }",
    );
    let program = lowered.as_program();
    assert_eq!(program.external_callables.len(), 1);
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "caller");
    let interface = &program.external_callables[0].interface;
    assert_eq!(interface.parameters.len(), 2);
    assert!(interface.result.is_some());
    assert!(matches!(
        program
            .types
            .get(interface.parameters[0])
            .map(|ty| &ty.kind),
        Some(TypeKind::Scalar(ScalarType::I64))
    ));
    assert!(matches!(
        program
            .types
            .get(interface.parameters[1])
            .map(|ty| &ty.kind),
        Some(TypeKind::Scalar(ScalarType::Bool))
    ));
    assert_eq!(external_calls(program), vec![ExternalCallableId(0)]);
}

#[test]
fn lowered_external_scalar_call_executes_differentially_through_both_engines() {
    let lowered = lower_source(
        "external fn transform(I64, Bool) -> U64; \
         fn caller() -> U64 { return transform(7, true); }",
    );
    let interface = lowered.as_program().external_callables[0].interface.clone();

    let realized = RealizedProgram::new_with_external_providers(
        &lowered,
        vec![WasmExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            interface.clone(),
            |arguments| {
                assert_eq!(
                    arguments,
                    &[
                        WasmExternalScalarValue::I64(7),
                        WasmExternalScalarValue::Bool(true),
                    ]
                );
                WasmExternalScalarValue::U64(42)
            },
        )],
    )
    .expect("lowered source external requirement must realize with the matching provider");
    let wasm_outcome = realized
        .execute(FunctionId(0))
        .expect("lowered source external call must execute through Core Wasm");
    assert_eq!(
        wasm_outcome,
        ExecutionOutcome::Returned(Some(runen_core_ir::Value::U64(42)))
    );

    let machine = Machine::new_with_external_providers(
        lowered,
        FunctionId(0),
        vec![ExternalProviderBinding::scalar_result(
            ExternalCallableId(0),
            interface,
            |arguments| {
                assert_eq!(
                    arguments,
                    &[ExternalScalarValue::I64(7), ExternalScalarValue::Bool(true),]
                );
                ExternalScalarValue::U64(42)
            },
        )],
    )
    .expect("lowered source external requirement must admit the matching provider");
    let report = machine
        .execute()
        .expect("lowered source external call is defined through the provider");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::U64(42)));
}

#[test]
fn lowered_floating_external_chain_agrees_between_core_wasm_and_reference() {
    let lowered = lower_source(
        "external fn transform(F32) -> F32; \
         external fn classify(F32) -> Bool; \
         fn caller() -> Bool { return classify(transform(1.0)); }",
    );
    let interfaces = lowered
        .as_program()
        .external_callables
        .iter()
        .map(|declaration| declaration.interface.clone())
        .collect::<Vec<_>>();
    assert_eq!(interfaces.len(), 2);
    let expected_source_value = BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand: 1_u64 << 23,
        exponent: 0,
    };

    let realized = RealizedProgram::new_with_external_providers(
        &lowered,
        vec![
            WasmExternalProviderBinding::scalar_result(
                ExternalCallableId(0),
                interfaces[0].clone(),
                move |arguments| {
                    assert_eq!(
                        arguments,
                        &[WasmExternalScalarValue::F32(
                            FloatingScalarValue::Represented(expected_source_value,)
                        )]
                    );
                    WasmExternalScalarValue::F32(FloatingScalarValue::NaNClass)
                },
            ),
            WasmExternalProviderBinding::scalar_result(
                ExternalCallableId(1),
                interfaces[1].clone(),
                |arguments| {
                    assert_eq!(
                        arguments,
                        &[WasmExternalScalarValue::F32(FloatingScalarValue::NaNClass)]
                    );
                    WasmExternalScalarValue::Bool(true)
                },
            ),
        ],
    )
    .expect("lowered floating external chain must realize");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(runen_core_ir::Value::Bool(true)))
    );

    let report = Machine::new_with_external_providers(
        lowered,
        FunctionId(0),
        vec![
            ExternalProviderBinding::scalar_result(
                ExternalCallableId(0),
                interfaces[0].clone(),
                move |arguments| {
                    assert_eq!(
                        arguments,
                        &[ExternalScalarValue::F32(
                            ObservedBinaryFloatValue::Represented(expected_source_value,)
                        )]
                    );
                    ExternalScalarValue::F32(ObservedBinaryFloatValue::NaNClass)
                },
            ),
            ExternalProviderBinding::scalar_result(
                ExternalCallableId(1),
                interfaces[1].clone(),
                |arguments| {
                    assert_eq!(
                        arguments,
                        &[ExternalScalarValue::F32(ObservedBinaryFloatValue::NaNClass)]
                    );
                    ExternalScalarValue::Bool(true)
                },
            ),
        ],
    )
    .expect("reference floating provider chain must admit")
    .execute()
    .expect("reference floating provider chain must execute");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::Bool(true)));
}

#[test]
fn generic_specialization_and_closure_reuse_one_external_identity() {
    let lowered = lower_source(
        "external fn ext(I64) -> I64; \
         fn generic[T](value: T) -> I64 { return ext(7); } \
         fn root() -> I64 { \
             let seed: I64 = 0; \
             let call = fn[seed](value: I64) -> I64 { return ext(value) + seed; }; \
             let first: I64 = generic[I64](1); \
             return call(first); \
         }",
    );
    let program = lowered.as_program();
    assert_eq!(program.external_callables.len(), 1);
    assert!(
        program
            .functions
            .iter()
            .all(|function| function.name != "ext")
    );
    let calls = external_calls(program);
    assert_eq!(
        calls.len(),
        2,
        "generic body and closure wrapper each call ext once"
    );
    assert!(calls.iter().all(|id| *id == ExternalCallableId(0)));
}

#[test]
fn qualified_exported_external_call_lowers_to_same_structural_external_category() {
    let dependency = parse("export external fn ext(I64) -> I64;");
    let caller = parse("import dep; fn caller() -> I64 { return dep::ext(9); }");
    let imports = [runen_hir::ImportTarget::new("dep", ModuleId::new(2)).unwrap()];
    let hir = build_typed_hir(&[
        SourceUnit::new(ModuleId::new(1), &caller, &imports),
        SourceUnit::new(ModuleId::new(2), &dependency, &[]),
    ])
    .expect("qualified exported external call is accepted HIR");
    let lowered = lower(&hir).expect("qualified external call lowers to Core");
    let program = lowered.as_program();
    assert_eq!(program.external_callables.len(), 1);
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "caller");
    assert_eq!(external_calls(program), vec![ExternalCallableId(0)]);
    let interface = &program.external_callables[0].interface;
    assert_eq!(interface.parameters.len(), 1);
    assert!(interface.result.is_some());
}
