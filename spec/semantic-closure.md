# Runen Semantic Closure Program

Status: **normative specification-status ledger**

This document does not itself define missing semantics. It defines the dependency order and acceptance gates required before Runen may claim a complete implementable language specification.

The ordering is semantic, not organizational. Later work MUST NOT be used to paper over unresolved prerequisites.

## P0-A — Value, memory, and safety semantics

**Status:** active; A0 subset accepted.

### Purpose

Close the coherent semantic model underlying all safe and unsafe memory interaction.

### Required topics

- value;
- place/storage location;
- memory object/allocation;
- initialization and partial initialization;
- move versus copy;
- destruction and destruction order;
- object lifetime and deallocation;
- shared versus exclusive access;
- borrow/reference;
- reborrowing;
- interior mutability;
- raw pointer;
- numeric address;
- provenance;
- address stability/pinning;
- value/representation validity;
- undefined behavior;
- unsafe-operation preconditions;
- safe-abstraction soundness.

### Accepted subset

[Annex A0](annex-a-memory.md) already specifies and the reference machine proves:

- values and places for the current structural A0 subset;
- hierarchical initialization;
- partial initialization;
- move/copy;
- assignment;
- deterministic destruction;
- return/fault cleanup.

Later P0-A work must extend or explicitly revise that accepted subset.

### Gate

P0-A is closed only when safe Core access rules, raw-pointer rules, validity/UB rules, and unsafe abstraction obligations form one internally coherent model and are executable through focused litmus/reference tests.

P0-B resource/coherence semantics MUST NOT rely on pointer or borrow assumptions not closed here.

---

## P0-B — Exec resource and concurrency semantics

**Depends on:** P0-A.

### Purpose

Define portable legal access and synchronization across CPU/GPU/accelerator realizations without treating physical placement as language semantics.

### Required topics

- physical allocation/storage spaces;
- logical `Buffer<T>` identity and version/coherence model;
- `View` / `ViewMut` permissions;
- mapped/pinned realizations;
- relocation and raw-address validity;
- formal CPU/GPU abstract memory model;
- ordinary conflicting access/data-race rules;
- atomic order vocabulary;
- atomic scope lattice;
- synchronization relations;
- task lifetime/spawn/await/fault propagation;
- asynchronous cancellation;
- `each` legality;
- reductions/collectives;
- hierarchical group/subgroup semantics;
- schedule/transformation legality contracts if an authoring API is retained.

### Gate

P0-B is closed only when representative CPU scalar, CPU parallel, and GPU realizations can be compared against one semantic contract and when Buffer mapping/coherence cannot invalidate Core pointer/borrow rules.

---

## P0-C — Numeric baseline

**Depends on:** enough P0-B realization semantics to state cross-realization guarantees.

### Purpose

Define arithmetic behavior precisely enough that optimization and heterogeneous equivalence are meaningful.

### Required topics

- default integer overflow behavior;
- checked/wrapping/saturating operation contracts;
- `standard` floating-point contract;
- `reproducible` contract;
- `fast` relaxations;
- operation accuracy;
- contraction/FMA rules;
- NaN semantics;
- subnormal handling;
- transcendental guarantees;
- conversion/rounding rules;
- reduction/reassociation equivalence;
- unsupported-realization emulation versus rejection.

### Gate

P0-C is closed only when a program's numeric contract determines which CPU/GPU transformations/results are legal without depending on debug mode, backend defaults, or undocumented hardware behavior.

---

## P0-D — Core language completion

**Depends on:** P0-A and the numeric baseline decisions that affect expression semantics.

### Purpose

Turn the semantic kernel into a complete source language rather than only a semantic architecture.

### Required topics

- lexical grammar;
- concrete grammar and parsing ambiguities;
- names/scopes;
- module/import resolution;
- generic parameter semantics;
- trait/interface coherence and dispatch boundaries;
- closure/capture semantics;
- pattern semantics/exhaustiveness;
- const-evaluation restrictions;
- static initialization;
- complete fault/panic/unwind/catch rules, if catching is retained;
- explicit ABI/layout mechanisms;
- FFI boundary rules;
- entry points/linkage where language-level;
- source-level unsafe forms.

### Gate

P0-D is closed only when an independent implementation can parse and validate the Core source language and lower it into the accepted semantic kernel without inventing language rules.

A parser implementation is not proof that this gate is closed unless the grammar and name/type rules are normative.

---

## P0-E — Minimal Model algebra completion

**Depends on:** Core value/type semantics and the relevant bridge/resource contracts.

### Purpose

Finish the declarative Model stratum precisely without importing a database, ECS, graph, or incremental implementation as semantics.

### Already accepted in the provisional language specification

The following are **not** open P0-E design questions anymore unless later proving deliberately revises them:

- `Relation<T>` is an unordered set;
- `Bag<T>` is an unordered multiset;
- `Sequence<T>` is an ordered sequence;
- query results preserve multiplicity by default (bag semantics);
- `distinct` removes multiplicity explicitly;
- Relation/Bag do not gain semantic iteration order from storage;
- `order by` produces a Sequence;
- tied ordering keys leave relative tie order unspecified unless further semantic keys distinguish them;
- the base query vocabulary is `from`, `where`, `select`, `derive`, `join`, `group`, `aggregate`, `distinct`, and `order`;
- Graph/Field/path/window/general set-combination algebras are deferred rather than silently part of the base Model algebra;
- `ObservationSet` is immutable for an evaluation/reaction wave and does not imply a global distributed snapshot;
- rule proposals/events are staged before commit;
- transition state changes and logical events acquire logical existence together with successful commit;
- ordinary rules may read many state domains but mutate one;
- incremental equivalence is measured against from-scratch evaluation of the corresponding source `ObservationSet`.

### Required remaining topics

- exact logical type checking;
- logical record typing/identity details;
- absence/null/optional semantics;
- complete join semantics;
- grouping semantics;
- aggregation semantics;
- query type/cardinality inference;
- identity and stable logical keys beyond declared uniqueness constraints;
- exact state-domain interface;
- revision ordering/visibility contracts where not domain-specific;
- immutable `ObservationSet` compatibility rules across state domains;
- exact freshness representation;
- complete `observe` observation protocol where needed;
- complete `materialize` observation/retention contract where needed;
- maintenance target admission/failure/retry/reconciliation/visibility contracts;
- a from-scratch reference evaluator/oracle for the accepted Model subset.

### Gate

P0-E is closed only when a from-scratch reference evaluator can determine Model query/rule results independently of storage/index/incremental implementation choices and when multi-domain observation/maintenance behavior is precise enough for independent implementations to agree.

---

## P0-F — Cross-stratum conformance

**Depends on:** the relevant preceding semantic closures.

### Purpose

Falsify the Core · Exec · Model architecture with integrated workloads before broad ecosystem work makes the architecture expensive to revise.

### Required proving classes

- Core value/memory litmus programs;
- safe abstraction adversarial cases;
- Buffer mapping/coherence cases;
- CPU scalar versus SIMD/multicore equivalence;
- CPU versus GPU numeric/memory cases;
- `each`/reduction legality cases;
- Model from-scratch query/rule oracle;
- incremental versus from-scratch differential tests;
- Model→Core snapshot/handle bridge tests;
- Model→Exec materialization/Buffer bridge tests;
- state-domain commit/event atomicity cases;
- optimizer/lowering refinement cases;
- conformance-profile composition cases.

### Gate

P0-F closes only when the integrated tests demonstrate that the strata compose without requiring routine source to expose implementation machinery that the language intended to abstract.

---

## Deferred work

The following are intentionally **not** prerequisites for the first complete Core/Exec/Model semantic specification unless later evidence makes them necessary:

- graph/path algebra;
- Datalog recursion;
- CRDT semantics;
- universal distributed replication;
- one universal network syntax;
- mandatory information-flow labels in ordinary Core;
- full hard-realtime surface syntax;
- live catalog/schema migration;
- stable serialized Logical IR;
- rendering language semantics;
- ECS semantics;
- UI diffing semantics;
- Field/spatial algebra;
- component/package versioning beyond the minimum required for Core language completion.

These may become later profiles, standard-environment modules, or separate proposals.

## Simplicity kill gate

Runen should proceed only while all three remain true:

1. **semantic rigor** — behavior is precise enough for independent implementation, optimization, heterogeneous realization, and testing;
2. **source simplicity** — ordinary users reason primarily with problem concepts rather than compiler machinery;
3. **differentiated value** — safe systems Core + deterministic heterogeneous Exec + logical declarative Model is materially more coherent than assembling unrelated language/library/runtime systems.

If normal source becomes dominated by provenance annotations, effect plumbing, region proofs, placement, clocks, revisions, coherence state, or scheduler controls, the preferred response is simplification or retreat, not another universal abstraction.