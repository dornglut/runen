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
    RecordType::new([(ENABLED, LogicalType::Bool), (CLASS, LogicalType::U32)]).unwrap()
}

fn row(record_type: &RecordType, enabled: bool, class: u32) -> Value {
    Value::record(
        record_type.clone(),
        [(ENABLED, Value::bool(enabled)), (CLASS, Value::u32(class))],
    )
    .unwrap()
}

fn row_reversed(record_type: &RecordType, enabled: bool, class: u32) -> Value {
    Value::record(
        record_type.clone(),
        [(CLASS, Value::u32(class)), (ENABLED, Value::bool(enabled))],
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
        [(earlier, earlier_root.clone()), (later, later_root.clone())],
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
fn same_observation_across_realizations_is_compared_by_model_equivalence() {
    let domain_id = StateDomainId::new(41);
    let observation = ObservationId::new(13);
    let first_root = BagValue::new(
        LogicalType::F32,
        [Value::float(FloatValue::nan(
            FloatFormat::F32,
            NaNRealizationId::new(7),
        ))],
    )
    .unwrap();
    let second_root = BagValue::new(
        LogicalType::F32,
        [Value::float(FloatValue::nan(
            FloatFormat::F32,
            NaNRealizationId::new(8),
        ))],
    )
    .unwrap();

    let first_realization =
        ObservedBagDomain::new(domain_id, LogicalType::F32, [(observation, first_root)]).unwrap();
    let second_realization =
        ObservedBagDomain::new(domain_id, LogicalType::F32, [(observation, second_root)]).unwrap();

    assert_eq!(
        first_realization.domain_id(),
        second_realization.domain_id()
    );
    assert!(bags_equivalent(
        first_realization.observed_bag(observation).unwrap(),
        second_realization.observed_bag(observation).unwrap()
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
        [(first, first_root.clone()), (second, second_root.clone())],
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

#[test]
fn singleton_context_requires_admitted_observation_and_preserves_association() {
    let domain_id = StateDomainId::new(80);
    let observation = ObservationId::new(1);
    let root = BagValue::new(
        LogicalType::Bool,
        [Value::bool(true), Value::bool(false)],
    )
    .unwrap();
    let domain = ObservedBagDomain::new(domain_id, LogicalType::Bool, [(observation, root)])
        .unwrap();

    assert!(
        domain
            .singleton_observation_set(ObservationId::new(999))
            .is_none()
    );

    let singleton = domain.singleton_observation_set(observation).unwrap();
    assert_eq!(singleton.domain_id(), domain_id);
    assert_eq!(singleton.observation_id(), observation);
    assert!(bags_equivalent(
        singleton.observed_bag(),
        domain.observed_bag(observation).unwrap()
    ));
}

#[test]
fn singleton_context_preserves_direct_query_meaning() {
    let record_type = row_type();
    let element_type = LogicalType::Record(record_type.clone());
    let observation = ObservationId::new(2);
    let domain = ObservedBagDomain::new(
        StateDomainId::new(81),
        element_type,
        [(
            observation,
            BagValue::new(
                LogicalType::Record(record_type.clone()),
                [
                    row(&record_type, true, 1),
                    row(&record_type, false, 2),
                    row(&record_type, true, 3),
                ],
            )
            .unwrap(),
        )],
    )
    .unwrap();

    let direct_filtered = filter_field_equivalent(
        domain.observed_bag(observation).unwrap(),
        ENABLED,
        &Value::bool(true),
    )
    .unwrap();
    let singleton = domain.singleton_observation_set(observation).unwrap();
    let singleton_filtered =
        filter_field_equivalent(singleton.observed_bag(), ENABLED, &Value::bool(true)).unwrap();

    assert!(bags_equivalent(&direct_filtered, &singleton_filtered));
    assert!(model_equivalent(
        &bag_cardinality(&direct_filtered),
        &bag_cardinality(&singleton_filtered)
    ));
    assert!(model_equivalent(
        &bag_cardinality(&singleton_filtered),
        &Value::cardinality_from_u128(2)
    ));
}

#[test]
fn equivalent_roots_do_not_collapse_distinct_singleton_observation_identity() {
    let first = ObservationId::new(3);
    let second = ObservationId::new(4);
    let first_root = BagValue::new(
        LogicalType::F32,
        [Value::float(FloatValue::nan(
            FloatFormat::F32,
            NaNRealizationId::new(10),
        ))],
    )
    .unwrap();
    let second_root = BagValue::new(
        LogicalType::F32,
        [Value::float(FloatValue::nan(
            FloatFormat::F32,
            NaNRealizationId::new(20),
        ))],
    )
    .unwrap();
    let domain = ObservedBagDomain::new(
        StateDomainId::new(82),
        LogicalType::F32,
        [(first, first_root), (second, second_root)],
    )
    .unwrap();

    let first_context = domain.singleton_observation_set(first).unwrap();
    let second_context = domain.singleton_observation_set(second).unwrap();

    assert_ne!(first_context.observation_id(), second_context.observation_id());
    assert_eq!(first_context.domain_id(), second_context.domain_id());
    assert!(bags_equivalent(
        first_context.observed_bag(),
        second_context.observed_bag()
    ));
}

#[test]
fn singleton_context_domain_scope_and_construction_order_are_independent() {
    let shared_observation = ObservationId::new(5);
    let left_domain = ObservedBagDomain::new(
        StateDomainId::new(83),
        LogicalType::Bool,
        [(shared_observation, BagValue::new(LogicalType::Bool, [Value::bool(true)]).unwrap())],
    )
    .unwrap();
    let right_domain = ObservedBagDomain::new(
        StateDomainId::new(84),
        LogicalType::Bool,
        [(shared_observation, BagValue::new(LogicalType::Bool, [Value::bool(false)]).unwrap())],
    )
    .unwrap();

    let left_context = left_domain
        .singleton_observation_set(shared_observation)
        .unwrap();
    let right_context = right_domain
        .singleton_observation_set(shared_observation)
        .unwrap();
    assert_ne!(left_context.domain_id(), right_context.domain_id());
    assert_eq!(
        left_context.observation_id(),
        right_context.observation_id()
    );

    let first = ObservationId::new(6);
    let second = ObservationId::new(7);
    let first_root = BagValue::new(LogicalType::Bool, [Value::bool(true)]).unwrap();
    let second_root = BagValue::new(LogicalType::Bool, [Value::bool(false)]).unwrap();
    let forward = ObservedBagDomain::new(
        StateDomainId::new(85),
        LogicalType::Bool,
        [(first, first_root.clone()), (second, second_root.clone())],
    )
    .unwrap();
    let reverse = ObservedBagDomain::new(
        StateDomainId::new(85),
        LogicalType::Bool,
        [(second, second_root), (first, first_root)],
    )
    .unwrap();

    for observation in [first, second] {
        let forward_context = forward.singleton_observation_set(observation).unwrap();
        let reverse_context = reverse.singleton_observation_set(observation).unwrap();
        assert_eq!(forward_context.domain_id(), reverse_context.domain_id());
        assert_eq!(
            forward_context.observation_id(),
            reverse_context.observation_id()
        );
        assert!(bags_equivalent(
            forward_context.observed_bag(),
            reverse_context.observed_bag()
        ));
    }
}
