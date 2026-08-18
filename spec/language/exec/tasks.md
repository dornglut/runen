# Exec Tasks

Status: **provisional normative; incomplete**

Exec owns execution-visible work whose legal physical realization may vary.

A normal function executes in its current execution context.

A task denotes computation visible to realization as an independent execution unit. Being a task does not by itself guarantee asynchrony, parallel execution, progress, or a particular execution target.

## Structured task scope

A **structured task scope** is a semantic execution boundary with a set of child tasks attached to that scope for normal-completion purposes.

Attachment is a semantic lifetime and ordering relationship relative to that scope. It does not imply a host thread, executor, worker, queue, physical parent task, scheduling policy, or execution target.

A structured task scope completes normally only after every child task that remains attached to that scope's normal-completion set has completed normally. Actions in the scope's normal continuation occur after the actions of every such normally completed attached child.

Attachment to the same structured task scope does not by itself establish a relative order between two child tasks. Any ordering or interaction between child tasks requires another applicable semantic contract.

This revision defines only normal structured task-scope completion. If an attached child faults, is cancelled, diverges, or otherwise fails to complete normally, the scope's resulting fault, cancellation, result, and completion behavior are not defined by this revision.

The source or runtime operations that create a task, attach it to a scope, wait for it, or produce its result are not defined by this revision.

## Borrowed and permission-bearing state

Exec work receives Core values, owned resources, or permission-bearing borrows or views according to the applicable ownership, resource, and bridge rules.

A child task may use a scope-bounded borrow or view only while that permission remains semantically valid. When a child is required to complete before a structured scope can complete normally, that task relationship does not by itself extend, renew, copy, or upgrade the validity of any borrow or view used by the child.

If a permission would cease to be valid before the child finishes every use that depends on it, the child work is not made valid merely by being attached. The surrounding ownership and scope relationships must instead ensure that the permission remains valid through every dependent use. This rule does not require a permission to remain live after the child's last use merely because the child itself continues executing.

When a structured task scope is the semantic boundary whose continued activity keeps scope-bounded child state valid, that scope cannot complete normally while an attached child may still perform a use that depends on that state.

This revision does not define a new Exec borrowing system, first-class source references, source lifetime inference, or a runtime borrow guard.

## Detachment

A child task is **detached from an originating structured task scope** when it no longer participates in that scope's normal-completion set.

Detachment therefore removes only the originating scope's structured completion obligation. It does not guarantee asynchrony, parallel execution, progress, survival, a physical execution target, or immunity from future cancellation semantics. It likewise does not itself establish an ordering relationship between the detached task and the originating scope's normal continuation.

A detached task MUST NOT depend, after detachment, on state whose validity is bounded only by the originating scope once that scope may complete. Every state dependency the detached work still requires MUST instead be either:

- owned by the detached work; or
- independently retained by a semantic contract whose validity does not depend on the originating scope remaining active.

Detachment does not convert a borrow or view into ownership or independent retention and does not extend the validity of borrowed state. State no longer required by the detached work imposes no retention obligation merely because it was used before detachment.

The operation that performs detachment, whether every task form is detachable, and any later operation that explicitly waits for or reorders detached work are not defined by this revision.

## Interaction with other Exec semantics

Structured task-scope membership is not a synchronization mechanism between sibling child tasks.

Ordinary accesses by child tasks remain governed by [Exec memory model](memory-model.md), and attachment does not legalize an otherwise-conflicting unordered ordinary interaction.

Buffer logical coherence may consume the semantic order from an attached child's normal completion to the structured scope's normal continuation according to [Exec Buffers](resources/buffers.md). Detachment alone supplies no corresponding ordering or visibility relationship after the originating scope may continue.

The exact spawn, await, cancellation, task result, and task fault-propagation rules remain not defined by this revision.
