use runen_exec_oracle::{
    Access, AccessKind, BarrierFixture, BarrierId, BufferId, BufferRegion, EachPhase, GroupId,
    HierarchyError, HierarchyFixture, HierarchyMembership, IterationId, PositionId, SubgroupId,
    each_orders,
};

fn membership(iteration: u32, group: u32, subgroup: u32) -> HierarchyMembership {
    let group = GroupId::new(group);
    HierarchyMembership::new(IterationId(iteration), SubgroupId::new(group, subgroup))
}

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

#[test]
fn hierarchy_requires_exact_iteration_coverage_and_accepts_empty_each() {
    assert!(HierarchyFixture::new(&[], vec![]).is_ok());

    let required = [IterationId(1), IterationId(2), IterationId(3)];
    let valid = vec![
        membership(3, 2, 1),
        membership(1, 1, 1),
        membership(2, 1, 2),
    ];
    assert!(HierarchyFixture::new(&required, valid).is_ok());

    assert!(matches!(
        HierarchyFixture::new(&required, vec![membership(1, 1, 1), membership(2, 1, 2)]),
        Err(HierarchyError::InvalidIterationCoverage)
    ));
    assert!(matches!(
        HierarchyFixture::new(
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
            &[IterationId(1), IterationId(1)],
            vec![membership(1, 1, 1), membership(1, 1, 1)]
        ),
        Err(HierarchyError::InvalidIterationCoverage)
    ));
    assert!(matches!(
        HierarchyFixture::new(&[], vec![membership(1, 1, 1)]),
        Err(HierarchyError::InvalidIterationCoverage)
    ));
}

#[test]
fn groups_and_subgroups_form_nested_partitions_of_required_iterations() {
    let hierarchy = HierarchyFixture::new(
        &[
            IterationId(1),
            IterationId(2),
            IterationId(3),
            IterationId(4),
        ],
        vec![
            membership(1, 10, 1),
            membership(2, 10, 1),
            membership(3, 10, 2),
            membership(4, 20, 1),
        ],
    )
    .unwrap();

    assert!(hierarchy.same_group(IterationId(1), IterationId(2)));
    assert!(hierarchy.same_group(IterationId(1), IterationId(3)));
    assert!(!hierarchy.same_group(IterationId(1), IterationId(4)));

    assert!(hierarchy.same_subgroup(IterationId(1), IterationId(2)));
    assert!(!hierarchy.same_subgroup(IterationId(1), IterationId(3)));
    assert!(!hierarchy.same_subgroup(IterationId(1), IterationId(4)));

    let third = hierarchy.membership(IterationId(3)).unwrap();
    assert_eq!(third.iteration(), IterationId(3));
    assert_eq!(third.group(), GroupId::new(10));
    assert_eq!(third.subgroup(), SubgroupId::new(GroupId::new(10), 2));
    assert!(hierarchy.membership(IterationId(9)).is_none());
}

#[test]
fn hierarchy_storage_order_does_not_change_semantic_relations() {
    let required = [IterationId(1), IterationId(2), IterationId(3)];
    let first = HierarchyFixture::new(
        &required,
        vec![
            membership(1, 10, 1),
            membership(2, 10, 1),
            membership(3, 20, 1),
        ],
    )
    .unwrap();
    let second = HierarchyFixture::new(
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
        first.same_group(IterationId(1), IterationId(2)),
        second.same_group(IterationId(1), IterationId(2))
    );
    assert_eq!(
        first.same_subgroup(IterationId(1), IterationId(2)),
        second.same_subgroup(IterationId(1), IterationId(2))
    );
}

#[test]
fn subgroup_identity_is_scoped_by_containing_group() {
    let first = SubgroupId::new(GroupId::new(10), 7);
    let second = SubgroupId::new(GroupId::new(20), 7);
    assert_ne!(first, second);

    let hierarchy = HierarchyFixture::new(
        &[IterationId(1), IterationId(2)],
        vec![membership(1, 10, 7), membership(2, 20, 7)],
    )
    .unwrap();

    assert!(!hierarchy.same_group(IterationId(1), IterationId(2)));
    assert!(!hierarchy.same_subgroup(IterationId(1), IterationId(2)));
}

#[test]
fn same_subgroup_implies_same_group() {
    let hierarchy = HierarchyFixture::new(
        &[IterationId(1), IterationId(2)],
        vec![membership(1, 10, 7), membership(2, 10, 7)],
    )
    .unwrap();

    assert!(hierarchy.same_subgroup(IterationId(1), IterationId(2)));
    assert!(hierarchy.same_group(IterationId(1), IterationId(2)));
}

#[test]
fn hierarchy_membership_supplies_no_sibling_order() {
    let hierarchy = HierarchyFixture::new(
        &[IterationId(1), IterationId(2)],
        vec![membership(1, 10, 7), membership(2, 10, 7)],
    )
    .unwrap();
    let first = EachPhase::Iteration(IterationId(1));
    let second = EachPhase::Iteration(IterationId(2));

    assert!(hierarchy.same_subgroup(IterationId(1), IterationId(2)));
    assert!(!each_orders(first, second));
    assert!(!each_orders(second, first));
}

#[test]
fn hierarchy_membership_does_not_legalize_ordinary_conflict() {
    let hierarchy = HierarchyFixture::new(
        &[IterationId(1), IterationId(2)],
        vec![membership(1, 10, 7), membership(2, 10, 7)],
    )
    .unwrap();
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(hierarchy.same_group(IterationId(1), IterationId(2)));
    assert!(first_access.conflicts_with(&second_access));
}

#[test]
fn root_barrier_order_is_independent_of_hierarchy_membership() {
    let required = [IterationId(1), IterationId(2)];
    let hierarchy =
        HierarchyFixture::new(&required, vec![membership(1, 10, 1), membership(2, 20, 1)]).unwrap();
    let barrier = BarrierFixture::root(BarrierId::new(5), &required).unwrap();
    let before = barrier.before(IterationId(1)).unwrap();
    let after = barrier.after(IterationId(2)).unwrap();

    assert!(!hierarchy.same_group(IterationId(1), IterationId(2)));
    assert!(barrier.orders(before, after));
}
