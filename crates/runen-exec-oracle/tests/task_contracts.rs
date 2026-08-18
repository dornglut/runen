use runen_exec_oracle::{
    Access, AccessKind, BufferId, BufferRegion, PositionId, TaskCancellationError,
    TaskCancellationFixture, TaskCancellationObservation, TaskCancellationState,
    TaskCompletionEvidence, TaskId, TaskScopePhase, TaskStateRetention,
    all_state_dependencies_are_detach_safe, has_exact_attached_completion, state_is_detach_safe,
    task_scope_orders,
};

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

fn normal(task: TaskId) -> TaskCompletionEvidence {
    TaskCompletionEvidence::Normal(task)
}

#[test]
fn normal_scope_completion_requires_exact_unordered_attached_child_coverage() {
    let required = [TaskId(1), TaskId(2), TaskId(3)];
    let permutation = [normal(TaskId(3)), normal(TaskId(1)), normal(TaskId(2))];
    let missing = [normal(TaskId(1)), normal(TaskId(3))];
    let duplicated = [normal(TaskId(1)), normal(TaskId(1)), normal(TaskId(3))];
    let invented = [normal(TaskId(1)), normal(TaskId(2)), normal(TaskId(4))];

    assert!(has_exact_attached_completion(&required, &permutation));
    assert!(!has_exact_attached_completion(&required, &missing));
    assert!(!has_exact_attached_completion(&required, &duplicated));
    assert!(!has_exact_attached_completion(&required, &invented));
    assert!(!has_exact_attached_completion(
        &[TaskId(1), TaskId(1)],
        &[normal(TaskId(1)), normal(TaskId(1))]
    ));
    assert!(has_exact_attached_completion(&[], &[]));
}

#[test]
fn structured_scope_orders_attached_children_only_before_normal_continuation() {
    let first = TaskScopePhase::AttachedChild(TaskId(1));
    let second = TaskScopePhase::AttachedChild(TaskId(2));
    let detached = TaskScopePhase::DetachedFromScope(TaskId(3));

    assert!(task_scope_orders(first, TaskScopePhase::After));
    assert!(task_scope_orders(second, TaskScopePhase::After));

    assert!(!task_scope_orders(first, second));
    assert!(!task_scope_orders(second, first));
    assert!(!task_scope_orders(detached, TaskScopePhase::After));
    assert!(!task_scope_orders(TaskScopePhase::After, first));
}

#[test]
fn only_owned_or_independently_retained_state_is_detach_safe() {
    assert!(!state_is_detach_safe(TaskStateRetention::ScopeBound));
    assert!(state_is_detach_safe(TaskStateRetention::Owned));
    assert!(state_is_detach_safe(
        TaskStateRetention::IndependentlyRetained
    ));

    assert!(all_state_dependencies_are_detach_safe(&[]));
    assert!(all_state_dependencies_are_detach_safe(&[
        TaskStateRetention::Owned,
        TaskStateRetention::IndependentlyRetained,
    ]));
    assert!(!all_state_dependencies_are_detach_safe(&[
        TaskStateRetention::Owned,
        TaskStateRetention::ScopeBound,
    ]));
}

#[test]
fn scope_membership_does_not_legalize_ordinary_sibling_conflict() {
    let first_task = TaskScopePhase::AttachedChild(TaskId(1));
    let second_task = TaskScopePhase::AttachedChild(TaskId(2));
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(!task_scope_orders(first_task, second_task));
    assert!(!task_scope_orders(second_task, first_task));
    assert!(first_access.conflicts_with(&second_access));
}

#[test]
fn cancellation_observation_without_request_continues() {
    let task = TaskId(1);
    let mut cancellation = TaskCancellationFixture::new(task);

    assert_eq!(cancellation.state(), TaskCancellationState::Running);
    assert_eq!(
        cancellation.observe(task),
        Ok(TaskCancellationObservation::Continue)
    );
    assert_eq!(cancellation.state(), TaskCancellationState::Running);
}

#[test]
fn cancellation_request_is_pending_until_observed() {
    let task = TaskId(1);
    let mut cancellation = TaskCancellationFixture::new(task);

    assert_eq!(cancellation.request(task), Ok(()));
    assert_eq!(
        cancellation.state(),
        TaskCancellationState::CancellationPending
    );
}

#[test]
fn pending_cancellation_observation_yields_terminal_cancelled() {
    let task = TaskId(1);
    let mut cancellation = TaskCancellationFixture::new(task);

    cancellation.request(task).unwrap();
    assert_eq!(
        cancellation.observe(task),
        Ok(TaskCancellationObservation::Cancelled)
    );
    assert_eq!(cancellation.state(), TaskCancellationState::Cancelled);
    assert_eq!(
        cancellation.observe(task),
        Err(TaskCancellationError::TerminalTask)
    );
}

#[test]
fn repeated_cancellation_requests_are_idempotent_while_pending() {
    let task = TaskId(1);
    let mut cancellation = TaskCancellationFixture::new(task);

    cancellation.request(task).unwrap();
    cancellation.request(task).unwrap();
    assert_eq!(
        cancellation.state(),
        TaskCancellationState::CancellationPending
    );
    assert_eq!(
        cancellation.observe(task),
        Ok(TaskCancellationObservation::Cancelled)
    );
}

#[test]
fn cancellation_rejects_foreign_task_transitions() {
    let task = TaskId(1);
    let foreign = TaskId(2);
    let mut cancellation = TaskCancellationFixture::new(task);

    assert_eq!(
        cancellation.request(foreign),
        Err(TaskCancellationError::ForeignTask)
    );
    assert_eq!(
        cancellation.observe(foreign),
        Err(TaskCancellationError::ForeignTask)
    );
    assert_eq!(cancellation.state(), TaskCancellationState::Running);
}

#[test]
fn cancellation_does_not_create_sibling_order_or_legalize_conflict() {
    let first = TaskId(1);
    let second = TaskId(2);
    let mut cancellation = TaskCancellationFixture::new(first);
    let first_task = TaskScopePhase::AttachedChild(first);
    let second_task = TaskScopePhase::AttachedChild(second);
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    cancellation.request(first).unwrap();
    assert_eq!(
        cancellation.state(),
        TaskCancellationState::CancellationPending
    );
    assert!(!task_scope_orders(first_task, second_task));
    assert!(!task_scope_orders(second_task, first_task));
    assert!(first_access.conflicts_with(&second_access));
}

#[test]
fn cancelled_attached_child_does_not_satisfy_normal_completion_coverage() {
    let first = TaskId(1);
    let second = TaskId(2);
    let mut cancellation = TaskCancellationFixture::new(second);

    cancellation.request(second).unwrap();
    assert_eq!(
        cancellation.observe(second),
        Ok(TaskCancellationObservation::Cancelled)
    );

    assert!(!has_exact_attached_completion(
        &[first, second],
        &[normal(first), TaskCompletionEvidence::Cancelled(second)]
    ));
}
