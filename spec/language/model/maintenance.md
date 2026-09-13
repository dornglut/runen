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

A realization may copy, reconstruct, cache, or otherwise produce any representative of the determined Model-equivalence class. Representative choice, allocation identity, address, storage layout, index shape, traversal order, caching strategy, and physical snapshot machinery are not result-target observation semantics.

The same admitted `(M, r)` MUST NOT determine two non-equivalent logical result values. Replacing, relocating, compacting, caching differently, or otherwise changing a physical realization after `r` is established cannot silently change the logical result denoted by that observation under this relation.

This stability is semantic target-observation identity, not a retention or availability guarantee. This profile does not require an implementation to retain physical data for `r`, make `r` reacquirable, or define a runtime failure when an unavailable target observation is requested. The relation applies when `(M, r)` is admitted.

Result target identity, target observation identity, and Model value equivalence are distinct. Two distinct result targets MAY expose Model-equivalent logical result values and remain distinct targets. Distinct target observations of one result target MAY determine either Model-equivalent or non-equivalent logical result values and remain distinct observations. This permission does not define any transition, ordering, predecessor/successor, update, or visibility relation between those observations.

Target observation identity is also distinct from source-domain observation identity and source `ObservationSet` identity. Equal observed result values do not establish that two target observations represent the same source observation context.

Neither target identity nor target observation identity is a state revision, target revision, timestamp, frame, transaction, causal frontier, freshness/staleness position, propagation-progress token, ECS change cursor, MVCC/history position, storage snapshot, allocation, address, index, or serialization identity. This profile defines no ordering, predecessor/successor relation, distinguished current observation, or observation-to-revision mapping.

A result target is not automatically a state domain. It does not acquire state-domain mutation admission, commit, revision, conflict, durability, recovery, or replication semantics merely by participating in this profile. Likewise, a target observation is not automatically a member of a source `ObservationSet`.

The profile above defines only result target identity, one exact result logical type, and stable logical meaning for an already-admitted target observation. It does not define target creation or destruction, repeated-materialization identity or reuse, acquisition or lookup APIs, retention duration, reacquisition, unavailable-target or unavailable-observation failure, cleanup, ownership or sharing, target mutation or update admission, target visibility scheduling, propagation progress, durability, recovery, replication, storage/index/cache strategy, or incremental maintenance algorithms.

This profile also does not bind a target observation to a source `ObservationSet` or defining computation. It therefore does not by itself define result provenance, source-observation correspondence, freshness or staleness, legal lag, propagation progress, reconciliation, target-update visibility, `materialize`, or `maintain` semantics. Those require stronger contracts.

## First represented exact source-result correspondence

For this first correspondence profile, let `S = singleton_observation_set(D, o)` be one already-admitted singleton source `ObservationSet` from [Model observation](observation.md). Let the observed logical root of `D` have exact represented logical type `A`.

Let `C : A -> B` denote one already-represented **pure unary Model query relation instance** from [Model queries](queries.md). `C` includes exactly the non-state semantic parameters already owned by that relation's query contract. `C` is a meta-semantic relation instance, not a represented Model logical value, source construct, callback or function value, query AST, compiler IR node, runtime descriptor, planner object, storage object, or new computation-identity namespace. This profile defines no equality, hashing, ordering, serialization, or first-class identity for computation instances.

Let `M` be one result target admitted under the profile above whose exact result logical type is `B`, and let `r` be one target observation identity already admitted for `M`.

The bounded exact correspondence assertion is written abstractly as:

```text
corresponds(C, S, M, r)
```

The notation identifies a semantic association only. It does not define source syntax, an API for creating correspondence, a runtime registration mechanism, a persistence record, or a physical dependency edge.

A correspondence assertion is well-formed only when `S` supplies exactly the logical input type `A` required by `C`, `C` produces exactly logical type `B`, and `M` has exactly result logical type `B`. The first profile is limited to a represented pure unary relation whose sole logical data input is the observed root supplied by `S`. Binary or multi-root relations, including the represented join, and arbitrary composition or pipelines are not covered merely because their component operations are represented.

Correspondence is an **explicit admitted association**. It MUST NOT be inferred solely from Model-equivalent source evaluations or target result values. Equal results do not identify source observations, computation instances, target observations, or correspondence assertions.

Every admitted well-formed correspondence assertion MUST satisfy the exact correctness obligation:

```text
observed_result(M, r)
    ≈Model
C(observed_value(D, o))
```

where `observed_value(D, o)` has the meaning defined by `observation.md`, evaluation of `C` follows only its existing operation-specific query contract and explicit semantic parameters, and `≈Model` is the canonical Model value-equivalence relation from [Model logical data](data.md). This correspondence profile defines no second result-equality relation and no query-operation semantics of its own.

The admitted association and its correctness obligation are distinct facts. Model value equivalence can establish whether the correctness obligation is satisfied after a correspondence assertion has been admitted; Model value equivalence alone cannot create, identify, merge, or substitute a correspondence assertion. Consequently, distinct source observations or distinct relation instances may produce Model-equivalent results without becoming the same correspondence, and distinct target observations may expose Model-equivalent values without acquiring the same source association.

Same-type NaN representative variation and signed-zero distinctions follow the existing Model value-equivalence rules when checking the correctness obligation. Correspondence introduces no representative identity, floating comparison rule, row/entity identity, stable key, storage identity, or physical result identity.

This first profile does not require correspondence to be globally unique, functional, or total. If a stronger future contract separately admits multiple correspondence assertions involving the same `(M, r)`, every admitted assertion independently owes the correctness obligation for its own `C` and `S`. This revision does not decide whether a future materialization or maintenance target has one defining computation, one source context per target observation, multiple valid source associations, or result reuse across computations.

An exact correspondence assertion is not by itself a freshness contract. It defines no current or latest source observation, older/newer relation, legal staleness or lag, freshness interval, revision order, timestamp, frame, transaction, causal frontier, MVCC/history relation, propagation progress, update admission, reconciliation, target-update visibility schedule, target creation or reuse, retention duration, acquisition or reacquisition, failure, cleanup, ownership, `materialize` request behavior, or `maintain` update behavior.

This first profile also does not define correspondence for general or multi-domain `ObservationSet` values, binary or multi-root computations, arbitrary query composition, stateful or effectful computations, stable entity keys, state-domain revision/visibility, or physical storage/index/cache/runtime realization.

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
