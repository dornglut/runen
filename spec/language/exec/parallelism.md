# Exec Parallelism

Status: **provisional normative; incomplete**

Ordered sequential iteration preserves source-defined relative iteration order.

`each`-style structured parallel iteration removes source-defined relative order among iterations.

Safe inter-iteration interaction requires an explicit legal interaction model. Ordinary non-atomic inter-iteration access is governed by the conflict and unordered-access rules in [Exec memory model](memory-model.md). Atomics, reductions, commutative accumulation, and collectives require their own defined contracts; listing them as interaction categories does not itself authorize behavior that those contracts have not yet defined.

Unordered physical scheduling does not by itself imply semantic nondeterminism.

A reduction realization MAY exploit only algebraic laws guaranteed by its operator contract.

Hierarchical execution concepts such as groups, subgroups, group-local storage, barriers, broadcast, and shuffle belong to Exec. Their precise portable semantics are not defined by this revision.
