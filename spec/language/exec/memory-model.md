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

## Structured barrier synchronization

The full-`each` structured barrier defined by [Exec parallelism](parallelism.md) is an explicit synchronization mechanism for ordinary accesses.

The phase relation owned by that construct induces this memory-model ordering for one normally completed barrier instance: every ordinary non-atomic access in every required iteration's before-barrier phase is ordered before every ordinary non-atomic access in every required iteration's after-barrier phase.

This relationship is semantic. It does not derive from physical arrival order, release order, worker scheduling, cache operations, or backend queue behavior.

The barrier does not change the ordinary conflict relation. Two overlapping ordinary accesses with at least one state-changing access still conflict. When one such access is in a before-barrier phase and the other is in an after-barrier phase of the same normally completed barrier instance, the barrier supplies semantic order between them, so they are not a source-unordered conflicting pair under the rule above. Every other applicable authority, validity, effect, resource, and operation-specific contract still has to permit both accesses.

Conflicting ordinary accesses performed by sibling iterations within the same before-barrier phase or within the same after-barrier phase remain source-unordered unless another semantic contract orders them.

A different barrier identity supplies no memory order by identity alone. Ordering between ordinary accesses around distinct barriers must arise from their placement or another applicable semantic relationship in the enclosing execution.

Buffer logical coherence consumes this barrier-established access order through its own canonical contract; this document does not redefine Buffer state or visibility.

No host, hardware, or backend memory model is normative by default.

## Open memory-model rules

The complete cross-realization memory model, the complete data-race definition involving synchronization or atomic operations beyond the structured barrier relation above, atomic order vocabulary, atomic scope lattice, and additional synchronization relations are not defined by this revision. Their absence does not authorize additional conflicting ordinary accesses.
