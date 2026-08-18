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
pub fn all_state_dependencies_are_detach_safe(required_state: &[TaskStateRetention]) -> bool {
    required_state.iter().copied().all(state_is_detach_safe)
}

/// Verification-only result of one explicit cancellation observation point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskCancellationObservation {
    Continue,
    Cancelled,
}

/// Invalid use of the focused one-task cancellation fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskCancellationError {
    ForeignTask,
    AlreadyCancelled,
}

/// Verification-only cooperative cancellation state for one represented task.
///
/// This fixture models only explicit semantic request/observation sequencing. It is
/// not a source task handle, cancellation capability, scheduler, timer, polling
/// loop, executor, or fault-propagation model.
pub struct TaskCancellationFixture {
    task: TaskId,
    request_pending: bool,
    cancelled: bool,
}

impl TaskCancellationFixture {
    #[must_use]
    pub const fn new(task: TaskId) -> Self {
        Self {
            task,
            request_pending: false,
            cancelled: false,
        }
    }

    /// Records a cancellation request for the represented task.
    ///
    /// Repeated requests while the task remains non-terminal are idempotent. A
    /// request alone does not make the task cancelled.
    pub fn request(&mut self, task: TaskId) -> Result<(), TaskCancellationError> {
        self.require_live_task(task)?;
        self.request_pending = true;
        Ok(())
    }

    /// Exercises one explicit cancellation observation point of the represented task.
    ///
    /// Without a pending request the task continues. A pending request is consumed
    /// by terminal cancellation; no later observation transition is represented.
    pub fn observe(
        &mut self,
        task: TaskId,
    ) -> Result<TaskCancellationObservation, TaskCancellationError> {
        self.require_live_task(task)?;

        if self.request_pending {
            self.cancelled = true;
            return Ok(TaskCancellationObservation::Cancelled);
        }

        Ok(TaskCancellationObservation::Continue)
    }

    #[must_use]
    pub const fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    fn require_live_task(&self, task: TaskId) -> Result<(), TaskCancellationError> {
        if task != self.task {
            return Err(TaskCancellationError::ForeignTask);
        }
        if self.cancelled {
            return Err(TaskCancellationError::AlreadyCancelled);
        }
        Ok(())
    }
}
