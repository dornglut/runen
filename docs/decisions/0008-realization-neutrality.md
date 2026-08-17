# 0008 — No Universal Realization Mechanism

Status: **accepted design constraint**  
Recorded: **2026-08-17**  
Normative owner: none; current semantics define observable contracts rather than one mandatory realization architecture  
Supersedes: none  
Superseded by: none

## Context

Many domains have strong implementation techniques: MVCC, CRDTs, ECS storage, render graphs, virtual DOMs, transactional memory, dependency graphs, differential dataflow, specialized schedulers, and physical query plans.

Making any one of these the universal Runen mechanism would couple unrelated domains to one implementation model and make the language narrower as it grows.

## Decision

Standardize semantic contracts before realization mechanisms. A physical or incremental technique becomes language-level only when independent proving demonstrates that its semantics are genuinely universal and cannot be represented cleanly by a more general contract.

Domain-specific mechanisms remain libraries, profiles, standard-environment facilities, compiler realizations, or future explicit proposals unless deliberately promoted.

## Alternatives considered

One universal state/runtime mechanism could simplify implementation integration but would leak its assumptions into Core, Exec, and Model. Leaving all interaction outside the language would preserve implementation freedom but lose the cross-stratum semantic coherence Runen is intended to provide.

## Consequences

Compiler/runtime implementations remain free to use specialized mechanisms when they preserve the normative contracts. New domain features must justify any proposed universal semantic primitive rather than inheriting language status from implementation popularity.
