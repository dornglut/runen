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

## Atomic element-location and logical-state bridge

When an applicable contract admits an atomic exchange for one logical element position of a Buffer, that position is one **Buffer atomic location** governed by the atomic-exchange semantics in [Exec memory model](../memory-model.md).

The semantic identity of a Buffer atomic location is exactly its Buffer identity together with its selected logical element position. The same logical position of the same Buffer denotes the same Buffer atomic location. Another logical position or a position belonging to another Buffer denotes a distinct Buffer atomic location. Atomic location identity is not physical address, allocation identity, mapping identity, execution-agent identity, cache-line identity, backend atomic identity, or another realization token.

The stored value of that atomic location is exactly the value of its selected position in the one logical Buffer state. The location does not own a second atomic-only semantic value. An admitted atomic exchange observes and replaces that same logical value according to the atomic exchange and location-local modification-order contract owned by Exec memory model.

Therefore, after an accepted atomic exchange changes a Buffer atomic location, the one logical Buffer state contains the resulting location value. A realization MUST NOT make an independently selectable atomic-only semantic copy, replica, cache value, or staging value observable as a second Buffer state. Physical atomic state may exist only as realization state that preserves the one logical Buffer state and every applicable memory rule.

The logical footprint of one Buffer atomic location is the singleton Buffer region containing exactly its selected position. This establishes only the Buffer-owned identity and overlap fact for that location. Mixed ordinary non-atomic/atomic conflict and source-unordered legality for an overlapping ordinary access are owned by [Exec memory model](../memory-model.md). When an ordinary Buffer access and an admitted atomic exchange are separately permitted and semantic order is already established between them, the logical-coherence rule above supplies the later access's logical pre-state for their shared position. Therefore an ordinary state change ordered before an exchange is included in that exchange's read pre-state, and an accepted exchange ordered before a later permitted ordinary read is included in that read's logical pre-state. This ordered-coherence consequence does not put the ordinary access into atomic modification order. The overlap fact by itself establishes no semantic order, synchronization, atomic access permission, or complete mixed data-race classification.

This bridge is conditional on an atomic exchange already being admitted by an applicable contract. It does not declare every `Buffer<T>` element atomic-capable, define which element types or replacement values admit atomic exchange, or define source/profile/library syntax for requesting atomic access. `View` and `ViewMut` retain only the ordinary access permissions defined above; neither becomes atomic access authority through this bridge.

The address-free typed mapping below defines the first physical servicing relation for an already-admitted Buffer atomic exchange. That physical relation does not itself admit an exchange or change the atomic location/state contract above.

## Address-free typed mapping

A **typed Buffer mapping** is a temporary realization relation that makes exactly one selected Buffer region physically accessible to one selected physical execution agent through one live physical allocation for the duration of that mapping. Physical execution-agent identity is defined by [Exec Realization](../realization.md); physical allocation identity, extent, and execution-agent accessibility are defined by [Exec Allocations](allocations.md).

A typed mapping MUST NOT be established unless its selected allocation is accessible to its selected physical execution agent for the duration required by that mapping. The mapping binds to that exact allocation and exact execution agent for its occurrence.

The mapping's logical selection is exactly its Buffer identity and Buffer region. Its physical allocation and execution agent do not become the Buffer's logical identity, and migration, replication, staging, placement, or other physical work MUST NOT retarget the mapping to another logical Buffer, region, allocation, or execution agent.

A typed mapping is not a `View` or `ViewMut` and grants no semantic read or state-changing access permission. It does not create ownership, a borrow, synchronization, ordering, visibility by itself, or an exemption from the ordinary access-conflict rules. Every semantic access serviced through a mapping MUST independently satisfy the applicable view/ownership, memory, effect, validity, and operation-specific contracts.

A mapped access under this contract is a typed semantic Buffer access to logical element positions inside the mapped region, physically serviced through that mapping's exact execution-agent/allocation relation. It is not a byte access, raw load or store, ABI operation, pointer dereference, observation of a numeric or physical address, or source-visible observation of the selected execution agent.

An active typed mapping MAY physically service an already-admitted atomic exchange only when the exact singleton region of that Buffer atomic location is contained in the mapping's selected Buffer region. Such servicing uses that mapping occurrence's exact selected physical execution agent and exact live physical allocation relation for the required physical-access interval. Mapping coverage does not admit atomic access, make every mapped element atomic-capable, or grant atomic authority to a `View`, `ViewMut`, or mapping occurrence.

A mapped atomic exchange remains an exchange on the same Buffer atomic location and the same one logical Buffer state. The realization MUST preserve the atomic exchange's indivisible observe-and-replace behavior, the one location-local modification order, every applicable semantic-order constraint on that order, exchange class, synchronization scope, and synchronization relation owned by [Exec memory model](../memory-model.md). Mapping identity, allocation identity, execution-agent identity, physical placement, or physical servicing order MUST NOT create another atomic location, partition that modification order, establish semantic order, or change scope compatibility.

For every mapped semantic access, including a mapped atomic exchange, the physical state used to service that access MUST represent the logical pre-state required by the logical-coherence contract above for the positions selected by the access. A stale staging allocation, cached copy, or other backing state MUST NOT cause a mapped access to omit an earlier semantically ordered logical state change.

A state-changing mapped access changes the one logical Buffer state according to that access's operation-specific semantics. Physical propagation, copy-back, transfer completion, cache update, or other realization work may occur separately only while every later semantically ordered access still receives the logical pre-state required above. No second semantic Buffer state is created by mapping.

Multiple legal mappings of the same logical Buffer region may use distinct physical allocations, distinct physical execution agents, or both. Those realization choices do not create distinct logical Buffer states: every such mapping remains subject to the one logical Buffer identity, region, state, and coherence contract above. For atomic exchange, distinct legal mapping choices likewise do not create distinct semantic atomic locations or mapping-specific modification orders.

Beginning or ending a typed mapping does not itself create semantic order, synchronization, ownership transfer, or Buffer visibility. In particular, beginning or ending a mapping is not an implicit atomic fence, flush, invalidate, barrier, task join, or publication event. Source-unordered conflicting accesses do not become legal merely because a realization maps or physically serializes them, including when the accesses are serviced by the same physical execution agent.

A legal realization MAY service a typed mapping directly from existing physical backing or through a staging/copying strategy. For mapped atomic exchange, any such choice is legal only when the complete atomic-exchange and Buffer contracts above are preserved, including indivisibility and every required logical-state and synchronization consequence. The chosen allocation identity, execution-agent identity, physical copy, transfer schedule, cache state, and mapping implementation are realization details and are not program-observable under this contract.

Ending a typed mapping terminates only that temporary physical-accessibility relation. It does not change Buffer identity, logical region identity, existing `View` / `ViewMut` permission semantics, atomic-location identity, or atomic admission. The backing allocation remains subject to the allocation-extent and execution-agent-accessibility requirements owned by [Exec Allocations](allocations.md).

This mapping form exposes no pointer, raw or numeric address, byte sequence, bit pattern, layout, alignment, stride, physical contiguity, or representation. It therefore establishes no address-stability, pinning, pointer-provenance, or representation-validity contract.

## Realization and open coherence mechanisms

A legal realization may maintain, migrate, replicate, stage, or replace physical backing only according to the logical coherence and typed-mapping contracts above and the applicable Exec memory rules.

This revision does not yet define a Buffer version representation, transfer-completion protocol, physical ownership or dirty-state protocol, replica directory, raw-address or byte-level mapping form, relocation mechanism, atomic Buffer operations beyond the admitted exchange bridge above, complete mixed ordinary/atomic data-race rules beyond the conflict/source-unordered legality relation owned by [Exec memory model](../memory-model.md), a hardware/backend atomic instruction contract, or a protocol for jointly servicing one atomic location through multiple physical mappings. It likewise does not define which physical copy services an access except for the typed mapping's requirement that its selected live allocation and selected physical execution agent form an accessible relation that can service that mapping while preserving logical coherence and every applicable atomic rule.

## Address exposure

The typed Buffer mapping defined above does not expose a raw physical address.

Any future raw-address exposure requires its own contract for address validity, relocation, and when necessary pinning, together with any Core pointer/provenance or representation rules made observable by that operation.

A `View`, `ViewMut`, or address-free typed mapping by itself does not establish such an address-stability contract.
