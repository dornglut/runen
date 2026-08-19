use std::collections::BTreeMap;

use runen_exec_oracle::{
    AtomicExchange, AtomicExchangeError, AtomicExchangeScope, AtomicExchangeSemantics,
    AtomicValueToken, BufferAtomicExchangeError, BufferAtomicExchangeFixture, BufferAtomicLocation,
    BufferId, BufferRegion, LogicalBufferState, LogicalStateError, PositionId, ValueToken,
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
    assert!(location
        .region()
        .overlaps(&BufferRegion::new(BufferId(1), [PositionId(6), PositionId(7)])));
}

#[test]
fn atomic_exchange_reads_and_replaces_the_same_logical_buffer_value() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let fixture = BufferAtomicExchangeFixture::new(location, 100);
    let exchange = fixture.exchange_id(1);
    let mut state = logical_state(buffer);

    let realization = fixture
        .realize(
            &mut state,
            vec![AtomicExchange::new(
                exchange,
                AtomicValueToken::new(20),
                AtomicExchangeSemantics::Base,
                AtomicExchangeScope::Unscoped,
            )],
            &[exchange],
            &[],
        )
        .unwrap();

    assert_eq!(
        realization.prior_value(exchange),
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
    let fixture = BufferAtomicExchangeFixture::new(location, 100);
    let first = fixture.exchange_id(1);
    let second = fixture.exchange_id(2);
    let mut state = logical_state(buffer);

    let realization = fixture
        .realize(
            &mut state,
            vec![
                AtomicExchange::new(
                    first,
                    AtomicValueToken::new(20),
                    AtomicExchangeSemantics::Base,
                    AtomicExchangeScope::Unscoped,
                ),
                AtomicExchange::new(
                    second,
                    AtomicValueToken::new(30),
                    AtomicExchangeSemantics::Base,
                    AtomicExchangeScope::Unscoped,
                ),
            ],
            &[second, first],
            &[],
        )
        .unwrap();

    assert_eq!(
        realization.prior_value(second),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        realization.prior_value(first),
        Some(AtomicValueToken::new(30))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(20));
    assert_eq!(value_at(&state, location), ValueToken(20));
}

#[test]
fn foreign_atomic_location_is_rejected_without_mutating_buffer_state() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let foreign_location = BufferAtomicLocation::new(buffer, PositionId(2));
    let fixture = BufferAtomicExchangeFixture::new(location, 100);
    let foreign_fixture = BufferAtomicExchangeFixture::new(foreign_location, 200);
    let foreign_exchange = foreign_fixture.exchange_id(1);
    let mut state = logical_state(buffer);
    let before = state.clone();

    assert!(matches!(
        fixture.realize(
            &mut state,
            vec![AtomicExchange::new(
                foreign_exchange,
                AtomicValueToken::new(20),
                AtomicExchangeSemantics::Base,
                AtomicExchangeScope::Unscoped,
            )],
            &[foreign_exchange],
            &[],
        ),
        Err(BufferAtomicExchangeError::Atomic(
            AtomicExchangeError::ForeignExchangeIdentity
        ))
    ));
    assert_eq!(state, before);
}

#[test]
fn rejected_atomic_order_is_transactional_with_respect_to_buffer_state() {
    let buffer = BufferId(1);
    let location = BufferAtomicLocation::new(buffer, PositionId(1));
    let fixture = BufferAtomicExchangeFixture::new(location, 100);
    let first = fixture.exchange_id(1);
    let second = fixture.exchange_id(2);
    let mut state = logical_state(buffer);
    let before = state.clone();

    assert!(matches!(
        fixture.realize(
            &mut state,
            vec![
                AtomicExchange::new(
                    first,
                    AtomicValueToken::new(20),
                    AtomicExchangeSemantics::Base,
                    AtomicExchangeScope::Unscoped,
                ),
                AtomicExchange::new(
                    second,
                    AtomicValueToken::new(30),
                    AtomicExchangeSemantics::Base,
                    AtomicExchangeScope::Unscoped,
                ),
            ],
            &[second, first],
            &[(first, second)],
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

    let wrong_buffer = BufferAtomicExchangeFixture::new(
        BufferAtomicLocation::new(BufferId(2), PositionId(1)),
        100,
    );
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

    let unknown_position = BufferAtomicExchangeFixture::new(
        BufferAtomicLocation::new(buffer, PositionId(99)),
        200,
    );
    assert_eq!(
        unknown_position.realize(&mut state, vec![], &[], &[]),
        Err(BufferAtomicExchangeError::LogicalState(
            LogicalStateError::UnknownPosition(PositionId(99))
        ))
    );
    assert_eq!(state, before);
}
