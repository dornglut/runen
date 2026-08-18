# Exec Memory Model

Status: **provisional normative; incomplete**

This document owns Exec access-conflict, data-race, memory-ordering, and synchronization relations. Resource-specific identity, region, and state semantics remain owned by the semantic owner of the storage or resource being accessed.

## Ordinary non-atomic access

An **ordinary non-atomic access** is a semantic access to one storage or resource region that is not defined by an atomic operation.

The canonical owner of the accessed storage or resource defines its region identity, overlap relation, and state transitions. Exec consumes those facts; it does not reinterpret them from host addresses, allocator identity, or backend memory operations.

For the conflict relation, an ordinary access is either:

- a **non-state-changing read**, which observes the selected region without changing its semantic value or storage state; or
- a **state-changing access**, which changes the selected region's semantic value or storage state, including applicable initialization, reinitialization, replacement, destruction, or ownership-consuming transfer.

Two ordinary non-atomic accesses **conflict** exactly when their semantic regions overlap and at least one of the accesses is state-changing.

This conflict relation does not erase additional effect, ordering, authority, validity, or resource rules owned elsewhere. An access that does not conflict under this relation is legal only when every other applicable semantic contract also permits it.

For Core-origin storage crossing into Exec, Core remains the owner of storage extent, stored-value state, structural region facts, place overlap, and alias authority. In particular, Core interior mutability under shared aliases is not by itself an Exec synchronization mechanism or permission for conflicting unordered execution.

## Unordered ordinary access

Access legality is determined by semantic ordering and interaction contracts, not by incidental physical scheduler order.

For source-unordered work with no separately defined synchronization or interaction mechanism:

- accesses to disjoint semantic regions do not conflict under the ordinary-access relation;
- overlapping non-state-changing reads do not conflict under the ordinary-access relation;
- a conflicting pair of ordinary non-atomic accesses is not a legal safe interaction.

Therefore a realization that happens to serialize source-unordered conflicting accesses does not make them legal, and physically parallel execution does not by itself make an otherwise permitted non-conflicting interaction illegal.

This is a language/profile semantic legality rule. It is not environment admission and is not an optimization preference. The source-language or compiler mechanism by which a program establishes the required interaction contract is not defined by this revision.

No host, hardware, or backend memory model is normative by default.

## Open memory-model rules

The complete cross-realization memory model, the complete data-race definition involving synchronization or atomic operations, atomic order vocabulary, atomic scope lattice, and synchronization relations are not defined by this revision. Their absence does not authorize additional conflicting ordinary accesses.
