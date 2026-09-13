use crate::{BufferAtomicLocation, BufferRegion};

/// Verification-only ordinary Exec access class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    StateChange,
}

/// Evaluate the accepted ordinary non-atomic Exec conflict relation from a
/// canonical-owner overlap fact.
///
/// `regions_overlap` must be supplied by the semantic owner of the accessed
/// storage or resource. This function does not define region identity or
/// overlap, and `false` does not establish that every other applicable semantic
/// contract permits the accesses.
#[must_use]
pub fn ordinary_accesses_conflict(
    left: AccessKind,
    right: AccessKind,
    regions_overlap: bool,
) -> bool {
    regions_overlap && (left == AccessKind::StateChange || right == AccessKind::StateChange)
}

/// Verification-only access to one logical Buffer region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Access {
    kind: AccessKind,
    region: BufferRegion,
}

impl Access {
    #[must_use]
    pub fn new(kind: AccessKind, region: BufferRegion) -> Self {
        Self { kind, region }
    }

    #[must_use]
    pub fn kind(&self) -> AccessKind {
        self.kind
    }

    #[must_use]
    pub fn region(&self) -> &BufferRegion {
        &self.region
    }

    /// Evaluates only the accepted ordinary non-atomic conflict relation.
    ///
    /// `false` means these accesses do not conflict under this relation; it does
    /// not establish that every other applicable semantic contract permits them.
    #[must_use]
    pub fn conflicts_with(&self, other: &Self) -> bool {
        ordinary_accesses_conflict(self.kind, other.kind, self.region.overlaps(&other.region))
    }

    /// Evaluates only the accepted mixed ordinary/non-atomic ↔ atomic-exchange
    /// conflict relation for the focused Buffer consumer.
    ///
    /// An atomic exchange is state-changing, so either ordinary access class
    /// conflicts when its Buffer region overlaps the exact singleton footprint of
    /// the atomic location. This relation does not admit either access, insert the
    /// ordinary access into atomic modification order, or define mixed visibility.
    #[must_use]
    pub fn conflicts_with_atomic_exchange_at(&self, location: BufferAtomicLocation) -> bool {
        self.region.overlaps(&location.region())
    }
}
