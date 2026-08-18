#![forbid(unsafe_code)]
//! Verification-only executable oracle for accepted Runen Exec relations.
//!
//! This package is not Runen source syntax, compiler Exec IR, a runtime, or a
//! normative semantic owner. Its finite identifiers and values exist only to
//! make accepted Exec contracts executable in conformance tests.

mod access;
mod buffer;
mod coverage;
mod reduction;
mod structured;
mod task;

pub use access::{Access, AccessKind};
pub use buffer::{
    BufferId, BufferRegion, LogicalBufferState, LogicalStateError, PositionId, ValueToken,
};
pub use reduction::{ContributionId, UnorderedReductionEvidence, has_exact_contribution_coverage};
pub use structured::{EachPhase, IterationId, each_orders};
pub use task::{
    TaskId, TaskScopePhase, TaskStateRetention, all_state_is_detach_safe,
    has_exact_attached_completion, state_is_detach_safe, task_scope_orders,
};
