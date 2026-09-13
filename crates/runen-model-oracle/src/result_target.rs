use std::collections::BTreeMap;

use crate::data::{LogicalType, Value};

/// Verification-only result-target identity token.
///
/// The numeric carrier exists only to distinguish finite conformance fixtures.
/// It is not a Model value, state-domain identity, source observation identity,
/// revision, freshness/progress position, storage identity, or semantic order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResultTargetId(u32);

impl ResultTargetId {
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Verification-only target-observation identity token scoped by one
/// [`ObservedResultTarget`] fixture.
///
/// The numeric carrier is not a revision, timestamp, frame, transaction,
/// freshness/progress position, source observation, or semantic observation
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TargetObservationId(u32);

impl TargetObservationId {
    pub const fn new(token: u32) -> Self {
        Self(token)
    }

    const fn storage_key(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResultObservationFixtureError {
    DuplicateTargetObservation(TargetObservationId),
    ResultTypeMismatch {
        expected: LogicalType,
        actual: LogicalType,
    },
}

/// Verification-only executable instantiation of the accepted result-target
/// observation profile.
///
/// The fixture is immutable after construction. Its private deterministic map
/// is lookup machinery only and does not define observation, revision,
/// freshness, or progress order. It is not a state domain, runtime target,
/// materialization service, or maintenance engine.
#[derive(Clone)]
pub struct ObservedResultTarget {
    target_id: ResultTargetId,
    result_type: LogicalType,
    observations: BTreeMap<u32, Value>,
}

impl ObservedResultTarget {
    pub fn new<I>(
        target_id: ResultTargetId,
        result_type: LogicalType,
        observations: I,
    ) -> Result<Self, ResultObservationFixtureError>
    where
        I: IntoIterator<Item = (TargetObservationId, Value)>,
    {
        let mut admitted = BTreeMap::new();
        for (observation_id, value) in observations {
            let storage_key = observation_id.storage_key();
            if admitted.contains_key(&storage_key) {
                return Err(ResultObservationFixtureError::DuplicateTargetObservation(
                    observation_id,
                ));
            }

            if value.logical_type() != &result_type {
                return Err(ResultObservationFixtureError::ResultTypeMismatch {
                    expected: result_type.clone(),
                    actual: value.logical_type().clone(),
                });
            }

            admitted.insert(storage_key, value);
        }

        Ok(Self {
            target_id,
            result_type,
            observations: admitted,
        })
    }

    pub const fn target_id(&self) -> ResultTargetId {
        self.target_id
    }

    pub fn result_type(&self) -> &LogicalType {
        &self.result_type
    }

    /// Observe one admitted logical result from this verification fixture.
    ///
    /// `None` means only that the finite fixture contains no such token. It is
    /// not a normative target acquisition, retention, availability, or failure
    /// contract.
    pub fn observed_result(&self, observation_id: TargetObservationId) -> Option<&Value> {
        self.observations.get(&observation_id.storage_key())
    }
}
