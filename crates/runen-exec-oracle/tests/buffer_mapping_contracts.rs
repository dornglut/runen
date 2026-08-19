use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, AllocationError, AllocationFixture, AllocationId, BufferId,
    BufferMappingError, BufferMappingFixture, BufferRegion, LogicalBufferState, PositionId,
    RealizationAgentId, ValueToken,
};

fn agent(token: u32) -> RealizationAgentId {
    RealizationAgentId::new(token)
}

fn allocation_fixture(token: u32, accessible_agents: &[u32]) -> AllocationFixture {
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
fn allocation_extent_cannot_end_while_mapping_is_active() {
    let mut allocation = allocation_fixture(7, &[1]);
    let mut mapping =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();

    assert_eq!(
        allocation.end_extent(),
        Err(AllocationError::ActiveMappings)
    );
    assert!(allocation.is_live());

    mapping.end(&mut allocation).unwrap();
    assert!(!mapping.is_active());
    assert_eq!(allocation.end_extent(), Ok(()));
    assert!(!allocation.is_live());

    assert!(matches!(
        BufferMappingFixture::new(2, region(1, &[0]), agent(1), &mut allocation),
        Err(BufferMappingError::Allocation(AllocationError::Ended))
    ));
}

#[test]
fn mapping_identity_is_scoped_by_exact_allocation() {
    let mut first_allocation = allocation_fixture(7, &[1]);
    let mut second_allocation = allocation_fixture(8, &[1]);
    let first =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut first_allocation).unwrap();
    let second =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut second_allocation).unwrap();

    assert_ne!(first.id(), second.id());
    assert_eq!(first.id().allocation(), AllocationId::new(7));
    assert_eq!(second.id().allocation(), AllocationId::new(8));
    assert_eq!(first.agent(), agent(1));
    assert_eq!(second.agent(), agent(1));
    assert_eq!(first.region(), second.region());
}

#[test]
fn duplicate_mapping_identity_is_rejected_within_one_allocation() {
    let mut allocation = allocation_fixture(7, &[1]);
    let _first =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();

    assert!(matches!(
        BufferMappingFixture::new(1, region(1, &[1]), agent(1), &mut allocation),
        Err(BufferMappingError::Allocation(
            AllocationError::DuplicateMappingIdentity
        ))
    ));
}

#[test]
fn ended_mapping_identity_cannot_be_reused_for_another_occurrence() {
    let mut allocation = allocation_fixture(7, &[1]);
    let mut first =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();
    let first_id = first.id();

    first.end(&mut allocation).unwrap();

    assert!(matches!(
        BufferMappingFixture::new(1, region(1, &[1]), agent(1), &mut allocation),
        Err(BufferMappingError::Allocation(
            AllocationError::DuplicateMappingIdentity
        ))
    ));
    assert_eq!(first.id(), first_id);
}

#[test]
fn inaccessible_agent_is_rejected_before_mapping_identity_is_consumed() {
    let mut allocation = allocation_fixture(7, &[1]);

    assert_eq!(
        BufferMappingFixture::new(1, region(1, &[0]), agent(2), &mut allocation).err(),
        Some(BufferMappingError::Allocation(
            AllocationError::AgentNotAccessible(agent(2))
        ))
    );

    let admitted =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();
    assert_eq!(admitted.agent(), agent(1));
}

#[test]
fn one_allocation_may_admit_multiple_distinct_agents() {
    let selected = region(1, &[0]);
    let mut allocation = allocation_fixture(7, &[1, 2]);
    let first =
        BufferMappingFixture::new(1, selected.clone(), agent(1), &mut allocation).unwrap();
    let second =
        BufferMappingFixture::new(2, selected.clone(), agent(2), &mut allocation).unwrap();
    let mut logical = state(1, &[(0, 10)]);

    first
        .mapped_change(&mut logical, &selected, ValueToken(20))
        .unwrap();

    assert_eq!(first.agent(), agent(1));
    assert_eq!(second.agent(), agent(2));
    assert_eq!(
        second.mapped_read(&logical, &selected).unwrap(),
        values(&[(0, 20)])
    );
}

#[test]
fn distinct_allocations_may_differ_in_accessibility_to_same_agent() {
    let selected = region(1, &[0]);
    let mut first_allocation = allocation_fixture(7, &[1]);
    let mut second_allocation = allocation_fixture(8, &[2]);

    let first = BufferMappingFixture::new(
        1,
        selected.clone(),
        agent(1),
        &mut first_allocation,
    )
    .unwrap();
    assert_eq!(first.agent(), agent(1));

    assert_eq!(
        BufferMappingFixture::new(1, selected.clone(), agent(1), &mut second_allocation).err(),
        Some(BufferMappingError::Allocation(
            AllocationError::AgentNotAccessible(agent(1))
        ))
    );

    let second = BufferMappingFixture::new(1, selected, agent(2), &mut second_allocation).unwrap();
    assert_eq!(second.agent(), agent(2));
}

#[test]
fn mapping_end_requires_its_exact_allocation_and_disables_access() {
    let mut allocation = allocation_fixture(7, &[1]);
    let mut foreign = allocation_fixture(8, &[1]);
    let selected = region(1, &[0]);
    let mut mapping =
        BufferMappingFixture::new(1, selected.clone(), agent(1), &mut allocation).unwrap();
    let logical = state(1, &[(0, 10)]);

    assert_eq!(
        mapping.end(&mut foreign),
        Err(BufferMappingError::ForeignAllocation {
            expected: AllocationId::new(7),
            actual: AllocationId::new(8),
        })
    );
    assert!(mapping.is_active());
    assert_eq!(mapping.agent(), agent(1));
    assert_eq!(
        mapping.mapped_read(&logical, &selected).unwrap(),
        values(&[(0, 10)])
    );

    mapping.end(&mut allocation).unwrap();
    assert_eq!(
        mapping.mapped_read(&logical, &selected),
        Err(BufferMappingError::Inactive)
    );
    assert_eq!(
        mapping.end(&mut allocation),
        Err(BufferMappingError::Inactive)
    );
}

#[test]
fn mapping_is_region_local_not_whole_buffer_accessibility() {
    let mut allocation = allocation_fixture(7, &[1]);
    let mapped = region(1, &[0]);
    let disjoint = region(1, &[1]);
    let mapping =
        BufferMappingFixture::new(1, mapped.clone(), agent(1), &mut allocation).unwrap();
    let logical = state(1, &[(0, 10), (1, 11)]);

    assert_eq!(
        mapping.mapped_read(&logical, &mapped).unwrap(),
        values(&[(0, 10)])
    );
    assert_eq!(
        mapping.mapped_read(&logical, &disjoint),
        Err(BufferMappingError::OutsideMappedRegion)
    );
}

#[test]
fn mapped_typed_access_reuses_one_logical_buffer_state() {
    let mut allocation = allocation_fixture(7, &[1]);
    let mapped = region(1, &[0, 1]);
    let changed = region(1, &[0]);
    let untouched = region(1, &[1]);
    let mapping = BufferMappingFixture::new(1, mapped, agent(1), &mut allocation).unwrap();
    let mut logical = state(1, &[(0, 10), (1, 11)]);
    let stale_snapshot = logical.clone();

    mapping
        .mapped_change(&mut logical, &changed, ValueToken(20))
        .unwrap();

    assert_eq!(
        mapping.mapped_read(&logical, &changed).unwrap(),
        values(&[(0, 20)])
    );
    assert_eq!(
        mapping.mapped_read(&logical, &untouched).unwrap(),
        values(&[(1, 11)])
    );
    assert_eq!(stale_snapshot.read(&changed).unwrap(), values(&[(0, 10)]));
}

#[test]
fn remapping_same_logical_region_across_agents_and_allocations_preserves_current_state() {
    let selected = region(1, &[0]);
    let mut first_allocation = allocation_fixture(7, &[1]);
    let mut second_allocation = allocation_fixture(8, &[2]);
    let first = BufferMappingFixture::new(
        1,
        selected.clone(),
        agent(1),
        &mut first_allocation,
    )
    .unwrap();
    let second = BufferMappingFixture::new(
        1,
        selected.clone(),
        agent(2),
        &mut second_allocation,
    )
    .unwrap();
    let mut logical = state(1, &[(0, 10)]);

    first
        .mapped_change(&mut logical, &selected, ValueToken(20))
        .unwrap();

    assert_eq!(
        second.mapped_read(&logical, &selected).unwrap(),
        values(&[(0, 20)])
    );
    assert_eq!(first.region(), second.region());
    assert_ne!(first.id().allocation(), second.id().allocation());
    assert_ne!(first.agent(), second.agent());
}

#[test]
fn invalid_mapped_change_is_rejected_before_logical_state_mutation() {
    let mut allocation = allocation_fixture(7, &[1]);
    let mapping =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();
    let mut logical = state(1, &[(0, 10), (1, 11)]);
    let original = logical.clone();

    assert_eq!(
        mapping.mapped_change(&mut logical, &region(1, &[1]), ValueToken(20)),
        Err(BufferMappingError::OutsideMappedRegion)
    );
    assert_eq!(logical, original);
}

#[test]
fn mapping_does_not_change_ordinary_conflict_legality() {
    let mut allocation = allocation_fixture(7, &[1]);
    let selected = region(1, &[0]);
    let _mapping =
        BufferMappingFixture::new(1, selected.clone(), agent(1), &mut allocation).unwrap();
    let read = Access::new(AccessKind::Read, selected.clone());
    let change = Access::new(AccessKind::StateChange, selected.clone());
    let second_change = Access::new(AccessKind::StateChange, selected);

    assert!(read.conflicts_with(&change));
    assert!(change.conflicts_with(&second_change));
}

#[test]
fn mapping_end_does_not_mutate_logical_buffer_state() {
    let mut allocation = allocation_fixture(7, &[1]);
    let selected = region(1, &[0]);
    let mut mapping =
        BufferMappingFixture::new(1, selected, agent(1), &mut allocation).unwrap();
    let logical = state(1, &[(0, 10)]);
    let original = logical.clone();

    mapping.end(&mut allocation).unwrap();

    assert_eq!(logical, original);
}

#[test]
fn equal_private_numeric_tokens_do_not_merge_identity_domains() {
    let mut allocation = allocation_fixture(1, &[1]);
    let mapping =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();

    assert_eq!(mapping.id().allocation(), AllocationId::new(1));
    assert_eq!(mapping.region().buffer(), BufferId(1));
    assert_eq!(mapping.agent(), RealizationAgentId::new(1));
    assert_eq!(allocation.id(), AllocationId::new(1));
}
