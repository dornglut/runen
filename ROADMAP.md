# Runen Roadmap

This document owns project sequencing and specification-closure planning. It is non-normative: the language specification under `spec/` defines semantics.

## Current baseline

The repository has an executable A0 Core value/place machine and a provisional decomposed language specification. The represented P0-A Core memory/safety, P0-B Exec resources/concurrency, and P0-C numeric foundations are closed; P0-D is the next sequencing frontier.

## P0-A — Value, memory, and safety

**Depends on:** accepted A0.

The represented Core foundation closes the P0-A obligations that A0 can express: object/storage lifetime, borrows/reborrows, interior mutability, raw-pointer formation and symbolic provenance root, defined raw target read/move/replacement behavior, unsafe-operation preconditions, undefined-behavior separation from defined outcomes, and the safe-abstraction soundness law needed by downstream phases.

This closure applies only to semantics that the accepted A0 Core can represent and observe. It does not predefine rules for operations or representations that do not yet exist, change any normative document's status, or convert an open specification item into defined behavior.

Consumer-dependent memory rules are introduced only with the first phase that can use them:

- relocation, address stability, and pinning are closed when a later operation or realization can relocate storage, expose address-sensitive behavior, or otherwise require a stability guarantee;
- representation-level value validity and invalid-bit-pattern rules are closed only when a phase first exposes bytes or representations, for example through a P0-B Buffer mapping contract if it becomes representation-observable or through P0-D ABI/FFI;
- operation-specific unsafe preconditions are added with the operation's canonical semantic owner, while undefined-behavior classification remains owned by Core unsafe semantics and expands only when a concrete new precondition requires it;
- source `unsafe`, first-class references/lifetimes, and concrete checking of safe public abstraction contracts are P0-D concerns built on the Core soundness law;
- cross-stratum evidence that later memory, source, and realization rules preserve Core safety belongs to P0-F.

A consuming phase that makes one of these rules necessary must update the appropriate normative owner in the same semantic slice. P0-A closure is not authority for behavior that the normative specification still marks open.

**Gate:** focused executable semantics and adversarial tests form one coherent safe/unsafe memory model for the represented Core operations.

## P0-B — Exec resources and concurrency

**Depends on:** the closed P0-A represented Core safety foundation.

The represented Exec foundation closes the P0-B obligations required by the current gate: Buffer logical mapping/coherence and physical accessibility, ordinary conflicting-access rules, structured `each`, hierarchy and cohort barriers, identity-bearing unordered reductions, represented structured-task lifetime/detachment, normal join and cooperative cancellation observation, atomic exchange/modification order/direct release-acquire relations across the represented unscoped/root/group/subgroup scope forms where defined, and cross-realization preservation evidence for the applicable contracts.

This closure applies only to the operations and relations that the accepted Exec specification currently represents. It does not define operation families or interactions that their normative owners still leave open, including additional atomic operations, release sequences, sequential consistency, fences, the remaining unscoped-to-structured atomic scope interactions or broader scopes, additional collectives or group-local storage, abnormal structured completion, source placement/target syntax, environment admission, hardware topology, or backend coherence and transfer protocols. P0-B closure is not authority for those open items.

The represented P0-B mapping surface does not expose bytes or raw physical addresses, so it does not by itself trigger the deferred representation-validity, address-stability, or pinning rules identified under P0-A. Integrated cross-stratum proof that later realization and resource rules preserve Core safety remains a P0-F obligation.

**Gate:** representative scalar CPU, parallel CPU, and GPU realizations can be checked against one semantic contract without violating Core safety.

## P0-C — Numerics

**Depends on:** the realization semantics needed to state heterogeneous guarantees.

Close integer overflow and the `standard`, `reproducible`, and `fast` numeric contracts, including contraction, NaNs, subnormals, transcendental guarantees, conversions, reductions, and emulation/rejection behavior.

The represented numeric foundation closes the P0-C obligations required by the current gate: fixed-width integer overflow outcomes and modes; `standard`, `reproducible`, and `fast` contract authority, defaulting, refinement, and named relaxations; binary floating rounding, special values, basic arithmetic, and conversions; correctly rounded sine as the represented transcendental baseline; same-format unordered floating sums including special-value participation and bounded `fast` tree/permutation freedom; realization-neutral preservation of the selected contract through direct realization, emulation, or admission/rejection where applicable; and executable/reference evidence for rounding, conversions, reductions, and the independent sine corpus.

This closure applies only to operations and relations that the accepted numeric specification currently represents. It does not define source contract-selection syntax or scoping, a concrete source floating-type inventory, broader transcendental families, sine-specific `fast` approximation latitude, NaN representation identity or canonicalization, vector forms, const evaluation, ABI/layout behavior, physical range-reduction or instruction algorithms, backend target taxonomy, or integrated CPU/GPU implementation equivalence. Source-language consumers belong to P0-D, while integrated source/lowering/backend proving remains a P0-F obligation.

**Gate:** legal transformations and cross-realization results follow from source numeric contracts rather than backend defaults.

## P0-D — Source-language completion

**Depends on:** the Core semantic foundations that affect source validation.

Close lexical/concrete grammar, names/scopes/modules/imports, generics and trait coherence, closures/captures, patterns, const/static semantics, the represented defined-fault abnormal-completion baseline, ABI/layout mechanisms, FFI, linkage, source-level unsafe forms, references/lifetime inference where required, and concrete safe-public-contract validation. For P0-D, represented defined faults are non-recoverable inside represented Runen source and follow the accepted outward propagation relation; recoverable domain/application failures remain ordinary values/results under their applicable contracts. `panic`/`throw`, fault payload/value types, catch/recovery, exception hierarchies, checked-exception/effect signatures, and physical unwinding are not P0-D baseline requirements and require a separately accepted future consumer/extension if introduced. Representation validity, address stability, or pinning rules required by those source/ABI/FFI mechanisms are closed here only when their first concrete consumer requires them.

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
