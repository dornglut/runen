/// Verification-only identity token for one sibling `each` iteration fixture.
///
/// The numeric representation is not a worker, lane, queue, scheduler, or source
/// iteration identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IterationId(pub u32);

/// Verification-only phases needed to exercise the accepted normal `each` boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EachPhase {
    Before,
    Iteration(IterationId),
    After,
}

/// Tests only the cross-phase ordering supplied by the accepted `each` contract.
///
/// This relation deliberately supplies no order between sibling iterations and no
/// intra-iteration ordering, even when both phases name the same iteration. It
/// also does not represent abnormal completion.
#[must_use]
pub const fn each_orders(earlier: EachPhase, later: EachPhase) -> bool {
    matches!(
        (earlier, later),
        (EachPhase::Before, EachPhase::Iteration(_))
            | (EachPhase::Before, EachPhase::After)
            | (EachPhase::Iteration(_), EachPhase::After)
    )
}
