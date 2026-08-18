use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, BufferId, BufferRegion, ContributionId, EachPhase, IterationId,
    LogicalBufferState, LogicalStateError, PositionId, ReductionLaws, ValueToken, each_orders,
    has_exact_contribution_coverage,
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
fn buffer_overlap_uses_logical_identity_and_position_sets() {
    let left = region(1, &[0, 1]);
    let overlapping = region(1, &[1, 2]);
    let disjoint = region(1, &[2, 3]);
    let other_buffer_same_positions = region(2, &[0, 1]);

    assert!(left.overlaps(&overlapping));
    assert!(!left.overlaps(&disjoint));
    assert!(!left.overlaps(&other_buffer_same_positions));
}

#[test]
fn ordinary_conflict_requires_overlap_and_a_state_change() {
    let shared = region(1, &[0]);
    let disjoint = region(1, &[1]);

    let read = Access::new(AccessKind::Read, shared.clone());
    let second_read = Access::new(AccessKind::Read, shared.clone());
    let change = Access::new(AccessKind::StateChange, shared);
    let disjoint_change = Access::new(AccessKind::StateChange, disjoint);

    assert!(!read.conflicts_with(&second_read));
    assert!(read.conflicts_with(&change));
    assert!(change.conflicts_with(&read));
    assert!(change.conflicts_with(&Access::new(AccessKind::StateChange, region(1, &[0]))));
    assert!(!change.conflicts_with(&disjoint_change));
}

#[test]
fn each_orders_only_across_the_structured_boundary() {
    let first = EachPhase::Iteration(IterationId(1));
    let second = EachPhase::Iteration(IterationId(2));

    assert!(each_orders(EachPhase::Before, first));
    assert!(each_orders(EachPhase::Before, EachPhase::After));
    assert!(each_orders(first, EachPhase::After));
    assert!(each_orders(second, EachPhase::After));

    assert!(!each_orders(first, second));
    assert!(!each_orders(second, first));
    assert!(!each_orders(first, first));
    assert!(!each_orders(EachPhase::After, first));
}

#[test]
fn ordered_changes_update_one_logical_state_not_a_stale_snapshot() {
    let selected = region(1, &[0]);
    let mut current = state(1, &[(0, 10)]);
    let stale_snapshot = current.clone();

    current.apply_change(&selected, ValueToken(20)).unwrap();
    current.apply_change(&selected, ValueToken(30)).unwrap();

    assert_eq!(current.read(&selected).unwrap(), values(&[(0, 30)]));
    assert_eq!(stale_snapshot.read(&selected).unwrap(), values(&[(0, 10)]));
}

#[test]
fn invalid_fixture_change_is_rejected_before_mutation() {
    let mut current = state(1, &[(0, 10), (1, 11)]);
    let original = current.clone();

    let wrong_buffer = region(2, &[0]);
    assert_eq!(
        current.apply_change(&wrong_buffer, ValueToken(20)),
        Err(LogicalStateError::WrongBuffer {
            expected: BufferId(1),
            actual: BufferId(2),
        })
    );
    assert_eq!(current, original);

    let partly_unknown = region(1, &[0, 2]);
    assert_eq!(
        current.apply_change(&partly_unknown, ValueToken(20)),
        Err(LogicalStateError::UnknownPosition(PositionId(2)))
    );
    assert_eq!(current, original);
}

#[test]
fn disjoint_sibling_changes_commute_in_the_logical_fixture() {
    let left_region = region(1, &[0]);
    let right_region = region(1, &[1]);
    let left_access = Access::new(AccessKind::StateChange, left_region.clone());
    let right_access = Access::new(AccessKind::StateChange, right_region.clone());

    assert!(!left_access.conflicts_with(&right_access));

    let mut left_then_right = state(1, &[(0, 0), (1, 0)]);
    left_then_right
        .apply_change(&left_region, ValueToken(10))
        .unwrap();
    left_then_right
        .apply_change(&right_region, ValueToken(20))
        .unwrap();

    let mut right_then_left = state(1, &[(0, 0), (1, 0)]);
    right_then_left
        .apply_change(&right_region, ValueToken(20))
        .unwrap();
    right_then_left
        .apply_change(&left_region, ValueToken(10))
        .unwrap();

    assert_eq!(left_then_right, right_then_left);
    assert_eq!(
        left_then_right.read(&region(1, &[0, 1])).unwrap(),
        values(&[(0, 10), (1, 20)])
    );
}

#[test]
fn overlapping_sibling_changes_conflict_independently_of_physical_order() {
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0, 1]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[1, 2]));
    let first_iteration = EachPhase::Iteration(IterationId(1));
    let second_iteration = EachPhase::Iteration(IterationId(2));

    assert!(first_access.conflicts_with(&second_access));
    assert!(!each_orders(first_iteration, second_iteration));
    assert!(!each_orders(second_iteration, first_iteration));
}

#[test]
fn unordered_reduction_requires_every_accepted_operator_law() {
    let lawful = ReductionLaws::new(true, true, true);
    assert!(lawful.permits_unordered_reduction());

    for insufficient in [
        ReductionLaws::new(false, true, true),
        ReductionLaws::new(true, false, true),
        ReductionLaws::new(true, true, false),
    ] {
        assert!(!insufficient.permits_unordered_reduction());
    }
}

#[test]
fn reduction_contribution_coverage_ignores_order_but_rejects_count_changes() {
    let required = [ContributionId(1), ContributionId(2), ContributionId(3)];
    let permutation = [ContributionId(3), ContributionId(1), ContributionId(2)];
    let omitted = [ContributionId(1), ContributionId(3)];
    let duplicated = [ContributionId(1), ContributionId(1), ContributionId(3)];
    let invented = [ContributionId(1), ContributionId(2), ContributionId(4)];

    assert!(has_exact_contribution_coverage(&required, &permutation));
    assert!(!has_exact_contribution_coverage(&required, &omitted));
    assert!(!has_exact_contribution_coverage(&required, &duplicated));
    assert!(!has_exact_contribution_coverage(&required, &invented));
    assert!(!has_exact_contribution_coverage(
        &[ContributionId(1), ContributionId(1)],
        &[ContributionId(1), ContributionId(1)]
    ));
}

#[test]
fn lawful_reduction_fixture_is_invariant_to_permutation_and_tree_shape() {
    let laws = ReductionLaws::new(true, true, true);
    assert!(laws.permits_unordered_reduction());

    let identity = 0_i64;
    let left_tree = (((identity + 1) + 2) + 3) + 4;
    let permuted_tree = (4 + 2) + (1 + (3 + identity));
    let right_tree = 1 + (2 + (3 + (4 + identity)));

    assert_eq!(left_tree, 10);
    assert_eq!(permuted_tree, left_tree);
    assert_eq!(right_tree, left_tree);
    assert_eq!(identity, 0);
}

#[test]
fn reduction_admission_does_not_legalize_ordinary_sibling_conflict() {
    let laws = ReductionLaws::new(true, true, true);
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(laws.permits_unordered_reduction());
    assert!(first_access.conflicts_with(&second_access));
}
