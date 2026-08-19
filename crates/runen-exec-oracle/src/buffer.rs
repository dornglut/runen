use std::collections::{BTreeMap, BTreeSet};

use crate::{
    allocation::{AllocationError, AllocationFixture, AllocationId},
    atomic::{
        AtomicExchange, AtomicExchangeError, AtomicExchangeFixture, AtomicExchangeId,
        AtomicExchangeRealization, AtomicExchangeScope, AtomicExchangeSemantics, AtomicLocationId,
        AtomicValueToken,
    },
    coverage::has_exact_unique_coverage,
    realization::RealizationAgentId,
};

/// Verification-only identity token for one logical Buffer fixture.
///
/// The numeric representation is not a Runen value, physical address,
/// allocation identity, version, or compiler IR identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferId(pub u32);

/// Verification-only identity token for one logical Buffer element position.
///
/// The numeric representation does not imply byte offset, layout, shape, stride,
/// physical location, or source-level indexing syntax. Ordering exists only to
/// support deterministic verification collections and is not Buffer semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionId(pub u32);

/// Verification-only semantic value token used by [`LogicalBufferState`].
///
/// It is not a Runen value representation or a physical byte pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueToken(pub u32);

/// Verification-only finite representation of one logical Buffer region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferRegion {
    buffer: BufferId,
    positions: BTreeSet<PositionId>,
}

impl BufferRegion {
    #[must_use]
    pub fn new(buffer: BufferId, positions: impl IntoIterator<Item = PositionId>) -> Self {
        Self {
            buffer,
            positions: positions.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn buffer(&self) -> BufferId {
        self.buffer
    }

    /// Implements the accepted Buffer logical-region overlap relation.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.buffer == other.buffer && !self.positions.is_disjoint(&other.positions)
    }

    fn contains(&self, other: &Self) -> bool {
        self.buffer == other.buffer && other.positions.is_subset(&self.positions)
    }
}

/// Verification-only identity of one Buffer-owned atomic element location.
///
/// Identity is exactly one logical Buffer identity plus one logical element
/// position. It is not an address, allocation, mapping, atomic-oracle token, or
/// source atomic handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferAtomicLocation {
    buffer: BufferId,
    position: PositionId,
}

impl BufferAtomicLocation {
    #[must_use]
    pub const fn new(buffer: BufferId, position: PositionId) -> Self {
        Self { buffer, position }
    }

    #[must_use]
    pub const fn buffer(self) -> BufferId {
        self.buffer
    }

    #[must_use]
    pub const fn position(self) -> PositionId {
        self.position
    }

    /// Singleton logical Buffer region occupied by this atomic location.
    #[must_use]
    pub fn region(self) -> BufferRegion {
        BufferRegion::new(self.buffer, [self.position])
    }
}

/// Verification-only identity for one atomic exchange occurrence on a Buffer
/// atomic location.
///
/// Identity is structurally scoped by the exact [`BufferAtomicLocation`]. The
/// private token is not an address, generic atomic-location identity, source handle,
/// or execution order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferAtomicExchangeId {
    location: BufferAtomicLocation,
    token: u32,
}

impl BufferAtomicExchangeId {
    const fn new(location: BufferAtomicLocation, token: u32) -> Self {
        Self { location, token }
    }

    #[must_use]
    pub const fn location(self) -> BufferAtomicLocation {
        self.location
    }
}

/// Verification-only description of one already-admitted atomic exchange on one
/// Buffer atomic location.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferAtomicExchange {
    id: BufferAtomicExchangeId,
    desired: AtomicValueToken,
    semantics: AtomicExchangeSemantics,
    scope: AtomicExchangeScope,
}

impl BufferAtomicExchange {
    #[must_use]
    pub const fn id(self) -> BufferAtomicExchangeId {
        self.id
    }
}

/// Invalid use of the finite logical Buffer-state verification fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalStateError {
    WrongBuffer {
        expected: BufferId,
        actual: BufferId,
    },
    UnknownPosition(PositionId),
}

/// Invalid use of the focused Buffer atomic-location/state bridge fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferAtomicExchangeError {
    Atomic(AtomicExchangeError),
    LogicalState(LogicalStateError),
    ForeignLocation {
        expected: BufferAtomicLocation,
        actual: BufferAtomicLocation,
    },
}

/// Verification-only accepted realization of one Buffer atomic exchange set.
pub struct BufferAtomicExchangeRealization {
    location: BufferAtomicLocation,
    internal_location: AtomicLocationId,
    atomic: AtomicExchangeRealization,
}

impl BufferAtomicExchangeRealization {
    #[must_use]
    pub const fn location(&self) -> BufferAtomicLocation {
        self.location
    }

    #[must_use]
    pub fn prior_value(&self, exchange: BufferAtomicExchangeId) -> Option<AtomicValueToken> {
        if exchange.location != self.location {
            return None;
        }

        self.atomic.prior_value(AtomicExchangeId::new(
            self.internal_location,
            exchange.token,
        ))
    }

    #[must_use]
    pub const fn final_value(&self) -> AtomicValueToken {
        self.atomic.final_value()
    }
}

/// Verification-only bridge from one Buffer atomic element location to one generic
/// atomic-exchange fixture.
///
/// Buffer-side exchange identities are structurally scoped by
/// [`BufferAtomicLocation`]. A private generic `AtomicLocationId` is synthesized only
/// inside realization so the existing atomic oracle can validate one represented
/// exchange set. It is not supplied by callers and is not another resource identity.
/// The fixture stores no second atomic value: successful realization starts from and
/// commits back to the exact position in [`LogicalBufferState`]. One bridge fixture
/// represents one atomic exchange set and is consumed when that set is realized.
pub struct BufferAtomicExchangeFixture {
    location: BufferAtomicLocation,
}

impl BufferAtomicExchangeFixture {
    #[must_use]
    pub const fn new(location: BufferAtomicLocation) -> Self {
        Self { location }
    }

    #[must_use]
    pub const fn location(&self) -> BufferAtomicLocation {
        self.location
    }

    /// Constructs one verification-only exchange occurrence already bound to this
    /// fixture's exact Buffer atomic location.
    #[must_use]
    pub const fn exchange(
        &self,
        token: u32,
        desired: AtomicValueToken,
        semantics: AtomicExchangeSemantics,
        scope: AtomicExchangeScope,
    ) -> BufferAtomicExchange {
        BufferAtomicExchange {
            id: BufferAtomicExchangeId::new(self.location, token),
            desired,
            semantics,
            scope,
        }
    }

    /// Physically services one complete represented exchange set through an exact
    /// per-exchange selection of active typed mappings.
    ///
    /// Mapping assignment is verification-only physical-servicing evidence. It must
    /// cover every represented exchange identity exactly once, and every selected
    /// mapping must cover this fixture's exact atomic singleton region. Assignment
    /// order, mapping identity, allocation identity, and execution-agent identity do
    /// not become atomic order or synchronization. All assignments are validated
    /// before the atomic fixture can observe or mutate logical Buffer state.
    pub fn realize_mapped_by_exchange(
        self,
        logical_state: &mut LogicalBufferState,
        exchanges: Vec<BufferAtomicExchange>,
        assignments: &[(BufferAtomicExchangeId, &BufferMappingFixture)],
        candidate_order: &[BufferAtomicExchangeId],
        ordered_before: &[(BufferAtomicExchangeId, BufferAtomicExchangeId)],
    ) -> Result<BufferAtomicExchangeRealization, BufferMappingError> {
        let required = exchanges.iter().map(|exchange| exchange.id()).collect::<Vec<_>>();
        let observed = assignments
            .iter()
            .map(|(exchange, _)| *exchange)
            .collect::<Vec<_>>();

        if !has_exact_unique_coverage(&required, &observed) {
            return Err(BufferMappingError::AtomicAssignmentCoverage);
        }

        let location_region = self.location.region();
        for (exchange, mapping) in assignments {
            self.validate_location(*exchange)
                .map_err(BufferMappingError::Atomic)?;
            mapping.validate_access_region(&location_region)?;
        }

        self.realize(logical_state, exchanges, candidate_order, ordered_before)
            .map_err(BufferMappingError::Atomic)
    }

    /// Checks this fixture's one represented atomic-exchange set against the
    /// existing generic oracle and commits its final value into the same logical
    /// Buffer element only after the complete candidate realization is accepted.
    ///
    /// This fixture does not admit atomic access, define mixed ordinary/atomic
    /// legality, or model physical atomic servicing.
    pub fn realize(
        self,
        logical_state: &mut LogicalBufferState,
        exchanges: Vec<BufferAtomicExchange>,
        candidate_order: &[BufferAtomicExchangeId],
        ordered_before: &[(BufferAtomicExchangeId, BufferAtomicExchangeId)],
    ) -> Result<BufferAtomicExchangeRealization, BufferAtomicExchangeError> {
        for exchange in &exchanges {
            self.validate_location(exchange.id)?;
        }
        for exchange in candidate_order {
            self.validate_location(*exchange)?;
        }
        for (before, after) in ordered_before {
            self.validate_location(*before)?;
            self.validate_location(*after)?;
        }

        let region = self.location.region();
        let current = logical_state
            .read(&region)
            .map_err(BufferAtomicExchangeError::LogicalState)?
            .get(&self.location.position)
            .copied()
            .expect("validated singleton Buffer region contains its exact position");

        // This identity is private mechanical namespace for exactly one consumed
        // generic atomic fixture. BufferAtomicLocation remains the resource identity.
        let internal_location = AtomicLocationId::new(0);
        let generic_id =
            |id: BufferAtomicExchangeId| AtomicExchangeId::new(internal_location, id.token);
        let generic_exchanges = exchanges
            .into_iter()
            .map(|exchange| {
                AtomicExchange::new(
                    generic_id(exchange.id),
                    exchange.desired,
                    exchange.semantics,
                    exchange.scope,
                )
            })
            .collect();
        let generic_order = candidate_order
            .iter()
            .copied()
            .map(generic_id)
            .collect::<Vec<_>>();
        let generic_constraints = ordered_before
            .iter()
            .copied()
            .map(|(before, after)| (generic_id(before), generic_id(after)))
            .collect::<Vec<_>>();

        let fixture = AtomicExchangeFixture::new(
            internal_location,
            AtomicValueToken::new(current.0),
            generic_exchanges,
        )
        .map_err(BufferAtomicExchangeError::Atomic)?;
        let atomic = fixture
            .realize(&generic_order, &generic_constraints)
            .map_err(BufferAtomicExchangeError::Atomic)?;

        logical_state
            .apply_change(&region, ValueToken(atomic.final_value().fixture_token()))
            .map_err(BufferAtomicExchangeError::LogicalState)?;

        Ok(BufferAtomicExchangeRealization {
            location: self.location,
            internal_location,
            atomic,
        })
    }

    fn validate_location(
        &self,
        exchange: BufferAtomicExchangeId,
    ) -> Result<(), BufferAtomicExchangeError> {
        if exchange.location != self.location {
            return Err(BufferAtomicExchangeError::ForeignLocation {
                expected: self.location,
                actual: exchange.location,
            });
        }

        Ok(())
    }
}

/// Verification-only equality identity for one typed Buffer mapping occurrence.
///
/// Identity is structurally scoped by the physical allocation fixture that backs the
/// mapping. The token is not a source mapping handle, address, pointer, allocation
/// offset, execution-agent identity, synchronization event, or ordering token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferMappingId {
    allocation: AllocationId,
    token: u32,
}

impl BufferMappingId {
    const fn new(allocation: AllocationId, token: u32) -> Self {
        Self { allocation, token }
    }

    #[must_use]
    pub const fn allocation(self) -> AllocationId {
        self.allocation
    }
}

/// Invalid use of the focused address-free typed Buffer mapping fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BufferMappingError {
    Allocation(AllocationError),
    Inactive,
    ForeignAllocation {
        expected: AllocationId,
        actual: AllocationId,
    },
    OutsideMappedRegion,
    AtomicAssignmentCoverage,
    LogicalState(LogicalStateError),
    Atomic(BufferAtomicExchangeError),
}

/// Verification-only finite evidence for one address-free typed Buffer mapping.
///
/// The fixture binds one logical Buffer region to one live physical allocation
/// extent and one exact physical execution agent admitted by that allocation. It
/// grants no logical read/write permission and creates no synchronization or
/// conflict exemption. Mapped operations below stand only for already-permitted
/// typed Buffer accesses and deliberately reuse [`LogicalBufferState`] as the sole
/// logical-state oracle.
pub struct BufferMappingFixture {
    id: BufferMappingId,
    region: BufferRegion,
    agent: RealizationAgentId,
    active: bool,
}

impl BufferMappingFixture {
    /// Begins one represented mapping for an admitted agent inside the supplied live
    /// allocation extent.
    ///
    /// `token` is verification-only occurrence identity within that allocation. It
    /// is not a source handle, address, byte offset, agent identity, or mapping order.
    pub fn new(
        token: u32,
        region: BufferRegion,
        agent: RealizationAgentId,
        allocation: &mut AllocationFixture,
    ) -> Result<Self, BufferMappingError> {
        allocation
            .begin_mapping(token, agent)
            .map_err(BufferMappingError::Allocation)?;

        Ok(Self {
            id: BufferMappingId::new(allocation.id(), token),
            region,
            agent,
            active: true,
        })
    }

    #[must_use]
    pub const fn id(&self) -> BufferMappingId {
        self.id
    }

    #[must_use]
    pub const fn agent(&self) -> RealizationAgentId {
        self.agent
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    #[must_use]
    pub fn region(&self) -> &BufferRegion {
        &self.region
    }

    /// Ends only this represented physical-accessibility relation.
    ///
    /// Ending a mapping does not mutate logical Buffer state and is not semantic
    /// synchronization or a flush event.
    pub fn end(&mut self, allocation: &mut AllocationFixture) -> Result<(), BufferMappingError> {
        if !self.active {
            return Err(BufferMappingError::Inactive);
        }
        if allocation.id() != self.id.allocation {
            return Err(BufferMappingError::ForeignAllocation {
                expected: self.id.allocation,
                actual: allocation.id(),
            });
        }

        allocation
            .end_mapping(self.id.token)
            .map_err(BufferMappingError::Allocation)?;
        self.active = false;
        Ok(())
    }

    /// Reads current logical Buffer state through one active represented mapping.
    ///
    /// This method is not read authority. The caller's fixture stands for an access
    /// already permitted by the applicable View/ownership/memory contracts and
    /// physically serviced by this mapping's exact agent/allocation relation.
    pub fn mapped_read(
        &self,
        logical_state: &LogicalBufferState,
        region: &BufferRegion,
    ) -> Result<BTreeMap<PositionId, ValueToken>, BufferMappingError> {
        self.validate_access_region(region)?;
        logical_state
            .read(region)
            .map_err(BufferMappingError::LogicalState)
    }

    /// Applies one already-permitted typed state change through an active mapping.
    ///
    /// The change is applied directly to the one logical Buffer-state fixture; the
    /// mapping carries no second physical/staging state or writeback protocol.
    pub fn mapped_change(
        &self,
        logical_state: &mut LogicalBufferState,
        region: &BufferRegion,
        value: ValueToken,
    ) -> Result<(), BufferMappingError> {
        self.validate_access_region(region)?;
        logical_state
            .apply_change(region, value)
            .map_err(BufferMappingError::LogicalState)
    }

    /// Physically services one already-admitted Buffer atomic exchange set through
    /// this exact active mapping relation.
    ///
    /// The mapping supplies only physical coverage through its exact agent and
    /// allocation. It does not admit atomic access, create atomic-location identity,
    /// change exchange scope or modification order, or establish synchronization.
    /// Mapping validity is checked before the atomic fixture can observe or mutate
    /// logical Buffer state.
    pub fn mapped_atomic_exchange(
        &self,
        logical_state: &mut LogicalBufferState,
        fixture: BufferAtomicExchangeFixture,
        exchanges: Vec<BufferAtomicExchange>,
        candidate_order: &[BufferAtomicExchangeId],
        ordered_before: &[(BufferAtomicExchangeId, BufferAtomicExchangeId)],
    ) -> Result<BufferAtomicExchangeRealization, BufferMappingError> {
        self.validate_access_region(&fixture.location().region())?;
        fixture
            .realize(logical_state, exchanges, candidate_order, ordered_before)
            .map_err(BufferMappingError::Atomic)
    }

    fn validate_access_region(&self, region: &BufferRegion) -> Result<(), BufferMappingError> {
        if !self.active {
            return Err(BufferMappingError::Inactive);
        }
        if !self.region.contains(region) {
            return Err(BufferMappingError::OutsideMappedRegion);
        }

        Ok(())
    }
}

/// Verification-only finite model of one logical Buffer state.
///
/// This fixture makes accepted ordered-coherence consequences executable. It is
/// not a public Buffer API, version store, replica protocol, or physical backing
/// model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalBufferState {
    buffer: BufferId,
    values: BTreeMap<PositionId, ValueToken>,
}

impl LogicalBufferState {
    #[must_use]
    pub fn new(buffer: BufferId, values: BTreeMap<PositionId, ValueToken>) -> Self {
        Self { buffer, values }
    }

    #[must_use]
    pub fn buffer(&self) -> BufferId {
        self.buffer
    }

    /// Applies one already-semantically-ordered fixture state change.
    ///
    /// The complete region is validated before mutation so invalid fixture input
    /// cannot partially update the logical state.
    pub fn apply_change(
        &mut self,
        region: &BufferRegion,
        value: ValueToken,
    ) -> Result<(), LogicalStateError> {
        self.validate_region(region)?;

        for position in &region.positions {
            *self
                .values
                .get_mut(position)
                .expect("validated region positions exist in logical state") = value;
        }

        Ok(())
    }

    /// Reads the current logical position/value relation for a Buffer region.
    ///
    /// The returned map is a deterministic verification representation. Its
    /// iteration order is not part of the accepted Buffer-region semantics.
    pub fn read(
        &self,
        region: &BufferRegion,
    ) -> Result<BTreeMap<PositionId, ValueToken>, LogicalStateError> {
        self.validate_region(region)?;

        Ok(region
            .positions
            .iter()
            .map(|position| {
                let value = *self
                    .values
                    .get(position)
                    .expect("validated region positions exist in logical state");
                (*position, value)
            })
            .collect())
    }

    fn validate_region(&self, region: &BufferRegion) -> Result<(), LogicalStateError> {
        if region.buffer != self.buffer {
            return Err(LogicalStateError::WrongBuffer {
                expected: self.buffer,
                actual: region.buffer,
            });
        }

        if let Some(position) = region
            .positions
            .iter()
            .copied()
            .find(|position| !self.values.contains_key(position))
        {
            return Err(LogicalStateError::UnknownPosition(position));
        }

        Ok(())
    }
}
