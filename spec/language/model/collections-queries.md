# Model Collections and Queries

Status: **provisional normative**

Model defines logical data independently of one physical storage representation.

The intended logical declaration vocabulary includes records, properties, tags, predicates, relations, queries, and other constructs whose exact source spelling remains illustrative.

Graph and Field are not fundamental universal Model value categories.

## Collection families

The base Model algebra distinguishes:

```text
Relation<T>   unordered set
Bag<T>        unordered multiset
Sequence<T>   ordered sequence
```

These distinctions are semantic, not physical storage choices.

Relation and Bag values have no semantic iteration order. An operation whose result depends on a value being first therefore requires an ordered input or an explicit arbitrary-selection contract.

## Multiplicity

Queries preserve multiplicity by default using bag semantics.

Projection does not silently deduplicate equal output rows.

`distinct` explicitly removes multiplicity.

## Query vocabulary

The accepted base query operations are represented illustratively by:

- `from`;
- `where`;
- `select`;
- `derive`;
- `join`;
- `group`;
- `aggregate`;
- `distinct`;
- `order` / `order by`.

Recursive graph/path semantics, window semantics, and general set-combination operators are not part of this base algebra unless added by a later normative revision.

## Ordering

`order by` produces a Sequence.

If ordering keys do not distinguish all elements, tied elements have unspecified relative order unless further semantic keys or a stronger ordered-source contract distinguish them.

An implementation MUST NOT use hidden object addresses, hash iteration, physical storage order, entity/archetype layout, or query-plan accident as an implicit tie-breaker.

## Purity

Queries are pure logical derivations unless an explicitly defined operation says otherwise.

A query plan, index, ECS archetype, database table, GPU representation, or incremental dependency graph is not the logical query semantics.

Exact logical typing, absence semantics, join semantics, grouping/aggregation semantics, identity/keys, and cardinality/type inference are unspecified in this revision.