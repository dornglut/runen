use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, AllocationError, AllocationFixture, AllocationId, BufferId,
    BufferMappingError, BufferMappingFixture, BufferRegion, LogicalBufferState, PositionId,
    ValueToken,
};

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
    let mut allocation = AllocationFixture::new(AllocationId(7));
    let mut mapping = BufferMappingFixture::new(1, region(1, &[0]), &mut allocation).unwrap();

    assert_eq!(allocation.end_extent(), Err(AllocationError::ActiveMappings));
    assert!(allocation.is_live());

    mapping.end(&mut allocation).unwrap();
    assert!(!mapping.is_active());
    assert_eq!(allocation.end_extent(), Ok(()));
    assert!(!allocation.is_live());

    assert!(matches!(
        BufferMappingFixture::new(2, region(1, &[0]), &mut allocation),
        Err(BufferMappingError::Allocation(AllocationError::Ended))
    ));
}

#[test]
fn mapping_identity_is_scoped_by_exact_allocation() {
    let mut first_allocation = AllocationFixture::new(AllocationId(7));
    let mut second_allocation = AllocationFixture::new(AllocationId(8));
    let first = BufferMappingFixture::new(1, region(1, &[0]), &mut first_allocation).unwrap();
    let second = BufferMappingFixture::new(1, region(1, &[0]), &mut second_allocation).unwrap();

    assert_ne!(first.id(), second.id());
    assert_eq!(first.id().allocation(), AllocationId(7));
    assert_eq!(second.id().allocation(), AllocationId(8));
    assert_eq!(first.region(), second.region());
}

#[test]
fn duplicate_mapping_identity_is_rejected_within_one_allocation() {
    let mut allocation = AllocationFixture::new(AllocationId(7));
    let _first = BufferMappingFixture::new(1, region(1, &[0]), &mut allocation).unwrap();

    assert!(matches!(
        BufferMappingFixture::new(1, region(1, &[1]), &mut allocation),
        Err(BufferMappingError::Allocation(
            AllocationError::MappingIdentityInUse
        ))
    ));
}

#[test]
fn mapping_end_requires_its_exact_allocation_and_disables_access() {
    let mut allocation = AllocationFixture::new(AllocationId(7));
    let mut foreign = AllocationFixture::new(AllocationId(8));
    let selected = region(1, &[0]);
    let mut mapping = BufferMappingFixture::new(1, selected.clone(), &mut allocation).unwrap();
    let logical = state(1, &[(0, 10)]);

    assert_eq!(
        mapping.end(&mut foreign),
        Err(BufferMappingError::ForeignAllocation {
            expected: AllocationId(7),
            actual: AllocationId(8),
        })
    );
    assert!(mapping.is_active());
    assert_eq!(mapping.mapped_read(&logical, &selected).unwrap(), values(&[(0, 10)]));

    mapping.end(&mut allocation).unwrap();
    assert_eq!(
        mapping.mapped_read(&logical, &selected),
        Err(BufferMappingError::Inactive)
    );
    assert_eq!(mapping.end(&mut allocation), Err(BufferMappingError::Inactive));
}

#[test]
fn mapping_is_region_local_not_whole_buffer_accessibility() {
    let mut allocation = AllocationFixture::new(AllocationId(7));
    let mapped = region(1, &[0]);
    let disjoint = region(1, &[1]);
    let mapping = BufferMappingFixture::new(1, mapped.clone(), &mut allocation).unwrap();
    let logical = state(1, &[(0, 10), (1, 11)]);

    assert_eq!(mapping.mapped_read(&logical, &mapped).unwrap(), values(&[(0, 10)]));
    assert_eq!(
        mapping.mapped_read(&logical, &disjoint),
        Err(BufferMappingError::OutsideMappedRegion)
    );
}

#[test]
fn mapped_typed_access_reuses_one_logical_buffer_state() {
    let mut allocation = AllocationFixture::new(AllocationId(7));
    let mapped = region(1, &[0, 1]);
    let changed = region(1, &[0]);
    let untouched = region(1, &[1]);
    let mapping = BufferMappingFixture::new(1, mapped, &mut allocation).unwrap();
    let mut logical = state(1, &[(0, 10), (1, 11)]);
    let stale_snapshot = logical.clone();

    mapping
        .mapped_change(&mut logical, &changed, ValueToken(20))
        .unwrap();

    assert_eq!(mapping.mapped_read(&logical, &changed).unwrap(), values(&[(0, 20)]));
    assert_eq!(mapping.mapped_read(&logical, &untouched).unwrap(), values(&[(1, 11)]));
    assert_eq!(stale_snapshot.read(&changed).unwrap(), values(&[(0, 10)]));
}

#[test]
fn remapping_same_logical_region_to_another_allocation_preserves_current_state() {
    let selected = region(1, &[0]);
    let mut first_allocation = AllocationFixture::new(AllocationId(7));
    let mut second_allocation = AllocationFixture::new(AllocationId(8));
    let first = BufferMappingFixture::new(1, selected.clone(), &mut first_allocation).unwrap();
    let second = BufferMappingFixture::new(1, selected.clone(), &mut second_allocation).unwrap();
    let mut logical = state(1, &[(0, 10)]);

    first
        .mapped_change(&mut logical, &selected, ValueToken(20))
        .unwrap();

    assert_eq!(second.mapped_read(&logical, &selected).unwrap(), values(&[(0, 20)]));
    assert_eq!(first.region(), second.region());
    assert_ne!(first.id().allocation(), second.id().allocation());
}

#[test]
fn invalid_mapped_change_is_rejected_before_logical_state_mutation() {
    let mut allocation = AllocationFixture::new(AllocationId(7));
    let mapping = BufferMappingFixture::new(1, region(1, &[0]), &mut allocation).unwrap();
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
    let mut allocation = AllocationFixture::new(AllocationId(7));
    let selected = region(1, &[0]);
    let _mapping = BufferMappingFixture::new(1, selected.clone(), &mut allocation).unwrap();
    let read = Access::new(AccessKind::Read, selected.clone());
    let change = Access::new(AccessKind::StateChange, selected.clone());
    let second_change = Access::new(AccessKind::StateChange, selected);

    assert!(read.conflicts_with(&change));
    assert!(change.conflicts_with(&second_change));
}

#[test]
fn mapping_end_does_not_mutate_logical_buffer_state() {
    let mut allocation = AllocationFixture::new(AllocationId(7));
    let selected = region(1, &[0]);
    let mut mapping = BufferMappingFixture::new(1, selected, &mut allocation).unwrap();
    let logical = state(1, &[(0, 10)]);
    let original = logical.clone();

    mapping.end(&mut allocation).unwrap();

    assert_eq!(logical, original);
}

#[test]
fn equal_private_numeric_tokens_do_not_merge_allocation_and_buffer_identity() {
    let mut allocation = AllocationFixture::new(AllocationId(1));
    let mapping = BufferMappingFixture::new(1, region(1, &[0]), &mut allocation).unwrap();

    assert_eq!(mapping.id().allocation(), AllocationId(1));
    assert_eq!(mapping.region().buffer(), BufferId(1));
    assert_eq!(allocation.id(), AllocationId(1));
}
