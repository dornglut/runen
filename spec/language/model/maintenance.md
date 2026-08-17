# Model Observation and Maintenance

Status: **provisional normative**

## Observe

`observe` requests logical observation semantics. It does not mandate one incremental implementation.

## Materialize

`materialize` requests retained realization. Full recomputation, caching, indexes, incremental views, GPU representations, or dependency graphs may all be legal realizations when they preserve the defining semantics.

## Maintain

`maintain` requests ongoing semantic correspondence between a defining source computation and a target.

A maintenance request is valid only when the target exposes a contract sufficient to define that correspondence.

Every maintenance target MUST define the applicable:

- source observation or revision identity relationship;
- update admission semantics;
- freshness contract;
- progress expectation;
- failure and reconciliation behavior;
- target commit/update visibility semantics.

`maintain` MUST NOT imply universal reliable synchronization, distributed transactions, unspecified retry semantics, or zero-latency propagation.

## Incremental equivalence

At each observation point permitted by its freshness contract, an observed materialized or maintained result MUST be observationally equivalent to evaluating the defining logical computation from scratch over the corresponding admitted source `ObservationSet`.

Type and memory safety do not imply that a reactive rule system reaches quiescence.