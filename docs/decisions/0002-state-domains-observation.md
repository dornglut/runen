# 0002 — State Domains and Explicit Observation

Status: **accepted**  
Recorded: **2026-08-17**  
Normative owners: [`spec/language/model/state-domains.md`](../../spec/language/model/state-domains.md), [`spec/language/model/observation.md`](../../spec/language/model/observation.md), [`spec/language/bridges.md`](../../spec/language/bridges.md)  
Supersedes: none  
Superseded by: none

## Context

Logical state may have realizations whose storage lifetime, layout, replication, or indexing does not match Core lexical borrowing. The word `authority` is also needed for security semantics.

## Decision

Use `state domain` for coherent logical-state responsibility, make multi-domain observation explicit through immutable `ObservationSet` values, and avoid implicit Core borrows into arbitrary Model storage.

## Alternatives considered

Direct lexical borrows into Model storage would couple Core lifetime rules to physical Model realization. A universal global snapshot would impose a distributed consistency guarantee that many valid state domains cannot provide.

## Consequences

Model storage can vary independently of Core layout, multi-domain evaluation has explicit observation identity, and security authority remains a separate concept.