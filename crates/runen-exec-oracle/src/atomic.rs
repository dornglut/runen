use crate::coverage::has_exact_unique_coverage;

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
}

/// Verification-only description of one semantic atomic exchange occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtomicExchange {
    id: AtomicExchangeId,
    desired: AtomicValueToken,
    semantics: AtomicExchangeSemantics,
}

impl AtomicExchange {
    #[must_use]
    pub const fn new(
        id: AtomicExchangeId,
        desired: AtomicValueToken,
        semantics: AtomicExchangeSemantics,
    ) -> Self {
        Self {
            id,
            desired,
            semantics,
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
/// modification order is supplied only when checking one realization.
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
            current = exchange.desired;
            previous = Some(exchange.id);
        }

        Ok(AtomicExchangeRealization {
            location: self.location,
            prior_values,
            predecessors,
            semantics,
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

    /// Returns the immediate modification-order predecessor of one represented
    /// exchange without exposing or enumerating the full modification order.
    #[must_use]
    pub fn immediate_predecessor(&self, exchange: AtomicExchangeId) -> Option<AtomicExchangeId> {
        if exchange.location() != self.location {
            return None;
        }

        self.predecessors
            .iter()
            .find_map(|(candidate, predecessor)| (*candidate == exchange).then_some(*predecessor))
            .flatten()
    }

    /// Whether the named represented Release exchange directly synchronizes with
    /// the named represented Acquire exchange under the accepted direct-predecessor
    /// relation.
    #[must_use]
    pub fn release_acquire_synchronizes(
        &self,
        release: AtomicExchangeId,
        acquire: AtomicExchangeId,
    ) -> bool {
        if release.location() != self.location || acquire.location() != self.location {
            return false;
        }

        self.semantics_of(release) == Some(AtomicExchangeSemantics::Release)
            && self.semantics_of(acquire) == Some(AtomicExchangeSemantics::Acquire)
            && self.immediate_predecessor(acquire) == Some(release)
    }

    #[must_use]
    pub const fn final_value(&self) -> AtomicValueToken {
        self.final_value
    }

    fn semantics_of(&self, exchange: AtomicExchangeId) -> Option<AtomicExchangeSemantics> {
        self.semantics
            .iter()
            .find_map(|(candidate, semantics)| (*candidate == exchange).then_some(*semantics))
    }
}
