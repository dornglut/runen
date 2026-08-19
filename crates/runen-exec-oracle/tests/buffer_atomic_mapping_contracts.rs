use std::collections::BTreeMap;

use runen_exec_oracle::{
    AllocationFixture, AllocationId, AtomicExchangeError, AtomicExchangeScope,
    AtomicExchangeSemantics, AtomicValueToken, BufferAtomicExchangeError,
    BufferAtomicExchangeFixture, BufferAtomicLocation, BufferId, BufferMappingError,
    BufferMappingFixture, BufferRegion, LogicalBufferState, PositionId, RealizationAgentId,
    ValueToken,
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

#[test]
fn active_mapping_can_service_admitted_atomic_exchange_inside_its_region() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut allocation = allocation(7, &[1]);
    let mapping =
        BufferMappingFixture::new(1, region(1, &[0, 1]), agent(1), &mut allocation).unwrap();
    let fixture = BufferAtomicExchangeFixture::new(location);
    let exchange = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let exchange_id = exchange.id();
    let mut logical = state(1, &[(0, 10), (1, 11)]);

    let realization = mapping
        .mapped_atomic_exchange(&mut logical, fixture, vec![exchange], &[exchange_id], &[])
        .unwrap();

    assert_eq!(realization.location(), location);
    assert_eq!(
        realization.prior_value(exchange_id),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(20));
    assert_eq!(
        logical.read(&region(1, &[0])).unwrap().get(&PositionId(0)),
        Some(&ValueToken(20))
    );
    assert_eq!(
        logical.read(&region(1, &[1])).unwrap().get(&PositionId(1)),
        Some(&ValueToken(11))
    );
    assert_eq!(mapping.agent(), agent(1));
    assert_eq!(mapping.id().allocation(), AllocationId::new(7));
}

#[test]
fn mapping_outside_atomic_location_rejects_before_logical_state_changes() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut allocation = allocation(7, &[1]);
    let mapping = BufferMappingFixture::new(1, region(1, &[1]), agent(1), &mut allocation).unwrap();
    let fixture = BufferAtomicExchangeFixture::new(location);
    let exchange = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let exchange_id = exchange.id();
    let mut logical = state(1, &[(0, 10), (1, 11)]);
    let before = logical.clone();

    assert!(matches!(
        mapping.mapped_atomic_exchange(
            &mut logical,
            fixture,
            vec![exchange],
            &[exchange_id],
            &[],
        ),
        Err(BufferMappingError::OutsideMappedRegion)
    ));
    assert_eq!(logical, before);
}

#[test]
fn equal_position_under_distinct_buffer_is_outside_mapping() {
    let location = BufferAtomicLocation::new(BufferId(2), PositionId(0));
    let mut allocation = allocation(7, &[1]);
    let mapping = BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();
    let fixture = BufferAtomicExchangeFixture::new(location);
    let exchange = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let exchange_id = exchange.id();
    let mut logical = state(1, &[(0, 10)]);
    let before = logical.clone();

    assert!(matches!(
        mapping.mapped_atomic_exchange(
            &mut logical,
            fixture,
            vec![exchange],
            &[exchange_id],
            &[],
        ),
        Err(BufferMappingError::OutsideMappedRegion)
    ));
    assert_eq!(logical, before);
}

#[test]
fn inactive_mapping_rejects_atomic_servicing_before_logical_state_changes() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut allocation = allocation(7, &[1]);
    let mut mapping =
        BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();
    mapping.end(&mut allocation).unwrap();

    let fixture = BufferAtomicExchangeFixture::new(location);
    let exchange = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let exchange_id = exchange.id();
    let mut logical = state(1, &[(0, 10)]);
    let before = logical.clone();

    assert!(matches!(
        mapping.mapped_atomic_exchange(
            &mut logical,
            fixture,
            vec![exchange],
            &[exchange_id],
            &[],
        ),
        Err(BufferMappingError::Inactive)
    ));
    assert_eq!(logical, before);
}

#[test]
fn rejected_atomic_realization_through_valid_mapping_is_transactional() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut allocation = allocation(7, &[1]);
    let mapping = BufferMappingFixture::new(1, region(1, &[0]), agent(1), &mut allocation).unwrap();
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
    let mut logical = state(1, &[(0, 10)]);
    let before = logical.clone();

    assert!(matches!(
        mapping.mapped_atomic_exchange(
            &mut logical,
            fixture,
            vec![first, second],
            &[second_id, first_id],
            &[(first_id, second_id)],
        ),
        Err(BufferMappingError::Atomic(BufferAtomicExchangeError::Atomic(
            AtomicExchangeError::OrderConstraintViolation
        )))
    ));
    assert_eq!(logical, before);
}

#[test]
fn physical_mapping_choice_does_not_change_atomic_modification_order_result() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let initial = state(1, &[(0, 10)]);
    let mut first_state = initial.clone();
    let mut second_state = initial;

    let mut first_allocation = allocation(7, &[1]);
    let mut second_allocation = allocation(8, &[2]);
    let first_mapping =
        BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut first_allocation).unwrap();
    let second_mapping =
        BufferMappingFixture::new(22, region(1, &[0]), agent(2), &mut second_allocation).unwrap();

    assert_ne!(first_mapping.agent(), second_mapping.agent());
    assert_ne!(
        first_mapping.id().allocation(),
        second_mapping.id().allocation()
    );
    assert_ne!(first_mapping.id(), second_mapping.id());

    let first_fixture = BufferAtomicExchangeFixture::new(location);
    let first_a = first_fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Release,
        AtomicExchangeScope::Unscoped,
    );
    let first_b = first_fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Acquire,
        AtomicExchangeScope::Unscoped,
    );
    let first_a_id = first_a.id();
    let first_b_id = first_b.id();

    let second_fixture = BufferAtomicExchangeFixture::new(location);
    let second_a = second_fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Release,
        AtomicExchangeScope::Unscoped,
    );
    let second_b = second_fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Acquire,
        AtomicExchangeScope::Unscoped,
    );
    let second_a_id = second_a.id();
    let second_b_id = second_b.id();

    let first = first_mapping
        .mapped_atomic_exchange(
            &mut first_state,
            first_fixture,
            vec![first_a, first_b],
            &[first_a_id, first_b_id],
            &[],
        )
        .unwrap();
    let second = second_mapping
        .mapped_atomic_exchange(
            &mut second_state,
            second_fixture,
            vec![second_a, second_b],
            &[second_a_id, second_b_id],
            &[],
        )
        .unwrap();

    assert_eq!(first.prior_value(first_a_id), second.prior_value(second_a_id));
    assert_eq!(first.prior_value(first_b_id), second.prior_value(second_b_id));
    assert_eq!(first.final_value(), second.final_value());
    assert_eq!(first_state, second_state);
    assert_eq!(first.final_value(), AtomicValueToken::new(30));
}
