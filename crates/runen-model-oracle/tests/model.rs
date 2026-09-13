use runen_model_oracle::{
    BagValue, FieldKey, FixtureError, FloatFormat, FloatValue, LogicalType, NaNRealizationId,
    RecordType, RelationValue, SequenceValue, Value, distinct, model_equivalent,
};

fn key(token: u32) -> FieldKey {
    FieldKey::new(token)
}

#[test]
fn structural_record_type_equality_ignores_construction_order() {
    let left = RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let right = RecordType::new([(key(2), LogicalType::Bool), (key(1), LogicalType::I32)]).unwrap();

    assert_eq!(left, right);
    assert_eq!(left.field_count(), 2);
    assert_eq!(left.field_type(key(1)), Some(&LogicalType::I32));
}

#[test]
fn duplicate_record_fields_reject() {
    assert_eq!(
        RecordType::new([(key(1), LogicalType::I32), (key(1), LogicalType::Bool)]),
        Err(FixtureError::DuplicateFieldKey(key(1)))
    );
}

#[test]
fn record_values_reject_missing_extra_and_wrong_type_fields() {
    let record_type = RecordType::new([(key(1), LogicalType::I32)]).unwrap();

    assert_eq!(
        Value::record(record_type.clone(), []).err().unwrap(),
        FixtureError::MissingField(key(1))
    );
    assert_eq!(
        Value::record(
            record_type.clone(),
            [(key(1), Value::i32(1)), (key(2), Value::bool(true))]
        )
        .err()
        .unwrap(),
        FixtureError::ExtraField(key(2))
    );
    assert_eq!(
        Value::record(record_type, [(key(1), Value::bool(true))])
            .err()
            .unwrap(),
        FixtureError::TypeMismatch {
            expected: LogicalType::I32,
            actual: LogicalType::Bool,
        }
    );
}

#[test]
fn record_value_equivalence_ignores_field_construction_order() {
    let record_type =
        RecordType::new([(key(1), LogicalType::I32), (key(2), LogicalType::Bool)]).unwrap();
    let left = Value::record(
        record_type.clone(),
        [(key(1), Value::i32(9)), (key(2), Value::bool(true))],
    )
    .unwrap();
    let right = Value::record(
        record_type,
        [(key(2), Value::bool(true)), (key(1), Value::i32(9))],
    )
    .unwrap();

    assert!(model_equivalent(&left, &right));
}

#[test]
fn empty_structural_values_are_valid() {
    let empty_record_type = RecordType::new(std::iter::empty::<(FieldKey, LogicalType)>()).unwrap();
    let empty_record = Value::record(
        empty_record_type.clone(),
        std::iter::empty::<(FieldKey, Value)>(),
    )
    .unwrap();
    let second_empty_record =
        Value::record(empty_record_type, std::iter::empty::<(FieldKey, Value)>()).unwrap();
    assert!(model_equivalent(&empty_record, &second_empty_record));

    assert!(
        RelationValue::new(LogicalType::I32, std::iter::empty::<Value>())
            .unwrap()
            .is_empty()
    );
    assert!(
        BagValue::new(LogicalType::I32, std::iter::empty::<Value>())
            .unwrap()
            .is_empty()
    );
    assert!(
        SequenceValue::new(LogicalType::I32, std::iter::empty::<Value>())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn typed_absence_and_nested_optional_tags_remain_distinct() {
    let absent_i32 = Value::absent(LogicalType::I32);
    let absent_bool = Value::absent(LogicalType::Bool);
    assert!(!model_equivalent(&absent_i32, &absent_bool));

    let optional_i32 = LogicalType::optional(LogicalType::I32);
    let absent_outer = Value::absent(optional_i32.clone());
    let present_absent =
        Value::present(optional_i32.clone(), Value::absent(LogicalType::I32)).unwrap();
    let present_present = Value::present(
        optional_i32,
        Value::present(LogicalType::I32, Value::i32(7)).unwrap(),
    )
    .unwrap();

    assert!(!model_equivalent(&absent_outer, &present_absent));
    assert!(!model_equivalent(&present_absent, &present_present));
}

#[test]
fn nested_constructors_reject_type_mismatch() {
    assert_eq!(
        Value::present(LogicalType::I32, Value::bool(true))
            .err()
            .unwrap(),
        FixtureError::TypeMismatch {
            expected: LogicalType::I32,
            actual: LogicalType::Bool,
        }
    );
    assert_eq!(
        BagValue::new(LogicalType::I32, [Value::bool(true)])
            .err()
            .unwrap(),
        FixtureError::TypeMismatch {
            expected: LogicalType::I32,
            actual: LogicalType::Bool,
        }
    );
    assert_eq!(
        SequenceValue::new(LogicalType::I32, [Value::u32(1)])
            .err()
            .unwrap(),
        FixtureError::TypeMismatch {
            expected: LogicalType::I32,
            actual: LogicalType::U32,
        }
    );
}

#[test]
fn all_intrinsic_integer_fixture_types_are_exact() {
    let values = [
        Value::i8(-1),
        Value::i16(-2),
        Value::i32(-3),
        Value::i64(-4),
        Value::u8(1),
        Value::u16(2),
        Value::u32(3),
        Value::u64(4),
    ];
    let expected = [
        LogicalType::I8,
        LogicalType::I16,
        LogicalType::I32,
        LogicalType::I64,
        LogicalType::U8,
        LogicalType::U16,
        LogicalType::U32,
        LogicalType::U64,
    ];

    for (value, expected_type) in values.iter().zip(&expected) {
        assert_eq!(value.logical_type(), expected_type);
    }
}

#[test]
fn finite_float_boundaries_validate_for_all_formats() {
    let cases = [
        (FloatFormat::F16, 11, -14, 15),
        (FloatFormat::F32, 24, -126, 127),
        (FloatFormat::F64, 53, -1022, 1023),
    ];

    for (format, precision, emin, emax) in cases {
        let normal_min = 1_u64 << (precision - 1);
        let normal_max = (1_u64 << precision) - 1;

        assert!(FloatValue::finite(format, false, normal_min, emin).is_ok());
        assert!(FloatValue::finite(format, true, normal_max, emax).is_ok());
        assert!(FloatValue::finite(format, false, 1, emin).is_ok());
        assert!(FloatValue::finite(format, true, normal_min - 1, emin).is_ok());

        assert!(FloatValue::finite(format, false, 0, emin).is_err());
        assert!(FloatValue::finite(format, false, normal_min - 1, emin + 1).is_err());
        assert!(FloatValue::finite(format, false, normal_max + 1, emin).is_err());
        assert!(FloatValue::finite(format, false, normal_min, emin - 1).is_err());
        assert!(FloatValue::finite(format, false, normal_min, emax + 1).is_err());
    }
}

#[test]
fn floating_special_values_keep_exact_model_equivalence() {
    let f16_nan_a = Value::float(FloatValue::nan(FloatFormat::F16, NaNRealizationId::new(1)));
    let f16_nan_b = Value::float(FloatValue::nan(FloatFormat::F16, NaNRealizationId::new(2)));
    let f32_nan = Value::float(FloatValue::nan(FloatFormat::F32, NaNRealizationId::new(1)));
    let plus_zero = Value::float(FloatValue::positive_zero(FloatFormat::F16));
    let minus_zero = Value::float(FloatValue::negative_zero(FloatFormat::F16));
    let plus_inf = Value::float(FloatValue::positive_infinity(FloatFormat::F16));
    let minus_inf = Value::float(FloatValue::negative_infinity(FloatFormat::F16));

    assert!(model_equivalent(&f16_nan_a, &f16_nan_b));
    assert!(!model_equivalent(&f16_nan_a, &f32_nan));
    assert!(!model_equivalent(&plus_zero, &minus_zero));
    assert!(!model_equivalent(&plus_inf, &minus_inf));
}

#[test]
fn relation_and_bag_are_equivalence_class_values_not_insertion_order() {
    let nan_a = Value::float(FloatValue::nan(FloatFormat::F32, NaNRealizationId::new(10)));
    let nan_b = Value::float(FloatValue::nan(FloatFormat::F32, NaNRealizationId::new(11)));
    let one = Value::float(FloatValue::finite(FloatFormat::F32, false, 1 << 23, 0).unwrap());

    let relation_left = RelationValue::new(
        LogicalType::F32,
        [nan_a.clone(), one.clone(), nan_b.clone()],
    )
    .unwrap();
    let relation_right = RelationValue::new(
        LogicalType::F32,
        [one.clone(), nan_b.clone(), nan_a.clone()],
    )
    .unwrap();
    assert_eq!(relation_left.class_count(), 2);
    assert!(relation_left.contains_equivalent(&nan_a));
    assert!(model_equivalent(
        &Value::relation(relation_left),
        &Value::relation(relation_right)
    ));

    let bag_left = BagValue::new(
        LogicalType::F32,
        [nan_a.clone(), one.clone(), nan_b.clone()],
    )
    .unwrap();
    let bag_right = BagValue::new(
        LogicalType::F32,
        [nan_b.clone(), nan_a.clone(), one.clone()],
    )
    .unwrap();
    assert_eq!(bag_left.class_count(), 2);
    assert_eq!(bag_left.total_multiplicity(), 3);
    assert_eq!(bag_left.multiplicity_of(&nan_a), 2);
    assert!(bag_left.contains_equivalent(&nan_b));
    assert!(model_equivalent(
        &Value::bag(bag_left),
        &Value::bag(bag_right)
    ));
}

#[test]
fn sequence_preserves_equivalent_occurrences_and_semantic_order() {
    let duplicate = SequenceValue::new(LogicalType::I32, [Value::i32(1), Value::i32(1)]).unwrap();
    assert_eq!(duplicate.len(), 2);
    assert!(model_equivalent(
        duplicate.at(0).unwrap(),
        duplicate.at(1).unwrap()
    ));

    let first = Value::i32(1);
    let second = Value::i32(2);
    let forward = SequenceValue::new(LogicalType::I32, [first.clone(), second.clone()]).unwrap();
    let reverse = SequenceValue::new(LogicalType::I32, [second, first]).unwrap();

    assert!(model_equivalent(forward.at(0).unwrap(), &Value::i32(1)));
    assert!(!model_equivalent(
        &Value::sequence(forward),
        &Value::sequence(reverse)
    ));
}

#[test]
fn recursive_record_optional_and_collection_equivalence_is_structural() {
    let record_type = RecordType::new([(key(1), LogicalType::optional(LogicalType::F16))]).unwrap();
    let left = Value::record(
        record_type.clone(),
        [(
            key(1),
            Value::present(
                LogicalType::F16,
                Value::float(FloatValue::nan(FloatFormat::F16, NaNRealizationId::new(1))),
            )
            .unwrap(),
        )],
    )
    .unwrap();
    let right = Value::record(
        record_type.clone(),
        [(
            key(1),
            Value::present(
                LogicalType::F16,
                Value::float(FloatValue::nan(FloatFormat::F16, NaNRealizationId::new(2))),
            )
            .unwrap(),
        )],
    )
    .unwrap();

    assert!(model_equivalent(&left, &right));

    let relation_left =
        RelationValue::new(LogicalType::Record(record_type.clone()), [left]).unwrap();
    let relation_right = RelationValue::new(LogicalType::Record(record_type), [right]).unwrap();
    assert!(model_equivalent(
        &Value::relation(relation_left),
        &Value::relation(relation_right)
    ));
}

#[test]
fn distinct_is_exact_bag_support() {
    let empty = BagValue::new(LogicalType::I32, []).unwrap();
    assert!(distinct(&empty).is_empty());

    let repeated = BagValue::new(
        LogicalType::I32,
        [Value::i32(7), Value::i32(7), Value::i32(7)],
    )
    .unwrap();
    let repeated_result = distinct(&repeated);
    assert_eq!(repeated_result.class_count(), 1);
    assert!(repeated_result.contains_equivalent(&Value::i32(7)));

    let multiple = BagValue::new(
        LogicalType::I32,
        [Value::i32(7), Value::i32(8), Value::i32(7)],
    )
    .unwrap();
    let multiple_result = distinct(&multiple);
    assert_eq!(multiple_result.class_count(), 2);
    assert!(multiple_result.contains_equivalent(&Value::i32(7)));
    assert!(multiple_result.contains_equivalent(&Value::i32(8)));
}

#[test]
fn distinct_uses_nan_and_signed_zero_model_classes() {
    let nan_a = Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(1)));
    let nan_b = Value::float(FloatValue::nan(FloatFormat::F64, NaNRealizationId::new(2)));
    let plus_zero = Value::float(FloatValue::positive_zero(FloatFormat::F64));
    let minus_zero = Value::float(FloatValue::negative_zero(FloatFormat::F64));
    let input = BagValue::new(
        LogicalType::F64,
        [nan_a, nan_b, plus_zero.clone(), minus_zero.clone()],
    )
    .unwrap();

    let result = distinct(&input);
    assert_eq!(result.class_count(), 3);
    assert!(result.contains_equivalent(&plus_zero));
    assert!(result.contains_equivalent(&minus_zero));
}

#[test]
fn nested_distinct_and_occurrence_order_are_semantically_irrelevant() {
    let inner_left = BagValue::new(LogicalType::I32, [Value::i32(1), Value::i32(1)]).unwrap();
    let inner_right = BagValue::new(LogicalType::I32, [Value::i32(1), Value::i32(1)]).unwrap();
    let element_type = LogicalType::bag(LogicalType::I32);

    let outer_left = BagValue::new(
        element_type.clone(),
        [
            Value::bag(inner_left.clone()),
            Value::bag(inner_right.clone()),
        ],
    )
    .unwrap();
    let outer_right = BagValue::new(
        element_type,
        [Value::bag(inner_right), Value::bag(inner_left)],
    )
    .unwrap();

    assert_eq!(outer_left.class_count(), 1);
    assert!(model_equivalent(
        &Value::relation(distinct(&outer_left)),
        &Value::relation(distinct(&outer_right))
    ));
}
