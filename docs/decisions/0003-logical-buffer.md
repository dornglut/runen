# 0003 — Logical Buffer Identity

Status: **accepted**  
Recorded: **2026-08-17**  
Normative owners: [`spec/language/exec/resources/buffers.md`](../../spec/language/exec/resources/buffers.md), [`spec/language/bridges.md`](../../spec/language/bridges.md)  
Supersedes: none  
Superseded by: none

## Context

Heterogeneous execution may move or replicate data across memory spaces. Identifying a Buffer permanently with one physical address prevents legal movement, while treating addresses as stable across movement is unsafe.

## Decision

Separate logical Buffer identity and permissions from physical backing, and require an explicit physical-stability contract before exposing a raw address.

## Alternatives considered

Permanent allocation identity would restrict placement and migration. Freely stable addresses would make relocation unsound.

## Consequences

Placement can vary without changing logical resource identity, while raw-pointer validity remains tied to a physical-stability guarantee.