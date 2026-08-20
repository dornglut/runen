# Repository Architecture

This document owns the structure and dependency boundaries of the Runen repository.

## Packages

### `crates/runen-syntax`

Owns implementation-only decoding, lexical tokenization, lossless concrete syntax trees, syntax diagnostics/recovery, and source ranges for the currently represented Runen source subset.

It consumes accepted source-text and concrete-grammar authority from `spec/language/source/` but owns no normative language semantics. In particular, it does not own module/name resolution, source type checking, callable or binding identity, ownership/availability validation, Typed HIR, Core MIR lowering, runtime behavior, or backend behavior.

The crate has no dependency on another Runen package in the currently accepted architecture.

### `crates/runen-hir`

Owns implementation-only resolved and type-checked source structure for the currently represented concrete Runen source subset.

It consumes accepted source semantics from `spec/language/source/` and lossless concrete structure from `runen-syntax`, but owns no normative language semantics. It represents source-compilation module assignments, resolved record/function/binding identities, source types, callable/body structure, and the ownership/availability consequences required by the accepted subset before lower compiler forms erase source structure.

It may depend on `runen-syntax`. It MUST NOT infer module identity from filesystem/package conventions or source-unit order, and it MUST NOT own Core MIR lowering, Exec/Model IR, runtime execution, realization, or backend behavior.

No existing Core, Exec, reference, or backend/proving package depends on `runen-hir` in the currently accepted architecture. Such a consumer edge requires its own accepted lowering/proving boundary.

### `crates/runen-core-ir`

Owns the canonical finite program/function/body semantic data model for the currently implemented Core subset and program-level MIR validation for the structural and language-validity rules expressible by that subset. There is no separate production one-body Core model or validation API.

MIR validation is a language-validation concern. It is not the environment-admission phase defined by `spec/language/lifecycle.md`.

The crate does not execute programs or define host/runtime behavior.

It MUST NOT depend on the reference machine, a production backend, host platform services, or repository tooling.

### `crates/runen-exec-oracle`

Owns executable verification-only conformance relations for the currently represented Exec subset.

It is not Runen source syntax, compiler Exec IR, a production runtime or backend, and it owns no normative language semantics. Its finite identities, regions, values, and structured-order tokens exist only to make accepted Exec contracts executable in conformance tests.

The package is currently dependency-free and independent of both `runen-core-ir` and `runen-reference`. It MUST NOT depend on compiler target IR, runtime scheduling or platform services, production backends, or repository tooling.

Future cross-stratum verification may compose independent proving packages only when accepted semantic evidence requires that dependency; package co-location does not itself justify coupling them.

### `crates/runen-reference`

Owns the single executable reference semantics for validated Core programs represented by `runen-core-ir`, including dynamic function activations for the currently represented direct-call relation. Invalid Core programs are rejected before this boundary; the package does not maintain an alternate Core semantic data model or validator.

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
runen-syntax
      │
      ▼
 runen-hir

runen-core-ir
      │
      ▼
runen-reference

runen-exec-oracle

repository tooling is orthogonal
```

`runen-hir` depends only on `runen-syntax` among Runen packages in the currently accepted source-frontend architecture. `runen-exec-oracle` remains independent of the source/HIR and Core/reference dependency chains.
