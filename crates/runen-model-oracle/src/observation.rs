use std::collections::BTreeMap;

use crate::data::{BagValue, LogicalType};

/// Verification-only state-domain identity token.
///
/// The numeric carrier exists only to distinguish finite conformance fixtures.
/// It is not a Model value, revision, clock, source identity, ECS identity,
/// storage identity, or observation order. The oracle provides no process-global
/// domain registry; reusing a token across independently constructed fixtures is
/// only a test claim about abstract identity, not an implementation service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StateDomainId(u32);

impl StateDomainId {
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Verification-only observation identity token scoped by an
/// [`ObservedBagDomain`] fixture.
///
/// The numeric carrier is not a revision, timestamp, frame, transaction,
/// freshness/progress position, change cursor, or semantic observation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObservationId(u32);

impl ObservationId {
    pub const fn new(token: u32) -> Self {
        Self(token)
    }

    const fn storage_key(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationFixtureError {
    DuplicateObservation(ObservationId),
    RootElementTypeMismatch {
        expected: LogicalType,
        actual: LogicalType,
    },
}

/// Verification-only construction failures for the accepted bounded
/// two-distinct-domain observation context.
///
/// These failures describe malformed finite conformance fixtures. They are not
/// normative runtime acquisition, synchronization, or observation-failure
/// categories.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwoDomainObservationFixtureError {
    SameDomain(StateDomainId),
    ObservationNotAdmitted {
        domain_id: StateDomainId,
        observation_id: ObservationId,
    },
}

/// Verification-only executable instantiation of the accepted single-domain
/// observed-root profile for a `Bag<T>` logical root.
///
/// The fixture is immutable after construction. Its private deterministic map
/// is only lookup machinery and does not define observation or revision order.
#[derive(Clone)]
pub struct ObservedBagDomain {
    domain_id: StateDomainId,
    element_type: LogicalType,
    observations: BTreeMap<u32, BagValue>,
}

impl ObservedBagDomain {
    pub fn new<I>(
        domain_id: StateDomainId,
        element_type: LogicalType,
        observations: I,
    ) -> Result<Self, ObservationFixtureError>
    where
        I: IntoIterator<Item = (ObservationId, BagValue)>,
    {
        let mut admitted = BTreeMap::new();
        for (observation_id, bag) in observations {
            let storage_key = observation_id.storage_key();
            if admitted.contains_key(&storage_key) {
                return Err(ObservationFixtureError::DuplicateObservation(
                    observation_id,
                ));
            }

            if bag.element_type() != &element_type {
                return Err(ObservationFixtureError::RootElementTypeMismatch {
                    expected: element_type.clone(),
                    actual: bag.element_type().clone(),
                });
            }

            admitted.insert(storage_key, bag);
        }

        Ok(Self {
            domain_id,
            element_type,
            observations: admitted,
        })
    }

    pub const fn domain_id(&self) -> StateDomainId {
        self.domain_id
    }

    pub fn root_type(&self) -> LogicalType {
        LogicalType::bag(self.element_type.clone())
    }

    /// Observe one admitted Bag root from this verification fixture.
    ///
    /// `None` means only that the finite fixture contains no such token. It is
    /// not a normative runtime acquisition or unavailable-observation failure.
    pub fn observed_bag(&self, observation_id: ObservationId) -> Option<&BagValue> {
        self.observations.get(&observation_id.storage_key())
    }

    /// Construct executable evidence for the accepted singleton
    /// `ObservationSet` around one already-admitted observation.
    ///
    /// The returned Rust borrow is only verification machinery. It does not
    /// define storage retention, reacquisition, a runtime handle, or a general
    /// multi-domain `ObservationSet` representation. `None` means only that this
    /// finite fixture has no observation under the supplied token.
    pub fn singleton_observation_set(
        &self,
        observation_id: ObservationId,
    ) -> Option<SingletonObservationSet<'_>> {
        let observed_bag = self.observed_bag(observation_id)?;
        Some(SingletonObservationSet {
            domain_id: self.domain_id,
            observation_id,
            observed_bag,
        })
    }
}

/// Verification-only executable singleton observation context for one admitted
/// `Bag<T>` root.
///
/// This type intentionally exposes no semantic equality, hashing, ordering,
/// iteration, serialization, Model-value conversion, or general collection API.
/// Its Rust lifetime only proves that the referenced finite fixture remains
/// available while this conformance witness is used.
pub struct SingletonObservationSet<'a> {
    domain_id: StateDomainId,
    observation_id: ObservationId,
    observed_bag: &'a BagValue,
}

impl SingletonObservationSet<'_> {
    pub const fn domain_id(&self) -> StateDomainId {
        self.domain_id
    }

    pub const fn observation_id(&self) -> ObservationId {
        self.observation_id
    }

    pub fn observed_bag(&self) -> &BagValue {
        self.observed_bag
    }
}

struct TwoDomainObservationMember<'a> {
    domain_id: StateDomainId,
    observation_id: ObservationId,
    observed_bag: &'a BagValue,
}

/// Verification-only executable evidence for the accepted bounded
/// two-distinct-domain `ObservationSet` over `Bag<T>` roots.
///
/// The two constructor positions are fixture mechanics only. Public access is
/// keyed by exact [`StateDomainId`], and this type intentionally exposes no
/// positional member API, semantic equality, hashing, ordering, iteration,
/// serialization, mutation, or general collection interface.
pub struct TwoDomainObservationSet<'a> {
    member_one: TwoDomainObservationMember<'a>,
    member_two: TwoDomainObservationMember<'a>,
}

impl<'a> TwoDomainObservationSet<'a> {
    pub fn new(
        first_domain: &'a ObservedBagDomain,
        first_observation: ObservationId,
        second_domain: &'a ObservedBagDomain,
        second_observation: ObservationId,
    ) -> Result<Self, TwoDomainObservationFixtureError> {
        let first_domain_id = first_domain.domain_id();
        let second_domain_id = second_domain.domain_id();
        if first_domain_id == second_domain_id {
            return Err(TwoDomainObservationFixtureError::SameDomain(
                first_domain_id,
            ));
        }

        let first_bag = first_domain.observed_bag(first_observation).ok_or(
            TwoDomainObservationFixtureError::ObservationNotAdmitted {
                domain_id: first_domain_id,
                observation_id: first_observation,
            },
        )?;
        let second_bag = second_domain.observed_bag(second_observation).ok_or(
            TwoDomainObservationFixtureError::ObservationNotAdmitted {
                domain_id: second_domain_id,
                observation_id: second_observation,
            },
        )?;

        Ok(Self {
            member_one: TwoDomainObservationMember {
                domain_id: first_domain_id,
                observation_id: first_observation,
                observed_bag: first_bag,
            },
            member_two: TwoDomainObservationMember {
                domain_id: second_domain_id,
                observation_id: second_observation,
                observed_bag: second_bag,
            },
        })
    }

    pub fn observation_id(&self, domain_id: StateDomainId) -> Option<ObservationId> {
        self.member(domain_id).map(|member| member.observation_id)
    }

    pub fn observed_bag(&self, domain_id: StateDomainId) -> Option<&BagValue> {
        self.member(domain_id).map(|member| member.observed_bag)
    }

    fn member(&self, domain_id: StateDomainId) -> Option<&TwoDomainObservationMember<'a>> {
        if self.member_one.domain_id == domain_id {
            Some(&self.member_one)
        } else if self.member_two.domain_id == domain_id {
            Some(&self.member_two)
        } else {
            None
        }
    }
}
