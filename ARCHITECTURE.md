# Runen Repository Architecture

Runen is currently a semantic-kernel proving repository, not a complete compiler implementation.

This file describes **repository implementation ownership**. Normative language semantics live under `spec/`.

## Specification authority

The authority chain is:

```text
spec/README.md
      │
      ├── spec/language.md              provisional normative language architecture
      │       │
      │       └── spec/annex-a-memory.md  precise accepted A0 subset
      │
      ├── spec/semantic-closure.md      normative status/dependency ledger
      └── spec/conformance.md           profile/conformance boundary
```

`spec/standard-environment.md` is normative only for the environment/profile facilities eventually claimed.

`spec/implementation-architecture.md` and `spec/rationale.md` are non-normative.

Repository implementation behavior MUST NOT silently redefine the normative specification.

## Current A0 implementation boundary

The repository currently has two semantic crates:

- `crates/runen-core-ir` owns typed A0 semantic data structures: type definitions, values, locals, places, projections, operands, statements, basic blocks, and terminators. It does not execute programs.
- `crates/runen-reference` owns the executable A0 abstract machine and its conformance tests. It interprets the semantic structures but is not a production runtime or backend.

`tools/xtask` is repository tooling only. It is not a Runen compiler layer and owns no language semantics.

## Dependency direction

```text
spec/language.md
       │
       └── spec/annex-a-memory.md
                    │
                    ▼
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

## North-Star semantic architecture

The language specification defines three top-level semantic strata:

```text
Core  ·  Exec  ·  Model
```

This repository does **not** currently implement Exec or Model.

The existence of those normative architecture sections does not authorize adding runtime/compiler layers without issue-owned proving work. The next implementation slices are governed by `spec/semantic-closure.md`.

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

## Long-term implementation guidance

The non-normative target decomposition is documented separately in `spec/implementation-architecture.md` so compiler architecture cannot accidentally become language semantics.

In particular, source/HIR/Core-MIR/Exec-IR/Logical-IR/realization/target-IR choices remain implementation strategy unless a normative rule explicitly refers to a semantic artifact.