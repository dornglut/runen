# 0006 — Source Complexity Budget

Status: **accepted design rationale**

## Context

Runen uses sophisticated semantic machinery to support safety, heterogeneous realization, and logical state. Exposing that machinery routinely would defeat the intended language ergonomics.

## Decision

Ordinary source should primarily express problem concepts. Provenance, effect details, placement, clocks, revisions, coherence state, regions, numeric relaxations, and scheduling controls should surface only where they represent a real semantic, optimization, or unsafe boundary.

## Consequences

When routine application code becomes dominated by proof or realization machinery, simplification has priority over adding another abstraction layer.