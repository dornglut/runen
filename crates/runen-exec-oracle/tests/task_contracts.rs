use runen_exec_oracle::{
    Access, AccessKind, BufferId, BufferRegion, PositionId, TaskId, TaskScopePhase,
    TaskStateRetention, all_state_dependencies_are_detach_safe, has_exact_attached_completion,
    state_is_detach_safe, task_scope_orders,
};

fn region(buffer: u32, positions: &[u32]) -> BufferRegion {
    BufferRegion::new(BufferId(buffer), positions.iter().copied().map(PositionId))
}

#[test]
fn normal_scope_completion_requires_exact_unordered_attached_child_coverage() {
    let required = [TaskId(1), TaskId(2), TaskId(3)];
    let permutation = [TaskId(3), TaskId(1), TaskId(2)];
    let missing = [TaskId(1), TaskId(3)];
    let duplicated = [TaskId(1), TaskId(1), TaskId(3)];
    let invented = [TaskId(1), TaskId(2), TaskId(4)];

    assert!(has_exact_attached_completion(&required, &permutation));
    assert!(!has_exact_attached_completion(&required, &missing));
    assert!(!has_exact_attached_completion(&required, &duplicated));
    assert!(!has_exact_attached_completion(&required, &invented));
    assert!(!has_exact_attached_completion(
        &[TaskId(1), TaskId(1)],
        &[TaskId(1), TaskId(1)]
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
