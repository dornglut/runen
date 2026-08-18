use crate::coverage::has_exact_unique_coverage;

/// Verification-only identity token for one task fixture.
///
/// The numeric representation carries no task order, worker identity, executor
/// identity, physical parentage, or source-language meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskId(pub u32);

/// Verification-only opaque identity for one dynamic structured task scope.
///
/// The private token carries equality only. It is not a source scope handle,
/// nesting depth, scheduler identity, executor identity, or ordering relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskScopeId(u32);

impl TaskScopeId {
    #[must_use]
    pub const fn new(token: u32) -> Self {
        Self(token)
    }
}

/// Verification-only phases relative to one originating structured task scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskScopePhase {
    AttachedChild { scope: TaskScopeId, task: TaskId },
    DetachedFromScope { scope: TaskScopeId, task: TaskId },
    After(TaskScopeId),
}

/// Tests only the ordering supplied by structured task-scope membership itself.
///
/// An attached child's actions precede the normal continuation of the same dynamic
/// scope. This relation deliberately supplies no sibling order, cross-scope order,
/// or detachment-derived order.
#[must_use]
pub fn task_scope_orders(earlier: TaskScopePhase, later: TaskScopePhase) -> bool {
    matches!(
        (earlier, later),
        (
            TaskScopePhase::AttachedChild { scope: child_scope, .. },
            TaskScopePhase::After(after_scope),
        ) if child_scope == after_scope
    )
}

/// Verification-only completion evidence for the task outcomes represented by the
/// current structured-scope/cancellation oracle.
///
/// This is not a complete Runen task-outcome enumeration. Fault and other abnormal
/// completion forms remain outside the represented task oracle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskCompletionEvidence {
    Normal(TaskId),
    Cancelled(TaskId),
}

/// Checks that every attached child required for normal scope completion has
/// completed normally exactly once, independent of completion-evidence order.
///
/// Cancelled evidence cannot satisfy normal completion. Duplicate required task
/// identities are rejected as invalid fixture input.
#[must_use]
pub fn has_exact_attached_completion(
    required_attached: &[TaskId],
    completion_evidence: &[TaskCompletionEvidence],
) -> bool {
    if completion_evidence
        .iter()
        .any(|evidence| matches!(evidence, TaskCompletionEvidence::Cancelled(_)))
    {
        return false;
    }

    let completed_normally = completion_evidence
        .iter()
        .map(|evidence| match evidence {
            TaskCompletionEvidence::Normal(task) => *task,
            TaskCompletionEvidence::Cancelled(_) => {
                unreachable!("cancelled completion evidence is rejected above")
            }
        })
        .collect::<Vec<_>>();

    has_exact_unique_coverage(required_attached, &completed_normally)
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

/// Verification-only cancellation state for one represented task.
///
/// The fixture records an explicitly sequenced semantic request/observation history.
/// It does not model source-unordered request/observation races, host timing,
/// scheduling, polling, or cancellation authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskCancellationState {
    Running,
    CancellationPending,
    Cancelled,
}

/// Verification-only result of one explicit cancellation observation transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskCancellationObservation {
    Continue,
    Cancelled,
}

/// Invalid transition for the focused cancellation verification fixture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskCancellationError {
    ForeignTask,
    TerminalTask,
}

/// Verification-only cooperative cancellation state for one task.
///
/// Transition call order stands only for semantic sequencing already established by
/// an applicable owner. The fixture is not a scheduler and supplies no interaction
/// rule for source-unordered request and observation operations.
pub struct TaskCancellationFixture {
    task: TaskId,
    state: TaskCancellationState,
}

impl TaskCancellationFixture {
    #[must_use]
    pub const fn new(task: TaskId) -> Self {
        Self {
            task,
            state: TaskCancellationState::Running,
        }
    }

    #[must_use]
    pub const fn state(&self) -> TaskCancellationState {
        self.state
    }

    /// Records one semantically sequenced cancellation request for the represented
    /// task. Repeated requests while cancellation is already pending are idempotent.
    pub fn request(&mut self, task: TaskId) -> Result<(), TaskCancellationError> {
        if task != self.task {
            return Err(TaskCancellationError::ForeignTask);
        }

        match self.state {
            TaskCancellationState::Running => {
                self.state = TaskCancellationState::CancellationPending;
                Ok(())
            }
            TaskCancellationState::CancellationPending => Ok(()),
            TaskCancellationState::Cancelled => Err(TaskCancellationError::TerminalTask),
        }
    }

    /// Observes the cancellation state at one explicit observation point belonging
    /// to the represented task.
    pub fn observe(
        &mut self,
        task: TaskId,
    ) -> Result<TaskCancellationObservation, TaskCancellationError> {
        if task != self.task {
            return Err(TaskCancellationError::ForeignTask);
        }

        match self.state {
            TaskCancellationState::Running => Ok(TaskCancellationObservation::Continue),
            TaskCancellationState::CancellationPending => {
                self.state = TaskCancellationState::Cancelled;
                Ok(TaskCancellationObservation::Cancelled)
            }
            TaskCancellationState::Cancelled => Err(TaskCancellationError::TerminalTask),
        }
    }
}
