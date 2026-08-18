#![forbid(unsafe_code)]
//! Verification-only executable oracle for accepted Runen Exec relations.
//!
//! This package is not Runen source syntax, compiler Exec IR, a runtime, or a
//! normative semantic owner. Its finite identifiers and values exist only to
//! make accepted Exec contracts executable in conformance tests.

mod access;
mod atomic;
mod barrier;
mod buffer;
mod coverage;
mod hierarchy;
mod reduction;
mod structured;
mod task;

pub use access::{Access, AccessKind};
pub use atomic::{
    AtomicExchange, AtomicExchangeError, AtomicExchangeFixture, AtomicExchangeId,
    AtomicExchangeRealization, AtomicExchangeScope, AtomicExchangeSemantics, AtomicLocationId,
    AtomicValueToken,
};
pub use barrier::{BarrierError, BarrierFixture, BarrierId, BarrierPhase};
pub use buffer::{
    BufferId, BufferRegion, LogicalBufferState, LogicalStateError, PositionId, ValueToken,
};
pub use hierarchy::{
    GroupId, HierarchyError, HierarchyFixture, HierarchyId, HierarchyMembership, SubgroupId,
};
pub use reduction::{
    ContributionId, ReductionContribution, ReductionError, ReductionFixture, ReductionId,
    UnorderedReductionEvidence,
};
pub use structured::{EachId, EachPhase, IterationId, each_orders};
pub use task::{
    TaskId, TaskScopePhase, TaskStateRetention, all_state_dependencies_are_detach_safe,
    has_exact_attached_completion, state_is_detach_safe, task_scope_orders,
};
