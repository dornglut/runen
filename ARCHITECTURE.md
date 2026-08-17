# Runen Architecture

Runen is currently a semantic-kernel proving repository, not a complete compiler implementation.

## A0 ownership boundary

The current repository has two semantic crates:

- `crates/runen-core-ir` owns typed A0 semantic data structures: type definitions, values, locals, places, projections, operands, statements, basic blocks, and terminators. It does not execute programs.
- `crates/runen-reference` owns the executable A0 abstract machine and its conformance tests. It interprets the semantic structures but is not a production runtime or backend.

`spec/annex-a-memory.md` defines the provisional A0 semantic contract. Code and tests must remain explainable in terms of that annex rather than Rust implementation behavior.

`tools/xtask` is repository tooling only. It is not a Runen compiler layer and owns no language semantics.

## Dependency direction

```text
spec/annex-a-memory.md
        │
        ├────────────── semantic contract
        │
runen-core-ir
        │
        ▼
runen-reference
        │
        ▼
A0 conformance tests

repository tooling (`tools/xtask`) is orthogonal
```

The reference machine may depend on Core IR. Core IR must not depend on the reference machine, host platform services, a backend, or later Runen strata.

## Current semantic boundary

A0 proves:

- typed storage places and structural projections;
- hierarchical initialization state;
- first initialization, read, move, copy, assignment, and explicit destruction;
- partial initialization and partial moves;
- deterministic reverse field/local destruction;
- defined `Return` and `Fault` cleanup;
- independence from host addresses and Rust `Drop` semantics.

A0 deliberately does not prove borrowing, raw storage/pointers, provenance, pinning, atomics, ABI/layout, parser/source syntax, traits, production lowering, Exec, GPU, or Model.

Those require later issue-owned proving slices. They must not be inferred from the A0 implementation.
