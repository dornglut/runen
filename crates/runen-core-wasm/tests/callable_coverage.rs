use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Field, Function, FunctionId, LocalDecl,
    LocalId, ReferencePermission, SafeReferenceResultContract, ScalarType, Terminator, TypeDef,
    TypeId, TypeTable, validate_program,
};
use runen_core_wasm::{
    CoverageError, CoverageErrorKind, CoverageLocation, RealizationError, RealizedProgram,
    UnsupportedTypeCategory,
};

fn assert_callable_local_rejected(types: TypeTable, callable: TypeId) {
    let program = validate_program(runen_core_ir::Program {
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
    .expect("excluded callable-interface fixture must remain valid Core");

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
            kind: CoverageErrorKind::UnsupportedType {
                ty: callable,
                category: UnsupportedTypeCategory::Callable,
            },
        }
    );
}

#[test]
fn floating_callable_interface_is_outside_first_order_realization_slice() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let callable = types.push(TypeDef::callable(
        "FloatConsumer",
        CallableInterface::new(vec![f32_ty], None, SafeReferenceResultContract::None),
    ));
    assert_callable_local_rejected(types, callable);
}

#[test]
fn aggregate_callable_interface_is_outside_first_order_realization_slice() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("value", i64_ty)],
    ));
    let callable = types.push(TypeDef::callable(
        "AggregateConsumer",
        CallableInterface::new(vec![pair_ty], None, SafeReferenceResultContract::None),
    ));
    assert_callable_local_rejected(types, callable);
}

#[test]
fn safe_reference_callable_interface_is_outside_first_order_realization_slice() {
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
    assert_callable_local_rejected(types, callable);
}

#[test]
fn tracked_fixture_callable_interface_is_outside_first_order_realization_slice() {
    let mut types = TypeTable::new();
    let tracked_ty = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let callable = types.push(TypeDef::callable(
        "TrackedConsumer",
        CallableInterface::new(vec![tracked_ty], None, SafeReferenceResultContract::None),
    ));
    assert_callable_local_rejected(types, callable);
}

#[test]
fn nested_callable_interface_is_outside_first_order_realization_slice() {
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
    assert_callable_local_rejected(types, outer);
}
