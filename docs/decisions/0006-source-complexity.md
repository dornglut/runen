# 0006 — Source Complexity Budget

Status: **accepted design constraint**  
Recorded: **2026-08-17**  
Normative owner: none; this constrains future design review rather than current program behavior  
Supersedes: none  
Superseded by: none

## Context

The implementation may need rich internal models for provenance, effects, placement, clocks, revisions, coherence, and scheduling. Exposing all of that machinery in routine source would defeat the intended language ergonomics.

## Decision

Treat routine source complexity as an explicit design constraint: implementation proof machinery should surface only where it represents a real semantic or control choice for the programmer.

## Alternatives considered

Making all internal proof state explicit would simplify some compiler reasoning but transfer complexity to every user. Hiding every advanced control would prevent low-level and performance-sensitive work.

## Consequences

Future features must justify source-level machinery by user-visible semantics or control, not merely compiler convenience. If that boundary cannot be maintained, simplification is preferred over another abstraction layer.