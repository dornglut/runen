use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Field, Function, FunctionId, LocalDecl,
    LocalId, Program, ReferencePermission, SafeReferenceResultContract, ScalarType, Terminator,
    TypeDef, TypeId, TypeTable, validate_program,
};
use runen_core_wasm::{
    CoverageError, CoverageErrorKind, CoverageLocation, ExecutionOutcome, RealizationError,
    RealizedProgram, UnsupportedTypeCategory,
};

fn realization_error(program: &runen_core_ir::ValidatedProgram) -> RealizationError {
    match RealizedProgram::new(program) {
        Err(error) => error,
        Ok(_) => panic!("unsupported aggregate coverage fixture was admitted"),
    }
}

fn assert_nested_leaf_rejected(
    mut types: TypeTable,
    leaf: TypeId,
    category: UnsupportedTypeCategory,
) {
    let outer = types.push(TypeDef::structure("Outer", vec![Field::new("leaf", leaf)]));
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("value", outer, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            },
        }],
    })
    .expect("nested unsupported coverage fixture must remain valid Core");

    assert_eq!(
        realization_error(&program),
        RealizationError::Coverage(CoverageError {
            location: CoverageLocation::Local {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind: CoverageErrorKind::UnsupportedType { ty: leaf, category },
        })
    );
}

#[test]
fn nested_floating_leaf_families_are_admitted() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("F16", ScalarType::F16));
    let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
    let f64_ty = types.push(TypeDef::scalar("F64", ScalarType::F64));
    let inner = types.push(TypeDef::structure(
        "Inner",
        vec![Field::new("half", f16_ty), Field::new("single", f32_ty)],
    ));
    let outer = types.push(TypeDef::structure(
        "Outer",
        vec![Field::new("inner", inner), Field::new("double", f64_ty)],
    ));
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("value", outer, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            },
        }],
    })
    .expect("nested floating aggregate fixture must remain valid Core");

    assert_eq!(
        RealizedProgram::new(&program)
            .expect("nested floating aggregate must realize")
            .execute(FunctionId(0))
            .expect("passive floating aggregate fixture must execute"),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn nested_excluded_leaf_families_reject_with_exact_type_categories() {
    let mut raw = TypeTable::new();
    let raw_pointee = raw.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_ty = raw.push(TypeDef::raw_pointer("RawI64", raw_pointee));
    assert_nested_leaf_rejected(raw, raw_ty, UnsupportedTypeCategory::RawPointer);

    let mut reference = TypeTable::new();
    let referent = reference.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = reference.push(TypeDef::reference(
        "SharedI64",
        referent,
        ReferencePermission::Shared,
    ));
    assert_nested_leaf_rejected(
        reference,
        reference_ty,
        UnsupportedTypeCategory::SafeReference,
    );

    let mut callable = TypeTable::new();
    let callable_ty = callable.push(TypeDef::callable(
        "Thunk",
        CallableInterface::new(Vec::new(), None, SafeReferenceResultContract::None),
    ));
    assert_nested_leaf_rejected(callable, callable_ty, UnsupportedTypeCategory::Callable);

    let mut tracked = TypeTable::new();
    let tracked_ty = tracked.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    assert_nested_leaf_rejected(tracked, tracked_ty, UnsupportedTypeCategory::TrackedFixture);

    let mut interior = TypeTable::new();
    let interior_ty =
        interior.push(TypeDef::scalar("Interior", ScalarType::I64).with_interior_mutability());
    assert_nested_leaf_rejected(
        interior,
        interior_ty,
        UnsupportedTypeCategory::InteriorMutable,
    );
}

#[test]
fn interior_mutable_aggregate_root_rejects_before_encoding() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let aggregate = types.push(
        TypeDef::structure("InteriorPair", vec![Field::new("value", i64_ty)])
            .with_interior_mutability(),
    );
    let program = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("value", aggregate, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            },
        }],
    })
    .expect("interior-mutable aggregate fixture must remain valid Core");

    assert_eq!(
        realization_error(&program),
        RealizationError::Coverage(CoverageError {
            location: CoverageLocation::Local {
                function: FunctionId(0),
                local: LocalId(0),
            },
            kind: CoverageErrorKind::UnsupportedType {
                ty: aggregate,
                category: UnsupportedTypeCategory::InteriorMutable,
            },
        })
    );
}
