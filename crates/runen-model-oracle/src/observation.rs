use std::collections::BTreeMap;

use crate::{BagValue, LogicalType};

/// Verification-only state-domain identity token.
///
/// The numeric carrier exists only to distinguish finite conformance fixtures.
/// It is not a Model value, revision, clock, source identity, ECS identity,
/// storage identity, or observation order.
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
}
