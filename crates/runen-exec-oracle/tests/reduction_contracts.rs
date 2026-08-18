use runen_exec_oracle::{
    Access, AccessKind, BufferId, BufferRegion, EachId, EachPhase, GroupId, HierarchyFixture,
    HierarchyId, HierarchyMembership, IterationId, PositionId, ReductionError, ReductionFixture,
    ReductionId, SubgroupId, UnorderedReductionEvidence, each_orders,
};

fn complete_reduction_evidence() -> UnorderedReductionEvidence {
    UnorderedReductionEvidence::none()
        .with_normal_and_closed()
        .with_result_only()
        .with_two_sided_identity()
        .with_associativity()
        .with_commutativity()
}

fn each_id() -> EachId {
    EachId::new(1)
}

fn iteration(token: u32) -> IterationId {
    IterationId::new(each_id(), token)
}

fn hierarchy_id() -> HierarchyId {
    HierarchyId::new(each_id(), 1)
}

fn group(token: u32) -> GroupId {
    GroupId::new(hierarchy_id(), token)
}

fn membership(iteration_token: u32, group_token: u32, subgroup: u32) -> HierarchyMembership {
    HierarchyMembership::new(
        iteration(iteration_token),
        SubgroupId::new(group(group_token), subgroup),
    )
}

fn hierarchy() -> HierarchyFixture {
    HierarchyFixture::new(
        hierarchy_id(),
        &[iteration(1), iteration(2), iteration(3), iteration(4)],
        vec![
            membership(1, 10, 1),
            membership(2, 10, 1),
            membership(3, 10, 2),
            membership(4, 20, 1),
        ],
    )
    .unwrap()
}

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

#[test]
fn unordered_reduction_requires_the_complete_combination_contract() {
    assert!(complete_reduction_evidence().permits_unordered_reduction());

    let missing_normality = UnorderedReductionEvidence::none()
        .with_result_only()
        .with_two_sided_identity()
        .with_associativity()
        .with_commutativity();
    let missing_result_only = UnorderedReductionEvidence::none()
        .with_normal_and_closed()
        .with_two_sided_identity()
        .with_associativity()
        .with_commutativity();
    let missing_identity = UnorderedReductionEvidence::none()
        .with_normal_and_closed()
        .with_result_only()
        .with_associativity()
        .with_commutativity();
    let missing_associativity = UnorderedReductionEvidence::none()
        .with_normal_and_closed()
        .with_result_only()
        .with_two_sided_identity()
        .with_commutativity();
    let missing_commutativity = UnorderedReductionEvidence::none()
        .with_normal_and_closed()
        .with_result_only()
        .with_two_sided_identity()
        .with_associativity();

    for insufficient in [
        missing_normality,
        missing_result_only,
        missing_identity,
        missing_associativity,
        missing_commutativity,
    ] {
        assert!(!insufficient.permits_unordered_reduction());
    }
}

#[test]
fn root_reduction_preserves_full_each_participation_and_empty_behavior() {
    let each = each_id();
    let reduction = ReductionFixture::root(
        ReductionId::new(7),
        each,
        &[iteration(1), iteration(2), iteration(3)],
    )
    .unwrap();

    assert!(reduction.contribution(iteration(1), 11).is_some());
    assert!(reduction.contribution(iteration(3), 29).is_some());
    assert!(reduction.contribution(iteration(4), 47).is_none());

    let empty = ReductionFixture::root(ReductionId::new(8), each, &[]).unwrap();
    assert!(empty.has_exact_contribution_coverage(&[], &[]));
    assert!(empty.contribution(iteration(1), 11).is_none());
    assert!(empty
        .contribution(IterationId::new(EachId::new(2), 1), 11)
        .is_none());

    assert!(matches!(
        ReductionFixture::root(
            ReductionId::new(9),
            each,
            &[iteration(1), iteration(1)]
        ),
        Err(ReductionError::InvalidRootIterations)
    ));
}

#[test]
fn root_reduction_rejects_foreign_each_iteration_tokens() {
    let local_each = each_id();
    let foreign_each = EachId::new(2);
    let local = iteration(1);
    let foreign_same_token = IterationId::new(foreign_each, 1);

    assert_ne!(local, foreign_same_token);
    assert!(matches!(
        ReductionFixture::root(ReductionId::new(7), local_each, &[foreign_same_token]),
        Err(ReductionError::InvalidRootIterations)
    ));

    let reduction = ReductionFixture::root(ReductionId::new(8), local_each, &[local]).unwrap();
    assert!(reduction.contribution(foreign_same_token, 11).is_none());
}

#[test]
fn group_reduction_selects_exact_existing_group() {
    let hierarchy = hierarchy();
    let reduction = ReductionFixture::group(ReductionId::new(7), &hierarchy, group(10)).unwrap();

    for (participant, token) in [
        (iteration(1), 17),
        (iteration(2), 31),
        (iteration(3), 43),
    ] {
        assert!(reduction.contribution(participant, token).is_some());
    }
    assert!(reduction.contribution(iteration(4), 59).is_none());
    assert!(reduction
        .contribution(IterationId::new(EachId::new(2), 1), 61)
        .is_none());

    assert!(matches!(
        ReductionFixture::group(ReductionId::new(8), &hierarchy, group(99)),
        Err(ReductionError::UnknownGroup)
    ));
}

#[test]
fn subgroup_reduction_selects_group_scoped_subgroup() {
    let hierarchy = hierarchy();
    let first_subgroup = SubgroupId::new(group(10), 1);
    let second_subgroup = SubgroupId::new(group(20), 1);
    let first =
        ReductionFixture::subgroup(ReductionId::new(7), &hierarchy, first_subgroup).unwrap();
    let second =
        ReductionFixture::subgroup(ReductionId::new(8), &hierarchy, second_subgroup).unwrap();

    assert_ne!(first_subgroup, second_subgroup);
    assert!(first.contribution(iteration(1), 13).is_some());
    assert!(first.contribution(iteration(2), 23).is_some());
    assert!(first.contribution(iteration(3), 37).is_none());
    assert!(first.contribution(iteration(4), 41).is_none());
    assert!(first
        .contribution(IterationId::new(EachId::new(2), 1), 47)
        .is_none());
    assert!(second.contribution(iteration(4), 53).is_some());
    assert!(second.contribution(iteration(1), 61).is_none());

    assert!(matches!(
        ReductionFixture::subgroup(
            ReductionId::new(9),
            &hierarchy,
            SubgroupId::new(group(10), 99)
        ),
        Err(ReductionError::UnknownSubgroup)
    ));
}

#[test]
fn reduction_rejects_foreign_hierarchy_group_and_subgroup_selectors() {
    let hierarchy = hierarchy();
    let foreign_group = GroupId::new(HierarchyId::new(EachId::new(2), 1), 10);
    let foreign_subgroup = SubgroupId::new(foreign_group, 1);

    assert!(matches!(
        ReductionFixture::group(ReductionId::new(7), &hierarchy, foreign_group),
        Err(ReductionError::UnknownGroup)
    ));
    assert!(matches!(
        ReductionFixture::subgroup(ReductionId::new(8), &hierarchy, foreign_subgroup),
        Err(ReductionError::UnknownSubgroup)
    ));
}

#[test]
fn one_participant_may_produce_multiple_distinct_contributions() {
    let reduction =
        ReductionFixture::root(ReductionId::new(7), each_id(), &[iteration(1)]).unwrap();
    let first = reduction.contribution(iteration(1), 11).unwrap();
    let second = reduction.contribution(iteration(1), 29).unwrap();

    assert_ne!(first, second);
    assert!(reduction.has_exact_contribution_coverage(&[first, second], &[second, first]));
}

#[test]
fn contribution_coverage_is_exact_and_order_neutral() {
    let reduction = ReductionFixture::root(
        ReductionId::new(7),
        each_id(),
        &[iteration(1), iteration(2), iteration(3)],
    )
    .unwrap();
    let first = reduction.contribution(iteration(1), 11).unwrap();
    let second = reduction.contribution(iteration(2), 29).unwrap();
    let third = reduction.contribution(iteration(3), 47).unwrap();
    let invented = reduction.contribution(iteration(1), 71).unwrap();

    assert!(
        reduction.has_exact_contribution_coverage(&[first, second, third], &[third, first, second])
    );
    assert!(!reduction.has_exact_contribution_coverage(&[first, second, third], &[first, third]));
    assert!(
        !reduction.has_exact_contribution_coverage(&[first, second, third], &[first, first, third])
    );
    assert!(
        !reduction
            .has_exact_contribution_coverage(&[first, second, third], &[first, second, invented])
    );
}

#[test]
fn duplicate_required_occurrence_id_is_invalid_even_across_producers() {
    let reduction = ReductionFixture::root(
        ReductionId::new(7),
        each_id(),
        &[iteration(1), iteration(2)],
    )
    .unwrap();
    let first = reduction.contribution(iteration(1), 17).unwrap();
    let conflicting_identity = reduction.contribution(iteration(2), 17).unwrap();

    assert!(!reduction.has_exact_contribution_coverage(
        &[first, conflicting_identity],
        &[conflicting_identity, first]
    ));
}

#[test]
fn same_occurrence_identity_cannot_change_producer() {
    let reduction = ReductionFixture::root(
        ReductionId::new(7),
        each_id(),
        &[iteration(1), iteration(2)],
    )
    .unwrap();
    let produced = reduction.contribution(iteration(1), 17).unwrap();
    let wrong_producer = reduction.contribution(iteration(2), 17).unwrap();

    assert!(!reduction.has_exact_contribution_coverage(&[produced], &[wrong_producer]));
}

#[test]
fn cross_reduction_contributions_do_not_satisfy_coverage() {
    let each = each_id();
    let required = [iteration(1), iteration(2)];
    let first_reduction = ReductionFixture::root(ReductionId::new(7), each, &required).unwrap();
    let second_reduction = ReductionFixture::root(ReductionId::new(8), each, &required).unwrap();
    let first = first_reduction.contribution(iteration(1), 17).unwrap();
    let other = second_reduction.contribution(iteration(1), 17).unwrap();

    assert!(!first_reduction.has_exact_contribution_coverage(&[first], &[other]));
    assert!(!first_reduction.has_exact_contribution_coverage(&[other], &[other]));
}

#[test]
fn equal_valued_contributions_remain_distinct_occurrences() {
    let reduction = ReductionFixture::root(
        ReductionId::new(7),
        each_id(),
        &[iteration(1), iteration(2)],
    )
    .unwrap();
    let first = reduction.contribution(iteration(1), 17).unwrap();
    let second = reduction.contribution(iteration(2), 31).unwrap();
    let same_value_contributions = [(first, 5_i64), (second, 5_i64)];

    assert!(reduction.has_exact_contribution_coverage(&[first, second], &[second, first]));
    assert_eq!(
        same_value_contributions
            .iter()
            .map(|(_, value)| *value)
            .sum::<i64>(),
        10
    );
}

#[test]
fn lawful_reduction_fixture_is_invariant_to_permutation_tree_and_neutral_identity() {
    fn combine(left: i64, right: i64) -> i64 {
        left + right
    }

    assert!(complete_reduction_evidence().permits_unordered_reduction());

    let identity = 0_i64;
    let left_tree = combine(combine(combine(combine(identity, 1), 2), 3), 4);
    let permuted_tree = combine(combine(4, 2), combine(1, combine(3, identity)));
    let right_tree = combine(1, combine(2, combine(3, combine(4, identity))));
    let extra_neutral_initializers = combine(
        combine(identity, combine(identity, 1)),
        combine(combine(identity, 2), combine(3, combine(identity, 4))),
    );

    assert_eq!(left_tree, 10);
    assert_eq!(permuted_tree, left_tree);
    assert_eq!(right_tree, left_tree);
    assert_eq!(extra_neutral_initializers, left_tree);
    assert_eq!(identity, 0);
}

#[test]
fn reduction_cohort_membership_does_not_order_or_legalize_sibling_conflict() {
    let hierarchy = hierarchy();
    let reduction = ReductionFixture::group(ReductionId::new(7), &hierarchy, group(10)).unwrap();
    let first = reduction.contribution(iteration(1), 17).unwrap();
    let second = reduction.contribution(iteration(2), 31).unwrap();
    let first_iteration = EachPhase::Iteration(iteration(1));
    let second_iteration = EachPhase::Iteration(iteration(2));
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(reduction.has_exact_contribution_coverage(&[first, second], &[second, first]));
    assert!(!each_orders(first_iteration, second_iteration));
    assert!(!each_orders(second_iteration, first_iteration));
    assert!(first_access.conflicts_with(&second_access));
}
