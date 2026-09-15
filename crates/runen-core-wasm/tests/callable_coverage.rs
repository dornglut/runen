use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Field, Function, FunctionId, LocalDecl,
    LocalId, MirValidationErrorKind, ReferencePermission, SafeReferenceResultContract, ScalarType,
    Terminator, TypeDef, TypeId, TypeTable, validate_program,
};
use runen_core_wasm::{
    CoverageError, CoverageErrorKind, CoverageLocation, ExecutionOutcome, RealizationError,
    RealizedProgram, UnsupportedTypeCategory,
};

fn callable_local_program(types: TypeTable, callable: TypeId) -> runen_core_ir::ValidatedProgram {
    validate_program(runen_core_ir::Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("callee", callable, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            },
        }],
    })
    .expect("callable-interface fixture must remain valid Core")
}

fn assert_callable_local_rejected(types: TypeTable, callable: TypeId, kind: CoverageErrorKind) {
    let program = callable_local_program(types, callable);
    let error = match RealizedProgram::new(&program) {
        Err(RealizationError::Coverage(error)) => error,
        Err(other) => panic!("expected coverage rejection, got realization error: {other:?}"),
        Ok(_) => panic!("excluded callable interface was admitted"),
    };
    assert_eq!(
        error,
        CoverageError {
            location: CoverageLocation::Local {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind,
        }
    );
}

#[test]
fn floating_callable_parameter_is_admitted_as_a_direct_scalar_component() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let callable = types.push(TypeDef::callable(
        "FloatConsumer",
        CallableInterface::new(vec![f32_ty], None, SafeReferenceResultContract::None),
    ));
    let program = callable_local_program(types, callable);
    let outcome = RealizedProgram::new(&program)
        .expect("direct floating callable parameter must realize")
        .execute(FunctionId(0))
        .expect("passive callable fixture must execute");
    assert_eq!(outcome, ExecutionOutcome::Returned(None));
}

#[test]
fn nested_floating_callable_result_is_admitted_for_internal_transport() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let wrapper_ty = types.push(TypeDef::structure(
        "FloatWrapper",
        vec![Field::new("value", f32_ty)],
    ));
    let callable = types.push(TypeDef::callable(
        "FloatProducer",
        CallableInterface::new(
            Vec::new(),
            Some(wrapper_ty),
            SafeReferenceResultContract::None,
        ),
    ));
    let program = callable_local_program(types, callable);
    assert_eq!(
        RealizedProgram::new(&program)
            .expect("floating aggregate callable result must realize internally")
            .execute(FunctionId(0))
            .expect("passive callable fixture must execute"),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn reference_free_aggregate_callable_interface_is_admitted_for_locals_and_parameters() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let empty_ty = types.push(TypeDef::structure("Empty", Vec::new()));
    let nested_ty = types.push(TypeDef::structure(
        "Nested",
        vec![Field::new("empty", empty_ty), Field::new("value", i64_ty)],
    ));
    let callable = types.push(TypeDef::callable(
        "AggregateConsumer",
        CallableInterface::new(
            vec![nested_ty],
            Some(nested_ty),
            SafeReferenceResultContract::None,
        ),
    ));
    let local_program = callable_local_program(types.clone(), callable);
    let outcome = RealizedProgram::new(&local_program)
        .expect("reference-free structural callable local must realize")
        .execute(FunctionId(0))
        .expect("admitted callable-interface local fixture must execute");
    assert_eq!(outcome, ExecutionOutcome::Returned(None));

    let parameter_program = validate_program(runen_core_ir::Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![Function {
            name: "accept".into(),
            parameters: vec![LocalId(0)],
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("callee", callable, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            },
        }],
    })
    .expect("reference-free structural callable parameter must remain valid Core");
    RealizedProgram::new(&parameter_program)
        .expect("reference-free structural callable parameter must realize");
}

#[test]
fn safe_reference_callable_parameter_reports_exact_interface_role_and_type() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let callable = types.push(TypeDef::callable(
        "ReferenceConsumer",
        CallableInterface::new(vec![reference_ty], None, SafeReferenceResultContract::None),
    ));
    assert_callable_local_rejected(
        types,
        callable,
        CoverageErrorKind::UnsupportedCallableParameterType {
            callable,
            parameter: 0,
            ty: reference_ty,
            category: UnsupportedTypeCategory::SafeReference,
        },
    );
}

#[test]
fn raw_pointer_callable_interface_is_rejected_by_core_before_realization() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_ty = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let wrapper_ty = types.push(TypeDef::structure(
        "RawWrapper",
        vec![Field::new("pointer", raw_ty)],
    ));
    let callable = types.push(TypeDef::callable(
        "RawConsumer",
        CallableInterface::new(vec![wrapper_ty], None, SafeReferenceResultContract::None),
    ));
    let error = validate_program(runen_core_ir::Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("callee", callable, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            },
        }],
    })
    .expect_err("raw-bearing callable interface must fail canonical Core validation");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ParameterTransferUnsafe(wrapper_ty)
    );
}

#[test]
fn interior_mutable_callable_parameter_reports_exact_nested_leaf() {
    let mut types = TypeTable::new();
    let interior_ty =
        types.push(TypeDef::scalar("Interior", ScalarType::I64).with_interior_mutability());
    let wrapper_ty = types.push(TypeDef::structure(
        "InteriorWrapper",
        vec![Field::new("value", interior_ty)],
    ));
    let callable = types.push(TypeDef::callable(
        "InteriorConsumer",
        CallableInterface::new(vec![wrapper_ty], None, SafeReferenceResultContract::None),
    ));
    assert_callable_local_rejected(
        types,
        callable,
        CoverageErrorKind::UnsupportedCallableParameterType {
            callable,
            parameter: 0,
            ty: interior_ty,
            category: UnsupportedTypeCategory::InteriorMutable,
        },
    );
}

#[test]
fn tracked_fixture_callable_parameter_reports_exact_interface_role_and_type() {
    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let callable = types.push(TypeDef::callable(
        "TrackedConsumer",
        CallableInterface::new(vec![tracked_ty], None, SafeReferenceResultContract::None),
    ));
    assert_callable_local_rejected(
        types,
        callable,
        CoverageErrorKind::UnsupportedCallableParameterType {
            callable,
            parameter: 0,
            ty: tracked_ty,
            category: UnsupportedTypeCategory::TrackedFixture,
        },
    );
}

#[test]
fn nested_callable_parameter_is_admitted_recursively() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let inner = types.push(TypeDef::callable(
        "Inner",
        CallableInterface::new(
            vec![i64_ty],
            Some(i64_ty),
            SafeReferenceResultContract::None,
        ),
    ));
    let outer = types.push(TypeDef::callable(
        "Outer",
        CallableInterface::new(vec![inner], None, SafeReferenceResultContract::None),
    ));
    let program = callable_local_program(types, outer);
    let outcome = RealizedProgram::new(&program)
        .expect("nested callable parameter must realize recursively")
        .execute(FunctionId(0))
        .expect("nested callable local fixture must execute");
    assert_eq!(outcome, ExecutionOutcome::Returned(None));
}

#[test]
fn callable_safe_reference_result_contract_remains_outside_realization_slice() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let callable = types.push(TypeDef::callable(
        "ReferenceIdentity",
        CallableInterface::new(
            vec![reference_ty],
            Some(reference_ty),
            SafeReferenceResultContract::SharedIdentity { origin: 0 },
        ),
    ));
    assert_callable_local_rejected(
        types,
        callable,
        CoverageErrorKind::UnsupportedSafeReferenceResultContract,
    );
}
