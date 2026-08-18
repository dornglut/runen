#![forbid(unsafe_code)]
//! Verification-only executable oracle for accepted Runen Exec relations.
//!
//! This package is not Runen source syntax, compiler Exec IR, a runtime, or a
//! normative semantic owner. Its finite identifiers and values exist only to
//! make accepted Exec contracts executable in conformance tests.

mod access;
mod buffer;
mod reduction;
mod structured;

pub use access::{Access, AccessKind};
pub use buffer::{
    BufferId, BufferRegion, LogicalBufferState, LogicalStateError, PositionId, ValueToken,
};
pub use reduction::{ContributionId, ReductionLaws, has_exact_contribution_coverage};
pub use structured::{EachPhase, IterationId, each_orders};
