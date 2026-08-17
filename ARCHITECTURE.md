# Repository Architecture

This document owns the structure and dependency boundaries of the Runen repository.

## Packages

### `crates/runen-core-ir`

Owns semantic data structures for the currently implemented Core subset and the admission checks that establish structural and statically decidable well-formedness for that MIR.

It does not execute programs or define path-state behavior.

It MUST NOT depend on the reference machine, a production backend, host platform services, or repository tooling.

### `crates/runen-reference`

Owns executable reference semantics for admitted Core MIR represented by `runen-core-ir`.

It may depend on `runen-core-ir`.

### `tools/xtask`

Owns repository validation tooling and orchestration. It owns no Runen language semantics.

## Top-level artifact areas

- `crates/` — implementation packages;
- `tools/` — repository tooling;
- `spec/` — normative specification artifacts;
- `docs/` — non-normative engineering and design artifacts.

## Dependency direction

```text
runen-core-ir
      │
      ▼
runen-reference

repository tooling is orthogonal
```
