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

## Realization and coherence boundary

A legal realization may maintain, migrate, or replicate physical backing only according to the Buffer coherence contract and the applicable Exec memory rules.

Because one `Buffer<T>` denotes one logical coherent resource, multiple physical backing copies or placements are realization state rather than independently selectable logical Buffer states. A realization MUST preserve the behavior of accesses to the same logical Buffer and regions rather than expose replica identity as an additional source-level choice.

This revision does not yet define the precise Buffer version, visibility, transfer-completion, coherence, mapping, relocation, or multi-realization state machine. In particular, it does not define version counters, dirty states, ownership states, replica protocols, or rules selecting which physical copy services an access.

## Address exposure

Exposing a raw physical address requires a stable allocation or an explicit mapped or pinned realization whose contract preserves address validity for the required duration.

A `View` or `ViewMut` by itself does not establish that address-stability contract.
