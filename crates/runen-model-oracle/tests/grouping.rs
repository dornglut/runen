use runen_model_oracle::{
    BagValue, FieldKey, FixtureError, FloatFormat, FloatValue, LogicalType, NaNRealizationId,
    RecordType, Value, distinct, group_by_fields, model_equivalent, project_fields,
};

fn key(token: u32) -> FieldKey {
    FieldKey::new(token)
}

fn record(record_type: &RecordType, fields: Vec<(FieldKey, Value)>) -> Value {
    Value::record(record_type.clone(), fields).unwrap()
}

#[test]
fn grouping_rejects_non_record_inputs_and_unknown_fields() {
    let scalar = BagValue::new(LogicalType::I32, [Value::i32(1)]).unwrap();
    assert_eq!(
        group_by_fields(&scalar, &[]).err().unwrap(),
        FixtureError::GroupingRequiresRecord {
            actual: LogicalType::I32,
        }
    );

    let record_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let input = BagValue::new(LogicalType::Record(record_type), []).unwrap();
    assert_eq!(
        group_by_fields(&input, &[key(2)]).err().unwrap(),
        FixtureError::GroupingFieldNotFound(key(2))
    );
}

#[test]
fn grouping_handles_empty_empty_key_and_complete_key_cases() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let element_type = LogicalType::Record(record_type.clone());
    let relation_element_type = LogicalType::bag(element_type.clone());

    let empty = BagValue::new(element_type.clone(), []).unwrap();
    let empty_result = group_by_fields(&empty, &[key(1)]).unwrap();
    assert_eq!(empty_result.element_type(), &relation_element_type);
    assert!(empty_result.is_empty());

    let a = record(
        &record_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(true))],
    );
    let b = record(
        &record_type,
        vec![(key(1), Value::i32(8)), (key(2), Value::bool(false))],
    );
    let input = BagValue::new(
        element_type.clone(),
        [a.clone(), b.clone(), a.clone()],
    )
    .unwrap();

    let empty_key_result = group_by_fields(&input, &[]).unwrap();
    assert_eq!(empty_key_result.class_count(), 1);
    assert!(empty_key_result.contains_equivalent(&Value::bag(input.clone())));

    let complete_key_result = group_by_fields(&input, &[key(2), key(1), key(2)]).unwrap();
    let group_a = BagValue::new(element_type.clone(), [a.clone(), a]).unwrap();
    let group_b = BagValue::new(element_type, [b]).unwrap();
    assert_eq!(complete_key_result.class_count(), 2);
    assert!(complete_key_result.contains_equivalent(&Value::bag(group_a)));
    assert!(complete_key_result.contains_equivalent(&Value::bag(group_b)));
}

#[test]
fn grouping_partitions_by_projected_key_preserves_multiplicity_and_recovers_key() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::U8)]).unwrap();
    let element_type = LogicalType::Record(record_type.clone());

    let a = record(
        &record_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::u8(1))],
    );
    let b = record(
        &record_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::u8(2))],
    );
    let c = record(
        &record_type,
        vec![(key(1), Value::i32(8)), (key(2), Value::u8(3))],
    );
    let input = BagValue::new(
        element_type.clone(),
        [a.clone(), b.clone(), a.clone(), c.clone()],
    )
    .unwrap();

    let result = group_by_fields(&input, &[key(1)]).unwrap();
    let group_seven = BagValue::new(
        element_type.clone(),
        [a.clone(), a.clone(), b.clone()],
    )
    .unwrap();
    let group_eight = BagValue::new(element_type, [c.clone()]).unwrap();

    assert_eq!(result.class_count(), 2);
    assert!(result.contains_equivalent(&Value::bag(group_seven.clone())));
    assert!(result.contains_equivalent(&Value::bag(group_eight.clone())));
    assert_eq!(group_seven.multiplicity_of(&a), 2);
    assert_eq!(group_seven.multiplicity_of(&b), 1);
    assert_eq!(group_eight.multiplicity_of(&c), 1);

    let projected_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let projected_seven = record(&projected_type, vec![(key(1), Value::i32(7))]);
    let projected_eight = record(&projected_type, vec![(key(1), Value::i32(8))]);

    let seven_key = distinct(&project_fields(&group_seven, &[key(1)]).unwrap());
    assert_eq!(seven_key.class_count(), 1);
    assert!(seven_key.contains_equivalent(&projected_seven));

    let eight_key = distinct(&project_fields(&group_eight, &[key(1)]).unwrap());
    assert_eq!(eight_key.class_count(), 1);
    assert!(eight_key.contains_equivalent(&projected_eight));
}

#[test]
fn grouping_uses_exact_optional_tag_and_recursive_present_equivalence() {
    let optional_i32 = LogicalType::optional(LogicalType::I32);
    let record_type =
        RecordType::new([(key(1), optional_i32), (key(2), LogicalType::U8)]).unwrap();
    let element_type = LogicalType::Record(record_type.clone());

    let absent_one = record(
        &record_type,
        vec![
            (key(1), Value::absent(LogicalType::I32)),
            (key(2), Value::u8(1)),
        ],
    );
    let absent_two = record(
        &record_type,
        vec![
            (key(1), Value::absent(LogicalType::I32)),
            (key(2), Value::u8(2)),
        ],
    );
    let present_seven_one = record(
        &record_type,
        vec![
            (
                key(1),
                Value::present(LogicalType::I32, Value::i32(7)).unwrap(),
            ),
            (key(2), Value::u8(3)),
        ],
    );
    let present_seven_two = record(
        &record_type,
        vec![
            (
                key(1),
                Value::present(LogicalType::I32, Value::i32(7)).unwrap(),
            ),
            (key(2), Value::u8(4)),
        ],
    );
    let present_eight = record(
        &record_type,
        vec![
            (
                key(1),
                Value::present(LogicalType::I32, Value::i32(8)).unwrap(),
            ),
            (key(2), Value::u8(5)),
        ],
    );

    let input = BagValue::new(
        element_type.clone(),
        [
            absent_one.clone(),
            present_seven_one.clone(),
            present_eight.clone(),
            absent_two.clone(),
            present_seven_two.clone(),
        ],
    )
    .unwrap();
    let result = group_by_fields(&input, &[key(1)]).unwrap();

    let absent_group = BagValue::new(element_type.clone(), [absent_one, absent_two]).unwrap();
    let seven_group = BagValue::new(
        element_type.clone(),
        [present_seven_one, present_seven_two],
    )
    .unwrap();
    let eight_group = BagValue::new(element_type, [present_eight]).unwrap();

    assert_eq!(result.class_count(), 3);
    assert!(result.contains_equivalent(&Value::bag(absent_group)));
    assert!(result.contains_equivalent(&Value::bag(seven_group)));
    assert!(result.contains_equivalent(&Value::bag(eight_group)));
}

#[test]
fn grouping_reuses_nan_equivalence_and_signed_zero_distinction() {
    let record_type =
        RecordType::new([(key(1), LogicalType::F64), (key(2), LogicalType::U8)]).unwrap();
    let element_type = LogicalType::Record(record_type.clone());

    let nan_one = record(
        &record_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(1))),
            ),
            (key(2), Value::u8(1)),
        ],
    );
    let nan_two = record(
        &record_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(99))),
            ),
            (key(2), Value::u8(2)),
        ],
    );
    let plus = record(
        &record_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::positive_zero(FloatFormat::F64)),
            ),
            (key(2), Value::u8(3)),
        ],
    );
    let minus = record(
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
        element_type.clone(),
        [nan_one.clone(), plus.clone(), nan_two.clone(), minus.clone()],
    )
    .unwrap();
    let result = group_by_fields(&input, &[key(1)]).unwrap();

    let nan_group = BagValue::new(element_type.clone(), [nan_one, nan_two]).unwrap();
    let plus_group = BagValue::new(element_type.clone(), [plus]).unwrap();
    let minus_group = BagValue::new(element_type, [minus]).unwrap();

    assert_eq!(result.class_count(), 3);
    assert!(result.contains_equivalent(&Value::bag(nan_group)));
    assert!(result.contains_equivalent(&Value::bag(plus_group)));
    assert!(result.contains_equivalent(&Value::bag(minus_group)));
}

#[test]
fn grouping_reuses_recursive_nested_bag_equivalence() {
    let nested = LogicalType::bag(LogicalType::I32);
    let record_type = RecordType::new([(key(1), nested), (key(2), LogicalType::U8)]).unwrap();
    let element_type = LogicalType::Record(record_type.clone());

    let nested_a = BagValue::new(
        LogicalType::I32,
        [Value::i32(1), Value::i32(2), Value::i32(1)],
    )
    .unwrap();
    let nested_b = BagValue::new(
        LogicalType::I32,
        [Value::i32(2), Value::i32(1), Value::i32(1)],
    )
    .unwrap();
    let nested_other =
        BagValue::new(LogicalType::I32, [Value::i32(1), Value::i32(2)]).unwrap();

    let a = record(
        &record_type,
        vec![(key(1), Value::bag(nested_a)), (key(2), Value::u8(1))],
    );
    let b = record(
        &record_type,
        vec![(key(1), Value::bag(nested_b)), (key(2), Value::u8(2))],
    );
    let other = record(
        &record_type,
        vec![
            (key(1), Value::bag(nested_other)),
            (key(2), Value::u8(3)),
        ],
    );

    let input = BagValue::new(
        element_type.clone(),
        [a.clone(), other.clone(), b.clone()],
    )
    .unwrap();
    let result = group_by_fields(&input, &[key(1)]).unwrap();
    let equivalent_group = BagValue::new(element_type.clone(), [a, b]).unwrap();
    let other_group = BagValue::new(element_type, [other]).unwrap();

    assert_eq!(result.class_count(), 2);
    assert!(result.contains_equivalent(&Value::bag(equivalent_group)));
    assert!(result.contains_equivalent(&Value::bag(other_group)));
}

#[test]
fn grouping_is_independent_of_field_occurrence_and_candidate_order() {
    let record_type = RecordType::new([
        (key(1), LogicalType::I32),
        (key(2), LogicalType::Bool),
        (key(3), LogicalType::U8),
    ])
    .unwrap();
    let element_type = LogicalType::Record(record_type.clone());

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
            (key(2), Value::bool(true)),
            (key(1), Value::i32(7)),
        ],
    );
    let third = record(
        &record_type,
        vec![
            (key(2), Value::bool(false)),
            (key(1), Value::i32(7)),
            (key(3), Value::u8(3)),
        ],
    );

    let input_a = BagValue::new(
        element_type.clone(),
        [first.clone(), second.clone(), third.clone(), first.clone()],
    )
    .unwrap();
    let input_b = BagValue::new(element_type, [third, first.clone(), second, first]).unwrap();

    let result_a = group_by_fields(&input_a, &[key(1), key(2), key(1)]).unwrap();
    let result_b = group_by_fields(&input_b, &[key(2), key(1)]).unwrap();

    assert!(model_equivalent(
        &Value::relation(result_a),
        &Value::relation(result_b)
    ));
}
