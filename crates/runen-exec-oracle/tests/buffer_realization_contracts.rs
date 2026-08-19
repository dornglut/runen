use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, AllocationFixture, AllocationId, BufferId, BufferMappingFixture,
    BufferRegion, LogicalBufferState, PositionId, RealizationAgentId, ValueToken,
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
fn ordered_buffer_case_preserves_semantic_result_across_distinct_physical_arrangements() {
    let selected = region(1, &[0]);
    let initial = state(1, &[(0, 10)]);
    let mut single_state = initial.clone();
    let mut split_state = initial.clone();

    // Realization A uses one physical execution agent and one allocation for both
    // already-semantically-ordered accesses.
    let mut single_allocation = allocation(10, &[1]);
    let single_mapping = BufferMappingFixture::new(
        101,
        selected.clone(),
        agent(1),
        &mut single_allocation,
    )
    .unwrap();

    // Realization B uses distinct agents and allocations for the same semantic
    // state change and later read. The mapping occurrence tokens also differ.
    let mut split_write_allocation = allocation(20, &[2]);
    let mut split_read_allocation = allocation(30, &[3]);
    let split_write_mapping = BufferMappingFixture::new(
        201,
        selected.clone(),
        agent(2),
        &mut split_write_allocation,
    )
    .unwrap();
    let split_read_mapping = BufferMappingFixture::new(
        301,
        selected.clone(),
        agent(3),
        &mut split_read_allocation,
    )
    .unwrap();

    assert_ne!(single_mapping.agent(), split_write_mapping.agent());
    assert_ne!(single_mapping.agent(), split_read_mapping.agent());
    assert_ne!(split_write_mapping.agent(), split_read_mapping.agent());
    assert_ne!(
        single_mapping.id().allocation(),
        split_write_mapping.id().allocation()
    );
    assert_ne!(
        single_mapping.id().allocation(),
        split_read_mapping.id().allocation()
    );
    assert_ne!(
        split_write_mapping.id().allocation(),
        split_read_mapping.id().allocation()
    );
    assert_ne!(single_mapping.id(), split_write_mapping.id());
    assert_ne!(single_mapping.id(), split_read_mapping.id());

    // Call order here represents the same already-established semantic order in
    // each independent execution. Mapping or host call order is not the owner of
    // that ordering relation.
    single_mapping
        .mapped_change(&mut single_state, &selected, ValueToken(20))
        .unwrap();
    let single_read = single_mapping.mapped_read(&single_state, &selected).unwrap();

    // Running realization A must not mutate realization B's independent logical
    // execution state.
    assert_eq!(split_state, initial);

    split_write_mapping
        .mapped_change(&mut split_state, &selected, ValueToken(20))
        .unwrap();
    let split_read = split_read_mapping
        .mapped_read(&split_state, &selected)
        .unwrap();

    assert_eq!(single_read, values(&[(0, 20)]));
    assert_eq!(split_read, single_read);
    assert_eq!(split_state, single_state);
}

#[test]
fn physical_arrangement_does_not_change_ordinary_conflict_classification() {
    let selected = region(1, &[0]);
    let read = Access::new(AccessKind::Read, selected.clone());
    let change = Access::new(AccessKind::StateChange, selected.clone());

    // Both arrangements below are physically admissible mappings. Neither mapping
    // arrangement supplies semantic order or changes the ordinary conflict relation.
    let mut shared_allocation = allocation(10, &[1]);
    let _shared_mapping = BufferMappingFixture::new(
        101,
        selected.clone(),
        agent(1),
        &mut shared_allocation,
    )
    .unwrap();

    let mut split_read_allocation = allocation(20, &[2]);
    let mut split_change_allocation = allocation(30, &[3]);
    let _split_read_mapping = BufferMappingFixture::new(
        201,
        selected.clone(),
        agent(2),
        &mut split_read_allocation,
    )
    .unwrap();
    let _split_change_mapping = BufferMappingFixture::new(
        301,
        selected,
        agent(3),
        &mut split_change_allocation,
    )
    .unwrap();

    assert!(read.conflicts_with(&change));
    assert!(change.conflicts_with(&read));
}
