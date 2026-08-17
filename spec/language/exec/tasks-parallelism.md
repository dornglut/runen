# Exec Tasks and Parallelism

Status: **provisional normative**

Exec owns execution-visible work whose legal physical realization may vary.

## Functions and tasks

A normal function executes in its current execution context.

A task denotes computation visible to realization as an independent execution unit. A task may realize as a direct call, vectorized work, multicore work, GPU dispatch, accelerator operation, or another admitted realization when its contracts allow it.

Being a task does not itself guarantee asynchrony or GPU execution.

Borrowed resources used by child work MUST NOT silently outlive the scope that makes those borrows valid. Detached work must own or independently retain all state it requires.

Exact spawn, await, cancellation, and fault-propagation rules are unspecified in this revision.

## Ordered and unordered iteration

Ordered sequential iteration preserves source-defined relative iteration order.

`each`-style structured parallel iteration removes source-defined relative order among iterations.

Safe inter-iteration interaction requires an explicit valid interaction model such as disjoint mutation, shared reads, atomics, reductions, commutative accumulation, or collectives.

Unordered physical scheduling does not by itself imply semantic nondeterminism.

## Parallel patterns and transformations

Map, reduce, scan, partition, tile, and similar patterns may carry algebraic contracts that permit stronger realization freedom.

A reduction implementation MAY exploit only algebraic laws guaranteed by its operator contract.

Runen distinguishes:

- **algorithm/implementation** — defines the computation;
- **schedule** — changes physical arrangement while preserving permitted behavior;
- **specialization** — an alternative implementation of the same public semantic operation under stated assumptions.

A schedule transformation or specialization must preserve the applicable semantic contract.