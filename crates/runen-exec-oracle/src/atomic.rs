use crate::{
    GroupId, HierarchyFixture, HierarchyMembership, IterationId, SubgroupId,
    coverage::has_exact_unique_coverage,
};

/// Verification-only opaque identity for one semantic atomic location.
///
/// The private token carries equality only. It is not an address, allocation
/// identity, source atomic handle, hardware cache line, or scheduling order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicLocationId(u32);

impl AtomicLocationId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Verification-only identity for one atomic exchange occurrence.
///
/// Exchange identity is structurally scoped by the semantic location it modifies.
/// The private token carries no source order, physical execution order, or backend
/// instruction identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicExchangeId {
    location: AtomicLocationId,
    token: u32,
}

impl AtomicExchangeId {
    #[must_use]
    pub const fn new(location: AtomicLocationId, token: u32) -> Self {
        Self { location, token }
    }

    const fn location(self) -> AtomicLocationId {
        self.location
    }
}

/// Verification-only opaque semantic value used by atomic exchange fixtures.
///
/// This token does not define Runen value representation, numeric behavior, or
/// storage validity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicValueToken(u32);

impl AtomicValueToken {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }

    pub(crate) const fn fixture_token(self) -> u32 {
        self.0
    }
}

/// Verification-only classification of the exchange synchronization semantics
/// represented by one occurrence.
///
/// These variants do not freeze source memory-order spellings or define a complete
/// future atomic-order enumeration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicExchangeSemantics {
    Base,
    Release,
    Acquire,
    AcquireRelease,
}

/// Verification-only evidence that one producer iteration is admitted for
/// group-cohort atomic scope by one already-validated hierarchy fixture.
///
/// The producer and selected group are derived together from validated hierarchy
/// membership. Retaining that complete membership permits the focused mixed
/// group/subgroup and root/group scope relations to consume exact hierarchy evidence
/// without accepting a free-standing membership label. This is not a source hierarchy
/// handle, scheduling token, hardware work-group identity, or reusable
/// participant-domain abstraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicGroupScope {
    membership: HierarchyMembership,
}

impl AtomicGroupScope {
    #[must_use]
    pub fn from_hierarchy(hierarchy: &HierarchyFixture, producer: IterationId) -> Option<Self> {
        Some(Self {
            membership: hierarchy.membership(producer)?,
        })
    }

    const fn membership(self) -> HierarchyMembership {
        self.membership
    }

    const fn group(self) -> GroupId {
        self.membership.group()
    }

    const fn subgroup(self) -> SubgroupId {
        self.membership.subgroup()
    }
}

/// Verification-only evidence that one producer iteration is admitted for
/// subgroup-cohort atomic scope by one already-validated hierarchy fixture.
///
/// The producer and selected subgroup are derived together from validated hierarchy
/// membership. Retaining that complete membership permits the focused mixed
/// group/subgroup and root/subgroup scope relations to consume exact hierarchy
/// evidence without accepting a free-standing membership label. This is not a source
/// hierarchy handle, scheduling token, hardware subgroup identity, or reusable
/// participant-domain abstraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicSubgroupScope {
    membership: HierarchyMembership,
}

impl AtomicSubgroupScope {
    #[must_use]
    pub fn from_hierarchy(hierarchy: &HierarchyFixture, producer: IterationId) -> Option<Self> {
        Some(Self {
            membership: hierarchy.membership(producer)?,
        })
    }

    const fn membership(self) -> HierarchyMembership {
        self.membership
    }

    const fn group(self) -> GroupId {
        self.membership.group()
    }

    const fn subgroup(self) -> SubgroupId {
        self.membership.subgroup()
    }
}

/// Verification-only synchronization-scope classification for one atomic exchange.
///
/// `Root` records the iteration that performs the scoped exchange. The selected
/// root cohort is derived from that iteration's containing dynamic `each`.
/// `Group` and `Subgroup` carry validated evidence tying the producer to its actual
/// cohort in an already-established hierarchy fixture. These variants are not source
/// memory-scope spellings, a hardware scope lattice, hierarchy topology, or
/// scheduling metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicExchangeScope {
    Unscoped,
    Root(IterationId),
    Group(AtomicGroupScope),
    Subgroup(AtomicSubgroupScope),
}

/// Verification-only result of comparing two represented exchange scope forms.
///
/// `Open` records an interaction that the normative specification deliberately has
/// not defined yet; it is not another kind of incompatibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicScopeRelation {
    Compatible,
    Incompatible,
    Open,
}

/// Verification-only description of one semantic atomic exchange occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicExchange {
    id: AtomicExchangeId,
    desired: AtomicValueToken,
    semantics: AtomicExchangeSemantics,
    scope: AtomicExchangeScope,
}

impl AtomicExchange {
    #[must_use]
    pub const fn new(
        id: AtomicExchangeId,
        desired: AtomicValueToken,
        semantics: AtomicExchangeSemantics,
        scope: AtomicExchangeScope,
    ) -> Self {
        Self {
            id,
            desired,
            semantics,
            scope,
        }
    }
}

/// Invalid atomic-exchange verification fixture or proposed realization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtomicExchangeError {
    DuplicateExchangeIdentity,
    ForeignExchangeIdentity,
    InvalidModificationOrderCoverage,
    InvalidOrderConstraint,
    OrderConstraintViolation,
}

/// Validated verification-only exchange set for one semantic atomic location.
///
/// Exchange storage order is not semantic modification order. A candidate
/// modification order is supplied only when checking one realization. Exchange
/// synchronization scope does not partition or duplicate that location-local order.
pub struct AtomicExchangeFixture {
    location: AtomicLocationId,
    initial: AtomicValueToken,
    exchanges: Vec<AtomicExchange>,
}

impl AtomicExchangeFixture {
    pub fn new(
        location: AtomicLocationId,
        initial: AtomicValueToken,
        exchanges: Vec<AtomicExchange>,
    ) -> Result<Self, AtomicExchangeError> {
        if exchanges
            .iter()
            .any(|exchange| exchange.id.location() != location)
        {
            return Err(AtomicExchangeError::ForeignExchangeIdentity);
        }

        let ids = exchanges
            .iter()
            .map(|exchange| exchange.id)
            .collect::<Vec<_>>();
        if !has_exact_unique_coverage(&ids, &ids) {
            return Err(AtomicExchangeError::DuplicateExchangeIdentity);
        }

        Ok(Self {
            location,
            initial,
            exchanges,
        })
    }

    /// Checks one proposed location-local modification order and computes the
    /// resulting prior values plus final value.
    ///
    /// `ordered_before` carries verification-only semantic-order facts supplied by
    /// an applicable owner. It is not a generic execution-order graph or source API.
    pub fn realize(
        &self,
        candidate_order: &[AtomicExchangeId],
        ordered_before: &[(AtomicExchangeId, AtomicExchangeId)],
    ) -> Result<AtomicExchangeRealization, AtomicExchangeError> {
        let required = self
            .exchanges
            .iter()
            .map(|exchange| exchange.id)
            .collect::<Vec<_>>();

        if !has_exact_unique_coverage(&required, candidate_order) {
            return Err(AtomicExchangeError::InvalidModificationOrderCoverage);
        }

        if ordered_before.iter().any(|(before, after)| {
            before.location() != self.location
                || after.location() != self.location
                || !required.contains(before)
                || !required.contains(after)
                || before == after
        }) {
            return Err(AtomicExchangeError::InvalidOrderConstraint);
        }

        if ordered_before.iter().any(|(before, after)| {
            let before_index = candidate_order
                .iter()
                .position(|candidate| candidate == before)
                .expect("exact coverage guarantees constrained exchange presence");
            let after_index = candidate_order
                .iter()
                .position(|candidate| candidate == after)
                .expect("exact coverage guarantees constrained exchange presence");
            before_index >= after_index
        }) {
            return Err(AtomicExchangeError::OrderConstraintViolation);
        }

        let mut current = self.initial;
        let mut prior_values = Vec::with_capacity(candidate_order.len());
        let mut predecessors = Vec::with_capacity(candidate_order.len());
        let mut semantics = Vec::with_capacity(candidate_order.len());
        let mut scopes = Vec::with_capacity(candidate_order.len());
        let mut previous = None;

        for exchange_id in candidate_order {
            let exchange = self
                .exchanges
                .iter()
                .find(|exchange| exchange.id == *exchange_id)
                .expect("exact coverage guarantees represented exchange presence");
            prior_values.push((exchange.id, current));
            predecessors.push((exchange.id, previous));
            semantics.push((exchange.id, exchange.semantics));
            scopes.push((exchange.id, exchange.scope));
            current = exchange.desired;
            previous = Some(exchange.id);
        }

        Ok(AtomicExchangeRealization {
            location: self.location,
            prior_values,
            predecessors,
            semantics,
            scopes,
            final_value: current,
        })
    }
}

/// Verification-only result of one accepted candidate modification order.
///
/// The proposed order itself is intentionally not exposed after validation.
pub struct AtomicExchangeRealization {
    location: AtomicLocationId,
    prior_values: Vec<(AtomicExchangeId, AtomicValueToken)>,
    predecessors: Vec<(AtomicExchangeId, Option<AtomicExchangeId>)>,
    semantics: Vec<(AtomicExchangeId, AtomicExchangeSemantics)>,
    scopes: Vec<(AtomicExchangeId, AtomicExchangeScope)>,
    final_value: AtomicValueToken,
}

impl AtomicExchangeRealization {
    #[must_use]
    pub fn prior_value(&self, exchange: AtomicExchangeId) -> Option<AtomicValueToken> {
        if exchange.location() != self.location {
            return None;
        }

        self.prior_values
            .iter()
            .find_map(|(candidate, prior)| (*candidate == exchange).then_some(*prior))
    }

    /// The synchronization-scope relationship that can be decided from the two
    /// represented exchange scopes alone.
    ///
    /// `None` means one or both identities are not represented, or the represented
    /// pair is a root/group or root/subgroup interaction whose now-defined relation
    /// requires the exact hierarchy evidence selected by the narrower scope. Use
    /// [`Self::synchronization_scope_relation_in_hierarchy`] for that case.
    /// `Open` is reserved for a represented interaction whose normative relationship
    /// remains undefined by the current revision.
    #[must_use]
    pub fn synchronization_scope_relation(
        &self,
        left: AtomicExchangeId,
        right: AtomicExchangeId,
    ) -> Option<AtomicScopeRelation> {
        let left_scope = self.scope_of(left)?;
        let right_scope = self.scope_of(right)?;

        Some(match (left_scope, right_scope) {
            (AtomicExchangeScope::Unscoped, AtomicExchangeScope::Unscoped) => {
                AtomicScopeRelation::Compatible
            }
            (
                AtomicExchangeScope::Root(left_iteration),
                AtomicExchangeScope::Root(right_iteration),
            ) => {
                if left_iteration.each() == right_iteration.each() {
                    AtomicScopeRelation::Compatible
                } else {
                    AtomicScopeRelation::Incompatible
                }
            }
            (AtomicExchangeScope::Group(left), AtomicExchangeScope::Group(right)) => {
                if left.group() == right.group() {
                    AtomicScopeRelation::Compatible
                } else {
                    AtomicScopeRelation::Incompatible
                }
            }
            (AtomicExchangeScope::Subgroup(left), AtomicExchangeScope::Subgroup(right)) => {
                if left.subgroup() == right.subgroup() {
                    AtomicScopeRelation::Compatible
                } else {
                    AtomicScopeRelation::Incompatible
                }
            }
            (AtomicExchangeScope::Group(group), AtomicExchangeScope::Subgroup(subgroup))
            | (AtomicExchangeScope::Subgroup(subgroup), AtomicExchangeScope::Group(group)) => {
                if group.subgroup() == subgroup.subgroup() && subgroup.group() == group.group() {
                    AtomicScopeRelation::Compatible
                } else {
                    AtomicScopeRelation::Incompatible
                }
            }
            (AtomicExchangeScope::Root(_), AtomicExchangeScope::Group(_))
            | (AtomicExchangeScope::Group(_), AtomicExchangeScope::Root(_))
            | (AtomicExchangeScope::Root(_), AtomicExchangeScope::Subgroup(_))
            | (AtomicExchangeScope::Subgroup(_), AtomicExchangeScope::Root(_)) => return None,
            _ => AtomicScopeRelation::Open,
        })
    }

    /// The synchronization-scope relationship with one exact hierarchy fixture as
    /// focused evidence for root/group or root/subgroup membership.
    ///
    /// The supplied hierarchy is accepted for a narrower scope only when it returns
    /// the exact stored producer membership that created that scope. A distinct
    /// same-`each` hierarchy with equal private cohort tokens therefore cannot
    /// retarget the operation. `None` means an identity is unrepresented or the
    /// supplied hierarchy is not exact/sufficient evidence for the represented
    /// root-to-narrower pair.
    #[must_use]
    pub fn synchronization_scope_relation_in_hierarchy(
        &self,
        left: AtomicExchangeId,
        right: AtomicExchangeId,
        hierarchy: &HierarchyFixture,
    ) -> Option<AtomicScopeRelation> {
        let left_scope = self.scope_of(left)?;
        let right_scope = self.scope_of(right)?;

        match (left_scope, right_scope) {
            (AtomicExchangeScope::Root(root), AtomicExchangeScope::Group(group))
            | (AtomicExchangeScope::Group(group), AtomicExchangeScope::Root(root)) => {
                Self::root_group_scope_relation(root, group, hierarchy)
            }
            (AtomicExchangeScope::Root(root), AtomicExchangeScope::Subgroup(subgroup))
            | (AtomicExchangeScope::Subgroup(subgroup), AtomicExchangeScope::Root(root)) => {
                Self::root_subgroup_scope_relation(root, subgroup, hierarchy)
            }
            _ => self.synchronization_scope_relation(left, right),
        }
    }

    /// Whether the named release-capable exchange directly synchronizes with the
    /// named acquire-capable exchange under the context-free accepted
    /// direct-predecessor and synchronization-scope relations.
    ///
    /// Root/group and root/subgroup pairs whose defined relation requires exact
    /// hierarchy evidence do not synchronize through this context-free query; use
    /// [`Self::release_acquire_synchronizes_in_hierarchy`] for those pairs. A pair
    /// whose scope relationship is `Open` likewise receives no synchronization edge
    /// here, without proving that the open interaction is incompatible.
    #[must_use]
    pub fn release_acquire_synchronizes(
        &self,
        release: AtomicExchangeId,
        acquire: AtomicExchangeId,
    ) -> bool {
        if release.location() != self.location || acquire.location() != self.location {
            return false;
        }

        matches!(
            self.semantics_of(release),
            Some(AtomicExchangeSemantics::Release | AtomicExchangeSemantics::AcquireRelease)
        ) && matches!(
            self.semantics_of(acquire),
            Some(AtomicExchangeSemantics::Acquire | AtomicExchangeSemantics::AcquireRelease)
        ) && self.synchronization_scope_relation(release, acquire)
            == Some(AtomicScopeRelation::Compatible)
            && self.immediate_predecessor(acquire) == Some(release)
    }

    /// Whether the named release-capable exchange directly synchronizes with the
    /// named acquire-capable exchange when exact hierarchy evidence is required for
    /// a root/group or root/subgroup scope relationship.
    #[must_use]
    pub fn release_acquire_synchronizes_in_hierarchy(
        &self,
        release: AtomicExchangeId,
        acquire: AtomicExchangeId,
        hierarchy: &HierarchyFixture,
    ) -> bool {
        if release.location() != self.location || acquire.location() != self.location {
            return false;
        }

        matches!(
            self.semantics_of(release),
            Some(AtomicExchangeSemantics::Release | AtomicExchangeSemantics::AcquireRelease)
        ) && matches!(
            self.semantics_of(acquire),
            Some(AtomicExchangeSemantics::Acquire | AtomicExchangeSemantics::AcquireRelease)
        ) && self.synchronization_scope_relation_in_hierarchy(release, acquire, hierarchy)
            == Some(AtomicScopeRelation::Compatible)
            && self.immediate_predecessor(acquire) == Some(release)
    }

    #[must_use]
    pub const fn final_value(&self) -> AtomicValueToken {
        self.final_value
    }

    fn root_group_scope_relation(
        root: IterationId,
        group: AtomicGroupScope,
        hierarchy: &HierarchyFixture,
    ) -> Option<AtomicScopeRelation> {
        let group_membership = group.membership();
        if hierarchy.membership(group_membership.iteration()) != Some(group_membership) {
            return None;
        }
        if root.each() != group_membership.iteration().each() {
            return Some(AtomicScopeRelation::Incompatible);
        }

        let root_membership = hierarchy.membership(root)?;
        Some(if root_membership.group() == group.group() {
            AtomicScopeRelation::Compatible
        } else {
            AtomicScopeRelation::Incompatible
        })
    }

    fn root_subgroup_scope_relation(
        root: IterationId,
        subgroup: AtomicSubgroupScope,
        hierarchy: &HierarchyFixture,
    ) -> Option<AtomicScopeRelation> {
        let subgroup_membership = subgroup.membership();
        if hierarchy.membership(subgroup_membership.iteration()) != Some(subgroup_membership) {
            return None;
        }
        if root.each() != subgroup_membership.iteration().each() {
            return Some(AtomicScopeRelation::Incompatible);
        }

        let root_membership = hierarchy.membership(root)?;
        Some(if root_membership.subgroup() == subgroup.subgroup() {
            AtomicScopeRelation::Compatible
        } else {
            AtomicScopeRelation::Incompatible
        })
    }

    fn immediate_predecessor(&self, exchange: AtomicExchangeId) -> Option<AtomicExchangeId> {
        if exchange.location() != self.location {
            return None;
        }

        self.predecessors
            .iter()
            .find_map(|(candidate, predecessor)| (*candidate == exchange).then_some(*predecessor))
            .flatten()
    }

    fn semantics_of(&self, exchange: AtomicExchangeId) -> Option<AtomicExchangeSemantics> {
        self.semantics
            .iter()
            .find_map(|(candidate, semantics)| (*candidate == exchange).then_some(*semantics))
    }

    fn scope_of(&self, exchange: AtomicExchangeId) -> Option<AtomicExchangeScope> {
        if exchange.location() != self.location {
            return None;
        }

        self.scopes
            .iter()
            .find_map(|(candidate, scope)| (*candidate == exchange).then_some(*scope))
    }
}
