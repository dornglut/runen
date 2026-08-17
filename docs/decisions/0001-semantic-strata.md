# 0001 — Core, Exec, and Model Semantic Strata

Status: **accepted**  
Recorded: **2026-08-17**  
Normative owners: [`spec/language/strata.md`](../../spec/language/strata.md), [`spec/language/bridges.md`](../../spec/language/bridges.md)  
Supersedes: none  
Superseded by: none

## Context

Runen must cover ordinary systems values and memory, heterogeneous execution, and logical or declarative state without forcing one domain's realization mechanism into another domain's source model.

## Decision

Separate those responsibilities into Core, Exec, and Model, with explicit semantics at their boundaries.

## Alternatives considered

A single universal semantic layer would reduce visible categories but would also merge memory ownership, execution placement, and logical-state concerns. Separate languages or disconnected libraries would avoid that merge but lose one coherent type and bridge model.

## Consequences

Each stratum can evolve with domain-specific invariants while shared physical realizations remain possible. Cross-stratum behavior must be designed rather than inherited from implementation coincidence.