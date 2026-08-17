# Exec Parallelism

Status: **provisional normative; incomplete**

Ordered sequential iteration preserves source-defined relative iteration order.

`each`-style structured parallel iteration removes source-defined relative order among iterations.

Safe inter-iteration interaction requires an explicit legal interaction model such as disjoint mutation, shared reads, atomics, reductions, commutative accumulation, or collectives.

Unordered physical scheduling does not by itself imply semantic nondeterminism.

A reduction realization MAY exploit only algebraic laws guaranteed by its operator contract.

Hierarchical execution concepts such as groups, subgroups, group-local storage, barriers, broadcast, and shuffle belong to Exec. Their precise portable semantics are not defined by this revision.