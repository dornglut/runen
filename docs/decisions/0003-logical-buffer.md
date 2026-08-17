# 0003 — Logical Buffer Identity

Status: **accepted design rationale**

## Context

Heterogeneous execution may move or replicate data across memory spaces. Treating a Buffer as permanently identical to one address prevents legal movement; treating addresses as stable across movement is unsafe.

## Decision

Model `Buffer<T>` as a logical coherent Exec resource distinct from physical backing allocations. Treat views as logical permissions. Require a stable allocation or explicit mapped/pinned realization before exposing a raw physical address.

## Consequences

Placement and transfer can change without changing logical resource identity, while pointer validity remains tied to an explicit physical-stability contract.