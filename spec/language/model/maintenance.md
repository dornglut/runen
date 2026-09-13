# Model Materialization and Maintenance

Status: **provisional normative; incomplete**

## First represented result-target observation profile

The first represented result-side profile introduces an abstract **result target identity** `M`.

A result target identity is semantic target identity only. It is not a represented Model logical value, a state-domain identity, a source observation or `ObservationSet`, a Core value, storage place, borrow or handle, an Exec resource, a source declaration, an allocation or address, a database/view/table/storage/index identity, or a revision, clock, transaction, frame, causal frontier, freshness/progress token, or ECS identity merely because one realization can associate those things with a target.

A result target participates in this profile by fixing exactly one represented Model logical type `T` as its **result logical type**. The type comes from [Model logical data](data.md). Associating `M` with `T` does not change the value semantics of `T` or add hidden row, entity, stable-key, source, allocation, address, storage, or serialization identity. This one-result-type shape is the first proving profile only and does not require every future maintenance target to use the same shape.

A **target observation identity** `r` in this profile is scoped to exactly one result target. An observation identity admitted for one target is not interchangeable with an observation identity from another target merely because their implementation carriers or observed logical values happen to compare alike under some non-Model mechanism.

For one admitted pair `(M, r)`, the result-target observation relation determines exactly one canonical Model value-equivalence class of values of exact logical type `T`. Write this abstractly as:

```text
observed_result(M, r) : T
```

The notation identifies the logical relation only. It does not define source syntax, a runtime API, a storage handle, a Core borrow, an Exec resource, a database cursor, a physical snapshot object, or an implementation representation for `M`, `r`, or the observed result.

A realization may copy, reconstruct, cache, materialize, or otherwise produce any representative of the determined Model-equivalence class. Representative choice, allocation identity, address, storage layout, index shape, traversal order, caching strategy, and physical snapshot machinery are not result-target observation semantics.

The same admitted `(M, r)` MUST NOT determine two non-equivalent logical result values. Replacing, relocating, compacting, caching differently, or otherwise changing a physical realization after `r` is established cannot silently change the logical result denoted by that observation under this relation.

This stability is semantic target-observation identity, not a retention or availability guarantee. This profile does not require an implementation to retain physical data for `r`, make `r` reacquirable, or define a runtime failure when an unavailable target observation is requested. The relation applies when `(M, r)` is admitted.

Result target identity, target observation identity, and Model value equivalence are distinct. Two distinct result targets MAY expose Model-equivalent logical result values and remain distinct targets. Two distinct target observations of one result target MAY determine Model-equivalent logical result values and remain distinct observations.

Target observation identity is also distinct from source-domain observation identity and source `ObservationSet` identity. Equal observed result values do not establish that two target observations represent the same source observation context.

Neither target identity nor target observation identity is a state revision, target revision, timestamp, frame, transaction, causal frontier, freshness/staleness position, propagation-progress token, ECS change cursor, MVCC/history position, storage snapshot, allocation, address, index, or serialization identity. This profile defines no ordering, predecessor/successor relation, distinguished current observation, or observation-to-revision mapping.

A result target is not automatically a state domain. It does not acquire state-domain mutation admission, commit, revision, conflict, durability, recovery, or replication semantics merely by participating in this profile. Likewise, a target observation is not automatically a member of a source `ObservationSet`.

The profile above defines only result target identity, one exact result logical type, and stable logical meaning for an already-admitted target observation. It does not define target creation or destruction, repeated-materialization identity or reuse, acquisition or lookup APIs, retention duration, reacquisition, unavailable-target or unavailable-observation failure, cleanup, ownership or sharing, target mutation or update admission, target visibility scheduling, propagation progress, durability, recovery, replication, storage/index/cache strategy, or incremental maintenance algorithms.

This profile also does not bind a target observation to a source `ObservationSet` or defining computation. It therefore does not by itself define result provenance, source-observation correspondence, freshness or staleness, legal lag, propagation progress, reconciliation, target-update visibility, `materialize`, or `maintain` semantics. Those require stronger contracts.

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