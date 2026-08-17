# 0004 — Separate Determinism and Time Concepts

Status: **accepted design rationale**

## Context

Physical schedule order, numerical reproducibility, simulation clocks, state versions, and causal ordering are related in some programs but are not the same semantic property.

## Decision

Keep semantic determinism, schedule independence, and heterogeneous reproducibility distinct. Keep clock domain, state revision, and causal frontier distinct. Require explicit contracts when a system relates any of these concepts.

## Consequences

Parallel execution can be unordered yet deterministic, and a state revision need not become an artificial universal clock. Numeric portability can be strengthened without over-constraining scheduling.