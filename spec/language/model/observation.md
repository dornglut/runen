# Model Observation

Status: **provisional normative; incomplete**

A Model evaluation spanning state domains is evaluated relative to an explicit immutable `ObservationSet` identifying the admitted observations for that evaluation or reaction wave.

An `ObservationSet` is immutable for that wave.

An `ObservationSet` does not imply one globally synchronized distributed snapshot unless a stronger contract explicitly guarantees one.

## First represented single-domain observation profile

For the bounded observed-root state-domain profile in [Model state domains](state-domains.md), let `D` be one participating state domain whose exact observed logical-root type is `T`.

An **observation identity** in this profile is scoped to exactly one state domain. An observation identity admitted for `D` is not interchangeable with an observation identity from another domain merely because their implementation carriers, revisions, or observed logical values happen to compare alike under some non-Model mechanism.

For one admitted pair `(D, o)`, the observation relation determines exactly one canonical Model value-equivalence class of values of exact logical type `T`. Write this abstractly as:

```text
observed_value(D, o) : T
```

where the notation identifies the logical relation only. It does not define source syntax, a runtime API, a storage handle, a Core value or borrow, an Exec resource, a physical snapshot object, a database cursor, or an implementation representation for `o` or the observed value.

A realization may copy, reconstruct, cache, materialize, or otherwise produce any representative of the determined Model-equivalence class. Representative choice, allocation identity, address, storage layout, index shape, traversal order, and physical snapshot machinery are not observation semantics.

The same admitted `(D, o)` MUST NOT determine two non-equivalent logical values. Advancing, mutating, replacing, compacting, relocating, or otherwise changing live/current domain state after `o` is established cannot silently change the logical value denoted by `o` for evaluation under this relation.

This stability is semantic observation identity, not a storage-retention guarantee. This revision does not require an implementation to keep physical data for `o` indefinitely, make `o` reacquirable after its admitting context ends, or define a runtime failure when an unavailable observation is requested. The relation applies when `(D, o)` is admitted.

Observation identity is distinct from Model value equivalence. Two distinct observation identities of the same domain MAY determine Model-equivalent logical root values and remain distinct observations. Conversely, one observation identity cannot change meaning merely because a later observation exposes a different value.

Observation identity is also distinct from state revision. This profile defines no required observation-to-revision mapping, revision ordering, timestamp, frame number, transaction identity, causal frontier, freshness position, progress token, ECS change cursor, or MVCC/history relation.

Evaluation relative to admitted `(D, o)` uses the logical value determined by `observed_value(D, o)` as immutable input. Once that value is established, every already-represented Model query relation keeps exactly its existing pure semantics; this profile defines no stateful variant of projection, filtering, joining, grouping, cardinality, `distinct`, or another query operation.

Consequently, if two permitted realizations of the same admitted `(D, o)` produce Model-equivalent representatives of `T`, applying the same represented pure Model query with the same explicit query inputs cannot acquire a different meaning merely from the physical realization choice. Any operation-specific result relation remains owned by that operation's existing normative contract.

The observed root keeps the data semantics of its exact logical type. In particular, a Bag or Relation of record values does not gain hidden row, entity, source-declaration, allocation, address, storage, or stable-key identity merely because the value came from state observation.

The profile above is single-domain only. It supplies the per-domain observation object required by later composition, but it does not define the membership structure, compatibility, admission, consistency, or synchronization rules for composing observations from multiple state domains into an `ObservationSet`.

## First represented singleton `ObservationSet`

For one pair `(D, o)` admitted under the single-domain profile above, define the bounded singleton observation context:

```text
singleton_observation_set(D, o)
```

This singleton is a valid `ObservationSet` containing exactly the one admitted domain-scoped observation association `(D, o)`. It is immutable for its evaluation or reaction wave.

The singleton is an **evaluation-context construct**, not a represented Model logical value from [Model logical data](data.md). The word `Set` in `ObservationSet` does not give this context Model `Relation`, `Bag`, or `Sequence` semantics, Model value equivalence, query-data membership, source syntax, row/entity identity, serialization identity, or another logical-data operation.

For any already-represented pure Model query relation whose input is the observed logical root of `D`, evaluation relative to `singleton_observation_set(D, o)` uses exactly `observed_value(D, o)` as that root input. Replacing the singleton notation by its underlying admitted `(D, o)` relation therefore does not change the query's logical meaning or operation-specific result relation.

The singleton retains the exact observation identity it identifies. Two distinct observation identities of the same domain do not collapse into one singleton context merely because their observed logical roots are Model-equivalent. This rule distinguishes the underlying admitted observations; it does not define general `ObservationSet` equality, hashing, ordering, or a Model value-equivalence relation for observation contexts.

Because the represented singleton contains exactly one domain/observation association, it has no cross-domain compatibility question. The singleton rules alone do not determine whether or how another observation can participate in the same context; multi-domain admission is defined only by a separately represented profile such as the bounded two-domain profile below or by a future stronger contract.

Singleton construction does not identify `o` with a state revision, revision position, timestamp, frame, transaction, causal frontier, freshness or progress token, ECS change cursor, physical snapshot, storage object, or another realization identity. It also defines no runtime acquisition, retention, reacquisition, unavailable-observation failure, durability, replication, enumeration order, serialization, or physical realization contract.

This first singleton form deliberately does not define the general membership structure of `ObservationSet`. In particular, this revision does not decide whether a future general form is a finite map, relation, sequence, bag, opaque context, or another structure; does not define duplicate-domain behavior; and does not forbid a future separately accepted temporal or history consumer from requiring multiple observations associated with one state domain.

The singleton form is sufficient to serve as one source `ObservationSet` wherever another Model contract already refers to the corresponding admitted source observation context. Defining that source context does not by itself define freshness/staleness policy, a materialized or maintained target, propagation progress, reconciliation, or target visibility.

## First represented two-domain `ObservationSet`

For two pairs `(D1, o1)` and `(D2, o2)` already admitted under the single-domain profile above, where `D1` and `D2` are distinct state-domain identities, define the bounded two-domain observation context:

```text
two_domain_observation_set((D1, o1), (D2, o2))
```

This context is a valid `ObservationSet` containing exactly the two admitted domain-scoped observation associations. It is immutable for its evaluation or reaction wave.

The bounded context is admitted exactly when both member pairs are already admitted by the single-domain observation relation and `D1 != D2`. No additional generic compatibility or joint-admission predicate is required merely to form this read-only evaluation context.

The context supplies exactly `observed_value(D1, o1)` as the logical root associated with `D1` and exactly `observed_value(D2, o2)` as the logical root associated with `D2`. A consumer may explicitly supply either or both roots to an already-represented pure Model semantic relation when that relation's own input-type and admission rules permit it. This profile does not redefine projection, filtering, joining, grouping, cardinality, `distinct`, another query relation, or rule derivation semantics.

The two written associations have no semantic member order. Reversing their presentation or construction order cannot change which observed root is associated with either state domain or change the meaning of a represented pure evaluation that consumes those roots. This invariance does not define general `ObservationSet` equality, hashing, ordering, iteration, or serialization.

Like the singleton form, the two-domain context is an evaluation-context construct rather than represented Model logical data. It is not a Model `Relation`, `Bag`, or `Sequence`, has no Model value-equivalence relation, and gains no query-data membership, source syntax, row/entity/stable-key identity, serialization identity, or storage identity. Model-equivalent observed roots do not collapse distinct state-domain identities or observation identities.

Forming the context asserts no cross-domain consistency relation beyond the two explicit observation associations. In particular, it does not assert one synchronized or global snapshot, equal or ordered state revisions, predecessor/successor relations, timestamps, frames, transactions, clock-domain alignment, causal frontiers, freshness positions, propagation progress, MVCC/history compatibility, cross-domain invariants, or atomic-read guarantees.

A stronger consumer may require additional revision, transaction, causal, consistency, synchronization, or other relations before using a two-domain context for that stronger purpose. Those consumer-specific requirements do not become basic validity conditions for the bounded two-domain `ObservationSet` merely because they can relate the same member observations.

Each member retains exactly its accepted domain-scoped observation identity and stable logical meaning. Advancing or changing live/current state in either domain after `o1` or `o2` is established cannot silently change the logical root selected by this context. Physical acquisition order, storage order, allocation identity, or representation cannot become semantic context order or cross-domain consistency.

Reading two roots through this context does not authorize a rule to mutate or commit two state domains. The mutation and cross-domain coordination rules remain owned by [Model rules](rules.md).

This first multi-domain profile is deliberately bounded to exactly two distinct state domains. It does not define contexts with more than two domains, multiple observations from one state domain in one context, a general finite map/set/list/container representation, general multi-domain equality or traversal, cross-domain synchronization protocols, observation acquisition/retention/reacquisition/failure, durability, replication, freshness/materialization/maintenance semantics, stable entity/key identity, source/compiler query IR, or runtime/database APIs.

`observe` requests logical observation semantics; it does not mandate one incremental realization.

Beyond the bounded two-distinct-domain profile above, the general membership, compatibility, and admission rules for composing observations from multiple state domains are not defined by this revision.
