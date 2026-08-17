# Model Materialization and Maintenance

Status: **provisional normative; incomplete**

## Materialize

`materialize` requests retained realization while preserving the defining logical semantics.

## Maintain

`maintain` requests ongoing semantic correspondence between a defining source computation and a target.

A maintenance request is valid only when the target exposes a contract sufficient to define that correspondence.

Every maintenance target MUST define the applicable source-observation identity, update admission, freshness, progress, failure and reconciliation behavior, and target update visibility.

`maintain` MUST NOT imply universal reliable synchronization, distributed transactions, unspecified retry semantics, or zero-latency propagation.

## Freshness

Freshness identifies which source observation a maintained or materialized result represents and how stale it may legally be. Freshness is distinct from result correctness and propagation progress.

## Incremental equivalence obligation

At each observation point permitted by its freshness contract, an observed materialized or maintained result MUST be observationally equivalent to evaluating the defining logical computation from scratch over the corresponding admitted source `ObservationSet`.