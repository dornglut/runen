use runen_model_oracle::{
    BagValue, FieldKey, FixtureError, FloatFormat, FloatValue, LogicalType, NaNRealizationId,
    RecordType, Value, filter_field_equivalent, model_equivalent,
};

fn key(token: u32) -> FieldKey {
    FieldKey::new(token)
}

fn record(record_type: &RecordType, fields: Vec<(FieldKey, Value)>) -> Value {
    Value::record(record_type.clone(), fields).unwrap()
}

#[test]
fn filter_rejects_non_record_unknown_field_and_wrong_value_type() {
    let scalar_bag = BagValue::new(LogicalType::I32, [Value::i32(1)]).unwrap();
    assert_eq!(
        filter_field_equivalent(&scalar_bag, key(1), &Value::i32(1))
            .err()
            .unwrap(),
        FixtureError::FilterRequiresRecord {
            actual: LogicalType::I32,
        }
    );

    let record_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let record_bag = BagValue::new(LogicalType::Record(record_type), []).unwrap();
    assert_eq!(
        filter_field_equivalent(&record_bag, key(2), &Value::i32(1))
            .err()
            .unwrap(),
        FixtureError::FilterFieldNotFound(key(2))
    );
    assert_eq!(
        filter_field_equivalent(&record_bag, key(1), &Value::u8(1))
            .err()
            .unwrap(),
        FixtureError::TypeMismatch {
            expected: LogicalType::I32,
            actual: LogicalType::U8,
        }
    );
}

#[test]
fn filter_handles_empty_no_match_and_all_match_without_changing_record_type() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let element_type = LogicalType::Record(record_type.clone());
    let empty = BagValue::new(element_type.clone(), []).unwrap();
    let empty_result = filter_field_equivalent(&empty, key(1), &Value::i32(7)).unwrap();
    assert_eq!(empty_result.element_type(), &element_type);
    assert!(empty_result.is_empty());

    let first = record(
        &record_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(true))],
    );
    let second = record(
        &record_type,
        vec![(key(1), Value::i32(8)), (key(2), Value::bool(true))],
    );
    let input = BagValue::new(
        element_type.clone(),
        [first.clone(), second.clone(), first.clone()],
    )
    .unwrap();

    let no_match = filter_field_equivalent(&input, key(1), &Value::i32(99)).unwrap();
    assert!(no_match.is_empty());
    assert_eq!(no_match.element_type(), &element_type);

    let all_match = filter_field_equivalent(&input, key(2), &Value::bool(true)).unwrap();
    assert!(model_equivalent(
        &Value::bag(input.clone()),
        &Value::bag(all_match)
    ));
}

#[test]
fn filter_preserves_distinct_matching_record_classes_and_exact_multiplicity() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let matching_true = record(
        &record_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(true))],
    );
    let matching_false = record(
        &record_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(false))],
    );
    let rejected = record(
        &record_type,
        vec![(key(1), Value::i32(8)), (key(2), Value::bool(true))],
    );
    let input = BagValue::new(
        LogicalType::Record(record_type),
        [
            matching_true.clone(),
            rejected.clone(),
            matching_false.clone(),
            matching_true.clone(),
        ],
    )
    .unwrap();

    let result = filter_field_equivalent(&input, key(1), &Value::i32(7)).unwrap();
    assert_eq!(result.class_count(), 2);
    assert_eq!(result.total_multiplicity().unwrap(), 3);
    assert_eq!(result.multiplicity_of(&matching_true), 2);
    assert_eq!(result.multiplicity_of(&matching_false), 1);
    assert_eq!(result.multiplicity_of(&rejected), 0);
}

#[test]
fn filter_uses_exact_optional_absence_and_present_equivalence() {
    let optional_i32 = LogicalType::optional(LogicalType::I32);
    let record_type =
        RecordType::new([(key(1), optional_i32.clone()), (key(2), LogicalType::U8)]).unwrap();
    let absent = record(
        &record_type,
        vec![
            (key(1), Value::absent(LogicalType::I32)),
            (key(2), Value::u8(1)),
        ],
    );
    let present_seven_a = record(
        &record_type,
        vec![
            (
                key(1),
                Value::present(LogicalType::I32, Value::i32(7)).unwrap(),
            ),
            (key(2), Value::u8(2)),
        ],
    );
    let present_seven_b = record(
        &record_type,
        vec![
            (
                key(1),
                Value::present(LogicalType::I32, Value::i32(7)).unwrap(),
            ),
            (key(2), Value::u8(3)),
        ],
    );
    let present_eight = record(
        &record_type,
        vec![
            (
                key(1),
                Value::present(LogicalType::I32, Value::i32(8)).unwrap(),
            ),
            (key(2), Value::u8(4)),
        ],
    );
    let input = BagValue::new(
        LogicalType::Record(record_type),
        [
            absent.clone(),
            present_seven_a.clone(),
            present_eight.clone(),
            present_seven_b.clone(),
        ],
    )
    .unwrap();

    let absent_result =
        filter_field_equivalent(&input, key(1), &Value::absent(LogicalType::I32)).unwrap();
    assert_eq!(absent_result.class_count(), 1);
    assert_eq!(absent_result.multiplicity_of(&absent), 1);
    assert_eq!(absent_result.multiplicity_of(&present_seven_a), 0);

    let present_target = Value::present(LogicalType::I32, Value::i32(7)).unwrap();
    let present_result = filter_field_equivalent(&input, key(1), &present_target).unwrap();
    assert_eq!(present_result.class_count(), 2);
    assert_eq!(present_result.total_multiplicity().unwrap(), 2);
    assert_eq!(present_result.multiplicity_of(&present_seven_a), 1);
    assert_eq!(present_result.multiplicity_of(&present_seven_b), 1);
    assert_eq!(present_result.multiplicity_of(&present_eight), 0);
}

#[test]
fn filter_reuses_nan_equivalence_and_signed_zero_distinction() {
    let record_type =
        RecordType::new([(key(1), LogicalType::F64), (key(2), LogicalType::U8)]).unwrap();
    let nan_a = record(
        &record_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(1))),
            ),
            (key(2), Value::u8(1)),
        ],
    );
    let nan_b = record(
        &record_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(2))),
            ),
            (key(2), Value::u8(2)),
        ],
    );
    let plus_zero = record(
        &record_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::positive_zero(FloatFormat::F64)),
            ),
            (key(2), Value::u8(3)),
        ],
    );
    let minus_zero = record(
        &record_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::negative_zero(FloatFormat::F64)),
            ),
            (key(2), Value::u8(4)),
        ],
    );
    let input = BagValue::new(
        LogicalType::Record(record_type),
        [
            nan_a.clone(),
            plus_zero.clone(),
            nan_b.clone(),
            minus_zero.clone(),
        ],
    )
    .unwrap();

    let nan_target = Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(99)));
    let nan_result = filter_field_equivalent(&input, key(1), &nan_target).unwrap();
    assert_eq!(nan_result.class_count(), 2);
    assert_eq!(nan_result.total_multiplicity().unwrap(), 2);
    assert_eq!(nan_result.multiplicity_of(&nan_a), 1);
    assert_eq!(nan_result.multiplicity_of(&nan_b), 1);

    let plus_result = filter_field_equivalent(
        &input,
        key(1),
        &Value::float(FloatValue::positive_zero(FloatFormat::F64)),
    )
    .unwrap();
    assert_eq!(plus_result.class_count(), 1);
    assert_eq!(plus_result.multiplicity_of(&plus_zero), 1);
    assert_eq!(plus_result.multiplicity_of(&minus_zero), 0);
}

#[test]
fn filter_reuses_recursive_nested_bag_equivalence() {
    let nested_type = LogicalType::bag(LogicalType::I32);
    let record_type =
        RecordType::new([(key(1), nested_type.clone()), (key(2), LogicalType::U8)]).unwrap();

    let nested_left = BagValue::new(
        LogicalType::I32,
        [Value::i32(1), Value::i32(2), Value::i32(1)],
    )
    .unwrap();
    let nested_right = BagValue::new(
        LogicalType::I32,
        [Value::i32(2), Value::i32(1), Value::i32(1)],
    )
    .unwrap();
    let nested_other = BagValue::new(LogicalType::I32, [Value::i32(1), Value::i32(2)]).unwrap();

    let left = record(
        &record_type,
        vec![(key(1), Value::bag(nested_left)), (key(2), Value::u8(1))],
    );
    let right = record(
        &record_type,
        vec![(key(1), Value::bag(nested_right)), (key(2), Value::u8(2))],
    );
    let other = record(
        &record_type,
        vec![(key(1), Value::bag(nested_other)), (key(2), Value::u8(3))],
    );
    let input = BagValue::new(
        LogicalType::Record(record_type),
        [other.clone(), left.clone(), right.clone()],
    )
    .unwrap();
    let target = Value::bag(
        BagValue::new(
            LogicalType::I32,
            [Value::i32(1), Value::i32(1), Value::i32(2)],
        )
        .unwrap(),
    );

    let result = filter_field_equivalent(&input, key(1), &target).unwrap();
    assert_eq!(result.class_count(), 2);
    assert_eq!(result.total_multiplicity().unwrap(), 2);
    assert_eq!(result.multiplicity_of(&left), 1);
    assert_eq!(result.multiplicity_of(&right), 1);
    assert_eq!(result.multiplicity_of(&other), 0);
}

#[test]
fn filter_is_independent_of_record_and_occurrence_construction_order() {
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
            (key(1), Value::i32(8)),
            (key(2), Value::bool(false)),
        ],
    );
    let right_second = record(
        &record_type,
        vec![
            (key(3), Value::u8(1)),
            (key(2), Value::bool(true)),
            (key(1), Value::i32(7)),
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

    let left_result = filter_field_equivalent(&left, key(2), &Value::bool(true)).unwrap();
    let right_result = filter_field_equivalent(&right, key(2), &Value::bool(true)).unwrap();
    assert!(model_equivalent(
        &Value::bag(left_result),
        &Value::bag(right_result)
    ));
}
