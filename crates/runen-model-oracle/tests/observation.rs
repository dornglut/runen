use runen_model_oracle::{
    BagValue, FieldKey, FloatFormat, FloatValue, LogicalType, NaNRealizationId,
    ObservationFixtureError, ObservationId, ObservedBagDomain, RecordType, StateDomainId, Value,
    bag_cardinality, filter_field_equivalent, model_equivalent,
};

const ENABLED: FieldKey = FieldKey::new(1);
const CLASS: FieldKey = FieldKey::new(2);

fn bags_equivalent(left: &BagValue, right: &BagValue) -> bool {
    model_equivalent(&Value::bag(left.clone()), &Value::bag(right.clone()))
}

fn row_type() -> RecordType {
    RecordType::new([
        (ENABLED, LogicalType::Bool),
        (CLASS, LogicalType::U32),
    ])
    .unwrap()
}

fn row(record_type: &RecordType, enabled: bool, class: u32) -> Value {
    Value::record(
        record_type.clone(),
        [
            (ENABLED, Value::bool(enabled)),
            (CLASS, Value::u32(class)),
        ],
    )
    .unwrap()
}

fn row_reversed(record_type: &RecordType, enabled: bool, class: u32) -> Value {
    Value::record(
        record_type.clone(),
        [
            (CLASS, Value::u32(class)),
            (ENABLED, Value::bool(enabled)),
        ],
    )
    .unwrap()
}

#[test]
fn observed_bag_domain_validates_exact_root_element_type() {
    let observation = ObservationId::new(1);
    let wrong = BagValue::new(LogicalType::U32, [Value::u32(7)]).unwrap();

    let error = ObservedBagDomain::new(
        StateDomainId::new(10),
        LogicalType::Bool,
        [(observation, wrong)],
    )
    .err()
    .unwrap();

    assert_eq!(
        error,
        ObservationFixtureError::RootElementTypeMismatch {
            expected: LogicalType::Bool,
            actual: LogicalType::U32,
        }
    );

    let empty = ObservedBagDomain::new(
        StateDomainId::new(10),
        LogicalType::Bool,
        std::iter::empty::<(ObservationId, BagValue)>(),
    )
    .unwrap();
    assert_eq!(empty.root_type(), LogicalType::bag(LogicalType::Bool));
}

#[test]
fn duplicate_observation_identity_cannot_be_rebound() {
    let observation = ObservationId::new(4);
    let first = BagValue::new(LogicalType::Bool, [Value::bool(true)]).unwrap();
    let second = BagValue::new(LogicalType::Bool, [Value::bool(false)]).unwrap();

    let error = ObservedBagDomain::new(
        StateDomainId::new(20),
        LogicalType::Bool,
        [(observation, first), (observation, second)],
    )
    .err()
    .unwrap();

    assert_eq!(
        error,
        ObservationFixtureError::DuplicateObservation(observation)
    );
}

#[test]
fn earlier_observation_keeps_its_logical_root_when_later_observation_differs() {
    let earlier = ObservationId::new(1);
    let later = ObservationId::new(2);
    let earlier_root = BagValue::new(LogicalType::Bool, [Value::bool(true)]).unwrap();
    let later_root = BagValue::new(LogicalType::Bool, [Value::bool(false)]).unwrap();

    let domain = ObservedBagDomain::new(
        StateDomainId::new(30),
        LogicalType::Bool,
        [
            (earlier, earlier_root.clone()),
            (later, later_root.clone()),
        ],
    )
    .unwrap();

    assert!(bags_equivalent(
        domain.observed_bag(earlier).unwrap(),
        &earlier_root
    ));
    assert!(bags_equivalent(
        domain.observed_bag(later).unwrap(),
        &later_root
    ));
    assert!(!bags_equivalent(
        domain.observed_bag(earlier).unwrap(),
        domain.observed_bag(later).unwrap()
    ));
    assert!(domain.observed_bag(ObservationId::new(99)).is_none());
}

#[test]
fn distinct_observations_may_expose_model_equivalent_nan_roots() {
    let first_observation = ObservationId::new(11);
    let second_observation = ObservationId::new(12);
    let first_root = BagValue::new(
        LogicalType::F32,
        [Value::float(FloatValue::nan(
            FloatFormat::F32,
            NaNRealizationId::new(1),
        ))],
    )
    .unwrap();
    let second_root = BagValue::new(
        LogicalType::F32,
        [Value::float(FloatValue::nan(
            FloatFormat::F32,
            NaNRealizationId::new(999),
        ))],
    )
    .unwrap();

    let domain = ObservedBagDomain::new(
        StateDomainId::new(40),
        LogicalType::F32,
        [
            (first_observation, first_root),
            (second_observation, second_root),
        ],
    )
    .unwrap();

    assert_ne!(first_observation, second_observation);
    assert!(bags_equivalent(
        domain.observed_bag(first_observation).unwrap(),
        domain.observed_bag(second_observation).unwrap()
    ));
}

#[test]
fn observation_token_is_scoped_by_domain_fixture() {
    let observation = ObservationId::new(7);
    let true_root = BagValue::new(LogicalType::Bool, [Value::bool(true)]).unwrap();
    let false_root = BagValue::new(LogicalType::Bool, [Value::bool(false)]).unwrap();

    let left = ObservedBagDomain::new(
        StateDomainId::new(50),
        LogicalType::Bool,
        [(observation, true_root)],
    )
    .unwrap();
    let right = ObservedBagDomain::new(
        StateDomainId::new(51),
        LogicalType::Bool,
        [(observation, false_root)],
    )
    .unwrap();

    assert_ne!(left.domain_id(), right.domain_id());
    assert!(!bags_equivalent(
        left.observed_bag(observation).unwrap(),
        right.observed_bag(observation).unwrap()
    ));
}

#[test]
fn observation_construction_order_does_not_define_observation_order() {
    let first = ObservationId::new(21);
    let second = ObservationId::new(22);
    let first_root = BagValue::new(LogicalType::Bool, [Value::bool(true)]).unwrap();
    let second_root = BagValue::new(LogicalType::Bool, [Value::bool(false)]).unwrap();

    let forward = ObservedBagDomain::new(
        StateDomainId::new(60),
        LogicalType::Bool,
        [
            (first, first_root.clone()),
            (second, second_root.clone()),
        ],
    )
    .unwrap();
    let reverse = ObservedBagDomain::new(
        StateDomainId::new(60),
        LogicalType::Bool,
        [(second, second_root), (first, first_root)],
    )
    .unwrap();

    assert!(bags_equivalent(
        forward.observed_bag(first).unwrap(),
        reverse.observed_bag(first).unwrap()
    ));
    assert!(bags_equivalent(
        forward.observed_bag(second).unwrap(),
        reverse.observed_bag(second).unwrap()
    ));
}

#[test]
fn state_backed_query_pipeline_consumes_the_actually_observed_bag() {
    let record_type = row_type();
    let element_type = LogicalType::Record(record_type.clone());
    let first_observation = ObservationId::new(31);
    let second_observation = ObservationId::new(32);

    let first_root = BagValue::new(
        element_type.clone(),
        [
            row(&record_type, true, 1),
            row(&record_type, false, 2),
            row(&record_type, true, 3),
        ],
    )
    .unwrap();
    let second_root = BagValue::new(
        element_type.clone(),
        [
            row_reversed(&record_type, false, 5),
            row_reversed(&record_type, true, 6),
        ],
    )
    .unwrap();

    let domain = ObservedBagDomain::new(
        StateDomainId::new(70),
        element_type.clone(),
        [
            (second_observation, second_root),
            (first_observation, first_root),
        ],
    )
    .unwrap();

    assert_eq!(domain.root_type(), LogicalType::bag(element_type));

    let first_filtered = filter_field_equivalent(
        domain.observed_bag(first_observation).unwrap(),
        ENABLED,
        &Value::bool(true),
    )
    .unwrap();
    let second_filtered = filter_field_equivalent(
        domain.observed_bag(second_observation).unwrap(),
        ENABLED,
        &Value::bool(true),
    )
    .unwrap();

    assert!(model_equivalent(
        &bag_cardinality(&first_filtered),
        &Value::cardinality_from_u128(2)
    ));
    assert!(model_equivalent(
        &bag_cardinality(&second_filtered),
        &Value::cardinality_from_u128(1)
    ));
}
