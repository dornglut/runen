# Runen Specification

Version: **0.9-provisional**

This file is the index of the normative Runen specification. Interpretation rules are defined in [conventions.md](conventions.md).

## General semantics

- [Semantic strata](language/strata.md)
- [Program behavior](language/behavior.md)
- [Correctness relations](language/correctness.md)
- [Language lifecycle](language/lifecycle.md)
- [Capability, authority, and information flow](language/authority.md)
- [Clock domains](language/clocks.md)
- [Remote boundaries](language/remote.md)
- [Cross-stratum bridge laws](language/bridges.md)

## Core

- [Value and storage semantics](language/core/value-storage.md)
- [Borrowing](language/core/borrowing.md)
- [Pointers and provenance](language/core/pointers.md)
- [Unsafe semantics](language/core/unsafe.md)
- [Effects](language/core/effects.md)
- [Faults](language/core/faults.md)
- [Integer semantics](language/core/numerics/integers.md)
- [Floating-point semantics](language/core/numerics/floating-point.md)
- [Layout and ABI](language/core/layout-abi.md)

## Exec

- [Tasks](language/exec/tasks.md)
- [Parallelism](language/exec/parallelism.md)
- [Realization](language/exec/realization.md)
- [Allocations](language/exec/resources/allocations.md)
- [Buffers](language/exec/resources/buffers.md)
- [Memory model](language/exec/memory-model.md)

## Model

- [Logical data](language/model/data.md)
- [Queries](language/model/queries.md)
- [State domains](language/model/state-domains.md)
- [Observation](language/model/observation.md)
- [Rules](language/model/rules.md)
- [Materialization and maintenance](language/model/maintenance.md)

## Conformance and environment

- [Conformance profiles](conformance/profiles.md)
- [Standard Environment boundary](environment/boundary.md)