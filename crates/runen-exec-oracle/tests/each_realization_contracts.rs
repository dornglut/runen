use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, AllocationFixture, AllocationId, BufferId, BufferMappingFixture,
    BufferRegion, EachId, EachPhase, IterationId, LogicalBufferState, PositionId,
    RealizationAgentId, ValueToken, each_orders,
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

#[test]
fn disjoint_each_case_preserves_continuation_result_across_physical_schedules() {
    let whole = region(1, &[0, 1]);
    let left = region(1, &[0]);
    let right = region(1, &[1]);
    let left_access = Access::new(AccessKind::StateChange, left.clone());
    let right_access = Access::new(AccessKind::StateChange, right.clone());
    let each = EachId::new(1);
    let left_iteration = EachPhase::Iteration(IterationId::new(each, 1));
    let right_iteration = EachPhase::Iteration(IterationId::new(each, 2));
    let continuation = EachPhase::After(each);

    assert!(!left_access.conflicts_with(&right_access));
    assert!(!each_orders(left_iteration, right_iteration));
    assert!(!each_orders(right_iteration, left_iteration));
    assert!(each_orders(left_iteration, continuation));
    assert!(each_orders(right_iteration, continuation));

    let initial = state(1, &[(0, 0), (1, 0)]);
    let mut single_state = initial.clone();
    let mut split_state = initial.clone();

    // Physical arrangement A services both sibling changes and the continuation
    // read through one agent/allocation mapping. The left-then-right call order is
    // realization order only; the sibling iterations remain semantically unordered.
    let mut single_allocation = allocation(10, &[1]);
    let single_mapping =
        BufferMappingFixture::new(101, whole.clone(), agent(1), &mut single_allocation).unwrap();

    single_mapping
        .mapped_change(&mut single_state, &left, ValueToken(10))
        .unwrap();
    single_mapping
        .mapped_change(&mut single_state, &right, ValueToken(20))
        .unwrap();
    let single_read = single_mapping.mapped_read(&single_state, &whole).unwrap();

    // Running arrangement A does not mutate arrangement B's independent logical
    // execution state.
    assert_eq!(split_state, initial);

    // Physical arrangement B services the sibling changes through distinct agents
    // and allocations, then services the normal-continuation read through a third
    // legal mapping. The right-then-left call order is the opposite physical
    // schedule and still creates no semantic sibling order.
    let mut split_left_allocation = allocation(20, &[2]);
    let mut split_right_allocation = allocation(30, &[3]);
    let mut split_continuation_allocation = allocation(40, &[4]);
    let split_left_mapping =
        BufferMappingFixture::new(201, left.clone(), agent(2), &mut split_left_allocation).unwrap();
    let split_right_mapping =
        BufferMappingFixture::new(301, right.clone(), agent(3), &mut split_right_allocation)
            .unwrap();
    let split_continuation_mapping = BufferMappingFixture::new(
        401,
        whole.clone(),
        agent(4),
        &mut split_continuation_allocation,
    )
    .unwrap();

    assert_ne!(single_mapping.agent(), split_left_mapping.agent());
    assert_ne!(single_mapping.agent(), split_right_mapping.agent());
    assert_ne!(split_left_mapping.agent(), split_right_mapping.agent());
    assert_ne!(
        single_mapping.id().allocation(),
        split_left_mapping.id().allocation()
    );
    assert_ne!(
        split_left_mapping.id().allocation(),
        split_right_mapping.id().allocation()
    );

    split_right_mapping
        .mapped_change(&mut split_state, &right, ValueToken(20))
        .unwrap();
    split_left_mapping
        .mapped_change(&mut split_state, &left, ValueToken(10))
        .unwrap();
    let split_read = split_continuation_mapping
        .mapped_read(&split_state, &whole)
        .unwrap();

    assert_eq!(single_read, values(&[(0, 10), (1, 20)]));
    assert_eq!(split_read, single_read);
    assert_eq!(split_state, single_state);
}

#[test]
fn physical_serialization_does_not_legalize_overlapping_each_siblings() {
    let selected = region(1, &[0]);
    let first_access = Access::new(AccessKind::StateChange, selected.clone());
    let second_access = Access::new(AccessKind::StateChange, selected.clone());
    let each = EachId::new(1);
    let first_iteration = EachPhase::Iteration(IterationId::new(each, 1));
    let second_iteration = EachPhase::Iteration(IterationId::new(each, 2));

    // A single physical mapping could service both operations serially, but that
    // possibility does not create semantic sibling order or legalize the conflict.
    let mut shared_allocation = allocation(10, &[1]);
    let _shared_mapping =
        BufferMappingFixture::new(101, selected, agent(1), &mut shared_allocation).unwrap();

    assert!(first_access.conflicts_with(&second_access));
    assert!(!each_orders(first_iteration, second_iteration));
    assert!(!each_orders(second_iteration, first_iteration));
}
