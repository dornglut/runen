use crate::coverage::has_exact_unique_coverage;

/// Verification-only identity token for one task fixture.
///
/// The numeric representation carries no task order, worker identity, executor
/// identity, physical parentage, or source-language meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskId(pub u32);

/// Verification-only phases relative to one originating structured task scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskScopePhase {
    AttachedChild(TaskId),
    DetachedFromScope(TaskId),
    After,
}

/// Tests only the ordering supplied by structured task-scope membership itself.
///
/// An attached child's actions precede normal scope continuation. This relation
/// deliberately supplies no sibling order and no detachment-derived order.
#[must_use]
pub const fn task_scope_orders(earlier: TaskScopePhase, later: TaskScopePhase) -> bool {
    matches!(
        (earlier, later),
        (TaskScopePhase::AttachedChild(_), TaskScopePhase::After)
    )
}

/// Checks that every attached child required for normal scope completion has
/// completed normally exactly once, independent of completion-list order.
///
/// Duplicate required task identities are rejected as invalid fixture input.
#[must_use]
pub fn has_exact_attached_completion(
    required_attached: &[TaskId],
    completed_normally: &[TaskId],
) -> bool {
    has_exact_unique_coverage(required_attached, completed_normally)
}

/// Verification-only classification of how one task-state dependency survives
/// relative to the originating structured scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStateRetention {
    ScopeBound,
    Owned,
    IndependentlyRetained,
}

/// Whether one represented state dependency is safe to keep using after detachment
/// from the originating structured scope.
#[must_use]
pub const fn state_is_detach_safe(retention: TaskStateRetention) -> bool {
    matches!(
        retention,
        TaskStateRetention::Owned | TaskStateRetention::IndependentlyRetained
    )
}

/// Whether every represented state dependency required by detached work is
/// independently valid after the originating structured scope may complete.
#[must_use]
pub fn all_state_dependencies_are_detach_safe(
    required_state: &[TaskStateRetention],
) -> bool {
    required_state.iter().copied().all(state_is_detach_safe)
}
