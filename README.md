# Runen

Runen is currently in its semantic-kernel proving phase.

This repository intentionally starts **below surface syntax**. A0 executes hand-constructed, typed Core MIR so that value/place/init/move/copy/drop/fault semantics can be tested before a parser, native backend, Exec, GPU, or Model can accidentally define the language by implementation detail.

## A0 scope

Included:

- typed Core MIR identifiers, types, values, locals, places, and field projections;
- hierarchical initialization state, including partial initialization;
- `Init`, `Read`, `Move`, `Copy`, `Assign`, and `Drop` semantics;
- deterministic scope cleanup on `Return` and `Fault`;
- a reference machine and conformance tests.

Explicitly excluded:

- parser and source syntax;
- references/borrows and raw pointers;
- traits/generics;
- native/LLVM/GPU backends;
- Exec and Model.

## Authority

- [A0 semantic contract](spec/annex-a-memory.md)
- [Architecture](ARCHITECTURE.md)
- [Testing and validation](TESTING.md)
- [Agent/contributor boundary](AGENTS.md)

## Validation

`cargo validate` is the single maintained local and CI validation command:

```text
cargo validate
```

It checks locked workspace metadata, formatting, all-target tests, denied-warning all-target Clippy, diff hygiene, and that validation does not mutate repository state. GitHub Actions invokes the same command through the immutable Dornglut shared Rust validation workflow.

The initial implementation has no third-party dependencies.
