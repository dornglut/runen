# Exec Verification Contract

Status: **non-normative conformance-obligation documentation**

This document records focused assurance obligations for defined Exec semantic slices. It does not define Runen semantics, conformance profiles, compiler architecture, or repository CI.

The normative ordinary-access rules exercised here are owned by `spec/language/exec/memory-model.md` and `spec/language/exec/parallelism.md`. Buffer-specific identity, region, and view-access facts are owned by `spec/language/exec/resources/buffers.md`. Core storage, overlap, borrowing, and interior-mutability facts remain owned by their Core specifications.

## Ordinary unordered-access boundary

The repository does not yet have an accepted executable Exec representation. The ordinary-access boundary is therefore evidenced with focused semantic litmus obligations rather than by inventing an Exec IR, scheduler, runtime, or backend model solely for testing.

In the cases below, region identity and overlap come from the canonical owner of the accessed storage or resource. `unordered` means that the language semantics provide no relative order between the two pieces of work; incidental backend execution order is ignored. A case described as non-conflicting passes only the ordinary Exec conflict relation; every other applicable semantic contract still has to permit the accesses.

Required cases:

- unordered `read(A)` with `read(A)` is non-conflicting under the ordinary-access relation;
- unordered state-changing access to `A` with state-changing access to disjoint `B` is non-conflicting under the ordinary-access relation;
- unordered `read(A)` with a state-changing access to overlapping `A` is not a legal ordinary safe interaction when no separately defined mechanism applies;
- unordered state-changing access to `A` with another state-changing access to overlapping `A` is not a legal ordinary safe interaction when no separately defined mechanism applies;
- semantically ordered sequential accesses to the same region are not rejected merely because the corresponding unordered pair would conflict;
- a backend that serializes source-unordered conflicting accesses does not make the interaction legal;
- physically parallel execution of a non-conflicting pair does not by itself make that pair conflicting;
- Core interior mutability under shared aliases does not by itself legalize overlapping state-changing accesses from source-unordered Exec work;
- for Core-origin structural storage, root/descendant overlap and sibling-field disjointness are consumed as Core-owned facts rather than redefined by Exec.

These obligations do not authorize atomics, reductions, commutative accumulation, collectives, barriers, or other synchronization mechanisms whose normative contracts remain open.

## Buffer logical-region boundary

Buffer cases consume the Buffer owner's logical identity and overlap relation and the memory-model owner's ordinary conflict relation. They must not infer semantics from physical storage arrangement.

Required cases:

- regions belonging to distinct Buffer identities are disjoint under the Buffer region relation even if a realization can co-locate, reuse, or otherwise relate their physical backing;
- two regions of one Buffer with disjoint logical element-position sets are disjoint;
- two regions of one Buffer whose logical element-position sets intersect overlap;
- overlapping `View` read/read access is non-conflicting under the ordinary-access relation when every other applicable contract permits both accesses;
- source-unordered overlapping `View` read with a state-changing `ViewMut` access conflicts when no separately defined interaction mechanism applies;
- source-unordered overlapping state-changing `ViewMut` accesses conflict when no separately defined interaction mechanism applies;
- source-unordered state-changing accesses through `ViewMut` to disjoint Buffer regions are non-conflicting under the ordinary-access relation;
- migration, replication, or replacement of physical backing does not change the logical Buffer identity or Buffer region selected by a continuing logical view;
- physical address, allocation identity, host slice aliasing, host borrow checking, and host scheduler behavior are not semantic oracles for Buffer identity, overlap, view permission, or conflict.

These obligations do not define view construction/lifetime rules, mapping, raw-address exposure, version selection, visibility, synchronization, or a coherence protocol.

## Future executable evidence

When an accepted Exec operation or resource representation creates a concrete executable consumer, its reference/conformance tests should realize the applicable obligations above without using host thread scheduling, host memory ordering, physical addresses, or backend behavior as the semantic oracle.

A future executable representation may refine the evidence mechanism. It must not silently strengthen, weaken, or replace the normative owners listed above.
