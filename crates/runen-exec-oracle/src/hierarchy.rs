use crate::{IterationId, coverage::has_exact_unique_coverage};

/// Verification-only opaque identity for one logical group fixture.
///
/// The token carries equality only. It is not a numeric group index, coordinate,
/// worker identity, launch parameter, hardware cohort, or scheduling order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupId(u32);

impl GroupId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Verification-only opaque subgroup token interpreted within one containing group.
///
/// Equal subgroup tokens under distinct groups do not denote one cross-group subgroup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubgroupId(u32);

impl SubgroupId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Verification-only membership of one required `each` iteration in one group and
/// one subgroup within that group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HierarchyMembership {
    iteration: IterationId,
    group: GroupId,
    subgroup: SubgroupId,
}

impl HierarchyMembership {
    #[must_use]
    pub const fn new(iteration: IterationId, group: GroupId, subgroup: SubgroupId) -> Self {
        Self {
            iteration,
            group,
            subgroup,
        }
    }

    #[must_use]
    pub const fn iteration(self) -> IterationId {
        self.iteration
    }

    #[must_use]
    pub const fn group(self) -> GroupId {
        self.group
    }

    #[must_use]
    pub const fn subgroup(self) -> SubgroupId {
        self.subgroup
    }
}

/// Invalid finite hierarchy verification fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HierarchyError {
    InvalidIterationCoverage,
}

/// Validated verification-only hierarchy for one required `each` iteration set.
///
/// The fixture stores no semantic ordering of groups, subgroups, or iterations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HierarchyFixture {
    memberships: Vec<HierarchyMembership>,
}

impl HierarchyFixture {
    pub fn new(
        required_iterations: &[IterationId],
        memberships: Vec<HierarchyMembership>,
    ) -> Result<Self, HierarchyError> {
        let observed_iterations = memberships
            .iter()
            .map(|membership| membership.iteration)
            .collect::<Vec<_>>();

        if !has_exact_unique_coverage(required_iterations, &observed_iterations) {
            return Err(HierarchyError::InvalidIterationCoverage);
        }

        Ok(Self { memberships })
    }

    #[must_use]
    pub fn membership(&self, iteration: IterationId) -> Option<HierarchyMembership> {
        self.memberships
            .iter()
            .copied()
            .find(|membership| membership.iteration == iteration)
    }

    #[must_use]
    pub fn same_group(&self, left: IterationId, right: IterationId) -> bool {
        let (Some(left), Some(right)) = (self.membership(left), self.membership(right)) else {
            return false;
        };

        left.group == right.group
    }

    #[must_use]
    pub fn same_subgroup(&self, left: IterationId, right: IterationId) -> bool {
        let (Some(left), Some(right)) = (self.membership(left), self.membership(right)) else {
            return false;
        };

        left.group == right.group && left.subgroup == right.subgroup
    }
}
