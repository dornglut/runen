use std::collections::BTreeMap;

use runen_exec_oracle::{
    Access, AccessKind, BarrierId, BarrierPhase, BufferId, BufferRegion, IterationId,
    LogicalBufferState, PositionId, ValueToken, barrier_orders, has_exact_before_phase_completion,
};

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
fn structured_barrier_requires_exact_before_phase_coverage() {
    let required = [IterationId(1), IterationId(2), IterationId(3)];
    let permutation = [IterationId(3), IterationId(1), IterationId(2)];
    let missing = [IterationId(1), IterationId(3)];
    let duplicated = [IterationId(1), IterationId(1), IterationId(3)];
    let invented = [IterationId(1), IterationId(2), IterationId(4)];

    assert!(has_exact_before_phase_completion(&required, &permutation));
    assert!(!has_exact_before_phase_completion(&required, &missing));
    assert!(!has_exact_before_phase_completion(&required, &duplicated));
    assert!(!has_exact_before_phase_completion(&required, &invented));
    assert!(!has_exact_before_phase_completion(
        &[IterationId(1), IterationId(1)],
        &[IterationId(1), IterationId(1)]
    ));
    assert!(has_exact_before_phase_completion(&[], &[]));
}

#[test]
fn barrier_orders_every_before_participant_before_every_after_participant() {
    let barrier = BarrierId(7);
    let before_first = BarrierPhase::Before {
        barrier,
        iteration: IterationId(1),
    };
    let before_second = BarrierPhase::Before {
        barrier,
        iteration: IterationId(2),
    };
    let after_first = BarrierPhase::After {
        barrier,
        iteration: IterationId(1),
    };
    let after_second = BarrierPhase::After {
        barrier,
        iteration: IterationId(2),
    };

    assert!(barrier_orders(before_first, after_first));
    assert!(barrier_orders(before_first, after_second));
    assert!(barrier_orders(before_second, after_first));
    assert!(barrier_orders(before_second, after_second));
}

#[test]
fn barrier_supplies_no_within_phase_or_cross_identity_order() {
    let first_barrier = BarrierId(7);
    let second_barrier = BarrierId(8);
    let before_first = BarrierPhase::Before {
        barrier: first_barrier,
        iteration: IterationId(1),
    };
    let before_second = BarrierPhase::Before {
        barrier: first_barrier,
        iteration: IterationId(2),
    };
    let after_first = BarrierPhase::After {
        barrier: first_barrier,
        iteration: IterationId(1),
    };
    let after_second = BarrierPhase::After {
        barrier: first_barrier,
        iteration: IterationId(2),
    };
    let other_after = BarrierPhase::After {
        barrier: second_barrier,
        iteration: IterationId(2),
    };

    assert!(!barrier_orders(before_first, before_second));
    assert!(!barrier_orders(before_second, before_first));
    assert!(!barrier_orders(after_first, after_second));
    assert!(!barrier_orders(after_second, after_first));
    assert!(!barrier_orders(before_first, other_after));
}

#[test]
fn barrier_orders_a_conflicting_cross_phase_pair_without_erasing_conflict() {
    let selected = region(1, &[0]);
    let before_change = Access::new(AccessKind::StateChange, selected.clone());
    let after_read = Access::new(AccessKind::Read, selected);
    let barrier = BarrierId(7);

    assert!(before_change.conflicts_with(&after_read));
    assert!(barrier_orders(
        BarrierPhase::Before {
            barrier,
            iteration: IterationId(1),
        },
        BarrierPhase::After {
            barrier,
            iteration: IterationId(2),
        }
    ));
}

#[test]
fn same_phase_sibling_conflict_remains_unordered() {
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let barrier = BarrierId(7);
    let first_before = BarrierPhase::Before {
        barrier,
        iteration: IterationId(1),
    };
    let second_before = BarrierPhase::Before {
        barrier,
        iteration: IterationId(2),
    };

    assert!(first_access.conflicts_with(&second_access));
    assert!(!barrier_orders(first_before, second_before));
    assert!(!barrier_orders(second_before, first_before));
}

#[test]
fn barrier_order_composes_with_logical_buffer_state() {
    let selected = region(1, &[0]);
    let barrier = BarrierId(7);
    let before = BarrierPhase::Before {
        barrier,
        iteration: IterationId(1),
    };
    let after = BarrierPhase::After {
        barrier,
        iteration: IterationId(2),
    };
    let mut logical = state(1, &[(0, 10)]);

    assert!(barrier_orders(before, after));
    logical.apply_change(&selected, ValueToken(20)).unwrap();

    assert_eq!(
        logical.read(&selected).unwrap(),
        BTreeMap::from([(PositionId(0), ValueToken(20))])
    );
}
