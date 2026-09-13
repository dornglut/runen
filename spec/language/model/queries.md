# Model Queries

Status: **provisional normative; incomplete**

Queries are pure logical derivations unless an explicitly defined operation states otherwise.

The represented logical type/value algebra, explicit absence, record structure, collection families, and canonical Model value-equivalence relation are owned by [Model logical data](data.md). This document consumes those relations; it does not redefine logical equality, absence, record identity, collection membership, or multiplicity.

## Multiplicity

Query results preserve multiplicity by default using Bag semantics from `data.md`.

Projection does not silently deduplicate Model-equivalent output values. If two input occurrences produce Model-equivalent outputs, their occurrences remain represented by Bag multiplicity unless an explicitly defined multiplicity-removal operation applies.

The first represented multiplicity-removal operation is the bounded `distinct` relation defined below.

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

The existence of those operation names does not supply semantics that their canonical owners have not yet defined. In particular, Model value equivalence is not by itself a query predicate language, join condition, grouping rule, aggregate equality rule, or ordering relation.

The bounded Bag `distinct` relation above is the only exact evaluator operation added by this revision; no semantics for the other illustrative names follow from it.

## Ordering

Relation and Bag values have no semantic iteration order under `data.md`.

`order by` produces a Sequence.

If ordering keys do not distinguish all elements, the specification does not constrain the relative order of tied elements unless further semantic keys or a stronger ordered-source contract distinguishes them.

A physical realization MUST NOT manufacture an implicit semantic tie-breaker from storage position, index order, hash order, worker order, scheduler order, physical address, or other information not present in the query contract.

This revision does not define the ordering relation for logical scalar/record values, absence ordering, comparator semantics, or exact `order by` typing/execution.

The exact join, grouping, aggregation, general query typing, query-schema propagation, predicate absence behavior, projection/filter execution, and static cardinality rules are not defined by this revision.