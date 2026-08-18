use crate::{IterationId, coverage::has_exact_unique_coverage};

/// Verification-only identity token for one dynamic structured barrier fixture.
///
/// The numeric representation carries no barrier order, worker topology,
/// physical rendezvous identity, or source-language meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarrierId(pub u32);

/// Verification-only point relative to one structured barrier instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarrierPhase {
    Before {
        barrier: BarrierId,
        iteration: IterationId,
    },
    After {
        barrier: BarrierId,
        iteration: IterationId,
    },
}

/// Tests only the cross-phase order supplied by the accepted structured barrier.
///
/// Every before phase is ordered before every after phase of the same barrier
/// instance. This relation supplies no within-phase sibling order and no order
/// between distinct barrier identities.
#[must_use]
pub const fn barrier_orders(earlier: BarrierPhase, later: BarrierPhase) -> bool {
    match (earlier, later) {
        (
            BarrierPhase::Before {
                barrier: earlier_barrier,
                ..
            },
            BarrierPhase::After {
                barrier: later_barrier,
                ..
            },
        ) => earlier_barrier.0 == later_barrier.0,
        _ => false,
    }
}

/// Checks that every iteration required by one structured barrier has completed
/// its before-barrier phase exactly once, independent of completion-list order.
///
/// Duplicate required iteration identities are rejected as invalid fixture input.
#[must_use]
pub fn has_exact_before_phase_completion(
    required_iterations: &[IterationId],
    completed_before: &[IterationId],
) -> bool {
    has_exact_unique_coverage(required_iterations, completed_before)
}
