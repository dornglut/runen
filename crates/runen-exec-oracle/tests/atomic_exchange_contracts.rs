use runen_exec_oracle::{
    Access, AccessKind, AtomicExchange, AtomicExchangeError, AtomicExchangeFixture,
    AtomicExchangeId, AtomicExchangeSemantics, AtomicLocationId, AtomicValueToken, BufferId,
    BufferRegion, EachPhase, IterationId, PositionId, each_orders,
};

fn location(token: u32) -> AtomicLocationId {
    AtomicLocationId::new(token)
}

fn exchange_id(location: AtomicLocationId, token: u32) -> AtomicExchangeId {
    AtomicExchangeId::new(location, token)
}

fn exchange(
    location: AtomicLocationId,
    token: u32,
    desired: u32,
    semantics: AtomicExchangeSemantics,
) -> AtomicExchange {
    AtomicExchange::new(
        exchange_id(location, token),
        AtomicValueToken::new(desired),
        semantics,
    )
}

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

#[test]
fn empty_exchange_set_preserves_initial_value() {
    let fixture =
        AtomicExchangeFixture::new(location(1), AtomicValueToken::new(10), vec![]).unwrap();
    let realization = fixture.realize(&[], &[]).unwrap();

    assert_eq!(realization.final_value(), AtomicValueToken::new(10));
}

#[test]
fn one_base_exchange_returns_initial_value_and_installs_desired_value() {
    let location = location(1);
    let first = exchange_id(location, 1);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![exchange(
            location,
            1,
            20,
            AtomicExchangeSemantics::Base,
        )],
    )
    .unwrap();
    let realization = fixture.realize(&[first], &[]).unwrap();

    assert_eq!(
        realization.prior_value(first),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(realization.immediate_predecessor(first), None);
    assert_eq!(realization.final_value(), AtomicValueToken::new(20));
    assert!(!realization.release_acquire_synchronizes(first, first));
}

#[test]
fn unordered_base_exchanges_admit_both_location_local_linearizations() {
    let location = location(1);
    let first = exchange_id(location, 1);
    let second = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            exchange(location, 2, 30, AtomicExchangeSemantics::Base),
            exchange(location, 1, 20, AtomicExchangeSemantics::Base),
        ],
    )
    .unwrap();

    let first_then_second = fixture.realize(&[first, second], &[]).unwrap();
    assert_eq!(
        first_then_second.prior_value(first),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        first_then_second.prior_value(second),
        Some(AtomicValueToken::new(20))
    );
    assert_eq!(first_then_second.immediate_predecessor(first), None);
    assert_eq!(
        first_then_second.immediate_predecessor(second),
        Some(first)
    );
    assert_eq!(first_then_second.final_value(), AtomicValueToken::new(30));
    assert!(!first_then_second.release_acquire_synchronizes(first, second));

    let second_then_first = fixture.realize(&[second, first], &[]).unwrap();
    assert_eq!(
        second_then_first.prior_value(second),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        second_then_first.prior_value(first),
        Some(AtomicValueToken::new(30))
    );
    assert_eq!(second_then_first.immediate_predecessor(second), None);
    assert_eq!(
        second_then_first.immediate_predecessor(first),
        Some(second)
    );
    assert_eq!(second_then_first.final_value(), AtomicValueToken::new(20));
    assert!(!second_then_first.release_acquire_synchronizes(second, first));
}

#[test]
fn equal_desired_values_do_not_collapse_exchange_occurrences() {
    let location = location(1);
    let first = exchange_id(location, 1);
    let second = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            exchange(location, 1, 20, AtomicExchangeSemantics::Base),
            exchange(location, 2, 20, AtomicExchangeSemantics::Base),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[second, first], &[]).unwrap();

    assert_ne!(first, second);
    assert_eq!(
        realization.prior_value(second),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        realization.prior_value(first),
        Some(AtomicValueToken::new(20))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(20));
}

#[test]
fn fixture_rejects_duplicate_and_foreign_exchange_identities() {
    let local = location(1);
    let duplicate = exchange_id(local, 1);

    assert!(matches!(
        AtomicExchangeFixture::new(
            local,
            AtomicValueToken::new(10),
            vec![
                AtomicExchange::new(
                    duplicate,
                    AtomicValueToken::new(20),
                    AtomicExchangeSemantics::Base
                ),
                AtomicExchange::new(
                    duplicate,
                    AtomicValueToken::new(30),
                    AtomicExchangeSemantics::Release
                ),
            ]
        ),
        Err(AtomicExchangeError::DuplicateExchangeIdentity)
    ));

    let foreign = location(2);
    assert!(matches!(
        AtomicExchangeFixture::new(
            local,
            AtomicValueToken::new(10),
            vec![exchange(
                foreign,
                1,
                20,
                AtomicExchangeSemantics::Acquire
            )]
        ),
        Err(AtomicExchangeError::ForeignExchangeIdentity)
    ));
}

#[test]
fn modification_order_requires_exact_local_exchange_coverage() {
    let local = location(1);
    let first = exchange_id(local, 1);
    let second = exchange_id(local, 2);
    let fixture = AtomicExchangeFixture::new(
        local,
        AtomicValueToken::new(10),
        vec![
            exchange(local, 1, 20, AtomicExchangeSemantics::Base),
            exchange(local, 2, 30, AtomicExchangeSemantics::Base),
        ],
    )
    .unwrap();

    assert!(matches!(
        fixture.realize(&[first], &[]),
        Err(AtomicExchangeError::InvalidModificationOrderCoverage)
    ));
    assert!(matches!(
        fixture.realize(&[first, first], &[]),
        Err(AtomicExchangeError::InvalidModificationOrderCoverage)
    ));
    assert!(matches!(
        fixture.realize(&[first, exchange_id(local, 99)], &[]),
        Err(AtomicExchangeError::InvalidModificationOrderCoverage)
    ));
    assert!(matches!(
        fixture.realize(&[first, exchange_id(location(2), 2)], &[]),
        Err(AtomicExchangeError::InvalidModificationOrderCoverage)
    ));
    assert!(fixture.realize(&[second, first], &[]).is_ok());
}

#[test]
fn semantic_order_constraint_must_be_local_represented_and_respected() {
    let local = location(1);
    let first = exchange_id(local, 1);
    let second = exchange_id(local, 2);
    let fixture = AtomicExchangeFixture::new(
        local,
        AtomicValueToken::new(10),
        vec![
            exchange(local, 1, 20, AtomicExchangeSemantics::Base),
            exchange(local, 2, 30, AtomicExchangeSemantics::Base),
        ],
    )
    .unwrap();

    assert!(matches!(
        fixture.realize(&[second, first], &[(first, second)]),
        Err(AtomicExchangeError::OrderConstraintViolation)
    ));

    let realized = fixture
        .realize(&[first, second], &[(first, second)])
        .unwrap();
    assert_eq!(realized.prior_value(first), Some(AtomicValueToken::new(10)));
    assert_eq!(
        realized.prior_value(second),
        Some(AtomicValueToken::new(20))
    );
    assert_eq!(realized.final_value(), AtomicValueToken::new(30));

    assert!(matches!(
        fixture.realize(&[first, second], &[(first, exchange_id(local, 99))]),
        Err(AtomicExchangeError::InvalidOrderConstraint)
    ));
    assert!(matches!(
        fixture.realize(&[first, second], &[(first, exchange_id(location(2), 2))]),
        Err(AtomicExchangeError::InvalidOrderConstraint)
    ));
    assert!(matches!(
        fixture.realize(&[first, second], &[(first, first)]),
        Err(AtomicExchangeError::InvalidOrderConstraint)
    ));
}

#[test]
fn distinct_atomic_locations_have_independent_exchange_orders() {
    let first_location = location(1);
    let second_location = location(2);
    let first = exchange_id(first_location, 1);
    let second = exchange_id(second_location, 1);
    let first_fixture = AtomicExchangeFixture::new(
        first_location,
        AtomicValueToken::new(10),
        vec![exchange(
            first_location,
            1,
            20,
            AtomicExchangeSemantics::Base,
        )],
    )
    .unwrap();
    let second_fixture = AtomicExchangeFixture::new(
        second_location,
        AtomicValueToken::new(30),
        vec![exchange(
            second_location,
            1,
            40,
            AtomicExchangeSemantics::Base,
        )],
    )
    .unwrap();

    let first_realized = first_fixture.realize(&[first], &[]).unwrap();
    let second_realized = second_fixture.realize(&[second], &[]).unwrap();

    assert_ne!(first, second);
    assert_eq!(
        first_realized.prior_value(first),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        second_realized.prior_value(second),
        Some(AtomicValueToken::new(30))
    );
    assert_eq!(first_realized.final_value(), AtomicValueToken::new(20));
    assert_eq!(second_realized.final_value(), AtomicValueToken::new(40));
    assert!(matches!(
        first_fixture.realize(&[first], &[(first, second)]),
        Err(AtomicExchangeError::InvalidOrderConstraint)
    ));
    assert!(!first_realized.release_acquire_synchronizes(first, second));
}

#[test]
fn direct_release_predecessor_synchronizes_with_acquire() {
    let local = location(1);
    let release = exchange_id(local, 1);
    let acquire = exchange_id(local, 2);
    let fixture = AtomicExchangeFixture::new(
        local,
        AtomicValueToken::new(10),
        vec![
            exchange(local, 1, 20, AtomicExchangeSemantics::Release),
            exchange(local, 2, 30, AtomicExchangeSemantics::Acquire),
        ],
    )
    .unwrap();

    let release_then_acquire = fixture.realize(&[release, acquire], &[]).unwrap();
    assert_eq!(
        release_then_acquire.immediate_predecessor(acquire),
        Some(release)
    );
    assert!(release_then_acquire.release_acquire_synchronizes(release, acquire));

    let acquire_then_release = fixture.realize(&[acquire, release], &[]).unwrap();
    assert_eq!(
        acquire_then_release.immediate_predecessor(acquire),
        None
    );
    assert!(!acquire_then_release.release_acquire_synchronizes(release, acquire));
}

#[test]
fn synchronization_requires_release_and_acquire_classes() {
    let local = location(1);
    let first = exchange_id(local, 1);
    let second = exchange_id(local, 2);

    let base_then_acquire = AtomicExchangeFixture::new(
        local,
        AtomicValueToken::new(10),
        vec![
            exchange(local, 1, 20, AtomicExchangeSemantics::Base),
            exchange(local, 2, 30, AtomicExchangeSemantics::Acquire),
        ],
    )
    .unwrap()
    .realize(&[first, second], &[])
    .unwrap();
    assert!(!base_then_acquire.release_acquire_synchronizes(first, second));

    let release_then_base = AtomicExchangeFixture::new(
        local,
        AtomicValueToken::new(10),
        vec![
            exchange(local, 1, 20, AtomicExchangeSemantics::Release),
            exchange(local, 2, 30, AtomicExchangeSemantics::Base),
        ],
    )
    .unwrap()
    .realize(&[first, second], &[])
    .unwrap();
    assert!(!release_then_base.release_acquire_synchronizes(first, second));
}

#[test]
fn intervening_exchange_blocks_direct_release_acquire_synchronization() {
    let local = location(1);
    let release = exchange_id(local, 1);
    let middle = exchange_id(local, 2);
    let acquire = exchange_id(local, 3);
    let fixture = AtomicExchangeFixture::new(
        local,
        AtomicValueToken::new(20),
        vec![
            exchange(local, 1, 20, AtomicExchangeSemantics::Release),
            exchange(local, 2, 20, AtomicExchangeSemantics::Base),
            exchange(local, 3, 20, AtomicExchangeSemantics::Acquire),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, middle, acquire], &[]).unwrap();

    assert_eq!(
        realization.prior_value(acquire),
        Some(AtomicValueToken::new(20))
    );
    assert_eq!(realization.immediate_predecessor(acquire), Some(middle));
    assert!(!realization.release_acquire_synchronizes(release, acquire));
}

#[test]
fn release_acquire_synchronization_is_location_scoped() {
    let first_location = location(1);
    let second_location = location(2);
    let release = exchange_id(first_location, 1);
    let acquire = exchange_id(first_location, 2);
    let foreign_acquire = exchange_id(second_location, 2);
    let fixture = AtomicExchangeFixture::new(
        first_location,
        AtomicValueToken::new(10),
        vec![
            exchange(
                first_location,
                1,
                20,
                AtomicExchangeSemantics::Release,
            ),
            exchange(
                first_location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert!(realization.release_acquire_synchronizes(release, acquire));
    assert!(!realization.release_acquire_synchronizes(release, foreign_acquire));
}

#[test]
fn release_acquire_sync_does_not_change_ordinary_conflict_predicate() {
    let local = location(1);
    let release = exchange_id(local, 1);
    let acquire = exchange_id(local, 2);
    let fixture = AtomicExchangeFixture::new(
        local,
        AtomicValueToken::new(10),
        vec![
            exchange(local, 1, 20, AtomicExchangeSemantics::Release),
            exchange(local, 2, 30, AtomicExchangeSemantics::Acquire),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(first_access.conflicts_with(&second_access));
    assert!(realization.release_acquire_synchronizes(release, acquire));
}

#[test]
fn base_atomic_exchange_relation_does_not_supply_ordinary_access_order() {
    let local = location(1);
    let first_exchange = exchange_id(local, 1);
    let fixture = AtomicExchangeFixture::new(
        local,
        AtomicValueToken::new(10),
        vec![exchange(
            local,
            1,
            20,
            AtomicExchangeSemantics::Base,
        )],
    )
    .unwrap();
    assert!(fixture.realize(&[first_exchange], &[]).is_ok());

    let first_iteration = EachPhase::Iteration(IterationId(1));
    let second_iteration = EachPhase::Iteration(IterationId(2));
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(!each_orders(first_iteration, second_iteration));
    assert!(!each_orders(second_iteration, first_iteration));
    assert!(first_access.conflicts_with(&second_access));
}
