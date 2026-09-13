use runen_model_oracle::{
    BagValue, FixtureError, LogicalType, RelationValue, Value, bag_cardinality, distinct,
    model_equivalent,
};

#[test]
fn cardinality_is_a_distinct_exact_model_scalar() {
    let seven_a = Value::cardinality_from_u128(7);
    let seven_b = Value::cardinality_from_u128(7);
    let eight = Value::cardinality_from_u128(8);

    assert_eq!(seven_a.logical_type(), &LogicalType::Cardinality);
    assert_ne!(seven_a.logical_type(), &LogicalType::U64);
    assert!(model_equivalent(&seven_a, &seven_b));
    assert!(!model_equivalent(&seven_a, &eight));
    assert!(!model_equivalent(&seven_a, &Value::u64(7)));
}

#[test]
fn cardinality_composes_through_existing_structural_equivalence() {
    let present_left =
        Value::present(LogicalType::Cardinality, Value::cardinality_from_u128(17)).unwrap();
    let present_right =
        Value::present(LogicalType::Cardinality, Value::cardinality_from_u128(17)).unwrap();
    assert!(model_equivalent(&present_left, &present_right));

    let relation_left = RelationValue::new(
        LogicalType::Cardinality,
        [
            Value::cardinality_from_u128(3),
            Value::cardinality_from_u128(5),
        ],
    )
    .unwrap();
    let relation_right = RelationValue::new(
        LogicalType::Cardinality,
        [
            Value::cardinality_from_u128(5),
            Value::cardinality_from_u128(3),
        ],
    )
    .unwrap();
    assert!(model_equivalent(
        &Value::relation(relation_left),
        &Value::relation(relation_right)
    ));
}

#[test]
fn bag_cardinality_counts_exact_occurrences() {
    let empty = BagValue::new(LogicalType::Bool, std::iter::empty::<Value>()).unwrap();
    let zero = bag_cardinality(&empty);
    assert_eq!(zero.logical_type(), &LogicalType::Cardinality);
    assert!(model_equivalent(&zero, &Value::cardinality_from_u128(0)));

    let input = BagValue::new(
        LogicalType::Bool,
        [Value::bool(true), Value::bool(false), Value::bool(true)],
    )
    .unwrap();
    assert!(model_equivalent(
        &bag_cardinality(&input),
        &Value::cardinality_from_u128(3)
    ));
}

#[test]
fn explicit_multiplicity_fixtures_validate_and_merge_classes() {
    let merged = BagValue::from_multiplicities(
        LogicalType::Bool,
        [
            (Value::bool(true), 2),
            (Value::bool(true), 3),
            (Value::bool(false), 4),
        ],
    )
    .unwrap();
    assert_eq!(merged.class_count(), 2);
    assert_eq!(merged.multiplicity_of(&Value::bool(true)), 5);
    assert_eq!(merged.multiplicity_of(&Value::bool(false)), 4);
    assert_eq!(merged.total_multiplicity().unwrap(), 9);

    assert_eq!(
        BagValue::from_multiplicities(LogicalType::Bool, [(Value::bool(true), 0)])
            .err()
            .unwrap(),
        FixtureError::ZeroMultiplicity
    );
    assert_eq!(
        BagValue::from_multiplicities(LogicalType::Bool, [(Value::i32(1), 1)])
            .err()
            .unwrap(),
        FixtureError::TypeMismatch {
            expected: LogicalType::Bool,
            actual: LogicalType::I32,
        }
    );
    assert_eq!(
        BagValue::from_multiplicities(
            LogicalType::Bool,
            [(Value::bool(true), u64::MAX), (Value::bool(true), 1)],
        )
        .err()
        .unwrap(),
        FixtureError::MultiplicityOverflow
    );
}

#[test]
fn bag_cardinality_exceeds_u64_without_semantic_overflow() {
    let forward = BagValue::from_multiplicities(
        LogicalType::Bool,
        [
            (Value::bool(false), u64::MAX),
            (Value::bool(true), u64::MAX),
        ],
    )
    .unwrap();
    let reverse = BagValue::from_multiplicities(
        LogicalType::Bool,
        [
            (Value::bool(true), u64::MAX),
            (Value::bool(false), u64::MAX),
        ],
    )
    .unwrap();
    let expected = Value::cardinality_from_u128(u128::from(u64::MAX) * 2);

    let forward_cardinality = bag_cardinality(&forward);
    let reverse_cardinality = bag_cardinality(&reverse);
    assert!(model_equivalent(&forward_cardinality, &expected));
    assert!(model_equivalent(&reverse_cardinality, &expected));
    assert!(model_equivalent(&forward_cardinality, &reverse_cardinality));

    assert_eq!(
        forward.total_multiplicity(),
        Err(FixtureError::MultiplicityOverflow)
    );
}

#[test]
fn distinct_uses_existing_generic_semantics_for_cardinality() {
    let input = BagValue::new(
        LogicalType::Cardinality,
        [
            Value::cardinality_from_u128(3),
            Value::cardinality_from_u128(3),
            Value::cardinality_from_u128(4),
        ],
    )
    .unwrap();
    let result = distinct(&input);

    assert_eq!(result.element_type(), &LogicalType::Cardinality);
    assert_eq!(result.class_count(), 2);
    assert!(result.contains_equivalent(&Value::cardinality_from_u128(3)));
    assert!(result.contains_equivalent(&Value::cardinality_from_u128(4)));
}
