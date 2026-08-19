use std::collections::{BTreeMap, BTreeSet};

use crate::allocation::{AllocationError, AllocationFixture, AllocationId};

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

/// Invalid use of the finite logical Buffer-state verification fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalStateError {
    WrongBuffer {
        expected: BufferId,
        actual: BufferId,
    },
    UnknownPosition(PositionId),
}

/// Verification-only equality identity for one typed Buffer mapping occurrence.
///
/// Identity is structurally scoped by the physical allocation fixture that backs the
/// mapping. The token is not a source mapping handle, address, pointer, allocation
/// offset, synchronization event, or ordering token.
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
    LogicalState(LogicalStateError),
}

/// Verification-only finite evidence for one address-free typed Buffer mapping.
///
/// The fixture binds one logical Buffer region to one live physical allocation
/// extent. It grants no logical read/write permission and creates no synchronization
/// or conflict exemption. Mapped operations below stand only for already-permitted
/// typed Buffer accesses and deliberately reuse [`LogicalBufferState`] as the sole
/// logical-state oracle.
pub struct BufferMappingFixture {
    id: BufferMappingId,
    region: BufferRegion,
    active: bool,
}

impl BufferMappingFixture {
    /// Begins one represented mapping inside the supplied live allocation extent.
    ///
    /// `token` is verification-only occurrence identity within that allocation. It
    /// is not a source handle, address, byte offset, or mapping order.
    pub fn new(
        token: u32,
        region: BufferRegion,
        allocation: &mut AllocationFixture,
    ) -> Result<Self, BufferMappingError> {
        allocation
            .begin_mapping(token)
            .map_err(BufferMappingError::Allocation)?;

        Ok(Self {
            id: BufferMappingId::new(allocation.id(), token),
            region,
            active: true,
        })
    }

    #[must_use]
    pub const fn id(&self) -> BufferMappingId {
        self.id
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
    pub fn end(
        &mut self,
        allocation: &mut AllocationFixture,
    ) -> Result<(), BufferMappingError> {
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
    /// already permitted by the applicable View/ownership/memory contracts.
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
