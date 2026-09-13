# Model Queries

Status: **provisional normative; incomplete**

Queries are pure logical derivations unless an explicitly defined operation states otherwise.

The represented logical type/value algebra, explicit absence, record structure, collection families, and canonical Model value-equivalence relation are owned by [Model logical data](data.md). This document consumes those relations; it does not redefine logical equality, absence, record identity, collection membership, or multiplicity.

## Multiplicity

Query results preserve multiplicity by default using Bag semantics from `data.md`.

Projection does not silently deduplicate Model-equivalent output values. If two input occurrences produce Model-equivalent outputs, their occurrences remain represented by Bag multiplicity unless an explicitly defined multiplicity-removal operation applies.

`distinct` explicitly removes multiplicity and consumes the canonical Model value-equivalence relation to identify duplicate value classes. This revision does not yet define the exact input/result typing, execution relation, or schema propagation for `distinct`.

## Query operations

The accepted base query operations are represented illustratively by `from`, `where`, `select`, `derive`, `join`, `group`, `aggregate`, `distinct`, and `order` or `order by`.

The existence of those operation names does not supply semantics that their canonical owners have not yet defined. In particular, Model value equivalence is not by itself a query predicate language, join condition, grouping rule, aggregate equality rule, or ordering relation.

## Ordering

Relation and Bag values have no semantic iteration order under `data.md`.

`order by` produces a Sequence.

If ordering keys do not distinguish all elements, the specification does not constrain the relative order of tied elements unless further semantic keys or a stronger ordered-source contract distinguishes them.

A physical realization MUST NOT manufacture an implicit semantic tie-breaker from storage position, index order, hash order, worker order, scheduler order, physical address, or other information not present in the query contract.

This revision does not define the ordering relation for logical scalar/record values, absence ordering, comparator semantics, or exact `order by` typing/execution.

The exact join, grouping, aggregation, query typing, query-schema propagation, predicate absence behavior, `distinct` execution/result typing, and static cardinality rules are not defined by this revision.