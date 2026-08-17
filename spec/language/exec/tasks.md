# Exec Tasks

Status: **provisional normative; incomplete**

Exec owns execution-visible work whose legal physical realization may vary.

A normal function executes in its current execution context.

A task denotes computation visible to realization as an independent execution unit. Being a task does not by itself guarantee asynchrony, parallel execution, or a particular execution target.

Borrowed resources used by child work MUST NOT silently outlive the scope that makes those borrows valid. Detached work MUST own or independently retain all state it requires.

The exact spawn, await, cancellation, and task fault-propagation rules are not defined by this revision.
