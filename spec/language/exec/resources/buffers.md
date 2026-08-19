# Exec Buffers

Status: **provisional normative; incomplete**

`Buffer<T>` is a logical coherent Exec resource. Its logical identity is distinct from any one physical backing allocation or raw address.

## Logical element domain

A `Buffer<T>` has one logical element domain whose positions identify the semantic elements belonging to that Buffer resource.

A logical element position is not a byte offset, physical address, allocation coordinate, ABI-layout fact, or promise of physical contiguity. This revision does not define source-level indexing syntax, range syntax, dimensional shape, stride, or physical layout.

Changing, migrating, replicating, or replacing physical backing does not by itself change the Buffer's logical identity or logical element domain.

## Buffer regions and overlap

A **Buffer region** selects a set of logical element positions within one Buffer identity.

Two Buffer regions overlap exactly when they belong to the same logical Buffer identity and their selected logical element-position sets intersect.

Regions belonging to distinct Buffer identities are disjoint under the Buffer region relation. A physical realization MUST NOT make logically distinct Buffer resources interfere observably merely because backing allocations are co-located, reused, or otherwise related physically.

The Buffer region relation is the resource-specific region and overlap relation consumed by the ordinary Exec access-conflict rules in [Exec memory model](../memory-model.md). This document does not redefine that general conflict relation.

## Views

`View` and `ViewMut` are logical permission-bearing views selecting Buffer regions. They do not promise permanently stable raw physical addresses.

For the currently defined access classes:

- `View` carries permission for non-state-changing semantic reads of its selected Buffer region when every other applicable rule permits the operation;
- `ViewMut` carries permission for non-state-changing reads and state-changing ordinary Buffer accesses to its selected region when every other applicable rule permits the operation.

These permissions do not themselves authorize a pair of conflicting source-unordered accesses, establish synchronization, weaken another authority rule, or make an otherwise-invalid operation legal.

The source or library mechanism for constructing, deriving, slicing, storing, transferring, or ending a view is not defined by this revision. View lifetime inference, runtime borrow guards, source reference representation, and concrete public API spelling are likewise not defined here.

A view selects logical Buffer state rather than one physical backing instance. Physical migration, replication, or backing replacement therefore does not by itself retarget an existing logical view to a different Buffer identity or Buffer region.

## Logical coherence and ordered visibility

Semantic Buffer accesses operate on one logical Buffer state. Physical backing copies, caches, placements, migration images, or replicas are realization state rather than independently selectable semantic Buffer states.

When the applicable Runen semantics establish that a state-changing Buffer access `A` occurs before a later Buffer access `B`, the logical pre-state presented to `B` for every logical element position selected by both accesses includes the state transition of `A` and every other state-changing access to that position semantically ordered between `A` and `B`. Any read component of `B` reads from that logical pre-state, and any state transition performed by `B` is applied to that pre-state according to `B`'s operation-specific semantics.

A realization MUST NOT service such a later access from stale physical backing in a way that omits an earlier semantically ordered state change from `B`'s logical pre-state or resurrects an older logical state for a read performed by `B`.

Buffer coherence consumes semantic order established by the applicable program or execution contract; it does not create order between source-unordered accesses. Conflicting source-unordered ordinary accesses remain governed by [Exec memory model](../memory-model.md) and are not legalized merely because a coherence mechanism could serialize or transfer their physical backing.

Coherence is region-local. State changes and visibility obligations for one Buffer region do not by themselves impose semantic order, transfer, or whole-Buffer serialization on a disjoint region. A realization MAY track, place, migrate, replicate, or synchronize disjoint regions independently when every applicable semantic contract is preserved.

Physical migration, replication, caching, or placement may occur before, during, or between accesses only when the resulting behavior is equivalent to access to the one logical Buffer state under the applicable ordering and memory rules. Replica identity, physical-copy selection, migration history, and implementation version metadata are not made program-observable by this contract.

## Address-free typed mapping

A **typed Buffer mapping** is a temporary realization relation that makes exactly one selected Buffer region physically accessible through one live physical allocation for the duration of that mapping. Physical allocation identity and allocation extent are defined by [Exec Allocations](allocations.md).

The mapping's logical selection is exactly its Buffer identity and Buffer region. Its physical allocation does not become the Buffer's logical identity, and migration, replication, staging, or other physical work MUST NOT retarget the mapping to another logical Buffer or region.

A typed mapping is not a `View` or `ViewMut` and grants no semantic read or state-changing access permission. It does not create ownership, a borrow, synchronization, ordering, visibility by itself, or an exemption from the ordinary access-conflict rules. Every semantic access serviced through a mapping MUST independently satisfy the applicable view/ownership, memory, effect, validity, and operation-specific contracts.

A mapped access under this contract is a typed semantic Buffer access to logical element positions inside the mapped region. It is not a byte access, raw load or store, ABI operation, pointer dereference, or observation of a numeric or physical address.

For every mapped semantic access, the physical state used to service that access MUST represent the logical pre-state required by the logical-coherence contract above for the positions selected by the access. A stale staging allocation, cached copy, or other backing state MUST NOT cause a mapped access to omit an earlier semantically ordered logical state change.

A state-changing mapped access changes the one logical Buffer state according to that access's operation-specific semantics. Physical propagation, copy-back, transfer completion, cache update, or other realization work may occur separately only while every later semantically ordered access still receives the logical pre-state required above. No second semantic Buffer state is created by mapping.

Beginning or ending a typed mapping does not itself create semantic order, synchronization, ownership transfer, or Buffer visibility. In particular, ending a mapping is not an implicit flush, fence, barrier, task join, or publication event. Source-unordered conflicting accesses do not become legal merely because a realization maps or physically serializes them.

A legal realization MAY service a typed mapping directly from existing physical backing or through a staging/copying strategy. The chosen allocation identity, physical copy, transfer schedule, cache state, and mapping implementation are realization details and are not program-observable under this contract.

Ending a typed mapping terminates only that temporary physical-accessibility relation. It does not change Buffer identity, logical region identity, or existing `View` / `ViewMut` permission semantics. The backing allocation remains subject to the allocation-extent requirement owned by [Exec Allocations](allocations.md).

This mapping form exposes no pointer, raw or numeric address, byte sequence, bit pattern, layout, alignment, stride, physical contiguity, or representation. It therefore establishes no address-stability, pinning, pointer-provenance, or representation-validity contract.

## Realization and open coherence mechanisms

A legal realization may maintain, migrate, replicate, stage, or replace physical backing only according to the logical coherence and typed-mapping contracts above and the applicable Exec memory rules.

This revision does not yet define a Buffer version representation, transfer-completion protocol, physical ownership or dirty-state protocol, replica directory, raw-address or byte-level mapping form, relocation mechanism, atomic access to Buffer storage, or mixed atomic/non-atomic Buffer access or visibility rules. It likewise does not define which physical copy services an access except for the typed mapping's requirement that its selected live allocation can service that mapping while preserving logical coherence.

## Address exposure

The typed Buffer mapping defined above does not expose a raw physical address.

Any future raw-address exposure requires its own contract for address validity, relocation, and when necessary pinning, together with any Core pointer/provenance or representation rules made observable by that operation.

A `View`, `ViewMut`, or address-free typed mapping by itself does not establish such an address-stability contract.
