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

/// Verification-only opaque identity for one logical subgroup fixture.
///
/// Subgroup identity is structurally scoped by its containing group. The private
/// token is not a numeric subgroup index, coordinate, lane identity, hardware
/// cohort, or scheduling order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubgroupId {
    group: GroupId,
    token: u32,
}

impl SubgroupId {
    #[must_use]
    pub const fn new(group: GroupId, token: u32) -> Self {
        Self { group, token }
    }

    #[must_use]
    pub const fn group(self) -> GroupId {
        self.group
    }
}

/// Verification-only membership of one required `each` iteration in one subgroup.
///
/// The containing group is derived from the subgroup identity, so an inconsistent
/// group/subgroup pair cannot be represented.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HierarchyMembership {
    iteration: IterationId,
    subgroup: SubgroupId,
}

impl HierarchyMembership {
    #[must_use]
    pub const fn new(iteration: IterationId, subgroup: SubgroupId) -> Self {
        Self {
            iteration,
            subgroup,
        }
    }

    #[must_use]
    pub const fn iteration(self) -> IterationId {
        self.iteration
    }

    #[must_use]
    pub const fn group(self) -> GroupId {
        self.subgroup.group()
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
/// It intentionally does not implement equality over its private storage order.
#[derive(Clone, Debug)]
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

        left.group() == right.group()
    }

    #[must_use]
    pub fn same_subgroup(&self, left: IterationId, right: IterationId) -> bool {
        let (Some(left), Some(right)) = (self.membership(left), self.membership(right)) else {
            return false;
        };

        left.subgroup == right.subgroup
    }
}
