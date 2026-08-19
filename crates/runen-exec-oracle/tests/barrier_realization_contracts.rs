use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, AllocationFixture, AllocationId, BarrierFixture, BarrierId, BufferId,
    BufferMappingFixture, BufferRegion, EachId, GroupId, HierarchyFixture, HierarchyId,
    HierarchyMembership, IterationId, LogicalBufferState, PositionId, RealizationAgentId,
    SubgroupId, ValueToken,
};

fn agent(token: u32) -> RealizationAgentId {
    RealizationAgentId::new(token)
}

fn allocation(token: u32, accessible_agents: &[u32]) -> AllocationFixture {
    AllocationFixture::new(
        AllocationId::new(token),
        accessible_agents.iter().copied().map(agent),
    )
}

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

fn values(entries: &[(u32, u32)]) -> BTreeMap<PositionId, ValueToken> {
    entries
        .iter()
        .copied()
        .map(|(position, value)| (PositionId(position), ValueToken(value)))
        .collect()
}

fn state(buffer: u32, entries: &[(u32, u32)]) -> LogicalBufferState {
    LogicalBufferState::new(BufferId(buffer), values(entries))
}

fn hierarchy_id(each: EachId) -> HierarchyId {
    HierarchyId::new(each, 1)
}

fn group_id(each: EachId, token: u32) -> GroupId {
    GroupId::new(hierarchy_id(each), token)
}

fn subgroup_id(each: EachId, group: u32, subgroup: u32) -> SubgroupId {
    SubgroupId::new(group_id(each, group), subgroup)
}

fn membership(each: EachId, iteration: u32, group: u32, subgroup: u32) -> HierarchyMembership {
    HierarchyMembership::new(
        IterationId::new(each, iteration),
        subgroup_id(each, group, subgroup),
    )
}

fn hierarchy(each: EachId) -> HierarchyFixture {
    let required = [
        IterationId::new(each, 1),
        IterationId::new(each, 2),
        IterationId::new(each, 3),
        IterationId::new(each, 4),
    ];
    HierarchyFixture::new(
        hierarchy_id(each),
        &required,
        vec![
            membership(each, 1, 10, 1),
            membership(each, 2, 10, 1),
            membership(each, 3, 10, 2),
            membership(each, 4, 20, 1),
        ],
    )
    .unwrap()
}

fn assert_barrier_visibility_is_preserved_across_physical_arrangements(
    barrier: &BarrierFixture,
    producer: IterationId,
    consumer: IterationId,
    participants: &[IterationId],
    nonparticipant: IterationId,
) {
    let before = barrier.before(producer).unwrap();
    let after = barrier.after(consumer).unwrap();
    let selected = region(1, &[0]);
    let before_change = Access::new(AccessKind::StateChange, selected.clone());
    let after_read = Access::new(AccessKind::Read, selected.clone());

    assert!(before_change.conflicts_with(&after_read));
    assert!(barrier.orders(before, after));
    assert!(barrier.has_exact_before_completion(participants));

    let mut reversed_participants = participants.to_vec();
    reversed_participants.reverse();
    assert!(barrier.has_exact_before_completion(&reversed_participants));
    assert!(barrier.before(nonparticipant).is_none());
    assert!(barrier.after(nonparticipant).is_none());

    let initial = state(1, &[(0, 10)]);
    let mut single_state = initial.clone();
    let mut split_state = initial.clone();

    // Physical arrangement A services both already-barrier-ordered accesses through
    // one execution agent and one allocation. Mapping and Rust call order do not
    // create the barrier relation established above.
    let mut single_allocation = allocation(10, &[1]);
    let single_mapping =
        BufferMappingFixture::new(101, selected.clone(), agent(1), &mut single_allocation).unwrap();

    single_mapping
        .mapped_change(&mut single_state, &selected, ValueToken(20))
        .unwrap();
    let single_read = single_mapping
        .mapped_read(&single_state, &selected)
        .unwrap();

    assert_eq!(split_state, initial);

    // Physical arrangement B services the same semantic before-change and after-read
    // through distinct execution agents and allocations. The semantic visibility
    // still comes from the same barrier relation, not from placement or mapping.
    let mut split_before_allocation = allocation(20, &[2]);
    let mut split_after_allocation = allocation(30, &[3]);
    let split_before_mapping = BufferMappingFixture::new(
        201,
        selected.clone(),
        agent(2),
        &mut split_before_allocation,
    )
    .unwrap();
    let split_after_mapping =
        BufferMappingFixture::new(301, selected.clone(), agent(3), &mut split_after_allocation)
            .unwrap();

    assert_ne!(single_mapping.agent(), split_before_mapping.agent());
    assert_ne!(single_mapping.agent(), split_after_mapping.agent());
    assert_ne!(split_before_mapping.agent(), split_after_mapping.agent());
    assert_ne!(
        single_mapping.id().allocation(),
        split_before_mapping.id().allocation()
    );
    assert_ne!(
        split_before_mapping.id().allocation(),
        split_after_mapping.id().allocation()
    );

    split_before_mapping
        .mapped_change(&mut split_state, &selected, ValueToken(20))
        .unwrap();
    let split_read = split_after_mapping
        .mapped_read(&split_state, &selected)
        .unwrap();

    assert_eq!(single_read, values(&[(0, 20)]));
    assert_eq!(split_read, single_read);
    assert_eq!(split_state, single_state);
}

#[test]
fn barrier_ordered_buffer_visibility_is_preserved_across_physical_arrangements() {
    let each = EachId::new(1);
    let producer = IterationId::new(each, 1);
    let consumer = IterationId::new(each, 2);
    let barrier = BarrierFixture::root(BarrierId::new(1), each, &[producer, consumer]).unwrap();

    assert_barrier_visibility_is_preserved_across_physical_arrangements(
        &barrier,
        producer,
        consumer,
        &[producer, consumer],
        IterationId::new(each, 3),
    );
}

#[test]
fn group_barrier_visibility_is_preserved_across_physical_arrangements() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each);
    let producer = IterationId::new(each, 1);
    let consumer = IterationId::new(each, 3);
    let nonparticipant = IterationId::new(each, 4);
    let participants = [producer, IterationId::new(each, 2), consumer];
    let barrier = BarrierFixture::group(BarrierId::new(2), &hierarchy, group_id(each, 10)).unwrap();

    // The selected group is semantic hierarchy structure. The distinct physical
    // arrangements below do not claim that this group is a hardware work-group.
    assert_barrier_visibility_is_preserved_across_physical_arrangements(
        &barrier,
        producer,
        consumer,
        &participants,
        nonparticipant,
    );
}

#[test]
fn subgroup_barrier_visibility_is_preserved_across_physical_arrangements() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each);
    let producer = IterationId::new(each, 1);
    let consumer = IterationId::new(each, 2);
    let nonparticipant = IterationId::new(each, 3);
    let participants = [producer, consumer];
    let barrier =
        BarrierFixture::subgroup(BarrierId::new(3), &hierarchy, subgroup_id(each, 10, 1)).unwrap();

    // The nonparticipant deliberately belongs to the same semantic group but a
    // different subgroup. Narrower cohort selection, not physical placement,
    // determines barrier participation.
    assert_barrier_visibility_is_preserved_across_physical_arrangements(
        &barrier,
        producer,
        consumer,
        &participants,
        nonparticipant,
    );
}

#[test]
fn physical_serialization_does_not_order_same_barrier_phase_conflicts() {
    let each = EachId::new(1);
    let first = IterationId::new(each, 1);
    let second = IterationId::new(each, 2);
    let barrier = BarrierFixture::root(BarrierId::new(1), each, &[first, second]).unwrap();
    let first_before = barrier.before(first).unwrap();
    let second_before = barrier.before(second).unwrap();
    let selected = region(1, &[0]);
    let first_change = Access::new(AccessKind::StateChange, selected.clone());
    let second_change = Access::new(AccessKind::StateChange, selected.clone());

    // One physical mapping could service both operations serially. The illegal
    // same-phase pair is deliberately not executed because that physical ability
    // supplies neither semantic order nor a conflict exemption.
    let mut shared_allocation = allocation(10, &[1]);
    let _shared_mapping =
        BufferMappingFixture::new(101, selected, agent(1), &mut shared_allocation).unwrap();

    assert!(first_change.conflicts_with(&second_change));
    assert!(!barrier.orders(first_before, second_before));
    assert!(!barrier.orders(second_before, first_before));
}
