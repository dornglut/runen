# Runen

Runen is a programming language currently in its **semantic-kernel proving phase**.

Its North-Star semantic architecture is:

```text
Core  ·  Exec  ·  Model
```

- **Core** — values, memory, ownership, ordinary computation, and low-level interaction;
- **Exec** — execution-visible work/resources and heterogeneous realization;
- **Model** — logical state, relations, queries, rules, observations, and maintained correspondence.

The strata compose only through explicit bridge semantics. They are semantic responsibilities, not mandatory runtime layers.

## Specification status

The current Runen language specification is **provisional**, not yet a complete independent-implementation-ready language.

The accepted North-Star architecture is documented now so implementation proving has an explicit target. Unresolved semantics remain visible and dependency-ordered rather than being filled in by compiler behavior.

Start with:

- [Specification set and authority](spec/README.md)
- [Runen Language Specification](spec/language.md)
- [Semantic Closure Program](spec/semantic-closure.md)
- [Conformance & Assurance](spec/conformance.md)

## Current executable subset — A0

The repository intentionally started **below surface syntax**. A0 executes hand-constructed typed Core MIR so value/place/init/move/copy/assignment/drop/fault semantics can be falsified before a parser, native backend, Exec, GPU, or Model implementation can accidentally define the language by implementation detail.

A0 currently proves:

- typed Core MIR identifiers, types, values, locals, places, and field projections;
- hierarchical initialization state, including partial initialization;
- `Init`, `Read`, `Move`, `Copy`, `Assign`, and `Drop` semantics;
- deterministic scope cleanup on `Return` and `Fault`;
- a reference machine and conformance tests.

The precise accepted A0 rules live in [Semantic Annex A0](spec/annex-a-memory.md).

A0 does **not** yet implement borrowing, raw pointer/provenance semantics, the formal CPU/GPU memory model, complete numerics, source grammar, Exec, or Model. Those are tracked in the Semantic Closure Program.

## Repository architecture

- `crates/runen-core-ir` — semantic Core IR data structures only;
- `crates/runen-reference` — executable reference semantics/conformance oracle for the accepted subset;
- `tools/xtask` — repository validation tooling only;
- `spec/` — normative specification, semantic status, conformance, and explicitly separated non-normative rationale/implementation guidance.

See [ARCHITECTURE.md](ARCHITECTURE.md) for implementation ownership and dependency direction.

## Validation

`cargo validate` is the single maintained repository-validation command:

```text
cargo validate
```

It checks locked workspace metadata, formatting, all-target tests, denied-warning all-target Clippy, diff hygiene, and repository-state non-mutation. GitHub Actions invokes the same command through the immutable Dornglut shared Rust validation workflow.

Repository validation is not itself the definition of Runen language conformance; semantic expectations come from the applicable normative specification/annex.

The initial implementation has no third-party dependencies.