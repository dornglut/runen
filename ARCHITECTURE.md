# Repository Architecture

This document owns the structure and dependency boundaries of the Runen repository. It does not define Runen language semantics or project sequencing.

## Packages

### `crates/runen-core-ir`

Owns semantic data structures for the currently implemented Core subset.

It MUST NOT depend on the reference machine, a production backend, host platform services, or repository tooling.

### `crates/runen-reference`

Owns executable reference semantics for the subset implemented by `runen-core-ir`.

It may depend on `runen-core-ir`.

### `tools/xtask`

Owns repository validation orchestration only.

It MUST NOT own language semantics.

## Dependency direction

```text
runen-core-ir
      │
      ▼
runen-reference

repository tooling is orthogonal
```

Normative specifications live under `spec/`; non-normative compiler, verification, and design material lives under `docs/`.

Repository code MUST NOT become an alternative source of normative language authority.