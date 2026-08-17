# Runen Specification

Version: **0.9-provisional**

This file is the index of the normative Runen specification. Specification conventions and precedence are defined in [conventions.md](conventions.md).

## General semantics

- [Program behavior](language/behavior.md)
- [Correctness relations](language/correctness.md)
- [Language lifecycle](language/lifecycle.md)
- [Time and ordering domains](language/time.md)
- [Remote boundaries](language/remote.md)

## Core

- [Values and memory](language/core/values-memory.md)
- [Ownership and safety](language/core/ownership-safety.md)
- [Effects and faults](language/core/effects-faults.md)
- [Numerics](language/core/numerics.md)
- [Layout and ABI](language/core/layout-abi.md)

## Exec

- [Tasks and parallelism](language/exec/tasks-parallelism.md)
- [Resources](language/exec/resources.md)
- [Memory model](language/exec/memory-model.md)

## Model

- [Collections and queries](language/model/collections-queries.md)
- [State domains and observation](language/model/state-domains-observation.md)
- [Rules](language/model/rules.md)
- [Observation and maintenance](language/model/maintenance.md)

## Cross-stratum semantics

- [Bridge laws](language/bridges.md)

## Conformance and environment

- [Conformance profiles](conformance/profiles.md)
- [Standard Environment boundary](environment/boundary.md)

## Normative annexes

- [Core A0 — values, places, initialization, move, copy, assignment, and destruction](annexes/core/a0-values-places.md)

Project planning, implementation architecture, verification strategy, and design rationale are intentionally outside `spec/`.