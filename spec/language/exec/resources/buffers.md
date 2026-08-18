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

## Realization and open coherence mechanisms

A legal realization may maintain, migrate, or replicate physical backing only according to the logical coherence contract above and the applicable Exec memory rules.

This revision does not yet define a Buffer version representation, transfer-completion protocol, physical ownership or dirty-state protocol, replica directory, mapping state machine, relocation mechanism, atomic access to Buffer storage, or mixed atomic/non-atomic Buffer access or visibility rules. It likewise does not define which physical copy services an access.

## Address exposure

Exposing a raw physical address requires a stable allocation or an explicit mapped or pinned realization whose contract preserves address validity for the required duration.

A `View` or `ViewMut` by itself does not establish that address-stability contract.
