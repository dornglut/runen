# Runen Roadmap

This document owns project sequencing and specification-closure planning. It is non-normative: the language specification under `spec/` defines semantics.

## Current baseline

The repository has an executable A0 Core value/place machine and a provisional decomposed language specification.

## P0-A — Value, memory, and safety

**Depends on:** accepted A0.

Close the remaining Core memory/safety model: object lifetime, borrows/reborrows, interior mutability, raw pointers, provenance, pinning/address stability, validity, undefined behavior, unsafe-operation preconditions, and safe-abstraction soundness.

**Gate:** focused executable semantics and adversarial tests form one coherent safe/unsafe memory model.

## P0-B — Exec resources and concurrency

**Depends on:** P0-A.

Close Buffer mapping/coherence, cross-realization memory semantics, conflicting-access rules, atomics/order/scope, task lifetime and cancellation, structured parallelism, reductions/collectives, and hierarchical execution.

**Gate:** representative scalar CPU, parallel CPU, and GPU realizations can be checked against one semantic contract without violating Core safety.

## P0-C — Numerics

**Depends on:** the realization semantics needed to state heterogeneous guarantees.

Close integer overflow and the `standard`, `reproducible`, and `fast` numeric contracts, including contraction, NaNs, subnormals, transcendental guarantees, conversions, reductions, and emulation/rejection behavior.

**Gate:** legal transformations and cross-realization results follow from source numeric contracts rather than backend defaults.

## P0-D — Source-language completion

**Depends on:** the Core semantic foundations that affect source validation.

Close lexical/concrete grammar, names/scopes/modules/imports, generics and trait coherence, closures/captures, patterns, const/static semantics, fault/panic completion, ABI/layout mechanisms, FFI, linkage, and source-level unsafe forms.

**Gate:** an independent frontend can validate and lower source without inventing language rules.

## P0-E — Model completion

**Depends on:** Core value/type rules and the required bridge/resource contracts.

Close logical typing and absence semantics, joins, grouping/aggregation, cardinality/type inference, identity/keys, state-domain interface details, multi-domain observation compatibility, freshness, and complete observation/materialization/maintenance contracts.

**Gate:** a from-scratch evaluator can determine Model results independently of storage or incremental implementation choices.

## P0-F — Cross-stratum proving

**Depends on:** the relevant preceding closures.

Exercise Core, Exec, Model, and their bridges together using memory litmus cases, Buffer mapping/coherence, CPU/GPU equivalence, structured parallel reductions, Model reference evaluation, incremental differential tests, bridge tests, commit/event cases, and lowering/refinement cases.

**Gate:** integrated proving does not require ordinary source to expose implementation machinery merely to preserve the intended semantics.

## Deferred until evidence requires them

Graph/path algebra, Datalog recursion, CRDT semantics, universal replication/network syntax, mandatory information-flow labels, full hard-realtime syntax, live schema migration, stable serialized logical IR, rendering/ECS/UI/field language semantics, and broad package/ecosystem work are not on the P0 critical path.