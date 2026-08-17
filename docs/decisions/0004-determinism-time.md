# 0004 — Separate Determinism and Ordering Concepts

Status: **accepted**  
Recorded: **2026-08-17**  
Normative owners: [`spec/language/correctness.md`](../../spec/language/correctness.md), [`spec/language/clocks.md`](../../spec/language/clocks.md), [`spec/language/model/state-domains.md`](../../spec/language/model/state-domains.md), [`spec/language/remote.md`](../../spec/language/remote.md)  
Supersedes: none  
Superseded by: none

## Context

Earlier designs risked treating physical schedule, heterogeneous numeric agreement, temporal clocks, state revisions, and causal ordering as variants of one ordering concept.

## Decision

Keep semantic determinism, schedule independence, heterogeneous reproducibility, clock domains, state revisions, and causal frontiers distinct unless an explicit contract relates them.

## Alternatives considered

One universal ordering or clock abstraction would simplify vocabulary but create false equivalences and overly constrain realizations.

## Consequences

Contracts can state exactly which kind of order or reproducibility they require without importing unrelated guarantees.