use std::collections::BTreeMap;

use runen_exec_oracle::{
    AllocationFixture, AllocationId, AtomicExchangeError, AtomicExchangeScope,
    AtomicExchangeSemantics, AtomicValueToken, BufferAtomicExchangeError,
    BufferAtomicExchangeFixture, BufferAtomicLocation, BufferId, BufferMappingError,
    BufferMappingFixture, BufferRegion, EachId, IterationId, LogicalBufferState, PositionId,
    RealizationAgentId, ValueToken,
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
fn distinct_exchanges_may_select_distinct_mappings_without_splitting_modification_order() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut first_allocation = allocation(7, &[1]);
    let mut second_allocation = allocation(8, &[2]);
    let first_mapping =
        BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut first_allocation).unwrap();
    let second_mapping =
        BufferMappingFixture::new(22, region(1, &[0]), agent(2), &mut second_allocation).unwrap();

    let fixture = BufferAtomicExchangeFixture::new(location);
    let release = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Release,
        AtomicExchangeScope::Unscoped,
    );
    let acquire_root = fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Acquire,
        AtomicExchangeScope::Root(IterationId::new(EachId::new(9), 1)),
    );
    let release_id = release.id();
    let acquire_id = acquire_root.id();
    let assignments = [
        (release_id, &first_mapping),
        (acquire_id, &second_mapping),
    ];
    let mut logical = state(1, &[(0, 10)]);

    let realization = fixture
        .realize_mapped_by_exchange(
            &mut logical,
            vec![release, acquire_root],
            &assignments,
            &[release_id, acquire_id],
            &[],
        )
        .unwrap();

    assert_ne!(first_mapping.agent(), second_mapping.agent());
    assert_ne!(
        first_mapping.id().allocation(),
        second_mapping.id().allocation()
    );
    assert_eq!(
        realization.prior_value(release_id),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        realization.prior_value(acquire_id),
        Some(AtomicValueToken::new(20))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(30));
    assert_eq!(
        logical.read(&region(1, &[0])).unwrap().get(&PositionId(0)),
        Some(&ValueToken(30))
    );
}

#[test]
fn one_mapping_may_service_multiple_exchange_occurrences() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut allocation = allocation(7, &[1]);
    let mapping = BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut allocation).unwrap();
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
    let assignments = [(first_id, &mapping), (second_id, &mapping)];
    let mut logical = state(1, &[(0, 10)]);

    let realization = fixture
        .realize_mapped_by_exchange(
            &mut logical,
            vec![first, second],
            &assignments,
            &[first_id, second_id],
            &[],
        )
        .unwrap();

    assert_eq!(
        realization.prior_value(first_id),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        realization.prior_value(second_id),
        Some(AtomicValueToken::new(20))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(30));
}

#[test]
fn assignment_storage_order_does_not_change_atomic_result() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut first_allocation = allocation(7, &[1]);
    let mut second_allocation = allocation(8, &[2]);
    let first_mapping =
        BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut first_allocation).unwrap();
    let second_mapping =
        BufferMappingFixture::new(22, region(1, &[0]), agent(2), &mut second_allocation).unwrap();
    let mut first_state = state(1, &[(0, 10)]);
    let mut second_state = first_state.clone();

    let first_fixture = BufferAtomicExchangeFixture::new(location);
    let first_a = first_fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let first_b = first_fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let first_a_id = first_a.id();
    let first_b_id = first_b.id();
    let first_assignments = [
        (first_a_id, &first_mapping),
        (first_b_id, &second_mapping),
    ];

    let second_fixture = BufferAtomicExchangeFixture::new(location);
    let second_a = second_fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let second_b = second_fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let second_a_id = second_a.id();
    let second_b_id = second_b.id();
    let reversed_assignments = [
        (second_b_id, &second_mapping),
        (second_a_id, &first_mapping),
    ];

    let first = first_fixture
        .realize_mapped_by_exchange(
            &mut first_state,
            vec![first_a, first_b],
            &first_assignments,
            &[first_a_id, first_b_id],
            &[],
        )
        .unwrap();
    let second = second_fixture
        .realize_mapped_by_exchange(
            &mut second_state,
            vec![second_a, second_b],
            &reversed_assignments,
            &[second_a_id, second_b_id],
            &[],
        )
        .unwrap();

    assert_eq!(first.prior_value(first_a_id), second.prior_value(second_a_id));
    assert_eq!(first.prior_value(first_b_id), second.prior_value(second_b_id));
    assert_eq!(first.final_value(), second.final_value());
    assert_eq!(first_state, second_state);
}

#[test]
fn invalid_assignment_coverage_rejects_without_mutating_logical_state() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut allocation = allocation(7, &[1]);
    let mapping = BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut allocation).unwrap();

    for mode in 0..3 {
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
        let invented = fixture
            .exchange(
                3,
                AtomicValueToken::new(40),
                AtomicExchangeSemantics::Base,
                AtomicExchangeScope::Unscoped,
            )
            .id();
        let first_id = first.id();
        let second_id = second.id();
        let assignments = match mode {
            0 => vec![(first_id, &mapping)],
            1 => vec![(first_id, &mapping), (first_id, &mapping)],
            _ => vec![(first_id, &mapping), (invented, &mapping)],
        };
        let mut logical = state(1, &[(0, 10)]);
        let before = logical.clone();

        assert!(matches!(
            fixture.realize_mapped_by_exchange(
                &mut logical,
                vec![first, second],
                &assignments,
                &[first_id, second_id],
                &[],
            ),
            Err(BufferMappingError::AtomicAssignmentCoverage)
        ));
        assert_eq!(logical, before);
    }
}

#[test]
fn foreign_location_assignment_set_rejects_without_mutating_logical_state() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let foreign_location = BufferAtomicLocation::new(BufferId(1), PositionId(1));
    let mut allocation = allocation(7, &[1]);
    let mapping =
        BufferMappingFixture::new(11, region(1, &[0, 1]), agent(1), &mut allocation).unwrap();
    let fixture = BufferAtomicExchangeFixture::new(location);
    let local = fixture.exchange(
        1,
        AtomicValueToken::new(20),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let foreign_fixture = BufferAtomicExchangeFixture::new(foreign_location);
    let foreign = foreign_fixture.exchange(
        2,
        AtomicValueToken::new(30),
        AtomicExchangeSemantics::Base,
        AtomicExchangeScope::Unscoped,
    );
    let local_id = local.id();
    let foreign_id = foreign.id();
    let assignments = [(local_id, &mapping), (foreign_id, &mapping)];
    let mut logical = state(1, &[(0, 10), (1, 11)]);
    let before = logical.clone();

    assert!(matches!(
        fixture.realize_mapped_by_exchange(
            &mut logical,
            vec![local, foreign],
            &assignments,
            &[local_id, foreign_id],
            &[],
        ),
        Err(BufferMappingError::Atomic(
            BufferAtomicExchangeError::ForeignLocation { expected, actual }
        )) if expected == location && actual == foreign_location
    ));
    assert_eq!(logical, before);
}

#[test]
fn invalid_selected_mapping_rejects_before_atomic_state_observation() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut active_allocation = allocation(7, &[1]);
    let mut inactive_allocation = allocation(8, &[2]);
    let active_mapping =
        BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut active_allocation).unwrap();
    let mut inactive_mapping =
        BufferMappingFixture::new(22, region(1, &[0]), agent(2), &mut inactive_allocation).unwrap();
    inactive_mapping.end(&mut inactive_allocation).unwrap();

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
    let assignments = [
        (first_id, &active_mapping),
        (second_id, &inactive_mapping),
    ];
    let mut logical = state(1, &[(0, 10)]);
    let before = logical.clone();

    assert!(matches!(
        fixture.realize_mapped_by_exchange(
            &mut logical,
            vec![first, second],
            &assignments,
            &[first_id, second_id],
            &[],
        ),
        Err(BufferMappingError::Inactive)
    ));
    assert_eq!(logical, before);
}

#[test]
fn outside_region_selected_mapping_rejects_before_atomic_state_observation() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut first_allocation = allocation(7, &[1]);
    let mut second_allocation = allocation(8, &[2]);
    let first_mapping =
        BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut first_allocation).unwrap();
    let outside_mapping =
        BufferMappingFixture::new(22, region(1, &[1]), agent(2), &mut second_allocation).unwrap();
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
    let assignments = [
        (first_id, &first_mapping),
        (second_id, &outside_mapping),
    ];
    let mut logical = state(1, &[(0, 10), (1, 11)]);
    let before = logical.clone();

    assert!(matches!(
        fixture.realize_mapped_by_exchange(
            &mut logical,
            vec![first, second],
            &assignments,
            &[first_id, second_id],
            &[],
        ),
        Err(BufferMappingError::OutsideMappedRegion)
    ));
    assert_eq!(logical, before);
}

#[test]
fn valid_assignments_with_invalid_atomic_order_remain_transactional() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut first_allocation = allocation(7, &[1]);
    let mut second_allocation = allocation(8, &[2]);
    let first_mapping =
        BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut first_allocation).unwrap();
    let second_mapping =
        BufferMappingFixture::new(22, region(1, &[0]), agent(2), &mut second_allocation).unwrap();
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
    let assignments = [
        (first_id, &first_mapping),
        (second_id, &second_mapping),
    ];
    let mut logical = state(1, &[(0, 10)]);
    let before = logical.clone();

    assert!(matches!(
        fixture.realize_mapped_by_exchange(
            &mut logical,
            vec![first, second],
            &assignments,
            &[second_id, first_id],
            &[(first_id, second_id)],
        ),
        Err(BufferMappingError::Atomic(
            BufferAtomicExchangeError::Atomic(AtomicExchangeError::OrderConstraintViolation)
        ))
    ));
    assert_eq!(logical, before);
}

#[test]
fn different_valid_physical_assignments_preserve_the_same_atomic_result() {
    let location = BufferAtomicLocation::new(BufferId(1), PositionId(0));
    let mut first_allocation = allocation(7, &[1]);
    let mut second_allocation = allocation(8, &[2]);
    let first_mapping =
        BufferMappingFixture::new(11, region(1, &[0]), agent(1), &mut first_allocation).unwrap();
    let second_mapping =
        BufferMappingFixture::new(22, region(1, &[0]), agent(2), &mut second_allocation).unwrap();
    let mut first_state = state(1, &[(0, 10)]);
    let mut second_state = first_state.clone();

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
        AtomicExchangeScope::Root(IterationId::new(EachId::new(9), 1)),
    );
    let first_a_id = first_a.id();
    let first_b_id = first_b.id();
    let first_assignments = [
        (first_a_id, &first_mapping),
        (first_b_id, &second_mapping),
    ];

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
        AtomicExchangeScope::Root(IterationId::new(EachId::new(9), 1)),
    );
    let second_a_id = second_a.id();
    let second_b_id = second_b.id();
    let swapped_assignments = [
        (second_a_id, &second_mapping),
        (second_b_id, &first_mapping),
    ];

    let first = first_fixture
        .realize_mapped_by_exchange(
            &mut first_state,
            vec![first_a, first_b],
            &first_assignments,
            &[first_a_id, first_b_id],
            &[],
        )
        .unwrap();
    let second = second_fixture
        .realize_mapped_by_exchange(
            &mut second_state,
            vec![second_a, second_b],
            &swapped_assignments,
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
