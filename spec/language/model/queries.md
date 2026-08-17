# Model Queries

Status: **provisional normative; incomplete**

Queries are pure logical derivations unless an explicitly defined operation states otherwise.

## Multiplicity

Query results preserve multiplicity by default using bag semantics.

Projection does not silently deduplicate equal output rows.

`distinct` explicitly removes multiplicity.

## Query operations

The accepted base query operations are represented illustratively by `from`, `where`, `select`, `derive`, `join`, `group`, `aggregate`, `distinct`, and `order` or `order by`.

## Ordering

`order by` produces a Sequence.

If ordering keys do not distinguish all elements, the specification does not constrain the relative order of tied elements unless further semantic keys or a stronger ordered-source contract distinguishes them.

A physical realization MUST NOT manufacture an implicit semantic tie-breaker from information not present in the query contract.

The exact join, grouping, aggregation, query typing, and cardinality rules are not defined by this revision.