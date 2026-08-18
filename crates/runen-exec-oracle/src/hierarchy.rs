use crate::{EachId, IterationId, coverage::has_exact_unique_coverage};

/// Verification-only opaque identity for one logical hierarchy fixture.
///
/// Hierarchy identity is structurally scoped by the dynamic `each` execution it
/// belongs to. The private token carries equality only. It is not a source
/// hierarchy handle, launch identity, hardware topology identifier, or scheduling
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HierarchyId {
    each: EachId,
    token: u32,
}

impl HierarchyId {
    #[must_use]
    pub const fn new(each: EachId, token: u32) -> Self {
        Self { each, token }
    }

    pub(crate) const fn each(self) -> EachId {
        self.each
    }
}

/// Verification-only opaque identity for one logical group fixture.
///
/// Group identity is structurally scoped by its containing hierarchy and therefore
/// transitively by the containing dynamic `each`. The private token carries no
/// numeric group index, coordinate, worker identity, launch parameter, hardware
/// cohort, or scheduling order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupId {
    hierarchy: HierarchyId,
    token: u32,
}

impl GroupId {
    #[must_use]
    pub const fn new(hierarchy: HierarchyId, token: u32) -> Self {
        Self { hierarchy, token }
    }

    const fn hierarchy(self) -> HierarchyId {
        self.hierarchy
    }
}

/// Verification-only opaque identity for one logical subgroup fixture.
///
/// Subgroup identity is structurally scoped by its containing group and therefore
/// transitively by the containing hierarchy and dynamic `each`. The private token
/// is not a numeric subgroup index, coordinate, lane identity, hardware cohort, or
/// scheduling order.
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
/// The containing group and hierarchy are derived from the subgroup identity, so
/// inconsistent hierarchy/group/subgroup identity cannot be represented inside one
/// membership value. Fixture validation additionally requires the iteration and
/// hierarchy to belong to the same dynamic `each`.
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
    ForeignHierarchyMembership,
    ForeignEachIteration,
}

/// Validated verification-only hierarchy for one required `each` iteration set.
///
/// The fixture stores its opaque hierarchy identity plus no semantic ordering of
/// groups, subgroups, or iterations. It exposes no equality, cloning, or debug
/// surface for its private storage order.
pub struct HierarchyFixture {
    id: HierarchyId,
    memberships: Vec<HierarchyMembership>,
}

impl HierarchyFixture {
    pub fn new(
        id: HierarchyId,
        required_iterations: &[IterationId],
        memberships: Vec<HierarchyMembership>,
    ) -> Result<Self, HierarchyError> {
        if memberships
            .iter()
            .any(|membership| membership.group().hierarchy() != id)
        {
            return Err(HierarchyError::ForeignHierarchyMembership);
        }

        if required_iterations
            .iter()
            .any(|iteration| iteration.each() != id.each())
            || memberships
                .iter()
                .any(|membership| membership.iteration().each() != id.each())
        {
            return Err(HierarchyError::ForeignEachIteration);
        }

        let observed_iterations = memberships
            .iter()
            .map(|membership| membership.iteration)
            .collect::<Vec<_>>();

        if !has_exact_unique_coverage(required_iterations, &observed_iterations) {
            return Err(HierarchyError::InvalidIterationCoverage);
        }

        Ok(Self { id, memberships })
    }

    pub(crate) const fn each(&self) -> EachId {
        self.id.each()
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

    pub(crate) fn group_members(&self, group: GroupId) -> Option<Vec<IterationId>> {
        if group.hierarchy() != self.id {
            return None;
        }

        let members = self
            .memberships
            .iter()
            .filter(|membership| membership.group() == group)
            .map(|membership| membership.iteration())
            .collect::<Vec<_>>();

        (!members.is_empty()).then_some(members)
    }

    pub(crate) fn subgroup_members(&self, subgroup: SubgroupId) -> Option<Vec<IterationId>> {
        if subgroup.group().hierarchy() != self.id {
            return None;
        }

        let members = self
            .memberships
            .iter()
            .filter(|membership| membership.subgroup() == subgroup)
            .map(|membership| membership.iteration())
            .collect::<Vec<_>>();

        (!members.is_empty()).then_some(members)
    }
}
