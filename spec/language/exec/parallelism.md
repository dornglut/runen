# Exec Parallelism

Status: **provisional normative; incomplete**

Ordered sequential iteration preserves source-defined relative iteration order.

## Structured unordered iteration

`each`-style structured parallel iteration removes source-defined relative order among sibling iterations of one execution.

Semantic actions sequenced before entry to an `each` in its containing execution context occur before actions performed by its iterations when those preceding actions belong to the same defined continuation.

An `each` execution completes normally only after every iteration required by that execution has completed normally. Actions sequenced in the normal continuation after that completed `each` occur after the actions performed by every completed iteration of the `each`.

These entry and normal-completion relationships do not impose a relative order among sibling iterations. In particular, a backend that happens to execute sibling iterations serially does not thereby create source-defined inter-iteration order.

A legal realization MAY execute sibling iterations serially, concurrently, vectorized, in chunks, on an accelerator, or using another permitted physical schedule, but it MUST preserve the same entry, sibling-unordered, and normal-completion relationships.

This revision defines only normal structured completion. The behavior of an `each` execution when an iteration faults, is cancelled, diverges, or otherwise does not complete normally is not defined by this revision.

## Inter-iteration interaction

Safe inter-iteration interaction requires an explicit legal interaction model. Ordinary non-atomic inter-iteration access is governed by the conflict and unordered-access rules in [Exec memory model](memory-model.md). Atomics, reductions, commutative accumulation, and collectives require their own defined contracts; listing them as interaction categories does not itself authorize behavior that those contracts have not yet defined.

The structured entry/completion boundary does not legalize a conflicting ordinary pair of sibling-iteration accesses. Conversely, disjoint or otherwise legally interacting sibling work need not be physically serialized merely because normal continuation waits for the structured operation to complete.

Buffer logical coherence consumes the semantic ordering relationships established here according to [Exec Buffers](resources/buffers.md); this document does not redefine Buffer visibility or coherence.

Unordered physical scheduling does not by itself imply semantic nondeterminism.

A reduction realization MAY exploit only algebraic laws guaranteed by its operator contract.

Hierarchical execution concepts such as groups, subgroups, group-local storage, barriers, broadcast, and shuffle belong to Exec. Their precise portable semantics are not defined by this revision.
