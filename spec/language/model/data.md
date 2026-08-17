# Model Logical Data

Status: **provisional normative; incomplete**

Model defines logical data independently of one physical storage representation.

The base logical vocabulary includes concepts such as records, properties, tags, predicates, and relations. Exact source spelling remains illustrative.

The base collection families are:

```text
Relation<T>   unordered set
Bag<T>        unordered multiset
Sequence<T>   ordered sequence
```

These distinctions are semantic rather than physical storage choices.

Relation and Bag values have no semantic iteration order. An operation whose result depends on a value being first therefore requires an ordered input or an explicit arbitrary-selection contract.

Exact logical typing, absence semantics, record identity, and stable logical key semantics are not defined by this revision.