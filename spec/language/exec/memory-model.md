# Exec Memory Model

Status: **provisional normative boundary; formal model incomplete**

Safe Runen does not permit conflicting ordinary non-atomic accesses unless the applicable permission and synchronization relationships make those accesses legal.

No host-language, CPU, GPU, or backend memory model is normative by default.

The formal cross-realization memory model, atomic order vocabulary, atomic scope lattice, synchronization relations, and data-race definition are unspecified in this revision.

Hierarchical group/subgroup operations, group-local memory, barriers, broadcast/shuffle, and related facilities belong to Exec rather than a separate GPU language. Their portable guarantees and APIs are unspecified in this revision.