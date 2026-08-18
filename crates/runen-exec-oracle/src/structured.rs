/// Verification-only identity token for one dynamic `each` execution fixture.
///
/// The private numeric representation carries equality only. It is not a source
/// handle, launch identifier, worker identity, scheduler token, or execution order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EachId(u32);

impl EachId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Verification-only identity token for one sibling iteration of a dynamic `each`.
///
/// Iteration identity is structurally scoped by the containing `each` execution.
/// The private token is not a worker, lane, queue, scheduler, or source iteration
/// identity, and it carries no semantic sibling ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IterationId {
    each: EachId,
    token: u32,
}

impl IterationId {
    #[must_use]
    pub const fn new(each: EachId, token: u32) -> Self {
        Self { each, token }
    }

    pub(crate) const fn each(self) -> EachId {
        self.each
    }
}

/// Verification-only phases needed to exercise one accepted normal `each` boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EachPhase {
    Before(EachId),
    Iteration(IterationId),
    After(EachId),
}

impl EachPhase {
    const fn each(self) -> EachId {
        match self {
            Self::Before(each) | Self::After(each) => each,
            Self::Iteration(iteration) => iteration.each(),
        }
    }
}

/// Tests only the cross-phase ordering supplied by one accepted `each` contract.
///
/// This relation deliberately supplies no order between sibling iterations and no
/// intra-iteration ordering, even when both phases name the same iteration. Phases
/// belonging to distinct dynamic `each` executions receive no order from this
/// relation. It also does not represent abnormal completion.
#[must_use]
pub const fn each_orders(earlier: EachPhase, later: EachPhase) -> bool {
    if earlier.each() != later.each() {
        return false;
    }

    matches!(
        (earlier, later),
        (EachPhase::Before(_), EachPhase::Iteration(_))
            | (EachPhase::Before(_), EachPhase::After(_))
            | (EachPhase::Iteration(_), EachPhase::After(_))
    )
}
