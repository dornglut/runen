use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, AtomicExchangeScope, AtomicExchangeSemantics, AtomicValueToken,
    BufferAtomicExchangeFixture, BufferAtomicLocation, BufferId, LogicalBufferState, PositionId,
    ValueToken,
};

fn logical_state(buffer: BufferId) -> LogicalBufferState {
    LogicalBufferState::new(
        buffer,
        BTreeMap::from([
            (PositionId(1), ValueToken(10)),
            (PositionId(2), ValueToken(11)),
        ]),
    )
}

#[test]
fn ordered_ordinary_change_before_exchange_supplies_exchange_prior_value() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let region = location.region();
    let ordinary_change = Access::new(AccessKind::StateChange, region.clone());
    let mut state = logical_state(buffer);

    assert!(ordinary_change.conflicts_with_atomic_exchange_at(location));

    // Fixture call order stands for semantic order already established by an
    // applicable contract; it is not host-call-order authority for Runen order.
    state.apply_change(&region, ValueToken(15)).unwrap();

    let fixture = BufferAtomicExchangeFixture::new(location);
    let exchange = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let exchange_id = exchange.id();
    let realization = fixture
        .realize(&mut state, vec![exchange], &[exchange_id], &[])
        .unwrap();

    assert_eq!(
        realization.prior_value(exchange_id),
        Some(AtomicValueToken::new(15))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(20));
    assert_eq!(
        state.read(&region).unwrap().get(&PositionId(1)),
        Some(&ValueToken(20))
    );
    assert_eq!(
        state
            .read(&BufferAtomicLocation::new(buffer, PositionId(2)).region())
            .unwrap()
            .get(&PositionId(2)),
        Some(&ValueToken(11))
    );
}

#[test]
fn ordered_exchange_before_ordinary_read_supplies_exchange_final_value() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let region = location.region();
    let ordinary_read = Access::new(AccessKind::Read, region.clone());
    let mut state = logical_state(buffer);

    assert!(ordinary_read.conflicts_with_atomic_exchange_at(location));

    let fixture = BufferAtomicExchangeFixture::new(location);
    let exchange = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let exchange_id = exchange.id();

    // Fixture call order stands for semantic order already established by an
    // applicable contract; it is not host-call-order authority for Runen order.
    let realization = fixture
        .realize(&mut state, vec![exchange], &[exchange_id], &[])
        .unwrap();
    let observed = state.read(&region).unwrap();

    assert_eq!(realization.final_value(), AtomicValueToken::new(20));
    assert_eq!(observed.get(&PositionId(1)), Some(&ValueToken(20)));
}
