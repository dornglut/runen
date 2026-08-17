# 0007 — Functional Bias and Explicit Declarative Boundaries

Status: **accepted design constraint**  
Recorded: **2026-08-17**  
Normative owner: none; this guides language design rather than defining current program behavior  
Supersedes: none  
Superseded by: none

## Context

Runen needs strong semantic machinery for ownership, effects, heterogeneous execution, and logical state, but routine source should not expose that machinery merely because the compiler reasons about it.

At the same time, forcing every problem into a declarative or relational style would make ordinary algorithms, synchronization-sensitive work, and representation-sensitive systems code less direct.

## Decision

Prefer immutable bindings, pure derivation, algebraic data, pattern-oriented reasoning, and inferred information where those choices do not hide a real semantic boundary.

Use declarative Model constructs where the problem itself is logical, relational, reactive, or maintained. Keep ordinary Core and Exec algorithms explicit when order, synchronization, representation, iteration strategy, or low-level control is semantically relevant.

## Alternatives considered

A uniformly imperative design would discard useful reasoning structure. A uniformly declarative design would force algorithms with meaningful operational structure into an unnatural model.

## Consequences

Future syntax and APIs should optimize for problem-level expression without erasing real control boundaries. This decision constrains ergonomics review, not the current semantics of any particular expression form.
