# Exec Tasks

Status: **provisional normative; incomplete**

Exec owns execution-visible work whose legal physical realization may vary.

A normal function executes in its current execution context.

A task denotes computation visible to realization as an independent execution unit. Being a task does not by itself guarantee asynchrony, parallel execution, progress, or a particular execution target.

## Structured task scope

A **structured task scope** is a semantic execution boundary with a set of child tasks attached to that scope for normal-completion purposes.

One dynamic structured task scope has opaque semantic identity only as needed to distinguish its attachment, detachment, and normal-continuation relationships from those of another dynamic structured task scope. Scope identity is not a numeric index, source handle, nesting depth, runtime parent object, executor identity, scheduler token, or ordering relation. Equal implementation or debug tokens used for distinct semantic scope instances do not by themselves identify one shared scope.

Attachment is a semantic lifetime and ordering relationship relative to that scope. It does not imply a host thread, executor, worker, queue, physical parent task, scheduling policy, or execution target.

A structured task scope completes normally only after every child task that remains attached to that scope's normal-completion set has completed normally. Actions in that scope's normal continuation occur after the actions of every such normally completed attached child of that same scope.

Attachment to the same structured task scope does not by itself establish a relative order between two child tasks. A child attached to one dynamic scope receives no order to a distinct dynamic scope's normal continuation merely because task/debug identities match or a realization nests or serializes the scopes. Any ordering or interaction between child tasks or distinct scopes requires another applicable semantic contract.

This revision defines only normal structured task-scope completion. If an attached child faults, is cancelled, diverges, or otherwise fails to complete normally, the scope's resulting fault, cancellation, result, and completion behavior are not defined by this revision.

The source or runtime operations that create a task, create or identify a structured task scope, attach a child to a scope, wait for it, or produce its result are not defined by this revision.

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

## Cooperative cancellation observation

This revision defines the first task-cancellation contract only for an explicitly semantically sequenced history of cancellation requests and cancellation observations for one task.

A **cancellation request** targets one task. The validity and authority by which a requester may target that task are not defined by this revision. When an applicable execution contract establishes a valid cancellation-request transition before a later cancellation observation of that task, cancellation is pending for that later observation.

A pending cancellation request does not by itself terminate the task, interrupt its current semantic action, order sibling tasks, guarantee progress, or require the task ever to reach a cancellation observation point. Repeated valid requests established while cancellation is already pending add no further cancellation effect beyond leaving cancellation pending.

A **cancellation observation point** is an explicit semantic point belonging to the target task.

When the task reaches such a point in an explicitly sequenced history:

- if no cancellation request is pending for that task, cancellation observation completes normally and the task may continue;
- if a cancellation request is pending for that task, the task begins defined cancellation termination at that observation point.

Defined cancellation termination consumes the function-termination cleanup consequence owned by [Core value and storage semantics](../core/value-storage.md) for represented active Core function storage. After that applicable cleanup, the task has the terminal **cancelled** outcome permitted by [Program Behavior](../behavior.md). Cancellation is not a Core `Fault`, normal completion, or divergence.

After the task reaches the terminal cancelled outcome, it performs no further ordinary task-body actions. This rule does not define a general unwind stack, catch behavior, cancellation propagation, custom destructor bodies, task results, or fault aggregation.

This cancellation contract supplies no semantic order between a cancellation request performed in one execution context and an observation performed in another. If request and observation are source-unordered and no other semantic contract orders or otherwise defines their interaction, their relative cancellation effect is **not defined by this revision**. Incidental host timing, worker scheduling, queue order, or physical arrival MUST NOT decide whether the observation sees the request.

A cancelled attached child has not completed normally and therefore does not satisfy the structured scope's existing normal-completion obligation. This revision does not define whether its containing structured scope then cancels, faults, cancels siblings, aggregates failure, retries work, exposes a result, or reaches another completion mode.

Detachment does not make a task immune from an otherwise valid cancellation request. Detachment still changes only the originating scope's normal-completion obligation and state-retention requirements; this revision does not define cancellation-request ownership or authority for detached work.

No implicit cancellation point, asynchronous interruption, timer, deadline, polling frequency, fairness guarantee, cancellation masking, or guarantee of eventual cancellation is defined by this revision.

## Interaction with other Exec semantics

Structured task-scope membership is not a synchronization mechanism between sibling child tasks.

Ordinary accesses by child tasks remain governed by [Exec memory model](memory-model.md), and attachment does not legalize an otherwise-conflicting unordered ordinary interaction. Cancellation request, pending state, and cancellation observation likewise do not by themselves legalize, order, or synchronize an otherwise-conflicting sibling interaction.

Buffer logical coherence may consume the semantic order from an attached child's normal completion to that same structured task scope's normal continuation according to [Exec Buffers](resources/buffers.md). Detachment alone supplies no corresponding ordering or visibility relationship after the originating scope may continue.

The exact spawn, await, task result, task fault-propagation, cancellation-request authority, source-unordered request/observation interaction, containing-scope abnormal-completion rules, source task-scope formation/identity, and scope nesting relationships remain not defined by this revision.
