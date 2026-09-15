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

Core semantic/realization packages, reference semantics, Exec/Model packages, and verification-only proving packages do not depend on `runen-hir`. Source-to-Core consumption belongs only to the dedicated lowering package below; the separately owned target-specific source-to-realization composition belongs only to `runen-core-wasm-driver`.

### `crates/runen-core-lowering`

Owns the implementation-only refinement from accepted `runen-hir` typed source structure to validated `runen-core-ir` programs for the currently represented source/Core subset.

It consumes resolved source intent from `runen-hir` and the target proving representation and canonical validator from `runen-core-ir`. It does not own source or Core semantics, source validation, source entry-point selection, Core execution, Exec/Model lowering, realization, or backend behavior.

Its production dependencies are `runen-hir` and `runen-core-ir`. It MUST NOT depend on `runen-reference`, runtime/platform services, a production backend, or filesystem/package discovery in production. Test-only construction of accepted HIR may use `runen-syntax`; explicitly owned end-to-end assurance may also use `runen-reference` and `runen-core-wasm` as test/dev execution consumers. Those proving edges do not transfer Core execution or realization ownership into lowering and do not make either execution consumer part of the production lowering dependency chain.

### `crates/runen-core-ir`

Owns the canonical finite program/function/body semantic data model for the currently implemented Core subset and program-level MIR validation for the structural and language-validity rules expressible by that subset. There is no separate production one-body Core model or validation API.

MIR validation is a language-validation concern. It is not the environment-admission phase defined by `spec/language/lifecycle.md`.

The crate does not execute programs or define host/runtime behavior.

It MUST NOT depend on the reference machine, a production backend, host platform services, or repository tooling.

### `crates/runen-core-wasm`

Owns the first production physical realization for an explicitly admitted bounded subset of validated `runen-core-ir` programs. It performs realization-coverage admission, lowers the admitted Core subset to private WebAssembly, and executes that private representation through Wasmtime/Cranelift.

The package owns no normative language semantics. Its WebAssembly modules, scalar carrier, status/payload protocol, exported function names, fault indexing, control-flow legalization, and Wasmtime configuration are implementation details rather than Runen ABI, layout, entry-point, target-IR, or source-language contracts.

Its production Runen dependency is only `runen-core-ir`. It may use `runen-reference` only as a test/dev dependency for differential conformance. It MUST NOT use the reference machine as a production fallback, depend on source/HIR/lowering or Exec/Model oracle packages, expose Wasm identities as Runen identities, or become a universal target representation for future Exec, Model, GPU, or other realization domains.

Unsupported valid Core remains a realization-coverage rejection rather than a language-validation failure. Backend compilation, instantiation, execution, trap, or physical-resource failures remain realization failures and MUST NOT be reclassified as Runen defined faults or undefined behavior.

### `crates/runen-core-wasm-driver`

Owns the bounded target-specific production composition from an already-built `runen-hir::TypedCompilation`, through the accepted `runen-core-lowering` refinement artifact and its ordinary-function correspondence, into `runen-core-wasm` realization and explicit caller-selected execution.

The package owns no normative language semantics, source executable-entry rule, package/filesystem discovery, backend selection, external-provider identity policy, public generic-specialization identity, public closure identity, stable ABI/layout/symbol identity, or Wasm identity. Its caller-visible function selection remains the existing opaque per-compilation HIR `FunctionId`; the driver does not infer selection from names, accessibility, source order, Core order, or a `main` convention.

Its production Runen dependencies are exactly `runen-hir`, `runen-core-lowering`, and `runen-core-wasm`. The Core function identity carried by the lowering artifact remains an implementation bridge between lowering and Core-Wasm and does not require a direct production dependency on `runen-core-ir` in this package. Tests may use `runen-core-ir` and `runen-syntax` as assurance dependencies. The package MUST NOT make lowering depend on a backend, make Core-Wasm depend on HIR/lowering, use the reference machine as a production fallback, or become a universal runtime/realization abstraction for future backends.

### `crates/runen-exec-oracle`

Owns executable verification-only conformance relations for the currently represented Exec subset.

It is not Runen source syntax, compiler Exec IR, a production runtime or backend, and it owns no normative language semantics. Its finite identities, regions, values, and structured-order tokens exist only to make accepted Exec contracts executable in conformance tests.

The package has no production dependency on another Runen package and remains independent of `runen-reference`. It consumes `runen-core-ir` only as a test/dev dependency for the accepted P0-F composition in which Core-owned `StorageRegion` overlap facts feed the generic Exec ordinary-access conflict relation. That proving dependency does not make Core storage an Exec resource, make Exec own Core region identity or overlap, or create a production Core→Exec package coupling. The package MUST NOT depend on compiler target IR, runtime scheduling or platform services, production backends, or repository tooling.

Future cross-stratum verification may compose independent proving packages only when accepted semantic evidence requires that dependency; package co-location does not itself justify coupling them.

### `crates/runen-numeric-oracle`

Owns executable verification-only conformance relations for the currently represented numeric subset, including binary floating rounding, scalar numeric conversions, same-format unordered floating sums, and bounded tree-rounded sum-candidate evidence.

It is not Runen source syntax, compiler numeric IR, a production numeric runtime, a backend model, or a normative semantic owner. Its bounded integer carriers, exponents, formats, and exact accumulators are verification representation only and do not define Runen implementation limits or physical reduction state.

The package has no production dependency on another Runen package. Accepted cross-stratum verification may add test-only proving dependencies when a normative composition requires evidence. The represented unordered-floating-sum evidence consumes `runen-exec-oracle` only in tests to validate the exact semantic contribution occurrences supplied to numeric evaluation; this does not make Exec identities part of numeric values, make the numeric oracle an Exec implementation, or authorize production coupling between the packages.

### `crates/runen-model-oracle`

Owns executable verification-only conformance relations for the currently represented Model logical-data, value-equivalence, finite-collection, bounded query, bounded single-domain observed Bag-root, bounded singleton/two-domain observation-context, bounded result-target observation, and bounded exact source-result correspondence subsets.

It is not Runen source syntax, compiler Model IR, a planner, a storage engine, a runtime or database system, an incremental engine, and it owns no normative language semantics. Its abstract field, NaN-witness, state-domain, source-observation, result-target, and target-observation tokens plus deterministic internal ordering exist only to make accepted Model contracts executable without exposing storage order, observation order, revision/freshness/progress order, or representative selection as Model semantics. The correspondence witness is test-only and introduces no generic query/computation descriptor or stored provenance object.

The package has no dependency on another Runen package and remains independent of the source/HIR, Core/reference, and Exec package chains. Future cross-stratum or differential verification may compose independent proving packages only when an accepted semantic or assurance consumer requires that dependency; package convenience is not sufficient authority for coupling them.

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
 runen-hir ─────────────┐
                        ▼
              runen-core-lowering
                        ▲
                        │
                 runen-core-ir
                    │       │
                    │       ├──────────────▶ runen-core-wasm
                    │       │                    │
                    │       │                    └──[test-only differential evidence]──▶ runen-reference
                    │       ▼
                    └──▶ runen-reference

runen-hir ───────────────────────────────────────────────▶ runen-core-wasm-driver
runen-core-lowering ─────────────────────────────────────▶ runen-core-wasm-driver
runen-core-wasm ─────────────────────────────────────────▶ runen-core-wasm-driver

runen-core-lowering ──[test-only end-to-end evidence]──▶ runen-reference
runen-core-lowering ──[test-only end-to-end evidence]──▶ runen-core-wasm
runen-core-ir ──────[test-only P0-F region-overlap evidence]──────▶ runen-exec-oracle
runen-exec-oracle ──[test-only P0-F reduction evidence]──────────▶ runen-numeric-oracle
runen-model-oracle

repository tooling is orthogonal
```

`runen-hir` depends only on `runen-syntax` among Runen packages in the source-frontend architecture. `runen-core-lowering` is the only accepted HIR-to-Core production consumer and depends on both `runen-hir` and `runen-core-ir` in production. Its test/dev edges to `runen-reference` and `runen-core-wasm` exist only for explicitly owned end-to-end refinement and differential evidence; they do not themselves create a production source-to-execution composition layer. `runen-core-wasm-driver` is the separate bounded production composition owner above HIR lowering and the Core-Wasm realization; its dependency on those layers does not transfer source ownership into Core-Wasm or realization ownership into lowering. `runen-reference` and `runen-core-wasm` remain independent consumers of validated Core programs: the former is the executable reference semantics, while the latter is a bounded production physical realization with explicit coverage admission. `runen-core-wasm` may consume `runen-reference` only in tests for differential conformance, never as production implementation or fallback. `runen-exec-oracle`, `runen-numeric-oracle`, and `runen-model-oracle` remain verification-only packages outside the source/HIR and Core/reference/realization production chains. The Exec oracle's accepted test-only consumption of Core structural-region overlap evidence composes the Core-owned overlap relation with the Exec-owned ordinary-access conflict relation without creating production coupling or transferring semantic ownership. The numeric oracle's accepted test-only consumption of Exec reduction evidence likewise composes independently owned proving relations; other proving-package composition still requires an accepted semantic or assurance consumer.
