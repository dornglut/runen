use runen_model_oracle::{
    BagValue, FieldKey, FloatFormat, FloatValue, LogicalType, NaNRealizationId,
    ObservedResultTarget, RecordType, ResultObservationFixtureError, ResultTargetId,
    TargetObservationId, Value, model_equivalent,
};

const FLAG: FieldKey = FieldKey::new(1);
const CODE: FieldKey = FieldKey::new(2);

fn f32_nan(witness: u32) -> Value {
    Value::float(FloatValue::nan(
        FloatFormat::F32,
        NaNRealizationId::new(witness),
    ))
}

fn row_type() -> RecordType {
    RecordType::new([(FLAG, LogicalType::Bool), (CODE, LogicalType::U32)]).unwrap()
}

fn row(record_type: &RecordType, flag: bool, code: u32) -> Value {
    Value::record(
        record_type.clone(),
        [(FLAG, Value::bool(flag)), (CODE, Value::u32(code))],
    )
    .unwrap()
}

#[test]
fn result_target_validates_exact_result_type_and_lookup_is_fixture_only() {
    let observation = TargetObservationId::new(1);
    let error = ObservedResultTarget::new(
        ResultTargetId::new(10),
        LogicalType::Bool,
        [(observation, Value::u32(7))],
    )
    .err()
    .unwrap();

    assert_eq!(
        error,
        ResultObservationFixtureError::ResultTypeMismatch {
            expected: LogicalType::Bool,
            actual: LogicalType::U32,
        }
    );

    let empty = ObservedResultTarget::new(
        ResultTargetId::new(10),
        LogicalType::Bool,
        std::iter::empty::<(TargetObservationId, Value)>(),
    )
    .unwrap();
    assert_eq!(empty.result_type(), &LogicalType::Bool);
    assert!(empty.observed_result(observation).is_none());
}

#[test]
fn duplicate_target_observation_cannot_be_rebound() {
    let observation = TargetObservationId::new(2);
    let error = ObservedResultTarget::new(
        ResultTargetId::new(20),
        LogicalType::Bool,
        [
            (observation, Value::bool(true)),
            (observation, Value::bool(false)),
        ],
    )
    .err()
    .unwrap();

    assert_eq!(
        error,
        ResultObservationFixtureError::DuplicateTargetObservation(observation)
    );
}

#[test]
fn one_target_may_have_distinct_observations_with_distinct_results() {
    let first = TargetObservationId::new(3);
    let second = TargetObservationId::new(4);
    let target = ObservedResultTarget::new(
        ResultTargetId::new(30),
        LogicalType::Bool,
        [(first, Value::bool(true)), (second, Value::bool(false))],
    )
    .unwrap();

    assert_eq!(target.target_id(), ResultTargetId::new(30));
    assert!(!model_equivalent(
        target.observed_result(first).unwrap(),
        target.observed_result(second).unwrap()
    ));
}

#[test]
fn distinct_target_observations_may_expose_equivalent_results() {
    let first = TargetObservationId::new(5);
    let second = TargetObservationId::new(6);
    let target = ObservedResultTarget::new(
        ResultTargetId::new(40),
        LogicalType::F32,
        [(first, f32_nan(1)), (second, f32_nan(999))],
    )
    .unwrap();

    assert_ne!(first, second);
    assert!(model_equivalent(
        target.observed_result(first).unwrap(),
        target.observed_result(second).unwrap()
    ));
}

#[test]
fn distinct_targets_remain_distinct_with_equivalent_results() {
    let observation = TargetObservationId::new(7);
    let left = ObservedResultTarget::new(
        ResultTargetId::new(50),
        LogicalType::Bool,
        [(observation, Value::bool(true))],
    )
    .unwrap();
    let right = ObservedResultTarget::new(
        ResultTargetId::new(51),
        LogicalType::Bool,
        [(observation, Value::bool(true))],
    )
    .unwrap();

    assert_ne!(left.target_id(), right.target_id());
    assert!(model_equivalent(
        left.observed_result(observation).unwrap(),
        right.observed_result(observation).unwrap()
    ));
}

#[test]
fn target_observation_token_is_scoped_by_target_fixture() {
    let shared = TargetObservationId::new(8);
    let left = ObservedResultTarget::new(
        ResultTargetId::new(60),
        LogicalType::Bool,
        [(shared, Value::bool(true))],
    )
    .unwrap();
    let right = ObservedResultTarget::new(
        ResultTargetId::new(61),
        LogicalType::Bool,
        [(shared, Value::bool(false))],
    )
    .unwrap();

    assert_ne!(left.target_id(), right.target_id());
    assert!(!model_equivalent(
        left.observed_result(shared).unwrap(),
        right.observed_result(shared).unwrap()
    ));
}

#[test]
fn same_target_observation_across_realizations_uses_model_equivalence() {
    let target_id = ResultTargetId::new(70);
    let observation = TargetObservationId::new(9);
    let first =
        ObservedResultTarget::new(target_id, LogicalType::F32, [(observation, f32_nan(11))])
            .unwrap();
    let second =
        ObservedResultTarget::new(target_id, LogicalType::F32, [(observation, f32_nan(12))])
            .unwrap();

    assert_eq!(first.target_id(), second.target_id());
    assert!(model_equivalent(
        first.observed_result(observation).unwrap(),
        second.observed_result(observation).unwrap()
    ));
}

#[test]
fn signed_zero_distinction_is_preserved_in_target_observations() {
    let positive = TargetObservationId::new(10);
    let negative = TargetObservationId::new(11);
    let target = ObservedResultTarget::new(
        ResultTargetId::new(80),
        LogicalType::F32,
        [
            (
                positive,
                Value::float(FloatValue::positive_zero(FloatFormat::F32)),
            ),
            (
                negative,
                Value::float(FloatValue::negative_zero(FloatFormat::F32)),
            ),
        ],
    )
    .unwrap();

    assert!(!model_equivalent(
        target.observed_result(positive).unwrap(),
        target.observed_result(negative).unwrap()
    ));
}

#[test]
fn structural_collection_results_gain_no_target_specific_value_identity() {
    let record_type = row_type();
    let element_type = LogicalType::Record(record_type.clone());
    let result_type = LogicalType::bag(element_type.clone());
    let observation = TargetObservationId::new(12);

    let forward = Value::bag(
        BagValue::new(
            element_type.clone(),
            [row(&record_type, true, 1), row(&record_type, false, 2)],
        )
        .unwrap(),
    );
    let reverse = Value::bag(
        BagValue::new(
            element_type,
            [row(&record_type, false, 2), row(&record_type, true, 1)],
        )
        .unwrap(),
    );

    let left = ObservedResultTarget::new(
        ResultTargetId::new(90),
        result_type.clone(),
        [(observation, forward)],
    )
    .unwrap();
    let right = ObservedResultTarget::new(
        ResultTargetId::new(91),
        result_type,
        [(observation, reverse)],
    )
    .unwrap();

    assert_ne!(left.target_id(), right.target_id());
    assert!(model_equivalent(
        left.observed_result(observation).unwrap(),
        right.observed_result(observation).unwrap()
    ));
}

#[test]
fn target_observation_construction_order_is_not_semantic_order() {
    let first = TargetObservationId::new(13);
    let second = TargetObservationId::new(14);
    let first_value = Value::u32(10);
    let second_value = Value::u32(20);

    let forward = ObservedResultTarget::new(
        ResultTargetId::new(100),
        LogicalType::U32,
        [(first, first_value.clone()), (second, second_value.clone())],
    )
    .unwrap();
    let reverse = ObservedResultTarget::new(
        ResultTargetId::new(100),
        LogicalType::U32,
        [(second, second_value), (first, first_value)],
    )
    .unwrap();

    for observation in [first, second] {
        assert!(model_equivalent(
            forward.observed_result(observation).unwrap(),
            reverse.observed_result(observation).unwrap()
        ));
    }
}
