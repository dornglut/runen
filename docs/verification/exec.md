# Exec Verification Contract

Status: **non-normative conformance-obligation documentation**

This document records focused assurance obligations for defined Exec semantic slices. It does not define Runen semantics, conformance profiles, compiler architecture, or repository CI.

The normative ordinary-access and structured-iteration rules exercised here are owned by `spec/language/exec/memory-model.md` and `spec/language/exec/parallelism.md`. Buffer-specific identity, region, view-access, and logical-coherence facts are owned by `spec/language/exec/resources/buffers.md`. Core storage, overlap, borrowing, and interior-mutability facts remain owned by their Core specifications.

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

These obligations do not define view construction/lifetime rules, mapping, raw-address exposure, version representation, synchronization, or a physical coherence protocol.

## Buffer ordered-coherence boundary

These cases exercise only the Buffer visibility consequence after some applicable semantic owner has already established an order between accesses. They do not infer semantic order from physical execution or backend submission.

Required cases:

- when a state-changing access to region `R` is semantically ordered before a permitted read of overlapping `R`, that read cannot observe the pre-change logical state solely because a stale physical replica exists;
- when several state-changing accesses to the same logical element positions are semantically ordered before a later permitted read, the read observes the logical state after those changes in their semantic order rather than an older physical copy;
- migration, replication, caching, or replacement of backing between an ordered state change and read does not change the logical result required by the established order;
- state changes and coherence work for region `A` do not require a disjoint region `B` to migrate, synchronize, or serialize unless another applicable contract requires it;
- Buffer coherence does not legalize source-unordered conflicting `View`/`ViewMut` or `ViewMut`/`ViewMut` ordinary accesses;
- selecting one physical replica rather than another is not program-observable when both realize the same required logical Buffer state;
- replica identity, implementation version metadata, physical address, allocation identity, backend queue order, host cache behavior, and transfer implementation are not semantic oracles for ordered Buffer visibility.

These obligations do not define version counters, replica ownership, transfer completion, future atomic/synchronization visibility, mapping, or a coherence implementation algorithm.

## Structured `each` normal-completion boundary

These cases exercise only normal structured entry/completion. They do not define abnormal iteration completion or infer sibling order from a physical schedule.

Required cases:

- a permitted state change sequenced before entry to `each` is semantically before a later permitted overlapping read performed by an iteration when both belong to the same defined continuation;
- sibling iterations remain source-unordered even when one legal realization executes them sequentially;
- source-unordered overlapping ordinary sibling read/write or write/write access remains conflicting under the accepted memory-model rule;
- source-unordered state-changing accesses to disjoint Buffer regions are non-conflicting under that rule and may execute physically concurrently;
- a normally completed `each` has no normal continuation until every required iteration has completed normally;
- after normal completion, a permitted continuation read of a Buffer region changed by an iteration receives logical state after that semantically ordered iteration change;
- after normal completion, the continuation may consume the combined effects of several disjoint sibling state changes without imposing a relative order among those sibling iterations;
- the normal join boundary makes no claim about an iteration that faults, is cancelled, diverges, or otherwise fails to complete normally;
- backend queue order, worker order, host thread timing, lane order, chunk order, and physical serialization are not semantic oracles for sibling iteration order.

These obligations do not define iteration construction, task semantics, fault aggregation, cancellation, early exit, atomics, barriers, reductions, collectives, or execution hierarchy.

## Future executable evidence

When an accepted Exec operation or resource representation creates a concrete executable consumer, its reference/conformance tests should realize the applicable obligations above without using host thread scheduling, host memory ordering, physical addresses, or backend behavior as the semantic oracle.

A future executable representation may refine the evidence mechanism. It must not silently strengthen, weaken, or replace the normative owners listed above.
