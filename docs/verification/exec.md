# Exec Verification Contract

Status: **non-normative conformance-obligation documentation**

This document records focused assurance obligations for the currently accepted Exec semantic slices. It does not define Runen semantics, conformance profiles, compiler architecture, or repository CI.

The normative ordinary-access rules exercised here are owned by `spec/language/exec/memory-model.md` and `spec/language/exec/parallelism.md`. Core storage, overlap, borrowing, and interior-mutability facts remain owned by their Core specifications.

## Ordinary unordered-access boundary

The repository does not yet have an accepted executable Exec representation. The first Exec memory slice is therefore evidenced with focused semantic litmus obligations rather than by inventing an Exec IR, scheduler, runtime, or backend model solely for testing.

In the cases below, region identity and overlap come from the canonical owner of the accessed storage or resource. `unordered` means that the language semantics provide no relative order between the two pieces of work; incidental backend execution order is ignored.

Required cases:

- unordered `read(A)` with `read(A)` is a legal ordinary interaction;
- unordered state-changing access to `A` with state-changing access to disjoint `B` is legal;
- unordered `read(A)` with a state-changing access to overlapping `A` is not a legal ordinary safe interaction when no separately defined mechanism applies;
- unordered state-changing access to `A` with another state-changing access to overlapping `A` is not a legal ordinary safe interaction when no separately defined mechanism applies;
- semantically ordered sequential accesses to the same region are not rejected merely because the corresponding unordered pair would conflict;
- a backend that serializes source-unordered conflicting accesses does not make the interaction legal;
- physically parallel execution of legal overlapping non-state-changing reads does not by itself introduce semantic nondeterminism;
- Core interior mutability under shared aliases does not by itself legalize overlapping state-changing accesses from source-unordered Exec work;
- for Core-origin structural storage, root/descendant overlap and sibling-field disjointness are consumed as Core-owned facts rather than redefined by Exec.

These obligations do not authorize atomics, reductions, commutative accumulation, collectives, barriers, or other synchronization mechanisms whose normative contracts remain open.

## Future executable evidence

When an accepted Exec operation or resource representation creates a concrete executable consumer, its reference/conformance tests should realize the applicable obligations above without using host thread scheduling, host memory ordering, physical addresses, or backend behavior as the semantic oracle.

A future executable representation may refine the evidence mechanism. It must not silently strengthen, weaken, or replace the normative owners listed above.