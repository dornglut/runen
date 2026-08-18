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

## Atomic exchange base relation

An **atomic exchange** is a normally completing read-modify-write interaction with exactly one semantic location.

The canonical owner of the targeted storage or resource defines that location's semantic identity, stored-value state, and whether the desired replacement value and replacement operation are admitted. This rule does not infer those facts from an address, allocation, backend atomic type, or machine instruction, and it does not define which source storage forms support atomic access.

For one admitted exchange targeting location `L` with desired value `d`, the operation indivisibly observes the value immediately preceding its replacement under the modification-order rule below, replaces the value of `L` with `d`, and returns that preceding value. The observe-and-replace step is indivisible with respect to every other atomic exchange governed by this contract on the same semantic location.

### Location-local modification order

All normally completing atomic exchanges governed by this contract that modify one semantic location occur in one total **modification order** for that location.

For a represented set of such exchanges, the first exchange returns the location value immediately before that set begins. Every later exchange returns the desired value installed by the immediately preceding exchange in modification order. After the final exchange, the location value is the desired value installed by that final exchange.

Any applicable Runen semantic ordering already established between two exchanges constrains their modification order. If exchange `A` is semantically ordered before exchange `B`, then `A` MUST precede `B` in the modification order of their common location.

Two source-unordered exchanges on the same location for which no applicable semantic rule establishes a relative order may appear in either relative position in that location's modification order, subject to all other established constraints. The prior values and final location value resulting from every such permitted order are explicitly permitted behaviors of this operation under [Correctness Relations](../correctness.md).

Incidental physical worker order, scheduler order, queue order, cache behavior, or instruction choice is not additional semantic input. A realization MUST NOT produce an exchange result that cannot arise from a permitted location-local modification order.

Modification orders of distinct semantic locations are independent under this base relation. This rule does not create one global order across atomic locations.

### Deliberate synchronization boundary

This revision defines only atomic-exchange indivisibility and location-local modification order. It supplies no synchronization or visibility ordering for ordinary non-atomic accesses or for accesses to another semantic location.

Acquire, release, acquire-release, sequential consistency, fences, atomic scope, mixed atomic/non-atomic access rules, the complete data-race rule involving atomics, other atomic operations, progress guarantees, and source atomic syntax or types are not defined by this revision. Their absence does not authorize behavior beyond rules already established by their canonical owners.

## Structured barrier synchronization

The cohort-scoped structured barrier defined by [Exec parallelism](parallelism.md) is an explicit synchronization mechanism for ordinary accesses among its selected participants.

The phase relation owned by that construct induces this memory-model ordering for one normally completed barrier instance: every ordinary non-atomic access by every participant in its before-barrier phase is ordered before every ordinary non-atomic access by every participant in its after-barrier phase.

An iteration outside the selected barrier cohort is a nonparticipant. An ordinary access performed by a nonparticipant receives no ordering or visibility guarantee from that barrier alone.

This relationship is semantic. It does not derive from physical arrival order, release order, worker scheduling, cache operations, backend queue behavior, or the fact that two iterations share a hierarchy group or subgroup.

The barrier does not change the ordinary conflict relation. Two overlapping ordinary accesses with at least one state-changing access still conflict. When one such participant access is in a before-barrier phase and the other participant access is in an after-barrier phase of the same normally completed barrier instance, the barrier supplies semantic order between them, so they are not a source-unordered conflicting pair under the rule above. Every other applicable authority, validity, effect, resource, and operation-specific contract still has to permit both accesses.

Conflicting ordinary accesses performed by sibling participants within the same before-barrier phase or within the same after-barrier phase remain source-unordered unless another semantic contract orders them.

Barrier identity alone does not order distinct barrier instances. Neither overlap/nesting between two selected barrier cohorts nor hierarchy membership alone supplies memory order. Ordering between ordinary accesses around distinct barriers must arise from their placement or another applicable semantic relationship in the enclosing execution.

Buffer logical coherence consumes this barrier-established participant access order through its own canonical contract; this document does not redefine Buffer state or visibility.

No host, hardware, or backend memory model is normative by default.

## Open memory-model rules

The complete cross-realization memory model, the complete data-race definition involving synchronization or atomic operations beyond the atomic-exchange and structured-barrier relations above, additional atomic order vocabulary, atomic scope lattice, mixed atomic/non-atomic access rules, and additional synchronization relations are not defined by this revision. Their absence does not authorize additional conflicting ordinary accesses.
