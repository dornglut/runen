use runen_exec_oracle::{
    Access, AccessKind, BarrierError, BarrierFixture, BarrierId, BufferId, BufferRegion, EachId,
    EachPhase, GroupId, HierarchyError, HierarchyFixture, HierarchyId, HierarchyMembership,
    IterationId, PositionId, ReductionError, ReductionFixture, ReductionId, SubgroupId, each_orders,
};

fn each_id() -> EachId {
    EachId::new(1)
}

fn iteration(token: u32) -> IterationId {
    IterationId::new(each_id(), token)
}

fn hierarchy_id() -> HierarchyId {
    HierarchyId::new(each_id(), 1)
}

fn membership_in(
    hierarchy: HierarchyId,
    iteration: IterationId,
    group: u32,
    subgroup: u32,
) -> HierarchyMembership {
    let group = GroupId::new(hierarchy, group);
    HierarchyMembership::new(iteration, SubgroupId::new(group, subgroup))
}

fn membership(iteration_token: u32, group: u32, subgroup: u32) -> HierarchyMembership {
    membership_in(hierarchy_id(), iteration(iteration_token), group, subgroup)
}

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

#[test]
fn hierarchy_requires_exact_iteration_coverage_and_accepts_empty_each() {
    assert!(HierarchyFixture::new(hierarchy_id(), &[], vec![]).is_ok());

    let required = [iteration(1), iteration(2), iteration(3)];
    let valid = vec![
        membership(3, 2, 1),
        membership(1, 1, 1),
        membership(2, 1, 2),
    ];
    assert!(HierarchyFixture::new(hierarchy_id(), &required, valid).is_ok());

    assert!(matches!(
        HierarchyFixture::new(
            hierarchy_id(),
            &required,
            vec![membership(1, 1, 1), membership(2, 1, 2)]
        ),
        Err(HierarchyError::InvalidIterationCoverage)
    ));
    assert!(matches!(
        HierarchyFixture::new(
            hierarchy_id(),
            &required,
            vec![
                membership(1, 1, 1),
                membership(2, 1, 2),
                membership(4, 2, 1)
            ]
        ),
        Err(HierarchyError::InvalidIterationCoverage)
    ));
    assert!(matches!(
        HierarchyFixture::new(
            hierarchy_id(),
            &required,
            vec![
                membership(1, 1, 1),
                membership(1, 2, 1),
                membership(3, 2, 1)
            ]
        ),
        Err(HierarchyError::InvalidIterationCoverage)
    ));
    assert!(matches!(
        HierarchyFixture::new(
            hierarchy_id(),
            &[iteration(1), iteration(1)],
            vec![membership(1, 1, 1), membership(1, 1, 1)]
        ),
        Err(HierarchyError::InvalidIterationCoverage)
    ));
    assert!(matches!(
        HierarchyFixture::new(hierarchy_id(), &[], vec![membership(1, 1, 1)]),
        Err(HierarchyError::InvalidIterationCoverage)
    ));
}

#[test]
fn hierarchy_rejects_membership_from_another_hierarchy() {
    let local = HierarchyId::new(each_id(), 1);
    let foreign = HierarchyId::new(each_id(), 2);

    assert!(matches!(
        HierarchyFixture::new(
            local,
            &[iteration(1)],
            vec![membership_in(foreign, iteration(1), 10, 7)]
        ),
        Err(HierarchyError::ForeignHierarchyMembership)
    ));
}

#[test]
fn hierarchy_rejects_iteration_from_another_dynamic_each() {
    let local_each = each_id();
    let foreign_each = EachId::new(2);
    let local_hierarchy = HierarchyId::new(local_each, 1);
    let local_group = GroupId::new(local_hierarchy, 10);
    let local_subgroup = SubgroupId::new(local_group, 7);
    let local_iteration = IterationId::new(local_each, 1);
    let foreign_same_token = IterationId::new(foreign_each, 1);

    assert_ne!(local_iteration, foreign_same_token);
    assert!(matches!(
        HierarchyFixture::new(
            local_hierarchy,
            &[local_iteration],
            vec![HierarchyMembership::new(foreign_same_token, local_subgroup)]
        ),
        Err(HierarchyError::ForeignEachIteration)
    ));
    assert!(matches!(
        HierarchyFixture::new(
            local_hierarchy,
            &[foreign_same_token],
            vec![HierarchyMembership::new(foreign_same_token, local_subgroup)]
        ),
        Err(HierarchyError::ForeignEachIteration)
    ));
}

#[test]
fn groups_and_subgroups_form_nested_partitions_of_required_iterations() {
    let hierarchy = HierarchyFixture::new(
        hierarchy_id(),
        &[iteration(1), iteration(2), iteration(3), iteration(4)],
        vec![
            membership(1, 10, 1),
            membership(2, 10, 1),
            membership(3, 10, 2),
            membership(4, 20, 1),
        ],
    )
    .unwrap();

    assert!(hierarchy.same_group(iteration(1), iteration(2)));
    assert!(hierarchy.same_group(iteration(1), iteration(3)));
    assert!(!hierarchy.same_group(iteration(1), iteration(4)));

    assert!(hierarchy.same_subgroup(iteration(1), iteration(2)));
    assert!(!hierarchy.same_subgroup(iteration(1), iteration(3)));
    assert!(!hierarchy.same_subgroup(iteration(1), iteration(4)));

    let third = hierarchy.membership(iteration(3)).unwrap();
    let group = GroupId::new(hierarchy_id(), 10);
    assert_eq!(third.iteration(), iteration(3));
    assert_eq!(third.group(), group);
    assert_eq!(third.subgroup(), SubgroupId::new(group, 2));
    assert!(hierarchy.membership(iteration(9)).is_none());
}

#[test]
fn hierarchy_storage_order_does_not_change_semantic_relations() {
    let required = [iteration(1), iteration(2), iteration(3)];
    let first = HierarchyFixture::new(
        hierarchy_id(),
        &required,
        vec![
            membership(1, 10, 1),
            membership(2, 10, 1),
            membership(3, 20, 1),
        ],
    )
    .unwrap();
    let second = HierarchyFixture::new(
        hierarchy_id(),
        &required,
        vec![
            membership(3, 20, 1),
            membership(1, 10, 1),
            membership(2, 10, 1),
        ],
    )
    .unwrap();

    for iteration in required {
        assert_eq!(first.membership(iteration), second.membership(iteration));
    }
    assert_eq!(
        first.same_group(iteration(1), iteration(2)),
        second.same_group(iteration(1), iteration(2))
    );
    assert_eq!(
        first.same_subgroup(iteration(1), iteration(2)),
        second.same_subgroup(iteration(1), iteration(2))
    );
}

#[test]
fn hierarchy_group_and_subgroup_identity_are_scoped_by_dynamic_each() {
    let first_each = EachId::new(1);
    let second_each = EachId::new(2);
    let first_hierarchy = HierarchyId::new(first_each, 1);
    let second_hierarchy = HierarchyId::new(second_each, 1);
    let first_group = GroupId::new(first_hierarchy, 10);
    let second_group = GroupId::new(second_hierarchy, 10);
    let first_subgroup = SubgroupId::new(first_group, 7);
    let second_subgroup = SubgroupId::new(second_group, 7);

    assert_ne!(first_hierarchy, second_hierarchy);
    assert_ne!(first_group, second_group);
    assert_ne!(first_subgroup, second_subgroup);
}

#[test]
fn distinct_same_each_hierarchies_bind_consumers_to_exact_instance() {
    let each = each_id();
    let required = [iteration(1), iteration(2)];
    let first_id = HierarchyId::new(each, 1);
    let second_id = HierarchyId::new(each, 2);
    let first_group = GroupId::new(first_id, 10);
    let second_group = GroupId::new(second_id, 10);
    let first_subgroup = SubgroupId::new(first_group, 7);
    let second_subgroup = SubgroupId::new(second_group, 7);

    let first = HierarchyFixture::new(
        first_id,
        &required,
        vec![
            membership_in(first_id, iteration(1), 10, 7),
            membership_in(first_id, iteration(2), 10, 7),
        ],
    )
    .unwrap();
    let second = HierarchyFixture::new(
        second_id,
        &required,
        vec![
            membership_in(second_id, iteration(1), 10, 7),
            membership_in(second_id, iteration(2), 10, 7),
        ],
    )
    .unwrap();

    assert_ne!(first_id, second_id);
    assert_ne!(first_group, second_group);
    assert_ne!(first_subgroup, second_subgroup);
    assert!(first.same_group(iteration(1), iteration(2)));
    assert!(second.same_group(iteration(1), iteration(2)));

    assert!(matches!(
        BarrierFixture::group(BarrierId::new(1), &first, second_group),
        Err(BarrierError::UnknownGroup)
    ));
    assert!(matches!(
        BarrierFixture::subgroup(BarrierId::new(2), &first, second_subgroup),
        Err(BarrierError::UnknownSubgroup)
    ));
    assert!(matches!(
        ReductionFixture::group(ReductionId::new(1), &first, second_group),
        Err(ReductionError::UnknownGroup)
    ));
    assert!(matches!(
        ReductionFixture::subgroup(ReductionId::new(2), &first, second_subgroup),
        Err(ReductionError::UnknownSubgroup)
    ));

    assert!(BarrierFixture::group(BarrierId::new(3), &first, first_group).is_ok());
    assert!(ReductionFixture::subgroup(ReductionId::new(3), &first, first_subgroup).is_ok());
}

#[test]
fn subgroup_identity_is_scoped_by_containing_group() {
    let first = SubgroupId::new(GroupId::new(hierarchy_id(), 10), 7);
    let second = SubgroupId::new(GroupId::new(hierarchy_id(), 20), 7);
    assert_ne!(first, second);

    let hierarchy = HierarchyFixture::new(
        hierarchy_id(),
        &[iteration(1), iteration(2)],
        vec![membership(1, 10, 7), membership(2, 20, 7)],
    )
    .unwrap();

    assert!(!hierarchy.same_group(iteration(1), iteration(2)));
    assert!(!hierarchy.same_subgroup(iteration(1), iteration(2)));
}

#[test]
fn same_subgroup_implies_same_group() {
    let hierarchy = HierarchyFixture::new(
        hierarchy_id(),
        &[iteration(1), iteration(2)],
        vec![membership(1, 10, 7), membership(2, 10, 7)],
    )
    .unwrap();

    assert!(hierarchy.same_subgroup(iteration(1), iteration(2)));
    assert!(hierarchy.same_group(iteration(1), iteration(2)));
}

#[test]
fn hierarchy_membership_supplies_no_sibling_order() {
    let hierarchy = HierarchyFixture::new(
        hierarchy_id(),
        &[iteration(1), iteration(2)],
        vec![membership(1, 10, 7), membership(2, 10, 7)],
    )
    .unwrap();
    let first = EachPhase::Iteration(iteration(1));
    let second = EachPhase::Iteration(iteration(2));

    assert!(hierarchy.same_subgroup(iteration(1), iteration(2)));
    assert!(!each_orders(first, second));
    assert!(!each_orders(second, first));
}

#[test]
fn hierarchy_membership_does_not_legalize_ordinary_conflict() {
    let hierarchy = HierarchyFixture::new(
        hierarchy_id(),
        &[iteration(1), iteration(2)],
        vec![membership(1, 10, 7), membership(2, 10, 7)],
    )
    .unwrap();
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(hierarchy.same_group(iteration(1), iteration(2)));
    assert!(first_access.conflicts_with(&second_access));
}

#[test]
fn root_barrier_order_is_independent_of_hierarchy_membership() {
    let each = each_id();
    let required = [iteration(1), iteration(2)];
    let hierarchy = HierarchyFixture::new(
        hierarchy_id(),
        &required,
        vec![membership(1, 10, 1), membership(2, 20, 1)],
    )
    .unwrap();
    let barrier = BarrierFixture::root(BarrierId::new(5), each, &required).unwrap();
    let before = barrier.before(iteration(1)).unwrap();
    let after = barrier.after(iteration(2)).unwrap();
    let foreign_same_token = IterationId::new(EachId::new(2), 1);

    assert!(!hierarchy.same_group(iteration(1), iteration(2)));
    assert!(barrier.orders(before, after));
    assert!(barrier.before(foreign_same_token).is_none());
}
