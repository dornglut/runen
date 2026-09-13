use runen_model_oracle::{
    BagValue, LogicalType, ObservationId, ObservedBagDomain, StateDomainId,
    TwoDomainObservationFixtureError, TwoDomainObservationSet, Value, bag_cardinality,
    model_equivalent,
};

fn bool_bag(values: impl IntoIterator<Item = bool>) -> BagValue {
    BagValue::new(LogicalType::Bool, values.into_iter().map(Value::bool)).unwrap()
}

fn bags_equivalent(left: &BagValue, right: &BagValue) -> bool {
    model_equivalent(&Value::bag(left.clone()), &Value::bag(right.clone()))
}

#[test]
fn two_distinct_admitted_domains_construct_domain_keyed_context() {
    let shared_observation = ObservationId::new(1);
    let left_id = StateDomainId::new(10);
    let right_id = StateDomainId::new(11);
    let left = ObservedBagDomain::new(
        left_id,
        LogicalType::Bool,
        [(shared_observation, bool_bag([true]))],
    )
    .unwrap();
    let right = ObservedBagDomain::new(
        right_id,
        LogicalType::Bool,
        [(shared_observation, bool_bag([false, false]))],
    )
    .unwrap();

    let context = TwoDomainObservationSet::new(
        &left,
        shared_observation,
        &right,
        shared_observation,
    )
    .unwrap();

    assert_eq!(context.observation_id(left_id), Some(shared_observation));
    assert_eq!(context.observation_id(right_id), Some(shared_observation));
    assert!(bags_equivalent(
        context.observed_bag(left_id).unwrap(),
        left.observed_bag(shared_observation).unwrap()
    ));
    assert!(bags_equivalent(
        context.observed_bag(right_id).unwrap(),
        right.observed_bag(shared_observation).unwrap()
    ));
    assert!(context.observation_id(StateDomainId::new(99)).is_none());
    assert!(context.observed_bag(StateDomainId::new(99)).is_none());
}

#[test]
fn same_domain_identity_is_rejected_across_distinct_fixture_objects() {
    let domain_id = StateDomainId::new(20);
    let observation = ObservationId::new(2);
    let first = ObservedBagDomain::new(
        domain_id,
        LogicalType::Bool,
        [(observation, bool_bag([true]))],
    )
    .unwrap();
    let second = ObservedBagDomain::new(
        domain_id,
        LogicalType::Bool,
        [(observation, bool_bag([false]))],
    )
    .unwrap();

    let error = TwoDomainObservationSet::new(&first, observation, &second, observation)
        .err()
        .unwrap();

    assert_eq!(
        error,
        TwoDomainObservationFixtureError::SameDomain(domain_id)
    );
}

#[test]
fn absent_member_observation_cannot_construct_context() {
    let left_id = StateDomainId::new(30);
    let right_id = StateDomainId::new(31);
    let admitted = ObservationId::new(3);
    let missing = ObservationId::new(300);
    let left = ObservedBagDomain::new(
        left_id,
        LogicalType::Bool,
        [(admitted, bool_bag([true]))],
    )
    .unwrap();
    let right = ObservedBagDomain::new(
        right_id,
        LogicalType::Bool,
        [(admitted, bool_bag([false]))],
    )
    .unwrap();

    let left_error = TwoDomainObservationSet::new(&left, missing, &right, admitted)
        .err()
        .unwrap();
    assert_eq!(
        left_error,
        TwoDomainObservationFixtureError::ObservationNotAdmitted {
            domain_id: left_id,
            observation_id: missing,
        }
    );

    let right_error = TwoDomainObservationSet::new(&left, admitted, &right, missing)
        .err()
        .unwrap();
    assert_eq!(
        right_error,
        TwoDomainObservationFixtureError::ObservationNotAdmitted {
            domain_id: right_id,
            observation_id: missing,
        }
    );
}

#[test]
fn equivalent_roots_do_not_collapse_distinct_domain_associations() {
    let observation = ObservationId::new(4);
    let left_id = StateDomainId::new(40);
    let right_id = StateDomainId::new(41);
    let left = ObservedBagDomain::new(
        left_id,
        LogicalType::Bool,
        [(observation, bool_bag([true, false]))],
    )
    .unwrap();
    let right = ObservedBagDomain::new(
        right_id,
        LogicalType::Bool,
        [(observation, bool_bag([false, true]))],
    )
    .unwrap();

    let context =
        TwoDomainObservationSet::new(&left, observation, &right, observation).unwrap();

    assert_ne!(left_id, right_id);
    assert_eq!(context.observation_id(left_id), Some(observation));
    assert_eq!(context.observation_id(right_id), Some(observation));
    assert!(bags_equivalent(
        context.observed_bag(left_id).unwrap(),
        context.observed_bag(right_id).unwrap()
    ));
}

#[test]
fn constructor_order_does_not_change_domain_to_observation_or_root_meaning() {
    let left_id = StateDomainId::new(50);
    let right_id = StateDomainId::new(51);
    let left_observation = ObservationId::new(5);
    let right_observation = ObservationId::new(6);
    let left = ObservedBagDomain::new(
        left_id,
        LogicalType::Bool,
        [(left_observation, bool_bag([true, true]))],
    )
    .unwrap();
    let right = ObservedBagDomain::new(
        right_id,
        LogicalType::Bool,
        [(right_observation, bool_bag([false]))],
    )
    .unwrap();

    let forward =
        TwoDomainObservationSet::new(&left, left_observation, &right, right_observation).unwrap();
    let reverse =
        TwoDomainObservationSet::new(&right, right_observation, &left, left_observation).unwrap();

    for (domain_id, observation_id) in [
        (left_id, left_observation),
        (right_id, right_observation),
    ] {
        assert_eq!(forward.observation_id(domain_id), Some(observation_id));
        assert_eq!(reverse.observation_id(domain_id), Some(observation_id));
        assert!(bags_equivalent(
            forward.observed_bag(domain_id).unwrap(),
            reverse.observed_bag(domain_id).unwrap()
        ));
    }
}

#[test]
fn both_selected_roots_feed_existing_pure_queries_independently() {
    let left_id = StateDomainId::new(60);
    let right_id = StateDomainId::new(61);
    let left_observation = ObservationId::new(7);
    let right_observation = ObservationId::new(8);
    let left = ObservedBagDomain::new(
        left_id,
        LogicalType::Bool,
        [(left_observation, bool_bag([true, true]))],
    )
    .unwrap();
    let right = ObservedBagDomain::new(
        right_id,
        LogicalType::Bool,
        [(right_observation, bool_bag([false, true, false]))],
    )
    .unwrap();
    let context =
        TwoDomainObservationSet::new(&left, left_observation, &right, right_observation).unwrap();

    assert!(model_equivalent(
        &bag_cardinality(context.observed_bag(left_id).unwrap()),
        &Value::cardinality_from_u128(2)
    ));
    assert!(model_equivalent(
        &bag_cardinality(context.observed_bag(right_id).unwrap()),
        &Value::cardinality_from_u128(3)
    ));
}
