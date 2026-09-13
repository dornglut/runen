# Model Verification Contract

Status: **non-normative assurance guidance**

`crates/runen-model-oracle` is executable verification evidence for the currently accepted Runen Model subset. It does not define Model semantics and is not source syntax, compiler Model IR, a planner, a storage engine, a runtime/database system, or an incremental-maintenance engine.

The canonical normative owners are:

- `spec/language/model/data.md` for represented logical types, explicit absence, structural records, finite Relation/Bag/Sequence values, and Model value equivalence;
- `spec/language/model/queries.md` for the accepted bounded record-field projection, bounded record-field equivalence filter, bounded disjoint-record field-equivalence join, bounded record-field partition grouping, and `distinct : Bag<T> -> Relation<T>` evaluator relations.

## Verification representation

The oracle uses finite verification fixtures only.

- `FieldKey` is an abstract logical schema-identity token for tests. Its numeric carrier and the private deterministic map ordering used internally are not Model field order, source spelling, stable entity identity, serialization, or storage order.
- `NaNRealizationId` distinguishes test witnesses for the NaN-member variation permitted by accepted Core numeric semantics. Model equivalence erases that witness within one exact floating logical type; it does not make the witness a Model-visible NaN identity.
- floating finite values are represented semantically by sign, integer significand, and semantic exponent under the accepted `F16`, `F32`, and `F64` format parameters. Host `f32`/`f64` equality, NaN payloads, and physical bit layout are not semantic oracles.
- Relation and Bag fixtures store private Model-equivalence-class keys. Their deterministic `BTreeSet`/`BTreeMap` order is implementation machinery and is not exposed as semantic iteration order or representative selection.
- Sequence fixtures retain semantic positional order through bounded verification-only position access. The Rust `usize` carrier is not source indexing syntax or a language indexing-base rule.

Public validated constructors reject values whose recursive logical shape does not match the declared Model type. The fixture error type, including rejection of invalid bounded-query fixtures, is verification machinery rather than a compiler diagnostic or normative runtime query-fault contract.

The projection API accepts a Rust slice of verification-only `FieldKey` values solely as a finite carrier for the accepted semantic retained-key set. Slice order is not Model order, and duplicate candidate keys are idempotent set membership. Projection restricts private record equivalence-class keys directly and merges multiplicities with checked arithmetic; it never selects or exposes representative record values or private tree order.

The bounded field-equivalence filter uses one verification-only `FieldKey` and one exact typed `Value` as carriers for its accepted predicate inputs. It compares the selected entry in each private record equivalence-class key with the constant value's private equivalence key and copies matching record classes with their unchanged multiplicities. It does not reconstruct or select a representative record, expose private tree order, or use host equality in place of Model equivalence.

The bounded disjoint-record field-equivalence join uses two verification-only `FieldKey` values as carriers for the accepted join fields. After validating disjoint record schemas and exact selected-field type equality, it compares the selected entries of private left/right record equivalence-class keys directly and merges complete matching key maps. Each matching input-class pair produces one unique output class, so duplicate output insertion is an internal invariant failure rather than a multiplicity-aggregation path. Matching multiplicities use checked `u64` multiplication only because the oracle fixture carrier is finite-width; `MultiplicityOverflow` is verification machinery and is not a normative Model query fault or semantic bound.

The bounded record-field partition grouping uses a Rust slice of verification-only `FieldKey` values solely as a finite carrier for the accepted grouping-key set. After validating record input and admitted keys, it derives a private projected record-equivalence key for every complete input class, partitions those classes by that key, and emits each non-empty block directly as one private Bag equivalence key in an outer Relation. Input multiplicities are copied unchanged into their unique groups. Candidate order and duplicates are non-semantic, and neither record representatives nor group identities are introduced.

## Executable evidence

The current oracle exercises exactly the accepted represented subset:

- structural logical type equality for intrinsic scalars, `Optional`, closed records, Relation, Bag, and Sequence;
- typed `Absent`/`Present` construction;
- closed structural record value validation;
- exact scalar fixture domains, including semantic finite floating members, signed zero, signed infinity, and NaN witnesses;
- canonical typed Model value equivalence, including same-type NaN collapse and distinct `+0`/`-0` members;
- finite Relation membership, Bag multiplicity, and Sequence positional behavior;
- recursive equivalence through optional, record, and collection nesting;
- `distinct : Bag<T> -> Relation<T>` as direct equivalence-class support mapping;
- bounded `project_fields<K> : Bag<R> -> Bag<R|K>` record-field restriction, including exact retained field identities/types, representative-free class projection, and occurrence-preserving multiplicity aggregation when projected classes merge;
- bounded `filter_field_equivalent<k, v> : Bag<R> -> Bag<R>` record-field equivalence filtering, including exact field/value type admission, representative-free matching, exact retained multiplicity, tagged optional absence/presence, same-type NaN matching, signed-zero distinction, and recursive nested-value equivalence;
- bounded `join_fields_equivalent<kL, kR> : Bag<R> × Bag<S> -> Bag<R ⊎ S>` disjoint-record field-equivalence joining, including exact schema/key/type admission, representative-free matching and merge, pair/output distinction, multiplicity products, tagged optional behavior, same-type NaN matching, signed-zero distinction, and recursive nested-value equivalence;
- bounded `group_by_fields<K> : Bag<R> -> Relation<Bag<R>>` record-field partition grouping, including exact key admission, representative-free projected-key partitioning, exact multiplicity preservation, empty/degenerate-key behavior, tagged Optional grouping, same-type NaN grouping, signed-zero distinction, recursive nested-value equivalence, and group-key recoverability through accepted projection plus `distinct`.

Tests intentionally vary construction and occurrence order where Relation/Bag semantics are unordered. Projection tests additionally vary retained-key candidate order and record-field construction order. Bounded filter tests vary record-field construction and Bag occurrence order and exercise exact typed rejection, multiplicity, Optional tags, NaN witnesses, signed zero, and nested Bag equivalence. Bounded join tests vary record-field construction and Bag occurrence order and exercise schema/type rejection, empty/no-match cases, one-to-many and many-to-one matching, multiplicity products, Optional tags, NaN witnesses, signed zero, and nested Bag equivalence. Bounded grouping tests vary record-field construction, Bag occurrence order, and grouping-key candidate order and exercise admission, empty input, empty/full key sets, multiplicity preservation, projected-key partitioning/recovery, Optional tags, NaN witnesses, signed zero, and nested Bag equivalence. A passing result must not depend on host hashing, private tree order, allocation identity, addresses, source declaration identity, SQL behavior, or a selected representative occurrence.

## Deliberate boundaries

This executable evidence does not define or implement:

- source Model syntax or source-to-Model lowering;
- compiler Model IR, generic query ASTs, planners, indexes, storage layouts, or runtime/database architecture;
- general projection expressions or field creation/rename/derivation, general filtering beyond the accepted bounded field-equivalence relation, general joins beyond the accepted bounded disjoint-record field-equivalence relation, general grouping beyond the accepted bounded record-field partition relation, aggregation, ordering, or static query type/cardinality inference beyond the accepted bounded record-field projection, bounded field-equivalence filter, bounded disjoint-record field-equivalence join, bounded record-field partition grouping, and `distinct` relations;
- state-domain execution, revision/visibility behavior, `ObservationSet` admission or multi-domain compatibility;
- stable entity/row key semantics;
- materialization, freshness, incremental maintenance, differential update algorithms, or replication;
- cross-stratum Model/Core or Model/Exec execution.

Future executable coverage must follow accepted normative semantics and repository architecture; implementation structure is not authority to select the next Model operation.
