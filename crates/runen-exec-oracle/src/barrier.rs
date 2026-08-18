use crate::{
    EachId, GroupId, HierarchyFixture, IterationId, SubgroupId,
    coverage::has_exact_unique_coverage,
};

/// Verification-only identity token for one dynamic structured barrier fixture.
///
/// The private numeric representation carries no barrier order, cohort topology,
/// physical rendezvous identity, or source-language meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarrierId(u32);

impl BarrierId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BarrierSide {
    Before,
    After,
}

/// Verification-only phase point created by a validated barrier fixture.
///
/// Fields are private so callers cannot fabricate phase participation for an
/// iteration outside the fixture's selected cohort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BarrierPhase {
    barrier: BarrierId,
    iteration: IterationId,
    side: BarrierSide,
}

/// Invalid finite structured-barrier verification fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarrierError {
    InvalidRootIterations,
    UnknownGroup,
    UnknownSubgroup,
}

/// Validated verification-only structured barrier over one selected cohort.
///
/// The participant collection is private and has no semantic enumeration order.
/// The fixture retains the dynamic `each` identity containing that cohort so a
/// foreign iteration cannot retarget by private-token coincidence. This fixture is
/// not a source barrier API, runtime rendezvous object, or atomic memory-scope
/// representation.
pub struct BarrierFixture {
    id: BarrierId,
    each: EachId,
    participants: Vec<IterationId>,
}

impl BarrierFixture {
    pub fn root(
        id: BarrierId,
        each: EachId,
        required_iterations: &[IterationId],
    ) -> Result<Self, BarrierError> {
        if required_iterations
            .iter()
            .any(|iteration| iteration.each() != each)
            || !has_exact_unique_coverage(required_iterations, required_iterations)
        {
            return Err(BarrierError::InvalidRootIterations);
        }

        Ok(Self {
            id,
            each,
            participants: required_iterations.to_vec(),
        })
    }

    pub fn group(
        id: BarrierId,
        hierarchy: &HierarchyFixture,
        group: GroupId,
    ) -> Result<Self, BarrierError> {
        let participants = hierarchy
            .group_members(group)
            .ok_or(BarrierError::UnknownGroup)?;

        Ok(Self {
            id,
            each: hierarchy.each(),
            participants,
        })
    }

    pub fn subgroup(
        id: BarrierId,
        hierarchy: &HierarchyFixture,
        subgroup: SubgroupId,
    ) -> Result<Self, BarrierError> {
        let participants = hierarchy
            .subgroup_members(subgroup)
            .ok_or(BarrierError::UnknownSubgroup)?;

        Ok(Self {
            id,
            each: hierarchy.each(),
            participants,
        })
    }

    #[must_use]
    pub fn before(&self, iteration: IterationId) -> Option<BarrierPhase> {
        self.phase(iteration, BarrierSide::Before)
    }

    #[must_use]
    pub fn after(&self, iteration: IterationId) -> Option<BarrierPhase> {
        self.phase(iteration, BarrierSide::After)
    }

    #[must_use]
    pub fn has_exact_before_completion(&self, completed_before: &[IterationId]) -> bool {
        has_exact_unique_coverage(&self.participants, completed_before)
    }

    /// Tests only the participant cross-phase order supplied by this barrier.
    ///
    /// This relation supplies no same-phase sibling order and no order for another
    /// barrier identity or an iteration outside this fixture's selected cohort.
    #[must_use]
    pub fn orders(&self, earlier: BarrierPhase, later: BarrierPhase) -> bool {
        earlier.barrier == self.id
            && later.barrier == self.id
            && earlier.side == BarrierSide::Before
            && later.side == BarrierSide::After
            && self.is_participant(earlier.iteration)
            && self.is_participant(later.iteration)
    }

    fn phase(&self, iteration: IterationId, side: BarrierSide) -> Option<BarrierPhase> {
        self.is_participant(iteration).then_some(BarrierPhase {
            barrier: self.id,
            iteration,
            side,
        })
    }

    fn is_participant(&self, iteration: IterationId) -> bool {
        iteration.each() == self.each && self.participants.contains(&iteration)
    }
}
