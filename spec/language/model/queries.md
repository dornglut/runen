# Model Queries

Status: **provisional normative; incomplete**

Queries are pure logical derivations unless an explicitly defined operation states otherwise.

The represented logical type/value algebra, explicit absence, record structure, collection families, and canonical Model value-equivalence relation are owned by [Model logical data](data.md). This document consumes those relations; it does not redefine logical equality, absence, record identity, collection membership, or multiplicity.

## Multiplicity

Query results preserve multiplicity by default using Bag semantics from `data.md`.

Projection does not silently deduplicate Model-equivalent output values. If two input occurrences produce Model-equivalent outputs, their occurrences remain represented by Bag multiplicity unless an explicitly defined multiplicity-removal operation applies.

Filtering preserves the multiplicity of each retained input equivalence class unless an explicitly defined operation states otherwise.

The first represented multiplicity-removal operation is the bounded `distinct` relation defined below.

## Bounded record-field projection

Let `R` be one represented closed structural record type from `data.md`, and let `K` be one finite subset of the logical field keys of `R`.

Write `R|K` for the restricted record type containing exactly the keys in `K`, with each retained key mapped to exactly the same represented Model logical type that key has in `R`.

The represented bounded record-field projection relation has exactly this semantic type:

```text
project_fields<K> : Bag<R> -> Bag<R|K>
```

This notation identifies the semantic relation only. It does not define source syntax, generic arguments, a query expression AST, compiler IR, or an implementation API.

The relation is admitted only when `K` is a subset of the field-key set of `R`. A requested key not present in `R` is therefore outside this represented relation rather than a runtime query fault.

For one record value `r : R`, its restriction `r|K : R|K` contains exactly the value from `r` for each retained key in `K` and contains no other fields. Retained values are unchanged Model values: projection does not unwrap, coalesce, compare, normalize, or otherwise reinterpret scalar, optional, record, or nested collection values.

Every retained field key keeps exactly its existing logical field-key identity and logical type from `R`. This relation creates no field key, field rename, derived field, field collision, source-name lookup, declaration-position identity, stable entity/row identity, physical column identity, storage/index identity, or serialization identity.

Projection is well-defined on Bag equivalence classes without selecting a representative. If two values of `R` are Model-equivalent, record equivalence from `data.md` requires their corresponding values at every retained key to be Model-equivalent. Restricting either value to the same `K` therefore yields Model-equivalent values of `R|K`, so each input record equivalence class determines exactly one output record equivalence class.

For an input `B : Bag<R>`, the output multiplicity of one `R|K` equivalence class is the finite sum of the multiplicities of all input `R` equivalence classes whose restrictions belong to that output class.

Consequences follow from the accepted record, Bag, and Model value-equivalence semantics:

- total Bag multiplicity is preserved;
- projection does not silently deduplicate;
- distinct input record classes can contribute to one output class when their differences occur only in removed fields;
- input classes whose retained fields remain non-equivalent remain distinct output classes;
- the empty input Bag yields the empty output Bag;
- empty `K` is valid and yields the accepted empty structural record type; every input occurrence then restricts to the unique empty record value, so a non-empty input yields one output class whose multiplicity equals the input's total multiplicity;
- when `K` is the complete field-key set of `R`, every record keeps all of its fields and the output Bag is Model-equivalent to the input Bag;
- retained `Absent` and `Present` values, nested values, same-type NaNs, and signed zeros follow the existing recursive Model value-equivalence relation without projection-specific equality rules.

The input and output are Bags and therefore have no semantic iteration order. Field declaration, construction, enumeration, storage, or presentation order cannot affect the result type, projected value, or multiplicity.

After a well-formed input and admitted field-key subset are established, the represented projection is pure, non-faulting, non-diverging, and deterministic. It consumes no state domain, `ObservationSet`, clock, external observation, stable entity identity, physical storage/index, or incremental-maintenance behavior.

This revision defines only field restriction over `Bag<R>`. It does not define projection that creates, renames, derives, computes, or collides fields; projection over non-record values; Relation or Sequence projection; a general `select`/`derive` expression relation; or source spelling for projection.

## Bounded record-field equivalence filtering

Let `R` be one represented closed structural record type from `data.md`. Let `k` be one logical field key present in `R`, let the declared logical type of that field be exactly `T`, and let `v` be one represented Model value of exactly logical type `T`.

The represented bounded record-field equivalence filter has exactly this semantic type:

```text
filter_field_equivalent<k, v> : Bag<R> -> Bag<R>
```

This notation identifies the semantic relation only. It does not define source `where` syntax, generic arguments, a query expression AST, compiler IR, a predicate-function value, or an implementation API.

The relation is admitted only when `k` exists in `R`, its declared logical type is exactly `T`, and `v` has exactly logical type `T`. A missing field key or mismatched comparison value is outside this represented relation rather than a runtime query fault.

For one record value `r : R`, the record is retained exactly when the value of field `k` in `r` is Model-equivalent under `data.md` to `v`. This relation explicitly consumes canonical Model value equivalence for this one bounded predicate. It does not make Model value equivalence source `==`, SQL equality, a universal query predicate language, a join condition, a grouping rule, an ordering relation, or a general comparison API.

The predicate is well-defined on Bag equivalence classes without selecting a representative. If two values of `R` are Model-equivalent, record equivalence requires their corresponding values at `k` to be Model-equivalent. Because Model value equivalence is an equivalence relation, either both field values are equivalent to `v` or neither is. Each input record equivalence class therefore has one representative-independent retain/reject result.

For an input `B : Bag<R>`, every retained input `R` equivalence class appears in the output with exactly its input multiplicity. Every rejected class has output multiplicity zero. Filtering cannot increase a class multiplicity, merge distinct record classes, split one class, normalize multiplicity, or silently deduplicate.

Consequences follow from the accepted record, Bag, Optional, floating, and recursive Model value-equivalence semantics:

- the empty input Bag yields the empty output Bag;
- when no input class matches `v` at `k`, the output Bag is empty;
- when every input class matches, the output Bag is Model-equivalent to the input Bag;
- a retained class with multiplicity greater than one keeps exactly that multiplicity;
- two non-equivalent input record classes remain distinct output classes even when both selected field values are equivalent to `v`;
- when `T` is `Optional<U>`, `Absent` matches exactly `Absent`, `Present(a)` matches `Present(b)` exactly when `a` and `b` are Model-equivalent under `U`, and `Absent` never matches `Present(_)`;
- this Optional behavior introduces no SQL `NULL`, unknown predicate result, truthiness, implicit Boolean conversion, coalescing, or absent propagation;
- for floating `T`, every same-type NaN field value matches a same-type NaN `v` under the accepted Model equivalence;
- for floating `T`, `+0` and `-0` do not match each other because they are not Model-equivalent;
- represented records, optionals, Relations, Bags, and Sequences used as `T` follow their existing recursive Model value-equivalence rules without a filter-specific comparison relation.

The input and output are Bags and therefore have no semantic iteration order. Field declaration, construction, enumeration, storage, index, hash, worker, scheduler, or presentation order cannot affect whether a class is retained or what multiplicity it has.

The field key `k` keeps exactly its existing logical field-key identity from `R`. This relation creates no field, rename, derived field, collision rule, source-name lookup, declaration-position identity, stable entity/row identity, physical column identity, storage/index identity, or serialization identity.

After a well-formed input and admitted `R`, `k`, `T`, and `v` are established, the represented filter is pure, non-faulting, non-diverging, and deterministic. It consumes no state domain, `ObservationSet`, clock, external observation, stable entity identity, physical storage/index, or incremental-maintenance behavior.

This revision defines only field-equivalence filtering over `Bag<R>`. It does not define Boolean-field truth filtering, truthiness, generic predicate or callback values, scalar ordering/comparator predicates, general `where` expressions, Relation or Sequence filtering, joins, grouping, aggregation, ordering, or source spelling for filtering.

## Bag distinct

For every represented Model logical type `T`, the first `distinct` relation has exactly this type:

```text
distinct : Bag<T> -> Relation<T>
```

For an input `B : Bag<T>`, `distinct(B)` is the unique `Relation<T>` containing exactly each `T` Model-equivalence class whose multiplicity in `B` is positive.

The element logical type remains exactly `T`. This operation introduces no conversion, coercion, element widening, schema/key rewriting, inferred type relation, or source spelling.

Consequences follow directly from the Bag, Relation, and Model value-equivalence semantics owned by `data.md`:

- the empty Bag yields the empty Relation;
- a class with multiplicity one and the same class with any greater positive finite multiplicity each contribute exactly one Relation member class;
- distinct input equivalence classes remain distinct Relation member classes;
- same-type NaN members belong to one output member class under the accepted Model equivalence;
- `+0` and `-0` remain distinct output member classes when both occur because they are not Model-equivalent;
- optional, record, and nested collection values use the existing recursive Model equivalence without a `distinct`-specific equality rule.

The result has no multiplicity dimension because it is a Relation rather than a Bag. `distinct` therefore removes multiplicity; it is not Bag normalization to multiplicity one.

The result has no semantic order under the existing Relation contract. Bag storage order, hash order, iteration order, source presentation order, worker order, scheduler order, or another realization detail cannot affect the result.

No representative-selection relation is introduced. Bag and Relation semantics are already defined over Model-equivalence classes, so `distinct` maps class support to class membership without selecting one physical or semantic representative from equivalent input occurrences.

After its input Bag value is established, the represented `distinct` operation is pure, non-faulting, non-diverging, and deterministic: one exact Relation result is determined by the input's positive-multiplicity equivalence classes.

This first slice admits only `Bag<T>` input. `distinct` over Relation or Sequence, including any order-preserving Sequence form, is not defined by this revision.

## Query operations

The accepted base query operations are represented illustratively by `from`, `where`, `select`, `derive`, `join`, `group`, `aggregate`, `distinct`, and `order` or `order by`.

The existence of those operation names does not supply semantics that their canonical owners have not yet defined. In particular, Model value equivalence is not by itself a query predicate language, join condition, grouping rule, aggregate equality rule, or ordering relation. The bounded field-equivalence filter above is an explicit operation-specific consumer of that equivalence relation and does not broaden it beyond that contract.

The exact evaluator relations represented by this revision are the bounded Bag record-field projection, the bounded Bag record-field equivalence filter, and the bounded Bag `distinct` relation. They do not define general `select`/`derive`, general predicate/filter expressions, join, grouping, aggregation, or ordering semantics.

## Ordering

Relation and Bag values have no semantic iteration order under `data.md`.

`order by` produces a Sequence.

If ordering keys do not distinguish all elements, the specification does not constrain the relative order of tied elements unless further semantic keys or a stronger ordered-source contract distinguishes them.

A physical realization MUST NOT manufacture an implicit semantic tie-breaker from storage position, index order, hash order, worker order, scheduler order, physical address, or other information not present in the query contract.

This revision does not define the ordering relation for logical scalar/record values, absence ordering, comparator semantics, or exact `order by` typing/execution.

The exact join, grouping, aggregation, general query typing, general query-schema propagation beyond the represented bounded projection, general predicate absence behavior beyond the represented field-equivalence filter, general filtering execution, general projection expressions, and static cardinality rules are not defined by this revision.
