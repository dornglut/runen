use runen_model_oracle::{
    BagValue, FieldKey, FixtureError, FloatFormat, FloatValue, LogicalType, NaNRealizationId,
    RecordType, Value, model_equivalent, project_fields,
};

fn key(token: u32) -> FieldKey {
    FieldKey::new(token)
}

fn record(record_type: &RecordType, fields: Vec<(FieldKey, Value)>) -> Value {
    Value::record(record_type.clone(), fields).unwrap()
}

#[test]
fn projection_rejects_non_record_and_unknown_field_fixtures() {
    let scalar_bag = BagValue::new(LogicalType::I32, [Value::i32(1)]).unwrap();
    assert_eq!(
        project_fields(&scalar_bag, &[]).err().unwrap(),
        FixtureError::ProjectionRequiresRecord {
            actual: LogicalType::I32,
        }
    );

    let record_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let record_bag = BagValue::new(LogicalType::Record(record_type), []).unwrap();
    assert_eq!(
        project_fields(&record_bag, &[key(2)]).err().unwrap(),
        FixtureError::ProjectionFieldNotFound(key(2))
    );
}

#[test]
fn projection_key_candidates_are_an_unordered_idempotent_set() {
    let record_type = RecordType::new([
        (key(1), LogicalType::I32),
        (key(2), LogicalType::Bool),
        (key(3), LogicalType::U8),
    ])
    .unwrap();
    let first = record(
        &record_type,
        vec![
            (key(1), Value::i32(7)),
            (key(2), Value::bool(true)),
            (key(3), Value::u8(1)),
        ],
    );
    let second = record(
        &record_type,
        vec![
            (key(3), Value::u8(2)),
            (key(2), Value::bool(false)),
            (key(1), Value::i32(8)),
        ],
    );
    let input = BagValue::new(LogicalType::Record(record_type), [first, second]).unwrap();

    let left = project_fields(&input, &[key(2), key(1), key(2)]).unwrap();
    let right = project_fields(&input, &[key(1), key(2)]).unwrap();

    assert!(model_equivalent(&Value::bag(left), &Value::bag(right)));
}

#[test]
fn projection_of_empty_bag_has_exact_restricted_record_type() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let input = BagValue::new(LogicalType::Record(record_type), []).unwrap();
    let result = project_fields(&input, &[key(2)]).unwrap();
    let expected_type =
        LogicalType::Record(RecordType::new([(key(2), LogicalType::Bool)]).unwrap());

    assert_eq!(result.element_type(), &expected_type);
    assert_eq!(result.class_count(), 0);
    assert_eq!(result.total_multiplicity().unwrap(), 0);
}

#[test]
fn empty_projection_collapses_to_one_empty_record_class_without_losing_occurrences() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let input = BagValue::new(
        LogicalType::Record(record_type.clone()),
        [
            record(
                &record_type,
                vec![(key(1), Value::i32(1)), (key(2), Value::bool(true))],
            ),
            record(
                &record_type,
                vec![(key(1), Value::i32(2)), (key(2), Value::bool(false))],
            ),
            record(
                &record_type,
                vec![(key(1), Value::i32(1)), (key(2), Value::bool(true))],
            ),
        ],
    )
    .unwrap();

    let result = project_fields(&input, &[]).unwrap();
    let empty_type = RecordType::new(std::iter::empty::<(FieldKey, LogicalType)>()).unwrap();
    let empty_value =
        Value::record(empty_type.clone(), std::iter::empty::<(FieldKey, Value)>()).unwrap();

    assert_eq!(result.element_type(), &LogicalType::Record(empty_type));
    assert_eq!(result.class_count(), 1);
    assert_eq!(result.total_multiplicity().unwrap(), 3);
    assert_eq!(result.multiplicity_of(&empty_value), 3);
}

#[test]
fn all_key_projection_is_model_equivalent_to_input() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let input = BagValue::new(
        LogicalType::Record(record_type.clone()),
        [
            record(
                &record_type,
                vec![(key(1), Value::i32(4)), (key(2), Value::bool(true))],
            ),
            record(
                &record_type,
                vec![(key(1), Value::i32(5)), (key(2), Value::bool(false))],
            ),
        ],
    )
    .unwrap();

    let result = project_fields(&input, &[key(2), key(1)]).unwrap();
    assert!(model_equivalent(&Value::bag(input), &Value::bag(result)));
}

#[test]
fn projection_merges_removed_differences_and_sums_multiplicity() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let retained_true = record(
        &record_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(true))],
    );
    let retained_false = record(
        &record_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(false))],
    );
    let input = BagValue::new(
        LogicalType::Record(record_type),
        [retained_true.clone(), retained_false, retained_true],
    )
    .unwrap();

    let result = project_fields(&input, &[key(1)]).unwrap();
    let output_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let expected = record(&output_type, vec![(key(1), Value::i32(7))]);

    assert_eq!(input.class_count(), 2);
    assert_eq!(result.class_count(), 1);
    assert_eq!(result.total_multiplicity().unwrap(), 3);
    assert_eq!(result.multiplicity_of(&expected), 3);
}

#[test]
fn projection_preserves_distinctions_that_remain_in_retained_fields() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let input = BagValue::new(
        LogicalType::Record(record_type.clone()),
        [
            record(
                &record_type,
                vec![(key(1), Value::i32(7)), (key(2), Value::bool(true))],
            ),
            record(
                &record_type,
                vec![(key(1), Value::i32(8)), (key(2), Value::bool(true))],
            ),
        ],
    )
    .unwrap();

    let result = project_fields(&input, &[key(1)]).unwrap();
    let output_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let seven = record(&output_type, vec![(key(1), Value::i32(7))]);
    let eight = record(&output_type, vec![(key(1), Value::i32(8))]);

    assert_eq!(result.class_count(), 2);
    assert_eq!(result.multiplicity_of(&seven), 1);
    assert_eq!(result.multiplicity_of(&eight), 1);
}

#[test]
fn projection_reuses_optional_and_nested_collection_equivalence() {
    let nested_type = LogicalType::bag(LogicalType::I32);
    let optional_nested_type = LogicalType::optional(nested_type.clone());
    let record_type = RecordType::new([
        (key(1), optional_nested_type.clone()),
        (key(2), LogicalType::U8),
    ])
    .unwrap();

    let inner_left = BagValue::new(
        LogicalType::I32,
        [Value::i32(1), Value::i32(2), Value::i32(1)],
    )
    .unwrap();
    let inner_right = BagValue::new(
        LogicalType::I32,
        [Value::i32(1), Value::i32(1), Value::i32(2)],
    )
    .unwrap();

    let present_left = Value::present(nested_type.clone(), Value::bag(inner_left)).unwrap();
    let present_right = Value::present(nested_type.clone(), Value::bag(inner_right)).unwrap();
    let absent = Value::absent(nested_type);

    let input = BagValue::new(
        LogicalType::Record(record_type.clone()),
        [
            record(
                &record_type,
                vec![(key(1), present_left), (key(2), Value::u8(1))],
            ),
            record(
                &record_type,
                vec![(key(1), present_right), (key(2), Value::u8(2))],
            ),
            record(&record_type, vec![(key(1), absent), (key(2), Value::u8(3))]),
        ],
    )
    .unwrap();

    let result = project_fields(&input, &[key(1)]).unwrap();
    assert_eq!(result.class_count(), 2);
    assert_eq!(result.total_multiplicity().unwrap(), 3);

    let output_type = RecordType::new([(key(1), optional_nested_type)]).unwrap();
    let expected_present = record(
        &output_type,
        vec![(
            key(1),
            Value::present(
                LogicalType::bag(LogicalType::I32),
                Value::bag(
                    BagValue::new(
                        LogicalType::I32,
                        [Value::i32(2), Value::i32(1), Value::i32(1)],
                    )
                    .unwrap(),
                ),
            )
            .unwrap(),
        )],
    );
    let expected_absent = record(
        &output_type,
        vec![(key(1), Value::absent(LogicalType::bag(LogicalType::I32)))],
    );

    assert_eq!(result.multiplicity_of(&expected_present), 2);
    assert_eq!(result.multiplicity_of(&expected_absent), 1);
}

#[test]
fn projection_reuses_nan_equivalence_and_signed_zero_distinction() {
    let record_type =
        RecordType::new([(key(1), LogicalType::F64), (key(2), LogicalType::U8)]).unwrap();
    let nan_a = Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(1)));
    let nan_b = Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(2)));
    let plus_zero = Value::float(FloatValue::positive_zero(FloatFormat::F64));
    let minus_zero = Value::float(FloatValue::negative_zero(FloatFormat::F64));
    let input = BagValue::new(
        LogicalType::Record(record_type.clone()),
        [
            record(&record_type, vec![(key(1), nan_a), (key(2), Value::u8(1))]),
            record(&record_type, vec![(key(1), nan_b), (key(2), Value::u8(2))]),
            record(
                &record_type,
                vec![(key(1), plus_zero), (key(2), Value::u8(3))],
            ),
            record(
                &record_type,
                vec![(key(1), minus_zero), (key(2), Value::u8(4))],
            ),
        ],
    )
    .unwrap();

    let result = project_fields(&input, &[key(1)]).unwrap();
    let output_type = RecordType::new([(key(1), LogicalType::F64)]).unwrap();
    let expected_nan = record(
        &output_type,
        vec![(
            key(1),
            Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(99))),
        )],
    );
    let expected_plus_zero = record(
        &output_type,
        vec![(
            key(1),
            Value::float(FloatValue::positive_zero(FloatFormat::F64)),
        )],
    );
    let expected_minus_zero = record(
        &output_type,
        vec![(
            key(1),
            Value::float(FloatValue::negative_zero(FloatFormat::F64)),
        )],
    );

    assert_eq!(result.class_count(), 3);
    assert_eq!(result.total_multiplicity().unwrap(), 4);
    assert_eq!(result.multiplicity_of(&expected_nan), 2);
    assert_eq!(result.multiplicity_of(&expected_plus_zero), 1);
    assert_eq!(result.multiplicity_of(&expected_minus_zero), 1);
}

#[test]
fn projection_is_independent_of_record_and_occurrence_construction_order() {
    let record_type = RecordType::new([
        (key(1), LogicalType::I32),
        (key(2), LogicalType::Bool),
        (key(3), LogicalType::U8),
    ])
    .unwrap();

    let left_first = record(
        &record_type,
        vec![
            (key(1), Value::i32(7)),
            (key(2), Value::bool(true)),
            (key(3), Value::u8(1)),
        ],
    );
    let left_second = record(
        &record_type,
        vec![
            (key(1), Value::i32(8)),
            (key(2), Value::bool(false)),
            (key(3), Value::u8(2)),
        ],
    );
    let right_first = record(
        &record_type,
        vec![
            (key(3), Value::u8(2)),
            (key(2), Value::bool(false)),
            (key(1), Value::i32(8)),
        ],
    );
    let right_second = record(
        &record_type,
        vec![
            (key(3), Value::u8(1)),
            (key(1), Value::i32(7)),
            (key(2), Value::bool(true)),
        ],
    );

    let left = BagValue::new(
        LogicalType::Record(record_type.clone()),
        [left_first, left_second],
    )
    .unwrap();
    let right = BagValue::new(
        LogicalType::Record(record_type),
        [right_first, right_second],
    )
    .unwrap();

    let left_result = project_fields(&left, &[key(2), key(1)]).unwrap();
    let right_result = project_fields(&right, &[key(1), key(2)]).unwrap();

    assert!(model_equivalent(
        &Value::bag(left_result),
        &Value::bag(right_result)
    ));
}
