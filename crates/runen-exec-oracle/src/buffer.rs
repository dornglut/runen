use std::collections::{BTreeMap, BTreeSet};

/// Verification-only identity token for one logical Buffer fixture.
///
/// The numeric representation is not a Runen value, physical address,
/// allocation identity, version, or compiler IR identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferId(pub u32);

/// Verification-only identity token for one logical Buffer element position.
///
/// The numeric representation does not imply byte offset, layout, shape, stride,
/// physical location, or source-level indexing syntax.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    pub fn positions(&self) -> impl Iterator<Item = PositionId> + '_ {
        self.positions.iter().copied()
    }

    /// Implements the accepted Buffer logical-region overlap relation.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.buffer == other.buffer && !self.positions.is_disjoint(&other.positions)
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

        for position in region.positions() {
            *self
                .values
                .get_mut(&position)
                .expect("validated region positions exist in logical state") = value;
        }

        Ok(())
    }

    /// Reads the current logical fixture state for a region in position-token order.
    pub fn read(
        &self,
        region: &BufferRegion,
    ) -> Result<Vec<(PositionId, ValueToken)>, LogicalStateError> {
        self.validate_region(region)?;

        Ok(region
            .positions()
            .map(|position| {
                let value = *self
                    .values
                    .get(&position)
                    .expect("validated region positions exist in logical state");
                (position, value)
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
            .positions()
            .find(|position| !self.values.contains_key(position))
        {
            return Err(LogicalStateError::UnknownPosition(position));
        }

        Ok(())
    }
}
