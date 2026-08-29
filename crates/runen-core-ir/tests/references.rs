use runen_core_ir::{
    Program, ReferencePermission, ScalarType, TypeDef, TypeTable, Value, validate_program,
};

#[test]
fn reference_type_identity_is_exact_referent_and_permission() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let exclusive_i64 = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));
    let shared_bool = types.push(TypeDef::reference(
        "SharedBool",
        bool_ty,
        ReferencePermission::Shared,
    ));

    assert_eq!(
        types.reference(shared_i64),
        Some((i64_ty, ReferencePermission::Shared))
    );
    assert_eq!(
        types.reference(exclusive_i64),
        Some((i64_ty, ReferencePermission::Exclusive))
    );
    assert_eq!(
        types.reference(shared_bool),
        Some((bool_ty, ReferencePermission::Shared))
    );
    assert_eq!(
        types.reference_type_id(i64_ty, ReferencePermission::Shared),
        Some(shared_i64)
    );
    assert_eq!(
        types.reference_type_id(i64_ty, ReferencePermission::Exclusive),
        Some(exclusive_i64)
    );
    assert_eq!(
        types.reference_type_id(bool_ty, ReferencePermission::Shared),
        Some(shared_bool)
    );
}

#[test]
fn duplicate_reference_type_pair_is_rejected_by_program_validation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    types.push(TypeDef::reference(
        "SharedI64A",
        i64_ty,
        ReferencePermission::Shared,
    ));
    types.push(TypeDef::reference(
        "SharedI64B",
        i64_ty,
        ReferencePermission::Shared,
    ));

    assert!(
        validate_program(Program {
            types,
            functions: Vec::new(),
        })
        .is_err(),
        "one semantic reference type pair must map to one exact TypeId"
    );
}

#[test]
fn reference_copyability_follows_permission_and_structural_recursion() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let exclusive = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));
    let exclusive_replace = types.push(TypeDef::reference(
        "ExclusiveReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let shared_pair = types.push(TypeDef::structure(
        "SharedPair",
        vec![
            runen_core_ir::Field::new("left", shared),
            runen_core_ir::Field::new("right", shared),
        ],
    ));
    let exclusive_pair = types.push(TypeDef::structure(
        "ExclusivePair",
        vec![
            runen_core_ir::Field::new("left", shared),
            runen_core_ir::Field::new("right", exclusive),
        ],
    ));

    assert!(types.is_copy(shared));
    assert!(!types.is_copy(exclusive));
    assert!(!types.is_copy(exclusive_replace));
    assert!(types.is_copy(shared_pair));
    assert!(!types.is_copy(exclusive_pair));
}

#[test]
fn reference_values_cannot_be_fabricated_as_core_constants() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let reference_ty = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    assert!(!types.value_matches(reference_ty, &Value::I64(0)));
}

#[test]
fn parameter_and_result_transfer_rules_are_distinct() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_i64 = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let shared_raw = types.push(TypeDef::reference(
        "SharedRawI64",
        raw_i64,
        ReferencePermission::Shared,
    ));
    let nested_shared = types.push(TypeDef::reference(
        "NestedSharedI64",
        shared_i64,
        ReferencePermission::Shared,
    ));

    assert!(types.is_parameter_transfer_safe(i64_ty));
    assert!(types.is_result_transfer_safe(i64_ty));

    assert!(types.is_parameter_transfer_safe(shared_i64));
    assert!(!types.is_result_transfer_safe(shared_i64));

    assert!(!types.is_parameter_transfer_safe(raw_i64));
    assert!(!types.is_result_transfer_safe(raw_i64));

    assert!(!types.is_parameter_transfer_safe(shared_raw));
    assert!(!types.is_parameter_transfer_safe(nested_shared));
}
