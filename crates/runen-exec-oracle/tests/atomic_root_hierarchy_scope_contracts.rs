use runen_exec_oracle::{
    Access, AccessKind, AtomicExchange, AtomicExchangeFixture, AtomicExchangeId,
    AtomicExchangeScope, AtomicExchangeSemantics, AtomicGroupScope, AtomicLocationId,
    AtomicScopeRelation, AtomicSubgroupScope, AtomicValueToken, BufferId, BufferRegion, EachId,
    EachPhase, GroupId, HierarchyFixture, HierarchyId, HierarchyMembership, IterationId, PositionId,
    SubgroupId, each_orders,
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

fn root_exchange(
    location: AtomicLocationId,
    token: u32,
    desired: u32,
    semantics: AtomicExchangeSemantics,
    producer: IterationId,
) -> AtomicExchange {
    AtomicExchange::new(
        exchange_id(location, token),
        AtomicValueToken::new(desired),
        semantics,
        AtomicExchangeScope::Root(producer),
    )
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
fn root_release_directly_synchronizes_with_group_acquire_when_root_producer_is_in_group() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 7, &[(1, 3, 1), (2, 3, 2)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            root_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
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

    assert_eq!(realization.synchronization_scope_relation(release, acquire), None);
    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(release, acquire, &hierarchy),
        Some(AtomicScopeRelation::Compatible)
    );
    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(acquire, release, &hierarchy),
        Some(AtomicScopeRelation::Compatible)
    );
    assert!(!realization.release_acquire_synchronizes(release, acquire));
    assert!(realization.release_acquire_synchronizes_in_hierarchy(
        release,
        acquire,
        &hierarchy
    ));
}

#[test]
fn group_release_directly_synchronizes_with_root_acquire_symmetrically() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 7, &[(1, 3, 1), (2, 3, 2)]);
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
            root_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(release, acquire, &hierarchy),
        Some(AtomicScopeRelation::Compatible)
    );
    assert!(realization.release_acquire_synchronizes_in_hierarchy(
        release,
        acquire,
        &hierarchy
    ));
}

#[test]
fn root_group_scope_is_incompatible_when_root_producer_belongs_to_another_group() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 7, &[(1, 3, 1), (2, 4, 1)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            root_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
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
        realization.synchronization_scope_relation_in_hierarchy(release, acquire, &hierarchy),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes_in_hierarchy(
        release,
        acquire,
        &hierarchy
    ));
    assert_eq!(realization.prior_value(release), Some(AtomicValueToken::new(10)));
    assert_eq!(realization.prior_value(acquire), Some(AtomicValueToken::new(20)));
    assert_eq!(realization.final_value(), AtomicValueToken::new(30));
}

#[test]
fn root_release_directly_synchronizes_with_subgroup_acquire_when_root_producer_is_in_subgroup() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 7, &[(1, 3, 5), (2, 3, 5)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            root_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
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

    assert_eq!(realization.synchronization_scope_relation(release, acquire), None);
    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(release, acquire, &hierarchy),
        Some(AtomicScopeRelation::Compatible)
    );
    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(acquire, release, &hierarchy),
        Some(AtomicScopeRelation::Compatible)
    );
    assert!(realization.release_acquire_synchronizes_in_hierarchy(
        release,
        acquire,
        &hierarchy
    ));
}

#[test]
fn subgroup_release_directly_synchronizes_with_root_acquire_symmetrically() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 7, &[(1, 3, 5), (2, 3, 5)]);
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
            root_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[release, acquire], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(release, acquire, &hierarchy),
        Some(AtomicScopeRelation::Compatible)
    );
    assert!(realization.release_acquire_synchronizes_in_hierarchy(
        release,
        acquire,
        &hierarchy
    ));
}

#[test]
fn root_subgroup_scope_is_incompatible_for_same_group_different_subgroup() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 7, &[(1, 3, 5), (2, 3, 6)]);
    let location = AtomicLocationId::new(1);
    let release = exchange_id(location, 1);
    let acquire = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            root_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
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
        realization.synchronization_scope_relation_in_hierarchy(release, acquire, &hierarchy),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes_in_hierarchy(
        release,
        acquire,
        &hierarchy
    ));
}

#[test]
fn root_to_hierarchical_scope_is_incompatible_across_dynamic_each_identities() {
    let hierarchy_each = EachId::new(1);
    let root_each = EachId::new(2);
    let hierarchy = hierarchy(hierarchy_each, 7, &[(1, 3, 5)]);
    let location = AtomicLocationId::new(1);
    let root = exchange_id(location, 1);
    let subgroup = exchange_id(location, 2);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            root_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                IterationId::new(root_each, 1),
            ),
            subgroup_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(hierarchy_each, 1),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[root, subgroup], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(root, subgroup, &hierarchy),
        Some(AtomicScopeRelation::Incompatible)
    );
    assert!(!realization.release_acquire_synchronizes_in_hierarchy(root, subgroup, &hierarchy));
}

#[test]
fn distinct_same_each_hierarchy_cannot_retarget_root_to_narrower_scope_evidence() {
    let each = EachId::new(1);
    let selected_hierarchy = hierarchy(each, 7, &[(1, 3, 5), (2, 3, 5)]);
    let foreign_hierarchy = hierarchy(each, 8, &[(1, 3, 5), (2, 3, 5)]);
    let location = AtomicLocationId::new(1);
    let root = exchange_id(location, 1);
    let group = exchange_id(location, 2);
    let subgroup = exchange_id(location, 3);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            root_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                IterationId::new(each, 1),
            ),
            group_exchange(
                location,
                2,
                30,
                AtomicExchangeSemantics::Acquire,
                &selected_hierarchy,
                IterationId::new(each, 2),
            ),
            subgroup_exchange(
                location,
                3,
                40,
                AtomicExchangeSemantics::Acquire,
                &selected_hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[root, group, subgroup], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(root, group, &foreign_hierarchy),
        None
    );
    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(
            root,
            subgroup,
            &foreign_hierarchy
        ),
        None
    );
    assert!(!realization.release_acquire_synchronizes_in_hierarchy(
        root,
        group,
        &foreign_hierarchy
    ));
}

#[test]
fn compatible_root_group_scope_still_requires_direct_release_acquire_shape() {
    let each = EachId::new(1);
    let hierarchy = hierarchy(each, 7, &[(1, 3, 1), (2, 3, 2)]);
    let location = AtomicLocationId::new(1);
    let root = exchange_id(location, 1);
    let middle = exchange_id(location, 2);
    let group = exchange_id(location, 3);
    let fixture = AtomicExchangeFixture::new(
        location,
        AtomicValueToken::new(10),
        vec![
            root_exchange(
                location,
                1,
                20,
                AtomicExchangeSemantics::Release,
                IterationId::new(each, 1),
            ),
            root_exchange(
                location,
                2,
                25,
                AtomicExchangeSemantics::Base,
                IterationId::new(each, 1),
            ),
            group_exchange(
                location,
                3,
                30,
                AtomicExchangeSemantics::Acquire,
                &hierarchy,
                IterationId::new(each, 2),
            ),
        ],
    )
    .unwrap();
    let realization = fixture.realize(&[root, middle, group], &[]).unwrap();

    assert_eq!(
        realization.synchronization_scope_relation_in_hierarchy(root, group, &hierarchy),
        Some(AtomicScopeRelation::Compatible)
    );
    assert!(!realization.release_acquire_synchronizes_in_hierarchy(root, group, &hierarchy));
}

#[test]
fn root_to_hierarchical_scope_does_not_create_sibling_order_or_legalize_conflict() {
    let each = EachId::new(1);
    let root_producer = IterationId::new(each, 1);
    let narrower_producer = IterationId::new(each, 2);
    let hierarchy = hierarchy(each, 7, &[(1, 3, 5), (2, 3, 5)]);

    assert!(AtomicGroupScope::from_hierarchy(&hierarchy, narrower_producer).is_some());
    assert!(AtomicSubgroupScope::from_hierarchy(&hierarchy, narrower_producer).is_some());
    assert!(!each_orders(
        EachPhase::Iteration(root_producer),
        EachPhase::Iteration(narrower_producer)
    ));
    assert!(!each_orders(
        EachPhase::Iteration(narrower_producer),
        EachPhase::Iteration(root_producer)
    ));

    let region = BufferRegion::new(BufferId(1), [PositionId(1)]);
    let read = Access::new(AccessKind::Read, region.clone());
    let write = Access::new(AccessKind::StateChange, region);
    assert!(read.conflicts_with(&write));
}
