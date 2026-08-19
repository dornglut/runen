use runen_exec_oracle::{
    Access, AccessKind, BufferId, BufferRegion, PositionId, TaskCancellationError,
    TaskCancellationFixture, TaskCancellationObservation, TaskCancellationState,
    TaskCompletionEvidence, TaskId, TaskJoinId, TaskJoinPhase, TaskScopeId, TaskScopePhase,
    TaskStateRetention, all_state_dependencies_are_detach_safe, has_exact_attached_completion,
    state_is_detach_safe, task_join_can_complete_normally, task_join_orders, task_scope_orders,
};

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

fn normal(task: TaskId) -> TaskCompletionEvidence {
    TaskCompletionEvidence::Normal(task)
}

fn attached(scope: TaskScopeId, task: TaskId) -> TaskScopePhase {
    TaskScopePhase::AttachedChild { scope, task }
}

fn detached(scope: TaskScopeId, task: TaskId) -> TaskScopePhase {
    TaskScopePhase::DetachedFromScope { scope, task }
}

fn joined_target(task: TaskId) -> TaskJoinPhase {
    TaskJoinPhase::TargetTask(task)
}

fn after_join(target: TaskId, token: u32) -> TaskJoinPhase {
    TaskJoinPhase::After(TaskJoinId::new(target, token))
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
fn structured_scope_orders_attached_children_only_before_same_scope_continuation() {
    let scope = TaskScopeId::new(7);
    let first = attached(scope, TaskId(1));
    let second = attached(scope, TaskId(2));
    let detached = detached(scope, TaskId(3));
    let after = TaskScopePhase::After(scope);

    assert!(task_scope_orders(first, after));
    assert!(task_scope_orders(second, after));

    assert!(!task_scope_orders(first, second));
    assert!(!task_scope_orders(second, first));
    assert!(!task_scope_orders(detached, after));
    assert!(!task_scope_orders(after, first));
}

#[test]
fn structured_scope_order_does_not_cross_dynamic_scope_identity() {
    let first_scope = TaskScopeId::new(7);
    let second_scope = TaskScopeId::new(8);
    let same_task_token = TaskId(1);
    let first_child = attached(first_scope, same_task_token);
    let second_child = attached(second_scope, same_task_token);

    assert!(!task_scope_orders(
        first_child,
        TaskScopePhase::After(second_scope)
    ));
    assert!(!task_scope_orders(
        second_child,
        TaskScopePhase::After(first_scope)
    ));
    assert!(task_scope_orders(
        first_child,
        TaskScopePhase::After(first_scope)
    ));
    assert!(task_scope_orders(
        second_child,
        TaskScopePhase::After(second_scope)
    ));
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
    let scope = TaskScopeId::new(7);
    let first_task = attached(scope, TaskId(1));
    let second_task = attached(scope, TaskId(2));
    let first_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let second_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(!task_scope_orders(first_task, second_task));
    assert!(!task_scope_orders(second_task, first_task));
    assert!(first_access.conflicts_with(&second_access));
}

#[test]
fn normal_join_requires_normal_completion_of_exact_target() {
    let target = TaskId(1);

    assert!(task_join_can_complete_normally(target, normal(target)));
    assert!(!task_join_can_complete_normally(target, normal(TaskId(2))));
    assert!(!task_join_can_complete_normally(
        target,
        TaskCompletionEvidence::Cancelled(target)
    ));
}

#[test]
fn join_identity_is_structurally_bound_to_exact_target() {
    let first = TaskJoinId::new(TaskId(1), 9);
    let same = TaskJoinId::new(TaskId(1), 9);
    let foreign_target_same_token = TaskJoinId::new(TaskId(2), 9);

    assert_eq!(first, same);
    assert_ne!(first, foreign_target_same_token);
}

#[test]
fn normal_join_orders_only_exact_target_before_post_join_continuation() {
    let target = TaskId(1);
    let unrelated = TaskId(2);
    let target_phase = joined_target(target);
    let after = after_join(target, 9);

    assert!(task_join_orders(target_phase, after));
    assert!(!task_join_orders(after, target_phase));
    assert!(!task_join_orders(joined_target(unrelated), after));
}

#[test]
fn distinct_join_occurrences_do_not_order_each_other() {
    let target = TaskId(1);
    let first_after = after_join(target, 9);
    let second_after = after_join(target, 10);

    assert!(task_join_orders(joined_target(target), first_after));
    assert!(task_join_orders(joined_target(target), second_after));
    assert!(!task_join_orders(first_after, second_after));
    assert!(!task_join_orders(second_after, first_after));
}

#[test]
fn detached_task_can_gain_later_join_order_without_regaining_scope_order() {
    let scope = TaskScopeId::new(7);
    let task = TaskId(1);
    let scope_detached = detached(scope, task);
    let scope_after = TaskScopePhase::After(scope);
    let join_after = after_join(task, 9);

    assert!(!task_scope_orders(scope_detached, scope_after));
    assert!(task_join_orders(joined_target(task), join_after));

    assert!(!state_is_detach_safe(TaskStateRetention::ScopeBound));
    assert!(state_is_detach_safe(TaskStateRetention::Owned));
    assert!(state_is_detach_safe(
        TaskStateRetention::IndependentlyRetained
    ));
}

#[test]
fn join_does_not_order_unrelated_task_or_legalize_its_conflict() {
    let target = TaskId(1);
    let sibling = TaskId(2);
    let after = after_join(target, 9);
    let target_access = Access::new(AccessKind::StateChange, region(1, &[0]));
    let sibling_access = Access::new(AccessKind::StateChange, region(1, &[0]));

    assert!(!task_join_orders(joined_target(sibling), after));
    assert!(target_access.conflicts_with(&sibling_access));
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
    let scope = TaskScopeId::new(7);
    let first = TaskId(1);
    let second = TaskId(2);
    let mut cancellation = TaskCancellationFixture::new(first);
    let first_task = attached(scope, first);
    let second_task = attached(scope, second);
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
