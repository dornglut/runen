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

`observe` requests logical observation semantics; it does not mandate one incremental realization.

The compatibility and admission rules for composing observations from multiple state domains are not defined by this revision.
