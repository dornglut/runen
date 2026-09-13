# Model Verification Contract

Status: **non-normative assurance guidance**

`crates/runen-model-oracle` is executable verification evidence for the currently accepted Runen Model subset. It does not define Model semantics and is not source syntax, compiler Model IR, a planner, a storage engine, a runtime/database system, or an incremental-maintenance engine.

The canonical normative owners are:

- `spec/language/model/data.md` for represented logical types, including Model-only `Cardinality`, explicit absence, structural records, finite Relation/Bag/Sequence values, and Model value equivalence;
- `spec/language/model/queries.md` for the accepted bounded record-field projection, bounded record-field equivalence filter, bounded disjoint-record field-equivalence join, bounded record-field partition grouping, exact `bag_cardinality : Bag<T> -> Cardinality`, and `distinct : Bag<T> -> Relation<T>` evaluator relations;
- `spec/language/model/state-domains.md` for the first bounded state-domain observed logical-root profile; and
- `spec/language/model/observation.md` for the accepted single-domain observation identity and stable observed-value relation.

## Verification representation

The oracle uses finite verification fixtures only.

- `FieldKey` is an abstract logical schema-identity token for tests. Its numeric carrier and the private deterministic map ordering used internally are not Model field order, source spelling, stable entity identity, serialization, or storage order.
- `NaNRealizationId` distinguishes test witnesses for the NaN-member variation permitted by accepted Core numeric semantics. Model equivalence erases that witness within one exact floating logical type; it does not make the witness a Model-visible NaN identity.
- floating finite values are represented semantically by sign, integer significand, and semantic exponent under the accepted `F16`, `F32`, and `F64` format parameters. Host `f32`/`f64` equality, NaN payloads, and physical bit layout are not semantic oracles.
- `Cardinality` members use a private arbitrary-precision `BigUint` carrier so executable equivalence and exact Bag-cardinality results have no fixed-width maximum. `BigUint` digits, physical representation, implementation ordering, formatting, and arithmetic APIs are not Model-visible. The public `u128` fixture constructor is only a bounded way to supply expected witnesses and is not a semantic Cardinality bound.
- Relation and Bag fixtures store private Model-equivalence-class keys. Their deterministic `BTreeSet`/`BTreeMap` order is implementation machinery and is not exposed as semantic iteration order or representative selection. Private ordering of Cardinality equivalence keys exists only to support these trees and does not define Model Cardinality comparison or order.
- Bag class multiplicities remain checked `u64` fixture carriers. This finite-width choice does not bound Model occurrence cardinality: explicit-multiplicity fixtures can represent large finite class counts, and exact `bag_cardinality` widens each represented multiplicity into an arbitrary-precision accumulator. `BagValue::total_multiplicity` is deliberately a checked bounded fixture observation and may return `MultiplicityOverflow` for a valid fixture whose semantic Cardinality remains exactly representable.
- Sequence fixtures retain semantic positional order through bounded verification-only position access. The Rust `usize` carrier is not source indexing syntax or a language indexing-base rule.
- `StateDomainId` and `ObservationId` are abstract finite fixture tokens. Their numeric carriers do not define domain order, observation order, revision order, timestamps, frames, transactions, freshness/progress positions, ECS change cursors, storage identities, or source identities. `ObservationId` is scoped by one `ObservedBagDomain` fixture rather than being a process-global observation namespace.
- `ObservedBagDomain` is the first executable instantiation of the generic accepted observed-root profile and covers only a `Bag<T>` logical root. It stores an exact Bag element type and a private finite map from admitted observation tokens to `BagValue` roots. The map is immutable after construction; duplicate observation tokens are rejected rather than rebound, and private `BTreeMap` ordering is lookup machinery rather than semantic observation/revision order. This Bag-only verification boundary does not narrow the normative profile, which remains generic over represented logical root type `T`.

Public validated constructors reject values whose recursive logical shape does not match the declared Model type. The fixture error types, including zero explicit fixture multiplicity, finite-carrier multiplicity overflow, invalid bounded-query fixtures, duplicate observation tokens, and observed-root element-type mismatch, are verification machinery rather than compiler diagnostics or normative runtime query/observation-fault contracts.

`ObservedBagDomain::observed_bag` returns the Bag root admitted under one finite observation token or `None` when that token is absent from the finite fixture. That `None` is fixture lookup behavior only; it does not define runtime observation acquisition, retention, reacquisition, durability, or unavailable-observation failure. The fixture has no mutation, commit, revision, clock, transaction, or state-transition API.

The projection API accepts a Rust slice of verification-only `FieldKey` values solely as a finite carrier for the accepted semantic retained-key set. Slice order is not Model order, and duplicate candidate keys are idempotent set membership. Projection restricts private record equivalence-class keys directly and merges multiplicities with checked arithmetic; it never selects or exposes representative record values or private tree order.

The bounded field-equivalence filter uses one verification-only `FieldKey` and one exact typed `Value` as carriers for its accepted predicate inputs. It compares the selected entry in each private record equivalence-class key with the constant value's private equivalence key and copies matching record classes with their unchanged multiplicities. It does not reconstruct or select a representative record, expose private tree order, or use host equality in place of Model equivalence.

The bounded disjoint-record field-equivalence join uses two verification-only `FieldKey` values as carriers for the accepted join fields. After validating disjoint record schemas and exact selected-field type equality, it compares the selected entries of private left/right record equivalence-class keys directly and merges complete matching key maps. Each matching input-class pair produces one unique output class, so duplicate output insertion is an internal invariant failure rather than a multiplicity-aggregation path. Matching multiplicities use checked `u64` multiplication only because the oracle fixture carrier is finite-width; `MultiplicityOverflow` is verification machinery and is not a normative Model query fault or semantic bound.

The bounded record-field partition grouping uses a Rust slice of verification-only `FieldKey` values solely as a finite carrier for the accepted grouping-key set. After validating record input and admitted keys, it derives a private projected record-equivalence key for every complete input class, partitions those classes by that key, and emits each non-empty block directly as one private Bag equivalence key in an outer Relation. Input multiplicities are copied unchanged into their unique groups. Candidate order and duplicates are non-semantic, and neither record representatives nor group identities are introduced.

The exact Bag-cardinality query reads only the Bag's private equivalence-class multiplicities, widens each `u64` fixture count into the private arbitrary-precision Cardinality carrier, and sums them exactly. It does not call the bounded `total_multiplicity` helper, select an element representative, or expose private map traversal order.

## Executable evidence

The current oracle exercises exactly the accepted represented subset:

- structural logical type equality for intrinsic scalars including `Cardinality`, `Optional`, closed records, Relation, Bag, and Sequence;
- exact Cardinality mathematical-member equivalence through a private arbitrary-precision carrier and structural composition through existing Optional/record/collection rules;
- typed `Absent`/`Present` construction;
- closed structural record value validation;
- exact scalar fixture domains, including semantic finite floating members, signed zero, signed infinity, and NaN witnesses;
- canonical typed Model value equivalence, including same-type NaN collapse and distinct `+0`/`-0` members;
- finite Relation membership, Bag multiplicity, and Sequence positional behavior;
- recursive equivalence through optional, record, and collection nesting;
- `distinct : Bag<T> -> Relation<T>` as direct equivalence-class support mapping, including generic use with Cardinality elements;
- exact `bag_cardinality : Bag<T> -> Cardinality`, including empty input, ordinary duplicate occurrence counts, distinct-class sums, construction-order independence, equivalent-Bag preservation, and a result strictly above `u64::MAX` while the bounded total helper reports fixture `MultiplicityOverflow`;
- bounded `project_fields<K> : Bag<R> -> Bag<R|K>` record-field restriction, including exact retained field identities/types, representative-free class projection, and occurrence-preserving multiplicity aggregation when projected classes merge;
- bounded `filter_field_equivalent<k, v> : Bag<R> -> Bag<R>` record-field equivalence filtering, including exact field/value type admission, representative-free matching, exact retained multiplicity, tagged optional absence/presence, same-type NaN matching, signed-zero distinction, and recursive nested-value equivalence;
- bounded `join_fields_equivalent<kL, kR> : Bag<R> × Bag<S> -> Bag<R ⊎ S>` disjoint-record field-equivalence joining, including exact schema/key/type admission, representative-free matching and merge, pair/output distinction, multiplicity products, tagged optional behavior, same-type NaN matching, signed-zero distinction, and recursive nested-value equivalence;
- bounded `group_by_fields<K> : Bag<R> -> Relation<Bag<R>>` record-field partition grouping, including exact key admission, representative-free projected-key partitioning, exact multiplicity preservation, empty/degenerate-key behavior, tagged Optional grouping, same-type NaN grouping, signed-zero distinction, recursive nested-value equivalence, and group-key recoverability through accepted projection plus `distinct`; and
- the bounded single-domain observed-root profile instantiated for `Bag<T>`, including exact root element-type admission, immutable observation-token binding, domain-scoped observation tokens, distinct observations with different or Model-equivalent roots, NaN-witness equivalence, construction-order independence, and direct composition of an actually observed `Bag<Record<...>>` with accepted field-equivalence filtering plus exact Bag cardinality.

Tests intentionally vary construction and occurrence order where Relation/Bag semantics are unordered. Cardinality tests distinguish the arbitrary-precision semantic result from bounded multiplicity fixture observations and exercise explicit-multiplicity fixture rejection/merging. Projection tests additionally vary retained-key candidate order and record-field construction order. Bounded filter tests vary record-field construction and Bag occurrence order and exercise exact typed rejection, multiplicity, Optional tags, NaN witnesses, signed zero, and nested Bag equivalence. Bounded join tests vary record-field construction and Bag occurrence order and exercise schema/type rejection, empty/no-match cases, one-to-many and many-to-one matching, multiplicity products, Optional tags, NaN witnesses, signed zero, and nested Bag equivalence. Bounded grouping tests vary record-field construction, Bag occurrence order, and grouping-key candidate order and exercise admission, empty input, empty/full key sets, multiplicity preservation, projected-key partitioning/recovery, Optional tags, NaN witnesses, signed zero, and nested Bag equivalence. Observation tests vary observation construction order, reuse the same finite observation-token carrier under distinct domain fixtures, keep earlier roots stable beside later different observations, preserve distinct observation identities for equivalent roots, and feed the observed Bag itself into accepted query execution. A passing result must not depend on host hashing, private tree order, allocation identity, addresses, source declaration identity, SQL behavior, a selected representative occurrence, observation-token numeric order, or revision/storage identity.

## Deliberate boundaries

This executable evidence does not define or implement:

- source Model syntax or source-to-Model lowering;
- compiler Model IR, generic query ASTs, planners, indexes, storage layouts, or runtime/database architecture;
- general projection expressions or field creation/rename/derivation, general filtering beyond the accepted bounded field-equivalence relation, general joins beyond the accepted bounded disjoint-record field-equivalence relation, general grouping beyond the accepted bounded record-field partition relation, aggregation beyond exact Bag cardinality, ordering, or static query type/cardinality inference beyond the accepted bounded record-field projection, bounded field-equivalence filter, bounded disjoint-record field-equivalence join, bounded record-field partition grouping, exact Bag-cardinality, and `distinct` relations;
- Cardinality arithmetic, comparison/order, conversions, formatting/parsing, source/Core type mapping, Relation/Sequence cardinality, or public arbitrary-precision representation APIs;
- observed-root executable coverage for logical root families other than the accepted `Bag<T>` verification instantiation;
- state-domain revision/visibility behavior, observation-to-revision mapping or ordering, runtime acquisition/retention/reacquisition/failure semantics, `ObservationSet` membership/admission, or multi-domain compatibility;
- stable entity/row key semantics;
- materialization, freshness, incremental maintenance, differential update algorithms, or replication;
- cross-stratum Model/Core or Model/Exec execution.

Future executable coverage must follow accepted normative semantics and repository architecture; implementation structure is not authority to select the next Model operation.
