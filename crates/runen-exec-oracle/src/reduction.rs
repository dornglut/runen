use crate::{
    EachId, GroupId, HierarchyFixture, IterationId, SubgroupId,
    coverage::has_exact_unique_coverage,
};

/// Verification-only evidence for the complete contract required by the accepted
/// identity-bearing unordered reduction form.
///
/// These flags do not prove arbitrary operator implementations satisfy the
/// represented obligations and are not a source-language trait system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnorderedReductionEvidence {
    normal_and_closed: bool,
    result_only: bool,
    two_sided_identity: bool,
    associative: bool,
    commutative: bool,
}

impl UnorderedReductionEvidence {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            normal_and_closed: false,
            result_only: false,
            two_sided_identity: false,
            associative: false,
            commutative: false,
        }
    }

    #[must_use]
    pub const fn with_normal_and_closed(mut self) -> Self {
        self.normal_and_closed = true;
        self
    }

    #[must_use]
    pub const fn with_result_only(mut self) -> Self {
        self.result_only = true;
        self
    }

    #[must_use]
    pub const fn with_two_sided_identity(mut self) -> Self {
        self.two_sided_identity = true;
        self
    }

    #[must_use]
    pub const fn with_associativity(mut self) -> Self {
        self.associative = true;
        self
    }

    #[must_use]
    pub const fn with_commutativity(mut self) -> Self {
        self.commutative = true;
        self
    }

    /// Whether the represented evidence admits the accepted unordered reduction form.
    #[must_use]
    pub const fn permits_unordered_reduction(self) -> bool {
        self.normal_and_closed
            && self.result_only
            && self.two_sided_identity
            && self.associative
            && self.commutative
    }
}

/// Verification-only identity token for one dynamic reduction fixture.
///
/// The private numeric representation carries no source identity, cohort order,
/// physical accumulator identity, or execution order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReductionId(u32);

impl ReductionId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Verification-only identity for one semantic contribution occurrence.
///
/// Occurrence identity is structurally scoped by its reduction. The private token
/// carries no contribution order, worker identity, lane identity, or physical
/// accumulator identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContributionId {
    reduction: ReductionId,
    token: u32,
}

impl ContributionId {
    const fn new(reduction: ReductionId, token: u32) -> Self {
        Self { reduction, token }
    }
}

/// Verification-only semantic contribution occurrence created by a validated
/// reduction fixture for one actual participant.
///
/// The containing reduction is derived from the occurrence identity, so an
/// inconsistent reduction/contribution pair cannot be represented. Fields are
/// private so callers cannot fabricate a contribution from a nonparticipant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReductionContribution {
    occurrence: ContributionId,
    producer: IterationId,
}

/// Invalid finite cohort-reduction verification fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReductionError {
    InvalidRootIterations,
    UnknownGroup,
    UnknownSubgroup,
}

/// Validated verification-only unordered reduction over one selected cohort.
///
/// The participant collection is private and has no semantic enumeration order.
/// The fixture retains the containing dynamic `each` identity so a foreign
/// iteration cannot retarget by private-token coincidence. This fixture does not
/// define source reduction syntax, local collective result distribution, a runtime
/// reduction object, or a physical reduction tree.
pub struct ReductionFixture {
    id: ReductionId,
    each: EachId,
    participants: Vec<IterationId>,
}

impl ReductionFixture {
    pub fn root(
        id: ReductionId,
        each: EachId,
        required_iterations: &[IterationId],
    ) -> Result<Self, ReductionError> {
        if required_iterations
            .iter()
            .any(|iteration| iteration.each() != each)
            || !has_exact_unique_coverage(required_iterations, required_iterations)
        {
            return Err(ReductionError::InvalidRootIterations);
        }

        Ok(Self {
            id,
            each,
            participants: required_iterations.to_vec(),
        })
    }

    pub fn group(
        id: ReductionId,
        hierarchy: &HierarchyFixture,
        group: GroupId,
    ) -> Result<Self, ReductionError> {
        let participants = hierarchy
            .group_members(group)
            .ok_or(ReductionError::UnknownGroup)?;

        Ok(Self {
            id,
            each: hierarchy.each(),
            participants,
        })
    }

    pub fn subgroup(
        id: ReductionId,
        hierarchy: &HierarchyFixture,
        subgroup: SubgroupId,
    ) -> Result<Self, ReductionError> {
        let participants = hierarchy
            .subgroup_members(subgroup)
            .ok_or(ReductionError::UnknownSubgroup)?;

        Ok(Self {
            id,
            each: hierarchy.each(),
            participants,
        })
    }

    /// Creates one semantic contribution occurrence for an actual participant.
    ///
    /// `token` is verification-only occurrence identity within this reduction and
    /// carries no order. A participant may create multiple distinct occurrences by
    /// using distinct tokens. Nonparticipants cannot obtain a contribution token.
    #[must_use]
    pub fn contribution(&self, producer: IterationId, token: u32) -> Option<ReductionContribution> {
        self.is_participant(producer)
            .then_some(ReductionContribution {
                occurrence: ContributionId::new(self.id, token),
                producer,
            })
    }

    /// Checks that one realization incorporates exactly the produced semantic
    /// contribution occurrences once each, independent of incorporation order.
    ///
    /// Contribution occurrence identities must be unique within the reduction.
    /// Tokens from another reduction or from a producer outside this fixture's
    /// participant cohort are rejected.
    #[must_use]
    pub fn has_exact_contribution_coverage(
        &self,
        produced: &[ReductionContribution],
        incorporated: &[ReductionContribution],
    ) -> bool {
        if !produced.iter().all(|contribution| self.owns(*contribution))
            || !incorporated
                .iter()
                .all(|contribution| self.owns(*contribution))
        {
            return false;
        }

        let produced_ids = produced
            .iter()
            .map(|contribution| contribution.occurrence)
            .collect::<Vec<_>>();
        let incorporated_ids = incorporated
            .iter()
            .map(|contribution| contribution.occurrence)
            .collect::<Vec<_>>();

        if !has_exact_unique_coverage(&produced_ids, &incorporated_ids) {
            return false;
        }

        incorporated.iter().all(|actual| {
            produced.iter().any(|expected| {
                expected.occurrence == actual.occurrence && expected.producer == actual.producer
            })
        })
    }

    fn is_participant(&self, iteration: IterationId) -> bool {
        iteration.each() == self.each && self.participants.contains(&iteration)
    }

    fn owns(&self, contribution: ReductionContribution) -> bool {
        contribution.occurrence.reduction == self.id && self.is_participant(contribution.producer)
    }
}
