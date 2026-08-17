# Cross-Stratum Bridge Laws

Status: **provisional normative**

Core, Exec, and Model compose only through explicit semantic bridges.

## Model to Core

A Model observation reifies as either:

- an ordinary immutable Core value or snapshot representation; or
- an explicit logical handle whose operations are defined by its contract.

Observing Model state MUST NOT implicitly create a lexical Core borrow directly into arbitrary state-domain internal storage.

## Core to Model

Ordinary lexical mutation MUST NOT directly mutate arbitrary state-domain internals through a Core reference.

State-domain mutation crosses through explicit state-domain operations, transactions, rule proposals, or another defined bridge contract.

## Core to Exec

Exec tasks receive Core values, owned resources, or permission-bearing borrows/views according to explicit ownership and resource rules.

Execution placement does not change the logical ownership contract.

A raw pointer valid in one physical realization MUST NOT be assumed valid after migration or relocation unless a mapping/pinning contract preserves that validity.

## Exec to Core

Exec completion reifies results into Core values/resources according to the task/resource contract.

A physical device result does not bypass Core validity or ownership rules merely because it was produced by a GPU or accelerator.

## Model to Exec

A live Model query or state-domain observation is not silently captured as mutable live state by an Exec task.

Execution over Model-derived data requires an explicit bridge such as immutable Core snapshot values, materialization, Buffer/resource realization, or a logical handle with defined execution semantics.

## Exec to Model

Exec computation does not gain state-domain commit authority merely by holding a physical resource or computing candidate values.

Changes to Model state re-enter through the applicable state-domain admission and commit contract.

## Non-leakage

An implementation MAY optimize a bridge away physically when it preserves the same semantics, but it MUST NOT expose an otherwise-forbidden borrow, address, mutation, observation, ordering, or authority merely because two strata share one runtime representation.