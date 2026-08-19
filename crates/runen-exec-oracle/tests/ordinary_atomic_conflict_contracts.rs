use runen_exec_oracle::{
    Access, AccessKind, AtomicExchangeScope, AtomicExchangeSemantics, AtomicValueToken,
    BufferAtomicExchangeFixture, BufferAtomicLocation, BufferId, BufferRegion, PositionId,
};

#[test]
fn overlapping_ordinary_read_and_atomic_exchange_conflict() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let read = Access::new(AccessKind::Read, location.region());

    assert!(read.conflicts_with_atomic_exchange_at(location));
}

#[test]
fn overlapping_ordinary_state_change_and_atomic_exchange_conflict() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let change = Access::new(AccessKind::StateChange, location.region());

    assert!(change.conflicts_with_atomic_exchange_at(location));
}

#[test]
fn disjoint_same_buffer_ordinary_accesses_do_not_conflict_with_atomic_location() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let read = Access::new(
        AccessKind::Read,
        BufferRegion::new(BufferId(1), [PositionId(8)]),
    );
    let change = Access::new(
        AccessKind::StateChange,
        BufferRegion::new(BufferId(1), [PositionId(9)]),
    );

    assert!(!read.conflicts_with_atomic_exchange_at(location));
    assert!(!change.conflicts_with_atomic_exchange_at(location));
}

#[test]
fn equal_position_token_under_distinct_buffer_is_disjoint_from_atomic_location() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let read = Access::new(
        AccessKind::Read,
        BufferRegion::new(BufferId(2), [PositionId(7)]),
    );

    assert!(!read.conflicts_with_atomic_exchange_at(location));
}

#[test]
fn mixed_conflict_depends_on_resource_overlap_not_atomic_class_or_scope() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let fixture = BufferAtomicExchangeFixture::new(location);
    let base = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let release = fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Release,
        AtomicExchangeScope::Unscoped,
    );
    let read = Access::new(AccessKind::Read, location.region());

    assert_eq!(base.id().location(), location);
    assert_eq!(release.id().location(), location);
    assert!(read.conflicts_with_atomic_exchange_at(base.id().location()));
    assert!(read.conflicts_with_atomic_exchange_at(release.id().location()));
}

#[test]
fn mixed_conflict_does_not_change_ordinary_conflict_classification() {
    let region = BufferRegion::new(BufferId(1), [PositionId(7)]);
    let read_a = Access::new(AccessKind::Read, region.clone());
    let read_b = Access::new(AccessKind::Read, region.clone());
    let change = Access::new(AccessKind::StateChange, region);

    assert!(!read_a.conflicts_with(&read_b));
    assert!(read_a.conflicts_with(&change));
}

#[test]
fn host_call_order_does_not_erase_source_unordered_mixed_conflict_classification() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let read = Access::new(AccessKind::Read, location.region());

    let first_observation = read.conflicts_with_atomic_exchange_at(location);
    let second_observation = read.conflicts_with_atomic_exchange_at(location);

    assert!(first_observation);
    assert!(second_observation);
}
