use crate::BufferRegion;

/// Verification-only ordinary Exec access class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    StateChange,
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
        self.region.overlaps(&other.region)
            && (self.kind == AccessKind::StateChange || other.kind == AccessKind::StateChange)
    }
}
