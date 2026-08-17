# Runen Testing and Validation

`cargo validate` is the single maintained repository validation authority.

It performs, in order:

1. locked Cargo metadata resolution without dependencies;
2. workspace formatting check;
3. locked workspace tests for all targets;
4. workspace Clippy for all targets with warnings denied;
5. Git diff hygiene;
6. a before/after repository-state comparison so validation cannot silently modify the checkout.

The command intentionally does not define Runen semantics. Semantic acceptance comes from the applicable specification annex plus executable conformance tests.

## Required command

```text
cargo validate
```

Focused crate or test commands are appropriate while editing, but they do not replace the canonical command before review or merge.

GitHub Actions invokes the same repository-owned authority through the immutable shared Dornglut Rust workflow. Pull-request validation must prove the reviewed feature-head revision rather than relying on a synthetic merge checkout.

## A0 semantic gate

The current A0 suite must cover at least:

- move invalidates the source;
- copy preserves the source;
- a partial move preserves disjoint live sub-places and makes the containing aggregate unreadable as a whole;
- independently initialized fields can form a fully initialized aggregate;
- non-copy values cannot be copied;
- `Init` cannot reinitialize storage that became dead after move;
- mutable `Assign` can initialize never-initialized storage;
- mutable `Assign` can reinitialize dead storage after move;
- immutable assignment is rejected;
- assignment destroys a live target before replacement;
- explicit drop of a partially initialized aggregate destroys only live subobjects;
- moved or explicitly dropped values are not destroyed twice;
- field and local cleanup order is deterministic and reversed as specified;
- defined fault cleanup destroys each live value exactly once.

No test should use Rust destructor behavior, pointer addresses, allocator behavior, or hash/container iteration order as a Runen semantic oracle.
