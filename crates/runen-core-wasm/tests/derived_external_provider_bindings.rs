use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, ExternalCallableDecl, ExternalCallableId,
    Function, FunctionId, LocalDecl, LocalId, Operand, Place, Program, SafeReferenceResultContract,
    ScalarType, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_core_wasm::{
    ExecutionOutcome, ExternalProviderAdmissionError, ExternalProviderBinding, ExternalScalarValue,
    RealizedProgram,
};

fn validated_with_external(result: Option<runen_core_ir::TypeId>) -> runen_core_ir::ValidatedProgram {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let external_result = result.map(|_| i64_ty);
    let external = ExternalCallableDecl::new(CallableInterface::new(
        Vec::new(),
        external_result,
        SafeReferenceResultContract::None,
    ));

    let (locals, blocks, function_result) = if external_result.is_some() {
        (
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
            Some(i64_ty),
        )
    } else {
        (
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::ExternalCall {
                        external: ExternalCallableId(0),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
            None,
        )
    };

    validate_program(Program {
        persistent: Vec::new(),
        external_callables: vec![external],
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: function_result,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals,
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks,
            },
        }],
    })
    .expect("derived provider fixture must be valid Core")
}

#[test]
fn scalar_result_binding_derives_canonical_interface_from_program() {
    let marker = runen_core_ir::TypeId(0);
    let program = validated_with_external(Some(marker));
    let binding = ExternalProviderBinding::scalar_result_for_program(
        &program,
        ExternalCallableId(0),
        |_| ExternalScalarValue::I64(42),
    )
    .expect("result-bearing provider shape must match declaration");

    let realized = RealizedProgram::new_with_external_providers(&program, vec![binding])
        .expect("derived provider binding must admit");
    assert_eq!(
        realized.execute(FunctionId(0)).unwrap(),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn derived_binding_rejects_result_shape_mismatch() {
    let marker = runen_core_ir::TypeId(0);
    let result_program = validated_with_external(Some(marker));
    assert!(matches!(
        ExternalProviderBinding::no_result_for_program(
            &result_program,
            ExternalCallableId(0),
            |_| {},
        ),
        Err(ExternalProviderAdmissionError::ProviderResultShapeMismatch {
            external: ExternalCallableId(0),
            declaration_has_result: true,
            provider_has_result: false,
        })
    ));

    let no_result_program = validated_with_external(None);
    assert!(matches!(
        ExternalProviderBinding::scalar_result_for_program(
            &no_result_program,
            ExternalCallableId(0),
            |_| ExternalScalarValue::I64(1),
        ),
        Err(ExternalProviderAdmissionError::ProviderResultShapeMismatch {
            external: ExternalCallableId(0),
            declaration_has_result: false,
            provider_has_result: true,
        })
    ));
}

#[test]
fn derived_binding_rejects_unknown_external_identity() {
    let program = validated_with_external(None);
    assert!(matches!(
        ExternalProviderBinding::no_result_for_program(
            &program,
            ExternalCallableId(1),
            |_| {},
        ),
        Err(ExternalProviderAdmissionError::UnknownProvider(
            ExternalCallableId(1)
        ))
    ));
}
