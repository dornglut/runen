# Exec Task Verification Contract

Status: **non-normative conformance-obligation documentation**

This document records focused assurance obligations for the currently represented Exec task semantics. It does not define Runen semantics, conformance profiles, compiler architecture, repository CI, or runtime task machinery.

Structured task lifetime, detachment, dynamic task-scope identity, explicit normal task join, and cooperative cancellation observation are owned normatively by `spec/language/exec/tasks.md`. The cancellation boundary also consumes the Core function-termination cleanup consequence owned by `spec/language/core/value-storage.md`.

`crates/runen-exec-oracle` is verification-only executable conformance evidence for these accepted task relations. Its fixture identities, phases, outcomes, retention classes, and cancellation states are not source syntax, runtime handles, executor state, or normative language values.

## Structured task lifetime and detachment boundary

These cases exercise structured task-scope lifetime/order and state-retention relations owned by `spec/language/exec/tasks.md`. They do not model task creation, execution, fault propagation, or scheduling.

Required cases:

- each represented dynamic structured task scope has an equality-only identity used solely to scope its attachment/detachment/normal-continuation ordering evidence;
- normal completion of one structured task scope requires every child attached to that scope's normal-completion set to have completed normally;
- attached-child completion coverage is insensitive to completion-list order but rejects missing, duplicate, invented, or ambiguous duplicate fixture task identities;
- the empty attached-child set permits normal completion without inventing a child task;
- actions of an attached child are ordered before the normal continuation of that same dynamic structured task scope;
- an attached-child phase associated with one dynamic task scope is not ordered before the normal continuation of a distinct dynamic task scope, including when the test phases reuse the same `TaskId` token;
- two children attached to the same scope receive no relative order from membership alone;
- a task detached from the originating scope receives no ordering to that scope's normal continuation from detachment alone;
- attachment or detachment does not extend, renew, copy, or upgrade a scope-bounded borrow/view permission;
- a scope-bounded state dependency is not safe to keep using after detachment once the originating scope may complete;
- owned and independently retained state dependencies are detach-safe under this lifetime relation;
- detached work is detach-safe only when every state dependency it still requires is owned or independently retained;
- task-scope identity or membership does not legalize an otherwise-conflicting ordinary sibling access;
- no fault/result behavior is inferred when an attached child does not complete normally.

`TaskScopeId`, `TaskId`, task-scope phases, and `TaskStateRetention` are verification-only tokens/classifications. `TaskScopeId` is not a source scope handle, runtime parent object, nesting level, executor identity, scheduler identity, or observable ordering token. `IndependentlyRetained` does not prescribe reference counting, allocation ownership, a runtime handle, or another retention implementation.

## Explicit normal task join boundary

These cases exercise only the explicit normal task-join target/completion/ordering relation owned by `spec/language/exec/tasks.md`. The oracle does not model source task handles, task results, physical waiting, or abnormal join behavior.

Required cases:

- normal join completion evidence succeeds only for `Normal` completion of the exact target task;
- normal completion evidence for another task does not satisfy the join target;
- cancelled completion evidence for the target task does not satisfy normal join completion, and no cancelled/faulted join result is inferred;
- actions of the exact target task are ordered before the post-join normal continuation;
- the reverse post-join-to-target direction is not inferred;
- an unrelated task receives no order to that post-join continuation;
- two distinct join occurrences do not order their post-join continuations merely by join identity, including when both target the same task;
- a detached task remains unordered to its originating structured scope's continuation from detachment alone, while a later explicit normal join may independently order that exact task to its post-join continuation;
- later join ordering does not change the existing detach-safe classification: scope-bounded state remains unsafe to require after detachment, while owned or independently retained state remains detach-safe;
- join targeting does not create sibling-task order or legalize an otherwise-conflicting unrelated ordinary access;
- target-to-post-join semantic order may be consumed by the existing Buffer ordered-coherence contract without creating a second Buffer visibility rule;
- no progress, fairness, physical blocking, scheduler, result transport, fault propagation, cancellation propagation, or source task-handle semantics are inferred.

`TaskJoinId`, `TaskJoinPhase`, `task_join_orders`, and `task_join_can_complete_normally` are verification representation only. `TaskJoinId` is not a source join handle, task handle, runtime wait object, scheduler event, progress token, or generic dependency-graph node. `TargetTask` and `After` are focused verification phase classifications, not source or runtime task states.

These obligations do not define source `spawn`/`await`/`join` syntax, task-handle acquisition or representation, task results, fault/cancellation behavior of a joining context, eligibility or multiplicity of joins, progress/fairness, physical suspension/wakeup, executor machinery, or task-scope parentage.

## Cooperative task cancellation observation boundary

These cases exercise only the explicitly sequenced cooperative cancellation relation owned by `spec/language/exec/tasks.md`. The terminal cancellation transition consumes the cleanup consequence owned by `spec/language/core/value-storage.md`; the task oracle does not add a Core `Cancel` instruction or duplicate Core destruction-domain semantics.

Required cases:

- a new one-task cancellation fixture starts running with no pending request;
- explicit cancellation observation with no pending request yields `Continue` and leaves the represented task running;
- an explicitly sequenced valid request changes only the cancellation state to pending and does not by itself make the task terminal;
- request followed by explicit observation yields terminal `Cancelled`;
- repeated valid requests while cancellation is pending are idempotent;
- a request or observation naming another fixture task is rejected without changing the represented task's state;
- the terminal fixture admits no further cancellation observation transition;
- cancellation pending state, request, and observation do not create sibling task order or legalize an otherwise-conflicting ordinary sibling access;
- a cancelled attached child does not count as a normally completed child for the existing exact attached-completion relation;
- detachment and detach-safe state-retention evidence remain unchanged by cancellation state;
- fixture transition call order stands only for semantic sequencing already supplied by an applicable contract and is not evidence for source-unordered request/observation races or host-timing order.

`TaskCancellationFixture`, `TaskCancellationState`, `TaskCancellationObservation`, and `TaskCancellationError` are verification representation only. `Running`, `CancellationPending`, `Cancelled`, and `Continue` are fixture classifications/results, not frozen source task states, runtime task handles, cancellation tokens, scheduler states, or a general outcome API. `TerminalTask` means the focused fixture has no further represented cancellation-state transition; it does not define source-handle validity or a post-completion request API.

The current Core MIR/reference machine is intentionally unchanged. Existing Core cleanup semantics remain authoritative for the reverse-local destruction-domain procedure that the normative cancellation contract consumes once cancellation termination has been selected.

These obligations do not define cancellation-request authority, source spawn/await/cancel forms, source-unordered request/observation interaction, implicit/asynchronous preemption, polling, timers, deadlines, fairness, masking, containing-scope or sibling propagation, task results, fault aggregation, catch/unwind policy, custom destructors, runtime executor machinery, or a Core cancellation instruction.
