use runen_exec_oracle::{
    Access, AccessKind, BarrierId, BarrierPhase, BufferId, BufferRegion, EachPhase, GroupId,
    HierarchyError, HierarchyFixture, HierarchyMembership, IterationId, PositionId, SubgroupId,
    barrier_orders, each_orders,
};

fn membership(iteration: u32, group: u32, subgroup: u32) -> HierarchyMembership {
    HierarchyMembership::new(
        IterationId(iteration),
        GroupId::new(group),
        SubgroupId::new(subgroup),
    )
}

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

#[test]
fn hierarchy_requires_exact_iteration_coverage_and_accepts_empty_each() {
    assert_eq!(HierarchyFixture::new(&[], vec![]), Ok(HierarchyFixture::new(&[], vec![]).unwrap()));

    let required = [IterationId(1), IterationId(2), IterationId(3)];
    let valid = vec![membership(3, 2, 1), membership(1, 1, 1), membership(2, 1, 2)];
    assert!(HierarchyFixture::new(&required, valid).is_ok());

    assert_eq!(
        HierarchyFixture::new(&required, vec![membership(1, 1, 1), membership(2, 1, 2)]),
        Err(HierarchyError::InvalidIterationCoverage)
    );
    assert_eq!(
        HierarchyFixture::new(
            &required,
            vec![membership(1, 1, 1), membership(2, 1, 2), membership(4, 2, 1)]
        ),
        Err(HierarchyError::InvalidIterationCoverage)
    );
    assert_eq!(
        HierarchyFixture::new(
            &required,
            vec![membership(1, 1, 1), membership(1, 2, 1), membership(3, 2, 1)]
        ),
        Err(HierarchyError::InvalidIterationCoverage)
    );
    assert_eq!(
        HierarchyFixture::new(
            &[IterationId(1), IterationId(1)],
            vec![membership(1, 1, 1), membership(1, 1, 1)]
        ),
        Err(HierarchyError::InvalidIterationCoverage)
    );
    assert_eq!(
        HierarchyFixture::new(&[], vec![membership(1, 1, 1)]),
        Err(HierarchyError::InvalidIterationCoverage)
    );
}

#[test]
fn groups_and_subgroups_form_nested_partitions_of_required_iterations() {
    let hierarchy = HierarchyFixture::new(
        &[IterationId(1), IterationId(2), IterationId(3), IterationId(4)],
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

    assert_eq!(
        hierarchy.membership(IterationId(3)),
        Some(membership(3, 10, 2))
    );
    assert_eq!(hierarchy.membership(IterationId(9)), None);
}

#[test]
fn subgroup_token_is_scoped_by_containing_group() {
    let hierarchy = HierarchyFixture::new(
        &[IterationId(1), IterationId(2)],
        vec![membership(1, 10, 7), membership(2, 20, 7)],
    )
    .unwrap();

    assert!(!hierarchy.same_group(IterationId(1), IterationId(2)));
    assert!(!hierarchy.same_subgroup(IterationId(1), IterationId(2)));
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
fn full_each_barrier_order_is_independent_of_hierarchy_membership() {
    let hierarchy = HierarchyFixture::new(
        &[IterationId(1), IterationId(2)],
        vec![membership(1, 10, 1), membership(2, 20, 1)],
    )
    .unwrap();
    let barrier = BarrierId(5);

    assert!(!hierarchy.same_group(IterationId(1), IterationId(2)));
    assert!(barrier_orders(
        BarrierPhase::Before {
            barrier,
            iteration: IterationId(1),
        },
        BarrierPhase::After {
            barrier,
            iteration: IterationId(2),
        }
    ));
}
