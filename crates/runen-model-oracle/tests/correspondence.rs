use runen_model_oracle::{
    BagValue, LogicalType, ObservationId, ObservedBagDomain, ObservedResultTarget, ResultTargetId,
    SingletonObservationSet, StateDomainId, TargetObservationId, Value, bag_cardinality,
    model_equivalent,
};

fn bool_bag(values: impl IntoIterator<Item = bool>) -> BagValue {
    BagValue::new(LogicalType::Bool, values.into_iter().map(Value::bool)).unwrap()
}

fn bag_cardinality_correspondence_holds(
    source: &SingletonObservationSet<'_>,
    target: &ObservedResultTarget,
    target_observation: TargetObservationId,
) -> bool {
    let Some(observed_result) = target.observed_result(target_observation) else {
        return false;
    };

    model_equivalent(observed_result, &bag_cardinality(source.observed_bag()))
}

#[test]
fn exact_bag_cardinality_correspondence_checks_actual_source_and_target_observations() {
    let source_observation = ObservationId::new(1);
    let source = ObservedBagDomain::new(
        StateDomainId::new(10),
        LogicalType::Bool,
        [(source_observation, bool_bag([true, false, true]))],
    )
    .unwrap();
    let source_context = source
        .singleton_observation_set(source_observation)
        .unwrap();

    let correct = TargetObservationId::new(1);
    let incorrect = TargetObservationId::new(2);
    let target = ObservedResultTarget::new(
        ResultTargetId::new(20),
        LogicalType::Cardinality,
        [
            (correct, Value::cardinality_from_u128(3)),
            (incorrect, Value::cardinality_from_u128(2)),
        ],
    )
    .unwrap();

    assert_eq!(target.result_type(), &LogicalType::Cardinality);
    assert!(bag_cardinality_correspondence_holds(
        &source_context,
        &target,
        correct
    ));
    assert!(!bag_cardinality_correspondence_holds(
        &source_context,
        &target,
        incorrect
    ));
}

#[test]
fn equivalent_results_do_not_collapse_distinct_source_or_target_observations() {
    let first_source = ObservationId::new(3);
    let second_source = ObservationId::new(4);
    let source = ObservedBagDomain::new(
        StateDomainId::new(30),
        LogicalType::Bool,
        [
            (first_source, bool_bag([true, false])),
            (second_source, bool_bag([false, false])),
        ],
    )
    .unwrap();
    let first_context = source.singleton_observation_set(first_source).unwrap();
    let second_context = source.singleton_observation_set(second_source).unwrap();

    let first_target = TargetObservationId::new(3);
    let second_target = TargetObservationId::new(4);
    let target = ObservedResultTarget::new(
        ResultTargetId::new(40),
        LogicalType::Cardinality,
        [
            (first_target, Value::cardinality_from_u128(2)),
            (second_target, Value::cardinality_from_u128(2)),
        ],
    )
    .unwrap();

    assert_ne!(
        first_context.observation_id(),
        second_context.observation_id()
    );
    assert_ne!(first_target, second_target);
    assert!(bag_cardinality_correspondence_holds(
        &first_context,
        &target,
        first_target
    ));
    assert!(bag_cardinality_correspondence_holds(
        &second_context,
        &target,
        second_target
    ));
    assert!(model_equivalent(
        target.observed_result(first_target).unwrap(),
        target.observed_result(second_target).unwrap()
    ));
}

#[test]
fn one_target_observation_can_be_checked_against_multiple_explicit_source_contexts() {
    let first = ObservationId::new(5);
    let second = ObservationId::new(6);
    let different = ObservationId::new(7);
    let source = ObservedBagDomain::new(
        StateDomainId::new(50),
        LogicalType::Bool,
        [
            (first, bool_bag([true, true])),
            (second, bool_bag([false, true])),
            (different, bool_bag([true])),
        ],
    )
    .unwrap();

    let target_observation = TargetObservationId::new(5);
    let target = ObservedResultTarget::new(
        ResultTargetId::new(60),
        LogicalType::Cardinality,
        [(target_observation, Value::cardinality_from_u128(2))],
    )
    .unwrap();

    let first_context = source.singleton_observation_set(first).unwrap();
    let second_context = source.singleton_observation_set(second).unwrap();
    let different_context = source.singleton_observation_set(different).unwrap();

    assert!(bag_cardinality_correspondence_holds(
        &first_context,
        &target,
        target_observation
    ));
    assert!(bag_cardinality_correspondence_holds(
        &second_context,
        &target,
        target_observation
    ));
    assert!(!bag_cardinality_correspondence_holds(
        &different_context,
        &target,
        target_observation
    ));
}

#[test]
fn correspondence_witness_is_independent_of_fixture_construction_order() {
    let first = ObservationId::new(8);
    let second = ObservationId::new(9);
    let forward_source = ObservedBagDomain::new(
        StateDomainId::new(70),
        LogicalType::Bool,
        [
            (first, bool_bag([true, false, true])),
            (second, bool_bag([false])),
        ],
    )
    .unwrap();
    let reverse_source = ObservedBagDomain::new(
        StateDomainId::new(70),
        LogicalType::Bool,
        [
            (second, bool_bag([false])),
            (first, bool_bag([false, true, true])),
        ],
    )
    .unwrap();

    let selected = TargetObservationId::new(8);
    let other = TargetObservationId::new(9);
    let forward_target = ObservedResultTarget::new(
        ResultTargetId::new(80),
        LogicalType::Cardinality,
        [
            (selected, Value::cardinality_from_u128(3)),
            (other, Value::cardinality_from_u128(1)),
        ],
    )
    .unwrap();
    let reverse_target = ObservedResultTarget::new(
        ResultTargetId::new(80),
        LogicalType::Cardinality,
        [
            (other, Value::cardinality_from_u128(1)),
            (selected, Value::cardinality_from_u128(3)),
        ],
    )
    .unwrap();

    let forward_context = forward_source.singleton_observation_set(first).unwrap();
    let reverse_context = reverse_source.singleton_observation_set(first).unwrap();

    assert!(bag_cardinality_correspondence_holds(
        &forward_context,
        &forward_target,
        selected
    ));
    assert!(bag_cardinality_correspondence_holds(
        &reverse_context,
        &reverse_target,
        selected
    ));
}
