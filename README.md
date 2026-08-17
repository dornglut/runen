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

See [`spec/annex-a-memory.md`](spec/annex-a-memory.md) for the A0 semantic contract.

## Intended validation

Once a Rust toolchain is available:

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The initial implementation has no third-party dependencies.
