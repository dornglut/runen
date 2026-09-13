# Cross-Stratum Bridge Laws

Status: **provisional normative**

Core, Exec, and Model compose only through explicit semantic bridges.

## Model to Core

A Model observation reifies as either an ordinary immutable Core value or an explicit logical handle whose operations are defined by its contract.

The scalar member domains reused by [Model logical data](model/data.md) do not make a Model logical scalar type identical to any Core per-program type definition. A concrete Model-to-Core reification contract must establish the exact Core result type/value or logical-handle contract it produces.

Model `Optional`, structural record, Relation, Bag, and Sequence types/values do not automatically become Core structural aggregates, storage, or other Core values merely because a realization can represent them physically.

A Model logical value is not identified by the physical storage layout, allocation identity, address, index, source declaration, or Core storage identity of its realization.

Observing Model state MUST NOT implicitly create a lexical Core borrow directly into arbitrary state-domain internal storage.

## Core to Model

An ordinary Core value does not automatically acquire Model logical type identity, record/schema identity, collection membership, absence semantics, or Model value equivalence. A Core-to-Model crossing requires an explicit semantic bridge/consumer contract.

Ordinary lexical mutation MUST NOT directly mutate arbitrary state-domain internals through a Core reference.

State-domain mutation crosses through an explicit state-domain operation or transition contract.

## Core to Exec

Exec work receives Core values, owned resources, or permission-bearing borrows or views according to explicit ownership and resource rules.

Execution placement does not change the logical ownership contract.

A raw pointer valid in one physical realization MUST NOT be assumed valid after migration or relocation unless a contract preserves that validity.

## Exec to Core

Exec completion reifies results into Core values or resources according to the applicable contract.

A physical realization does not bypass Core validity or ownership rules.

## Model to Exec

A live Model query or state-domain observation is not silently captured as mutable live state by Exec work.

Execution over Model-derived data requires an explicit bridge such as an immutable Core value, materialization, a logical Exec resource, or a logical handle with defined execution semantics.

## Exec to Model

Exec computation does not gain state-domain commit authority merely by holding a physical resource or computing candidate values.

Changes to Model state re-enter through the applicable state-domain admission and commit contract.

## Non-leakage

A realization MAY erase a bridge physically when it preserves the same semantics, but it MUST NOT expose an otherwise-forbidden borrow, address, mutation, observation, ordering, identity, or authority merely because two strata share one physical representation.