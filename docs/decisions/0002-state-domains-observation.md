# 0002 — State Domains and Explicit Observation

Status: **accepted design rationale**

## Context

Logical state may be realized by relational storage, ECS layouts, incremental engines, replicated services, or other structures. Ordinary lexical borrows into such storage would couple Core alias/lifetime semantics to one physical realization. The word `authority` is also needed for security/capability semantics.

## Decision

Use **state domain** for the unit controlling coherent logical state, revisions, admission, and commit. Reify Model observations as immutable Core values/snapshots or explicit logical handles rather than implicit lexical borrows into internal storage. Use immutable `ObservationSet` values to identify multi-domain observations without claiming a global distributed snapshot.

## Consequences

Model storage may evolve independently of Core memory layout. Security authority stays terminologically distinct. Cross-domain evaluation has an explicit observation identity.