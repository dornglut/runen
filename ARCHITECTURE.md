# Repository Architecture

This document owns the structure and dependency boundaries of the Runen repository.

## Packages

### `crates/runen-core-ir`

Owns semantic data structures for the currently implemented Core subset and MIR validation for the structural and language-validity rules expressible by that subset.

MIR validation is a language-validation concern. It is not the environment-admission phase defined by `spec/language/lifecycle.md`.

The crate does not execute programs or define host/runtime behavior.

It MUST NOT depend on the reference machine, a production backend, host platform services, or repository tooling.

### `crates/runen-reference`

Owns executable reference semantics for validated Core MIR represented by `runen-core-ir`. Invalid MIR is rejected before this boundary.

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
