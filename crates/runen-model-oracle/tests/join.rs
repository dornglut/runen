use runen_model_oracle::{
    BagValue, FieldKey, FixtureError, FloatFormat, FloatValue, LogicalType, NaNRealizationId,
    RecordType, Value, join_fields_equivalent, model_equivalent,
};

fn key(token: u32) -> FieldKey {
    FieldKey::new(token)
}

fn record(record_type: &RecordType, fields: Vec<(FieldKey, Value)>) -> Value {
    Value::record(record_type.clone(), fields).unwrap()
}

#[test]
fn join_rejects_invalid_input_schemas_and_fields() {
    let scalar = BagValue::new(LogicalType::I32, [Value::i32(1)]).unwrap();
    let right_type = RecordType::new([(key(3), LogicalType::I32)]).unwrap();
    let right = BagValue::new(LogicalType::Record(right_type.clone()), []).unwrap();

    assert_eq!(
        join_fields_equivalent(&scalar, key(1), &right, key(3))
            .err()
            .unwrap(),
        FixtureError::JoinLeftRequiresRecord {
            actual: LogicalType::I32,
        }
    );
    assert_eq!(
        join_fields_equivalent(&right, key(3), &scalar, key(1))
            .err()
            .unwrap(),
        FixtureError::JoinRightRequiresRecord {
            actual: LogicalType::I32,
        }
    );

    let overlapping_left_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let overlapping_right_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let overlapping_left =
        BagValue::new(LogicalType::Record(overlapping_left_type), []).unwrap();
    let overlapping_right =
        BagValue::new(LogicalType::Record(overlapping_right_type), []).unwrap();
    assert_eq!(
        join_fields_equivalent(&overlapping_left, key(1), &overlapping_right, key(1))
            .err()
            .unwrap(),
        FixtureError::JoinOverlappingField(key(1))
    );

    let left_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();
    let right_type = RecordType::new([(key(3), LogicalType::U32)]).unwrap();
    let left = BagValue::new(LogicalType::Record(left_type), []).unwrap();
    let right = BagValue::new(LogicalType::Record(right_type), []).unwrap();

    assert_eq!(
        join_fields_equivalent(&left, key(2), &right, key(3))
            .err()
            .unwrap(),
        FixtureError::JoinLeftFieldNotFound(key(2))
    );
    assert_eq!(
        join_fields_equivalent(&left, key(1), &right, key(4))
            .err()
            .unwrap(),
        FixtureError::JoinRightFieldNotFound(key(4))
    );
    assert_eq!(
        join_fields_equivalent(&left, key(1), &right, key(3))
            .err()
            .unwrap(),
        FixtureError::JoinFieldTypeMismatch {
            left: LogicalType::I32,
            right: LogicalType::U32,
        }
    );
}

#[test]
fn join_empty_and_no_match_preserve_exact_disjoint_union_type() {
    let left_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let right_type =
        RecordType::new([(key(3), LogicalType::I32), (key(4), LogicalType::U8)]).unwrap();
    let output_type = LogicalType::Record(
        RecordType::new([
            (key(1), LogicalType::I32),
            (key(2), LogicalType::Bool),
            (key(3), LogicalType::I32),
            (key(4), LogicalType::U8),
        ])
        .unwrap(),
    );

    let empty_left = BagValue::new(LogicalType::Record(left_type.clone()), []).unwrap();
    let right_record = record(
        &right_type,
        vec![(key(3), Value::i32(7)), (key(4), Value::u8(1))],
    );
    let right = BagValue::new(LogicalType::Record(right_type.clone()), [right_record]).unwrap();
    let empty_result = join_fields_equivalent(&empty_left, key(1), &right, key(3)).unwrap();
    assert_eq!(empty_result.element_type(), &output_type);
    assert!(empty_result.is_empty());

    let left_record = record(
        &left_type,
        vec![(key(1), Value::i32(8)), (key(2), Value::bool(true))],
    );
    let left = BagValue::new(LogicalType::Record(left_type), [left_record]).unwrap();
    let no_match = join_fields_equivalent(&left, key(1), &right, key(3)).unwrap();
    assert_eq!(no_match.element_type(), &output_type);
    assert!(no_match.is_empty());
}

#[test]
fn join_multiplies_multiplicity_and_preserves_distinct_matching_pairs() {
    let left_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let right_type =
        RecordType::new([(key(3), LogicalType::I32), (key(4), LogicalType::U8)]).unwrap();
    let output_type = RecordType::new([
        (key(1), LogicalType::I32),
        (key(2), LogicalType::Bool),
        (key(3), LogicalType::I32),
        (key(4), LogicalType::U8),
    ])
    .unwrap();

    let left_true = record(
        &left_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(true))],
    );
    let left_false = record(
        &left_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(false))],
    );
    let right_one = record(
        &right_type,
        vec![(key(3), Value::i32(7)), (key(4), Value::u8(1))],
    );
    let right_two = record(
        &right_type,
        vec![(key(3), Value::i32(7)), (key(4), Value::u8(2))],
    );

    let left = BagValue::new(
        LogicalType::Record(left_type),
        [left_true.clone(), left_false.clone(), left_true.clone()],
    )
    .unwrap();
    let right = BagValue::new(
        LogicalType::Record(right_type),
        [
            right_one.clone(),
            right_two.clone(),
            right_one.clone(),
            right_one.clone(),
        ],
    )
    .unwrap();

    let result = join_fields_equivalent(&left, key(1), &right, key(3)).unwrap();
    assert_eq!(result.class_count(), 4);
    assert_eq!(result.total_multiplicity(), 12);

    let true_one = record(
        &output_type,
        vec![
            (key(1), Value::i32(7)),
            (key(2), Value::bool(true)),
            (key(3), Value::i32(7)),
            (key(4), Value::u8(1)),
        ],
    );
    let true_two = record(
        &output_type,
        vec![
            (key(1), Value::i32(7)),
            (key(2), Value::bool(true)),
            (key(3), Value::i32(7)),
            (key(4), Value::u8(2)),
        ],
    );
    let false_one = record(
        &output_type,
        vec![
            (key(1), Value::i32(7)),
            (key(2), Value::bool(false)),
            (key(3), Value::i32(7)),
            (key(4), Value::u8(1)),
        ],
    );
    let false_two = record(
        &output_type,
        vec![
            (key(1), Value::i32(7)),
            (key(2), Value::bool(false)),
            (key(3), Value::i32(7)),
            (key(4), Value::u8(2)),
        ],
    );

    assert_eq!(result.multiplicity_of(&true_one), 6);
    assert_eq!(result.multiplicity_of(&true_two), 2);
    assert_eq!(result.multiplicity_of(&false_one), 3);
    assert_eq!(result.multiplicity_of(&false_two), 1);
}

#[test]
fn join_uses_exact_optional_absence_and_present_equivalence() {
    let optional_i32 = LogicalType::optional(LogicalType::I32);
    let left_type =
        RecordType::new([(key(1), optional_i32.clone()), (key(2), LogicalType::U8)]).unwrap();
    let right_type =
        RecordType::new([(key(3), optional_i32), (key(4), LogicalType::U8)]).unwrap();

    let left_absent = record(
        &left_type,
        vec![
            (key(1), Value::absent(LogicalType::I32)),
            (key(2), Value::u8(1)),
        ],
    );
    let left_present = record(
        &left_type,
        vec![
            (
                key(1),
                Value::present(LogicalType::I32, Value::i32(7)).unwrap(),
            ),
            (key(2), Value::u8(2)),
        ],
    );
    let right_absent = record(
        &right_type,
        vec![
            (key(3), Value::absent(LogicalType::I32)),
            (key(4), Value::u8(3)),
        ],
    );
    let right_present = record(
        &right_type,
        vec![
            (
                key(3),
                Value::present(LogicalType::I32, Value::i32(7)).unwrap(),
            ),
            (key(4), Value::u8(4)),
        ],
    );

    let left = BagValue::new(
        LogicalType::Record(left_type),
        [left_absent, left_present],
    )
    .unwrap();
    let right = BagValue::new(
        LogicalType::Record(right_type),
        [right_present, right_absent],
    )
    .unwrap();
    let result = join_fields_equivalent(&left, key(1), &right, key(3)).unwrap();
    assert_eq!(result.class_count(), 2);
    assert_eq!(result.total_multiplicity(), 2);
}

#[test]
fn join_reuses_nan_equivalence_and_signed_zero_distinction() {
    let left_type =
        RecordType::new([(key(1), LogicalType::F64), (key(2), LogicalType::U8)]).unwrap();
    let right_type =
        RecordType::new([(key(3), LogicalType::F64), (key(4), LogicalType::U8)]).unwrap();
    let output_type = RecordType::new([
        (key(1), LogicalType::F64),
        (key(2), LogicalType::U8),
        (key(3), LogicalType::F64),
        (key(4), LogicalType::U8),
    ])
    .unwrap();

    let left_nan = record(
        &left_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(1))),
            ),
            (key(2), Value::u8(1)),
        ],
    );
    let left_plus = record(
        &left_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::positive_zero(FloatFormat::F64)),
            ),
            (key(2), Value::u8(2)),
        ],
    );
    let right_nan = record(
        &right_type,
        vec![
            (
                key(3),
                Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(99))),
            ),
            (key(4), Value::u8(3)),
        ],
    );
    let right_minus = record(
        &right_type,
        vec![
            (
                key(3),
                Value::float(FloatValue::negative_zero(FloatFormat::F64)),
            ),
            (key(4), Value::u8(4)),
        ],
    );
    let right_plus = record(
        &right_type,
        vec![
            (
                key(3),
                Value::float(FloatValue::positive_zero(FloatFormat::F64)),
            ),
            (key(4), Value::u8(5)),
        ],
    );

    let left = BagValue::new(LogicalType::Record(left_type), [left_nan, left_plus]).unwrap();
    let right = BagValue::new(
        LogicalType::Record(right_type),
        [right_minus, right_nan, right_plus],
    )
    .unwrap();
    let result = join_fields_equivalent(&left, key(1), &right, key(3)).unwrap();

    let expected_nan = record(
        &output_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::nan(
                    FloatFormat::F64,
                    NaNRealizationId::new(777),
                )),
            ),
            (key(2), Value::u8(1)),
            (
                key(3),
                Value::float(FloatValue::nan(
                    FloatFormat::F64,
                    NaNRealizationId::new(555),
                )),
            ),
            (key(4), Value::u8(3)),
        ],
    );
    let expected_plus = record(
        &output_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::positive_zero(FloatFormat::F64)),
            ),
            (key(2), Value::u8(2)),
            (
                key(3),
                Value::float(FloatValue::positive_zero(FloatFormat::F64)),
            ),
            (key(4), Value::u8(5)),
        ],
    );
    let forbidden_mixed = record(
        &output_type,
        vec![
            (
                key(1),
                Value::float(FloatValue::positive_zero(FloatFormat::F64)),
            ),
            (key(2), Value::u8(2)),
            (
                key(3),
                Value::float(FloatValue::negative_zero(FloatFormat::F64)),
            ),
            (key(4), Value::u8(4)),
        ],
    );

    assert_eq!(result.class_count(), 2);
    assert_eq!(result.total_multiplicity(), 2);
    assert_eq!(result.multiplicity_of(&expected_nan), 1);
    assert_eq!(result.multiplicity_of(&expected_plus), 1);
    assert_eq!(result.multiplicity_of(&forbidden_mixed), 0);
}

#[test]
fn join_reuses_recursive_nested_bag_equivalence() {
    let nested = LogicalType::bag(LogicalType::I32);
    let left_type =
        RecordType::new([(key(1), nested.clone()), (key(2), LogicalType::U8)]).unwrap();
    let right_type = RecordType::new([(key(3), nested), (key(4), LogicalType::U8)]).unwrap();

    let left_nested = BagValue::new(
        LogicalType::I32,
        [Value::i32(1), Value::i32(2), Value::i32(1)],
    )
    .unwrap();
    let right_nested = BagValue::new(
        LogicalType::I32,
        [Value::i32(2), Value::i32(1), Value::i32(1)],
    )
    .unwrap();
    let right_other =
        BagValue::new(LogicalType::I32, [Value::i32(1), Value::i32(2)]).unwrap();

    let left_record = record(
        &left_type,
        vec![(key(1), Value::bag(left_nested)), (key(2), Value::u8(1))],
    );
    let matching_right = record(
        &right_type,
        vec![(key(3), Value::bag(right_nested)), (key(4), Value::u8(2))],
    );
    let other_right = record(
        &right_type,
        vec![(key(3), Value::bag(right_other)), (key(4), Value::u8(3))],
    );

    let left = BagValue::new(LogicalType::Record(left_type), [left_record]).unwrap();
    let right = BagValue::new(
        LogicalType::Record(right_type),
        [other_right, matching_right],
    )
    .unwrap();
    let result = join_fields_equivalent(&left, key(1), &right, key(3)).unwrap();
    assert_eq!(result.class_count(), 1);
    assert_eq!(result.total_multiplicity(), 1);
}

#[test]
fn join_is_independent_of_record_and_occurrence_construction_order() {
    let left_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let right_type =
        RecordType::new([(key(3), LogicalType::I32), (key(4), LogicalType::U8)]).unwrap();

    let left_first = record(
        &left_type,
        vec![(key(1), Value::i32(7)), (key(2), Value::bool(true))],
    );
    let left_second = record(
        &left_type,
        vec![(key(2), Value::bool(false)), (key(1), Value::i32(8))],
    );
    let right_first = record(
        &right_type,
        vec![(key(3), Value::i32(7)), (key(4), Value::u8(1))],
    );
    let right_second = record(
        &right_type,
        vec![(key(4), Value::u8(2)), (key(3), Value::i32(8))],
    );

    let left_a = BagValue::new(
        LogicalType::Record(left_type.clone()),
        [left_first.clone(), left_second.clone()],
    )
    .unwrap();
    let left_b = BagValue::new(
        LogicalType::Record(left_type),
        [left_second, left_first],
    )
    .unwrap();
    let right_a = BagValue::new(
        LogicalType::Record(right_type.clone()),
        [right_first.clone(), right_second.clone()],
    )
    .unwrap();
    let right_b = BagValue::new(
        LogicalType::Record(right_type),
        [right_second, right_first],
    )
    .unwrap();

    let result_a = join_fields_equivalent(&left_a, key(1), &right_a, key(3)).unwrap();
    let result_b = join_fields_equivalent(&left_b, key(1), &right_b, key(3)).unwrap();
    assert!(model_equivalent(
        &Value::bag(result_a),
        &Value::bag(result_b)
    ));
}
