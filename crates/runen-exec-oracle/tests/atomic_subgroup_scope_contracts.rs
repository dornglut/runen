use runen_exec_oracle::{
    Access, AccessKind, AtomicExchange, AtomicExchangeError, AtomicExchangeFixture,
    AtomicExchangeId, AtomicExchangeScope, AtomicExchangeSemantics, AtomicGroupScope,
    AtomicLocationId, AtomicScopeRelation, AtomicSubgroupScope, AtomicValueToken, BufferId,
    BufferRegion, EachId, EachPhase, GroupId, HierarchyFixture, HierarchyId, HierarchyMembership,
    IterationId, PositionId, SubgroupId, each_orders,
};

fn hierarchy(
    each: EachId,
    hierarchy_token: u32,
    assignments: &[(u32, u32, u32)],
) -> HierarchyFixture {
    let hierarchy = HierarchyId::new(each, hierarchy_token);
    let required = assignments
        .iter()
        .map(|(iteration, _, _)| IterationId::new(each, *iteration))
        .collect::<Vec<_>>();
    let memberships = assignments
        .iter()
        .map(|(iteration, group, subgroup)| {
            let group = GroupId::new(hierarchy, *group);
            let subgroup = SubgroupId::new(group, *subgroup);
            HierarchyMembership::new(IterationId::new(each, *iteration), subgroup)
        })
        .collect::<Vec<_>>();

    HierarchyFixture::new(hierarchy, &required, memberships).unwrap()
}

fn exchange_id(location: AtomicLocationId, token: u32) -> AtomicExchangeId {
    AtomicExchangeId::new(location, token)
}

fn group_exchange(
    location: AtomicLocationId,
    token: u32,
    desired: u32,
    semantics: AtomicExchangeSemantics,
    hierarchy: &HierarchyFixture,
    producer: IterationId,
) -> AtomicExchange {
    let scope = AtomicGroupScope::from_hierarchy(hierarchy, producer)
        .expect("test producer must belong to the validated hierarchy");
    AtomicExchange::new(
        exchange_id(location, token),
        AtomicValueToken::new(desired),
        semantics,
        AtomicExchangeScope::Group(scope),
    )
}

fn subgroup_exchange(
    location: AtomicLocationId,
    token: u32,
    desired: u32,
    semantics: AtomicExchangeSemantics,
    hierarchy: &HierarchyFixture,
    producer: IterationId,
) -> AtomicExchange {
    let scope = AtomicSubgroupScope::from_hierarchy(hierarchy, producer)
        .expect("test producer must belong to the validated hierarchy");
    AtomicExchange::new(
        exchange_id(location, token),
        AtomicValueToken::new(desired),
        semantics,
        AtomicExchangeScope::Subgroup(scope),
    )
}

#[test]
fn subgroup_scope_admission_requires_validated_producer_membership() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 2)]);

    assert!(AtomicSubgroupScope::from_hierarchy(&hierarchy, IterationId::new(each, 1)).is_some());
    assert!(AtomicSubgroupScope::from_hierarchy(&hierarchy, IterationId::new(each, 99)).is_none());
    assert!(
        AtomicSubgroupScope::from_hierarchy(&hierarchy, IterationId::new(EachId::new(2), 1))
            .is_none()
    );
}

#[test]
fn same_subgroup_sibling_producers_are_scope_compatible_and_may_synchronize() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 1)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            subgroup_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &hierarchy,
                IterationId::new(each, 1),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(release, acquire),
        Some(AtomicScopeRelation::Compatible)
    );
    assert!(realization.release_acquire_synchronizes(release, acquire));
}

#[test]
fn distinct_subgroups_are_incompatible_without_partitioning_modification_order() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 2)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            subgroup_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &hierarchy,
                IterationId::new(each, 1),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(release, acquire),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes(release, acquire));
    assert_eq!(
        realization.prior_value(release),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        realization.prior_value(acquire),
        Some(AtomicValueToken::new(20))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(30));
    assert!(matches!(
        fixture.realize(&[acquire, release], &[(release, acquire)]),
        Err(AtomicExchangeError::OrderConstraintViolation)
    ));
}

#[test]
fn equal_private_subgroup_tokens_under_distinct_groups_remain_incompatible() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 7), (2, 2, 7)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            subgroup_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &hierarchy,
                IterationId::new(each, 1),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(release, acquire),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes(release, acquire));
}

#[test]
fn equal_private_subgroup_tokens_under_distinct_same_each_hierarchies_remain_incompatible() {
    let each = EachId::new(1);
    let first_hierarchy = hierarchy(each, 7, &[(1, 3, 7), (2, 4, 9)]);
    let second_hierarchy = hierarchy(each, 8, &[(1, 4, 9), (2, 3, 7)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            subgroup_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &first_hierarchy,
                IterationId::new(each, 1),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &second_hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(release, acquire),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes(release, acquire));
}

#[test]
fn group_release_directly_synchronizes_with_exact_subgroup_acquire() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 1)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            group_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &hierarchy,
                IterationId::new(each, 1),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(release, acquire),
        Some(AtomicScopeRelation::Compatible)
    );
    assert_eq!(
        realization.synchronization_scope_relation(acquire, release),
        Some(AtomicScopeRelation::Compatible)
    );
    assert!(realization.release_acquire_synchronizes(release, acquire));
    assert_eq!(
        realization.prior_value(release),
        Some(AtomicValueToken::new(10))
    );
    assert_eq!(
        realization.prior_value(acquire),
        Some(AtomicValueToken::new(20))
    );
    assert_eq!(realization.final_value(), AtomicValueToken::new(30));
}

#[test]
fn subgroup_release_directly_synchronizes_with_exact_group_acquire() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 1)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            subgroup_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &hierarchy,
                IterationId::new(each, 1),
            ),
            group_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(release, acquire),
        Some(AtomicScopeRelation::Compatible)
    );
    assert_eq!(
        realization.synchronization_scope_relation(acquire, release),
        Some(AtomicScopeRelation::Compatible)
    );
    assert!(realization.release_acquire_synchronizes(release, acquire));
}

#[test]
fn group_subgroup_scope_is_incompatible_when_group_producer_is_outside_selected_subgroup() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 2)]);
    let location = AtomicLocationId::new(1);
    let group_release = exchange_id(location, 1);
    let subgroup_acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            group_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &hierarchy,
                IterationId::new(each, 2),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(each, 1),
            ),
        ],
    )
    .unwrap();
    let realization = fixture
        .realize(&[group_release, subgroup_acquire], &[])
        .unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(group_release, subgroup_acquire),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert_eq!(
        realization.synchronization_scope_relation(subgroup_acquire, group_release),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes(group_release, subgroup_acquire));
}

#[test]
fn group_subgroup_scope_is_incompatible_across_distinct_groups() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 2, 1)]);
    let location = AtomicLocationId::new(1);
    let group_release = exchange_id(location, 1);
    let subgroup_acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            group_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &hierarchy,
                IterationId::new(each, 1),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture
        .realize(&[group_release, subgroup_acquire], &[])
        .unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(group_release, subgroup_acquire),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes(group_release, subgroup_acquire));
}

#[test]
fn equal_mixed_scope_tokens_under_distinct_hierarchies_do_not_create_compatibility() {
    let each = EachId::new(1);
    let first_hierarchy = hierarchy(each, 7, &[(1, 3, 5), (2, 4, 9)]);
    let second_hierarchy = hierarchy(each, 8, &[(1, 4, 9), (2, 3, 5)]);
    let location = AtomicLocationId::new(1);
    let group_release = exchange_id(location, 1);
    let subgroup_acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            group_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                &first_hierarchy,
                IterationId::new(each, 1),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &second_hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture
        .realize(&[group_release, subgroup_acquire], &[])
        .unwrap();

    assert_eq!(
        realization.synchronization_scope_relation(group_release, subgroup_acquire),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes(group_release, subgroup_acquire));
}

#[test]
fn subgroup_mixed_with_root_or_unscoped_remains_open() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 1)]);
    let location = AtomicLocationId::new(1);
    let first = exchange_id(location, 1);
    let second = exchange_id(location, 2);
    let subgroup_release = subgroup_exchange(
        location,
        1,
        20,
        AtomicExchangeSemantics::Release,
        &hierarchy,
        IterationId::new(each, 1),
    );

    let with_root = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            subgroup_release,
            AtomicExchange::new(
                second,
                AtomicValueToken::new(30),
                AtomicExchangeSemantics::Acquire,
                AtomicExchangeScope::Root(IterationId::new(each, 2)),
            ),
        ],
    )
    .unwrap()
    .realize(&[first, second], &[])
    .unwrap();
    assert_eq!(
        with_root.synchronization_scope_relation(first, second),
        Some(AtomicScopeRelation::Open)
    );
    assert!(!with_root.release_acquire_synchronizes(first, second));

    let with_unscoped = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            subgroup_release,
            AtomicExchange::new(
                second,
                AtomicValueToken::new(30),
                AtomicExchangeSemantics::Acquire,
                AtomicExchangeScope::Unscoped,
            ),
        ],
    )
    .unwrap()
    .realize(&[first, second], &[])
    .unwrap();
    assert_eq!(
        with_unscoped.synchronization_scope_relation(first, second),
        Some(AtomicScopeRelation::Open)
    );
    assert!(!with_unscoped.release_acquire_synchronizes(first, second));
}

#[test]
fn group_mixed_with_root_or_unscoped_remains_open() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 1)]);
    let location = AtomicLocationId::new(1);
    let first = exchange_id(location, 1);
    let second = exchange_id(location, 2);
    let group_release = group_exchange(
        location,
        1,
        20,
        AtomicExchangeSemantics::Release,
        &hierarchy,
        IterationId::new(each, 1),
    );

    for scope in [
        AtomicExchangeScope::Root(IterationId::new(each, 2)),
        AtomicExchangeScope::Unscoped,
    ] {
        let realization = AtomicExchangeFixture::new(
            location,
            AtomicValueToken::new(10),
            vec![
                group_release,
                AtomicExchange::new(
                    second,
                    AtomicValueToken::new(30),
                    AtomicExchangeSemantics::Acquire,
                    scope,
                ),
            ],
        )
        .unwrap()
        .realize(&[first, second], &[])
        .unwrap();

        assert_eq!(
            realization.synchronization_scope_relation(first, second),
            Some(AtomicScopeRelation::Open)
        );
        assert!(!realization.release_acquire_synchronizes(first, second));
    }
}

#[test]
fn subgroup_scope_does_not_synchronize_foreign_atomic_locations() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 1)]);
    let first_location = AtomicLocationId::new(1);
    let second_location = AtomicLocationId::new(2);
    let first = exchange_id(first_location, 1);
    let second = exchange_id(second_location, 1);
    let first_fixture = AtomicExchangeFixture::new(
        first_location,
        AtomicValueToken::new(10),
        vec![subgroup_exchange(
            first_location,
            1,
            20,
            AtomicExchangeSemantics::Release,
            &hierarchy,
            IterationId::new(each, 1),
        )],
    )
    .unwrap();
    let second_fixture = AtomicExchangeFixture::new(
        second_location,
        AtomicValueToken::new(30),
        vec![subgroup_exchange(
            second_location,
            1,
            40,
            AtomicExchangeSemantics::Acquire,
            &hierarchy,
            IterationId::new(each, 2),
        )],
    )
    .unwrap();

    let first_realization = first_fixture.realize(&[first], &[]).unwrap();
    assert_eq!(
        first_realization.synchronization_scope_relation(first, second),
        None
    );
    assert!(!first_realization.release_acquire_synchronizes(first, second));
    assert!(second_fixture.realize(&[second], &[]).is_ok());
}

#[test]
fn mixed_group_subgroup_scope_does_not_create_sibling_order_or_legalize_conflict() {
    let each = EachId::new(1);
    let first = IterationId::new(each, 1);
    let second = IterationId::new(each, 2);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 1)]);

    assert!(AtomicGroupScope::from_hierarchy(&hierarchy, first).is_some());
    assert!(AtomicSubgroupScope::from_hierarchy(&hierarchy, second).is_some());
    assert!(!each_orders(
        EachPhase::Iteration(first),
        EachPhase::Iteration(second)
    ));
    assert!(!each_orders(
        EachPhase::Iteration(second),
        EachPhase::Iteration(first)
    ));

    let region = BufferRegion::new(BufferId(1), [PositionId(1)]);
    let read = Access::new(AccessKind::Read, region.clone());
    let write = Access::new(AccessKind::StateChange, region);
    assert!(read.conflicts_with(&write));
}

#[test]
fn subgroup_scope_does_not_create_sibling_order_or_legalize_an_ordinary_conflict() {
    let each = EachId::new(1);
    let first = IterationId::new(each, 1);
    let second = IterationId::new(each, 2);
    let hierarchy = hierarchy(each, 1, &[(1, 1, 1), (2, 1, 1)]);

    assert!(AtomicSubgroupScope::from_hierarchy(&hierarchy, first).is_some());
    assert!(AtomicSubgroupScope::from_hierarchy(&hierarchy, second).is_some());
    assert!(!each_orders(
        EachPhase::Iteration(first),
        EachPhase::Iteration(second)
    ));

    let region = BufferRegion::new(BufferId(1), [PositionId(1)]);
    let read = Access::new(AccessKind::Read, region.clone());
    let write = Access::new(AccessKind::StateChange, region);
    assert!(read.conflicts_with(&write));
}
