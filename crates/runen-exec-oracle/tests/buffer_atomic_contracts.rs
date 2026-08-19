use std::collections::BTreeMap;

use runen_exec_oracle::{
    AtomicExchangeError, AtomicExchangeScope, AtomicExchangeSemantics, AtomicValueToken,
    BufferAtomicExchangeError, BufferAtomicExchangeFixture, BufferAtomicLocation, BufferId,
    BufferRegion, LogicalBufferState, LogicalStateError, PositionId, ValueToken,
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

fn value_at(state: &LogicalBufferState, location: BufferAtomicLocation) -> ValueToken {
    *state
        .read(&location.region())
        .unwrap()
        .get(&location.position())
        .expect("singleton location region must contain its exact position")
}

#[test]
fn buffer_atomic_location_identity_is_exact_buffer_position_identity() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let same = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let other_position = BufferAtomicLocation::new(BufferId(1), PositionId(8));
    let other_buffer = BufferAtomicLocation::new(BufferId(2), PositionId(7));

    assert_eq!(location, same);
    assert_ne!(location, other_position);
    assert_ne!(location, other_buffer);
    assert_eq!(location.buffer(), BufferId(1));
    assert_eq!(location.position(), PositionId(7));

    assert!(location.region().overlaps(&same.region()));
    assert!(!location.region().overlaps(&other_position.region()));
    assert!(!location.region().overlaps(&other_buffer.region()));
    assert!(location.region().overlaps(&BufferRegion::new(
        BufferId(1),
        [PositionId(6), PositionId(7)]
    )));
}

#[test]
fn buffer_atomic_exchange_identity_is_structurally_scoped_by_location() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(7));
    let same_location = BufferAtomicExchangeFixture::new(location);
    let same_again = BufferAtomicExchangeFixture::new(location);
    let other_location =
        BufferAtomicExchangeFixture::new(BufferAtomicLocation::new(BufferId(1), PositionId(8)));

    let first = same_location
        .exchange(
            1,
            AtomicValueToken::new(20),
            AtomicExchangeSemantics::Base,
            AtomicExchangeScope::Unscoped,
        )
        .id();
    let same = same_again
        .exchange(
            1,
            AtomicValueToken::new(30),
            AtomicExchangeSemantics::Acquire,
            AtomicExchangeScope::Unscoped,
        )
        .id();
    let other = other_location
        .exchange(
            1,
            AtomicValueToken::new(20),
            AtomicExchangeSemantics::Base,
            AtomicExchangeScope::Unscoped,
        )
        .id();

    assert_eq!(first, same);
    assert_ne!(first, other);
    assert_eq!(first.location(), location);
    assert_eq!(
        other.location(),
        BufferAtomicLocation::new(BufferId(1), PositionId(8))
    );
}

#[test]
fn atomic_exchange_reads_and_replaces_the_same_logical_buffer_value() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let fixture = BufferAtomicExchangeFixture::new(location);
    let exchange = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let exchange_id = exchange.id();
    let mut state = logical_state(buffer);

    let realization = fixture
        .realize(&mut state, vec![exchange], &[exchange_id], &[])
        .unwrap();

    assert_eq!(realization.location(), location);
    assert_eq!(
        realization.prior_value(exchange_id),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(20));
    assert_eq!(value_at(&state, location), ValueToken(20));
    assert_eq!(
        state
            .read(&BufferRegion::new(buffer, [PositionId(2)]))
            .unwrap()
            .get(&PositionId(2)),
        Some(&ValueToken(11))
    );
}

#[test]
fn candidate_modification_order_commits_only_its_atomic_final_value_to_buffer_state() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let fixture = BufferAtomicExchangeFixture::new(location);
    let first = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let second = fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let first_id = first.id();
    let second_id = second.id();
    let mut state = logical_state(buffer);

    let realization = fixture
        .realize(&mut state, vec![first, second], &[second_id, first_id], &[])
        .unwrap();

    assert_eq!(
        realization.prior_value(second_id),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        realization.prior_value(first_id),
        Some(AtomicValueToken::new(30))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(20));
    assert_eq!(value_at(&state, location), ValueToken(20));
}

#[test]
fn foreign_buffer_atomic_location_is_rejected_without_mutating_state() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let foreign_location = BufferAtomicLocation::new(buffer, PositionId(2));
    let fixture = BufferAtomicExchangeFixture::new(location);
    let foreign_fixture = BufferAtomicExchangeFixture::new(foreign_location);
    let foreign_exchange = foreign_fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let foreign_id = foreign_exchange.id();
    let mut state = logical_state(buffer);
    let before = state.clone();

    assert!(matches!(
        fixture.realize(&mut state, vec![foreign_exchange], &[foreign_id], &[]),
        Err(BufferAtomicExchangeError::ForeignLocation {
            expected,
            actual
        }) if expected == location && actual == foreign_location
    ));
    assert_eq!(state, before);
}

#[test]
fn foreign_candidate_order_identity_is_rejected_without_mutating_state() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let foreign_location = BufferAtomicLocation::new(buffer, PositionId(2));
    let fixture = BufferAtomicExchangeFixture::new(location);
    let foreign_fixture = BufferAtomicExchangeFixture::new(foreign_location);
    let local_exchange = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let foreign_id = foreign_fixture
        .exchange(
            1,
            AtomicValueToken::new(30),
            AtomicExchangeSemantics::Base,
            AtomicExchangeScope::Unscoped,
        )
        .id();
    let mut state = logical_state(buffer);
    let before = state.clone();

    assert!(matches!(
        fixture.realize(&mut state, vec![local_exchange], &[foreign_id], &[]),
        Err(BufferAtomicExchangeError::ForeignLocation {
            expected,
            actual
        }) if expected == location && actual == foreign_location
    ));
    assert_eq!(state, before);
}

#[test]
fn foreign_order_constraint_identity_is_rejected_without_mutating_state() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let foreign_location = BufferAtomicLocation::new(buffer, PositionId(2));
    let fixture = BufferAtomicExchangeFixture::new(location);
    let foreign_fixture = BufferAtomicExchangeFixture::new(foreign_location);
    let first = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let second = fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let first_id = first.id();
    let second_id = second.id();
    let foreign_id = foreign_fixture
        .exchange(
            3,
            AtomicValueToken::new(40),
            AtomicExchangeSemantics::Base,
            AtomicExchangeScope::Unscoped,
        )
        .id();
    let mut state = logical_state(buffer);
    let before = state.clone();

    assert!(matches!(
        fixture.realize(
            &mut state,
            vec![first, second],
            &[first_id, second_id],
            &[(first_id, foreign_id)],
        ),
        Err(BufferAtomicExchangeError::ForeignLocation {
            expected,
            actual
        }) if expected == location && actual == foreign_location
    ));
    assert_eq!(state, before);
}

#[test]
fn rejected_atomic_order_is_transactional_with_respect_to_buffer_state() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let fixture = BufferAtomicExchangeFixture::new(location);
    let first = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let second = fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let first_id = first.id();
    let second_id = second.id();
    let mut state = logical_state(buffer);
    let before = state.clone();

    assert!(matches!(
        fixture.realize(
            &mut state,
            vec![first, second],
            &[second_id, first_id],
            &[(first_id, second_id)],
        ),
        Err(BufferAtomicExchangeError::Atomic(
            AtomicExchangeError::OrderConstraintViolation
        ))
    ));
    assert_eq!(state, before);
}

#[test]
fn wrong_buffer_and_unknown_position_are_rejected_before_atomic_realization() {
    let buffer = BufferId(1);
    let mut state = logical_state(buffer);
    let before = state.clone();

    let wrong_buffer =
        BufferAtomicExchangeFixture::new(BufferAtomicLocation::new(BufferId(2), PositionId(1)));
    assert!(matches!(
        wrong_buffer.realize(&mut state, vec![], &[], &[]),
        Err(BufferAtomicExchangeError::LogicalState(
            LogicalStateError::WrongBuffer {
                expected: BufferId(1),
                actual: BufferId(2)
            }
        ))
    ));
    assert_eq!(state, before);

    let unknown_position =
        BufferAtomicExchangeFixture::new(BufferAtomicLocation::new(buffer, PositionId(99)));
    assert!(matches!(
        unknown_position.realize(&mut state, vec![], &[], &[]),
        Err(BufferAtomicExchangeError::LogicalState(
            LogicalStateError::UnknownPosition(PositionId(99))
        ))
    ));
    assert_eq!(state, before);
}
