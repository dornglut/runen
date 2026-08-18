use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, BarrierError, BarrierFixture, BarrierId, BufferId, BufferRegion, GroupId,
    HierarchyFixture, HierarchyId, HierarchyMembership, IterationId, LogicalBufferState, PositionId,
    SubgroupId, ValueToken,
};

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

fn state(buffer: u32, entries: &[(u32, u32)]) -> LogicalBufferState {
    LogicalBufferState::new(
        BufferId(buffer),
        entries
            .iter()
            .copied()
            .map(|(position, value)| (PositionId(position), ValueToken(value)))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn hierarchy_id() -> HierarchyId {
    HierarchyId::new(1)
}

fn group(token: u32) -> GroupId {
    GroupId::new(hierarchy_id(), token)
}

fn membership(iteration: u32, group_token: u32, subgroup: u32) -> HierarchyMembership {
    HierarchyMembership::new(
        IterationId(iteration),
        SubgroupId::new(group(group_token), subgroup),
    )
}

fn hierarchy() -> HierarchyFixture {
    HierarchyFixture::new(
        hierarchy_id(),
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
    .unwrap()
}

#[test]
fn root_barrier_preserves_full_each_participation_and_empty_behavior() {
    let barrier = BarrierFixture::root(
        BarrierId::new(7),
        &[IterationId(1), IterationId(2), IterationId(3)],
    )
    .unwrap();

    assert!(barrier.before(IterationId(1)).is_some());
    assert!(barrier.before(IterationId(2)).is_some());
    assert!(barrier.before(IterationId(3)).is_some());
    assert!(barrier.before(IterationId(4)).is_none());

    let empty = BarrierFixture::root(BarrierId::new(8), &[]).unwrap();
    assert!(empty.has_exact_before_completion(&[]));
    assert!(empty.before(IterationId(1)).is_none());

    assert!(matches!(
        BarrierFixture::root(BarrierId::new(9), &[IterationId(1), IterationId(1)]),
        Err(BarrierError::InvalidRootIterations)
    ));
}

#[test]
fn group_barrier_selects_exact_existing_group() {
    let hierarchy = hierarchy();
    let barrier = BarrierFixture::group(BarrierId::new(7), &hierarchy, group(10)).unwrap();

    assert!(barrier.before(IterationId(1)).is_some());
    assert!(barrier.before(IterationId(2)).is_some());
    assert!(barrier.before(IterationId(3)).is_some());
    assert!(barrier.before(IterationId(4)).is_none());

    assert!(matches!(
        BarrierFixture::group(BarrierId::new(8), &hierarchy, group(99)),
        Err(BarrierError::UnknownGroup)
    ));
}

#[test]
fn subgroup_barrier_selects_exact_group_scoped_subgroup() {
    let hierarchy = hierarchy();
    let first_group = SubgroupId::new(group(10), 1);
    let second_group = SubgroupId::new(group(20), 1);
    let first = BarrierFixture::subgroup(BarrierId::new(7), &hierarchy, first_group).unwrap();
    let second = BarrierFixture::subgroup(BarrierId::new(8), &hierarchy, second_group).unwrap();

    assert_ne!(first_group, second_group);
    assert!(first.before(IterationId(1)).is_some());
    assert!(first.before(IterationId(2)).is_some());
    assert!(first.before(IterationId(3)).is_none());
    assert!(first.before(IterationId(4)).is_none());
    assert!(second.before(IterationId(4)).is_some());
    assert!(second.before(IterationId(1)).is_none());

    assert!(matches!(
        BarrierFixture::subgroup(BarrierId::new(9), &hierarchy, SubgroupId::new(group(10), 99)),
        Err(BarrierError::UnknownSubgroup)
    ));
}

#[test]
fn barrier_rejects_foreign_hierarchy_group_and_subgroup_selectors() {
    let hierarchy = hierarchy();
    let foreign_group = GroupId::new(HierarchyId::new(2), 10);
    let foreign_subgroup = SubgroupId::new(foreign_group, 1);

    assert!(matches!(
        BarrierFixture::group(BarrierId::new(7), &hierarchy, foreign_group),
        Err(BarrierError::UnknownGroup)
    ));
    assert!(matches!(
        BarrierFixture::subgroup(BarrierId::new(8), &hierarchy, foreign_subgroup),
        Err(BarrierError::UnknownSubgroup)
    ));
}

#[test]
fn barrier_requires_exact_participant_before_completion() {
    let hierarchy = hierarchy();
    let barrier = BarrierFixture::group(BarrierId::new(7), &hierarchy, group(10)).unwrap();

    assert!(barrier.has_exact_before_completion(&[IterationId(3), IterationId(1), IterationId(2)]));
    assert!(!barrier.has_exact_before_completion(&[IterationId(1), IterationId(2)]));
    assert!(!barrier.has_exact_before_completion(&[
        IterationId(1),
        IterationId(1),
        IterationId(3)
    ]));
    assert!(!barrier.has_exact_before_completion(&[
        IterationId(1),
        IterationId(2),
        IterationId(4)
    ]));
}

#[test]
fn barrier_orders_every_participant_before_every_participant_after() {
    let hierarchy = hierarchy();
    let barrier = BarrierFixture::subgroup(
        BarrierId::new(7),
        &hierarchy,
        SubgroupId::new(group(10), 1),
    )
    .unwrap();
    let before_first = barrier.before(IterationId(1)).unwrap();
    let before_second = barrier.before(IterationId(2)).unwrap();
    let after_first = barrier.after(IterationId(1)).unwrap();
    let after_second = barrier.after(IterationId(2)).unwrap();

    assert!(barrier.orders(before_first, after_first));
    assert!(barrier.orders(before_first, after_second));
    assert!(barrier.orders(before_second, after_first));
    assert!(barrier.orders(before_second, after_second));
}

#[test]
fn barrier_supplies_no_same_phase_or_cross_identity_order() {
    let required = [IterationId(1), IterationId(2)];
    let first = BarrierFixture::root(BarrierId::new(7), &required).unwrap();
    let second = BarrierFixture::root(BarrierId::new(8), &required).unwrap();
    let before_first = first.before(IterationId(1)).unwrap();
    let before_second = first.before(IterationId(2)).unwrap();
    let after_first = first.after(IterationId(1)).unwrap();
    let after_second = first.after(IterationId(2)).unwrap();
    let other_after = second.after(IterationId(2)).unwrap();

    assert!(!first.orders(before_first, before_second));
    assert!(!first.orders(before_second, before_first));
    assert!(!first.orders(after_first, after_second));
    assert!(!first.orders(after_second, after_first));
    assert!(!first.orders(before_first, other_after));
}

#[test]
fn nonparticipants_cannot_gain_barrier_phase_or_order() {
    let hierarchy = hierarchy();
    let barrier = BarrierFixture::subgroup(
        BarrierId::new(7),
        &hierarchy,
        SubgroupId::new(group(10), 1),
    )
    .unwrap();

    assert!(barrier.before(IterationId(3)).is_none());
    assert!(barrier.after(IterationId(3)).is_none());
    assert!(barrier.before(IterationId(4)).is_none());
    assert!(barrier.after(IterationId(4)).is_none());
}

#[test]
fn barrier_orders_conflicting_cross_phase_pair_without_erasing_conflict() {
    let selected = region(1, &[0]);
    let before_change = Access::new(AccessKind::StateChange, selected.clone());
    let after_read = Access::new(AccessKind::Read, selected);
    let barrier =
        BarrierFixture::root(BarrierId::new(7), &[IterationId(1), IterationId(2)]).unwrap();
    let before = barrier.before(IterationId(1)).unwrap();
    let after = barrier.after(IterationId(2)).unwrap();

    assert!(before_change.conflicts_with(&after_read));
    assert!(barrier.orders(before, after));
}

#[test]
fn same_phase_sibling_conflict_remains_unordered() {
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let barrier =
        BarrierFixture::root(BarrierId::new(7), &[IterationId(1), IterationId(2)]).unwrap();
    let first_before = barrier.before(IterationId(1)).unwrap();
    let second_before = barrier.before(IterationId(2)).unwrap();

    assert!(first_access.conflicts_with(&second_access));
    assert!(!barrier.orders(first_before, second_before));
    assert!(!barrier.orders(second_before, first_before));
}

#[test]
fn participant_barrier_order_composes_with_logical_buffer_state() {
    let hierarchy = hierarchy();
    let selected = region(1, &[0]);
    let barrier = BarrierFixture::group(BarrierId::new(7), &hierarchy, group(10)).unwrap();
    let before = barrier.before(IterationId(1)).unwrap();
    let after = barrier.after(IterationId(2)).unwrap();
    let mut logical = state(1, &[(0, 10)]);

    assert!(barrier.orders(before, after));
    logical.apply_change(&selected, ValueToken(20)).unwrap();

    assert_eq!(
        logical.read(&selected).unwrap(),
        BTreeMap::from([(PositionId(0), ValueToken(20))])
    );
}
