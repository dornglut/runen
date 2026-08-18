# Runen Roadmap

This document owns project sequencing and specification-closure planning. It is non-normative: the language specification under `spec/` defines semantics.

## Current baseline

The repository has an executable A0 Core value/place machine and a provisional decomposed language specification. The represented P0-A Core memory/safety foundation is closed; P0-B is the next sequencing frontier.

## P0-A — Value, memory, and safety

**Depends on:** accepted A0.

The represented Core foundation closes object/storage lifetime, borrows/reborrows, interior mutability, raw-pointer formation and symbolic provenance root, defined raw target read/move/replacement behavior, unsafe-operation preconditions, undefined-behavior separation from defined outcomes, and the safe-abstraction soundness law needed by downstream phases.

This closure applies to semantics that the accepted A0 Core can represent and observe. It does not predefine rules for operations or representations that do not yet exist.

Consumer-dependent memory rules are introduced only with the first phase that can use them:

- relocation, address stability, and pinning are closed when a later operation or realization can relocate storage, expose address-sensitive behavior, or otherwise require a stability guarantee;
- representation-level value validity and invalid-bit-pattern rules are closed with the first byte/representation consumer, such as Buffer mapping/coherence in P0-B or ABI/FFI in P0-D;
- new undefined-behavior cases and unsafe preconditions are owned with the operation that introduces them rather than by a premature complete UB registry;
- source `unsafe`, first-class references/lifetimes, and concrete checking of safe public abstraction contracts are P0-D concerns built on the Core soundness law;
- cross-stratum evidence that later memory, source, and realization rules preserve Core safety belongs to P0-F.

A consuming phase that makes one of these rules necessary must update the appropriate normative owner in the same semantic slice. P0-A closure is not authority for behavior that the normative specification still marks open.

**Gate:** focused executable semantics and adversarial tests form one coherent safe/unsafe memory model for the represented Core operations.

## P0-B — Exec resources and concurrency

**Depends on:** the closed P0-A represented Core safety foundation.

Close Buffer mapping/coherence, cross-realization memory semantics, conflicting-access rules, atomics/order/scope, task lifetime and cancellation, structured parallelism, reductions/collectives, and hierarchical execution. Where these operations first make representation validity, address stability, or additional unsafe/UB rules observable, P0-B closes those concrete rules through their canonical normative owners instead of assuming P0-A predeclared them.

**Gate:** representative scalar CPU, parallel CPU, and GPU realizations can be checked against one semantic contract without violating Core safety.

## P0-C — Numerics

**Depends on:** the realization semantics needed to state heterogeneous guarantees.

Close integer overflow and the `standard`, `reproducible`, and `fast` numeric contracts, including contraction, NaNs, subnormals, transcendental guarantees, conversions, reductions, and emulation/rejection behavior.

**Gate:** legal transformations and cross-realization results follow from source numeric contracts rather than backend defaults.

## P0-D — Source-language completion

**Depends on:** the Core semantic foundations that affect source validation.

Close lexical/concrete grammar, names/scopes/modules/imports, generics and trait coherence, closures/captures, patterns, const/static semantics, fault/panic completion, ABI/layout mechanisms, FFI, linkage, source-level unsafe forms, references/lifetime inference where required, and concrete safe-public-contract validation. Representation validity, address stability, or pinning rules required by those source/ABI/FFI mechanisms are closed here only when their first concrete consumer requires them.

**Gate:** an independent frontend can validate and lower source without inventing language rules.

## P0-E — Model completion

**Depends on:** Core value/type rules and the required bridge/resource contracts.

Close logical typing and absence semantics, joins, grouping/aggregation, cardinality/type inference, identity/keys, state-domain interface details, multi-domain observation compatibility, freshness, and complete observation/materialization/maintenance contracts.

**Gate:** a from-scratch evaluator can determine Model results independently of storage or incremental implementation choices.

## P0-F — Cross-stratum proving

**Depends on:** the relevant preceding closures.

Exercise Core, Exec, Model, and their bridges together using memory litmus cases, Buffer mapping/coherence, CPU/GPU equivalence, structured parallel reductions, Model reference evaluation, incremental differential tests, bridge tests, commit/event cases, and lowering/refinement cases. Include the cross-stratum safety evidence required for consumer-driven memory, unsafe, validity, and address-stability rules introduced by earlier phases.

**Gate:** integrated proving does not require ordinary source to expose implementation machinery merely to preserve the intended semantics.

## Deferred until evidence requires them

Graph/path algebra, Datalog recursion, CRDT semantics, universal replication/network syntax, mandatory information-flow labels, full hard-realtime syntax, live schema migration, stable serialized logical IR, rendering/ECS/UI/field language semantics, and broad package/ecosystem work are not on the P0 critical path.
