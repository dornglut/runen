# Runen Language Specification

Version: **0.9-provisional**  
Status: **provisional normative semantic architecture**

This document defines the currently accepted Runen language architecture. It is intentionally precise about frozen semantic invariants and intentionally explicit about unresolved P0 details.

Runen is not yet a complete implementable language specification. Conformance claims are governed by [Conformance & Assurance](conformance.md), and unresolved semantic work is governed by [Semantic Closure](semantic-closure.md).

Unless a section explicitly says otherwise, source syntax and API spelling are **illustrative**.

External papers, specifications, languages, libraries, and runtimes are not normative Runen authority.

---

## 1. Language purpose

Runen is one language spanning three semantic strata:

```text
                 Runen

        ┌──────────┬──────────┐
        │          │          │
       Core       Exec       Model
        │          │          │
   values and   executable   logical
    memory         work        state
        │          │          │
        └──── explicit bridges ┘

              Realization
                   │
          physical implementation
```

The strata are semantic responsibilities, not mandatory runtime layers or compiler crates.

- **Core** defines ordinary values, memory, ownership, control, effects, and low-level interaction.
- **Exec** defines execution-visible work and resources whose legal realization may vary across sequential, parallel, vector, GPU, or other execution environments.
- **Model** defines logical facts, relations, queries, reactions, maintained state, and state-domain observations independently of one mandatory physical storage model.

No fourth top-level semantic stratum is currently part of Runen.

### 1.1 Goals

Runen is designed to support, through one coherent type and semantic system:

- freestanding systems programming;
- ordinary native applications;
- deterministic heterogeneous computation where contracts permit it;
- declarative application/domain modeling where the problem is naturally relational or reactive;
- explicit low-level escape hatches without making low-level machinery routine source burden;
- independent implementations capable of agreeing on observable behavior;
- multiple independent runtime/application instances without hidden process-global language state.

Runen has a functional bias: immutable bindings, pure derivation, algebraic data, pattern matching, and inferred effects SHOULD be the ordinary case where order or mutation is not semantically required.

Runen is declarative-first only where the problem itself is declarative. Algorithms whose order, synchronization, representation, or iteration strategy is semantically relevant remain ordinary Core/Exec computation.

### 1.2 Freestanding completeness

A conforming Core implementation MUST NOT require, merely to implement Core semantics:

- an operating system;
- libc;
- a heap allocator;
- a tracing garbage collector;
- a thread runtime;
- an async executor;
- a JIT;
- Model facilities;
- a global catalog;
- a GPU runtime.

Profiles may require additional environment facilities.

### 1.3 Non-goals

Runen does **not** promise that:

- every function executes on every execution space;
- arbitrary sequential code automatically becomes efficient GPU code;
- all parallel algorithms are deterministic without sufficient contracts;
- all data belongs to Model;
- all state uses transactions, MVCC, CRDTs, ECS, render graphs, dependency graphs, or any other single implementation technique;
- remote machines behave as shared memory;
- all computations are incremental;
- floating-point results are bit-identical by default;
- all implementation specializations can be automatically proven equivalent;
- all live state changes can be migrated;
- semantic rigor justifies exposing compiler machinery in ordinary source.

---

## 2. Normative semantic dimensions

Runen deliberately does not collapse unrelated correctness questions into one universal mechanism.

Depending on the operation or profile, semantics may involve these orthogonal dimensions:

- values and types;
- observable behavior;
- ownership and resource permission;
- effects;
- determinism;
- ordering and time;
- execution features;
- environment admission;
- progress/liveness;
- numeric contract;
- authority/security;
- observation/isolation;
- freshness;
- durability;
- failure model.

An operation need not carry every dimension.

### 2.1 Effects

An effect is semantically observable interaction outside ordinary pure value derivation, for example I/O, volatile access, time/random observation, external mutation, event emission, or communication.

Effects SHOULD be inferred when they can be derived without obscuring a real semantic boundary.

Purity does not imply termination, absence of defined faults, safe speculation, or numeric reproducibility.

### 2.2 Execution feature versus authority

Runen distinguishes:

- **execution feature** — the environment is technically capable of performing an operation;
- **authority** — code is permitted to request or perform an operation.

Hardware capability MUST NOT implicitly grant security authority.

### 2.3 Information flow

Information-flow policy is distinct from ordinary access authority. A Security profile MAY define confidentiality/integrity labels, release/declassification, endorsement, or related rules.

Ordinary Core does not currently require mandatory information-flow annotations on every value.

---

## 3. Program behavior

A valid Runen program denotes a set of permitted observable behaviors.

Conceptually:

```text
Behavior = outcome + observable trace
```

The exact formalization will be refined by later annexes, but this abstraction is normative.

### 3.1 Observable behavior

Potential observations include, when admitted by the applicable profile:

- externally visible I/O;
- volatile/MMIO operations;
- state-domain commits;
- public logical events;
- network-visible actions;
- explicit host/environment effects.

Implementation choices such as register allocation, temporary storage, cache policy, CPU-core choice, GPU lane numbering, query-plan shape, or physical data layout are not observable merely because an implementation exposes them diagnostically.

### 3.2 Outcomes

Core includes normal return, defined fault, and divergence as semantically distinct possibilities where applicable.

Profiles may additionally define cancellation or environment-failure outcomes. Exact asynchronous cancellation semantics remain open and MUST NOT be inferred from host-language behavior.

Recoverable domain/application failure SHOULD normally be modeled as an ordinary value when that failure is part of the program's normal contract.

### 3.3 Behavior refinement

A correctness-preserving lowering or transformation MUST NOT introduce an observable behavior forbidden by its source semantics.

Conceptually:

```text
Behaviors(lowered) ⊆ Behaviors(source)
```

under the applicable abstraction mapping.

This trace/behavior refinement obligation applies to lowering, optimization, scheduling, physical layout, placement, query planning, specialization, incremental realization, and migration where those transformations are relevant.

Trace refinement is not sufficient for every Runen correctness question; see Section 4.

---

## 4. Distinct correctness obligations

Runen separates correctness relations that are often incorrectly conflated.

### 4.1 Trace/behavior refinement

A realization or transformation does not add forbidden observable behaviors.

### 4.2 Progress/liveness

Safety and typing do not imply eventual completion. Progress, fairness, deadlines, bounded response, or eventual propagation require separate assumptions and guarantees.

### 4.3 Numeric equivalence

Two realizations may be behaviorally legal yet differ numerically. The applicable numeric mode determines which differences are permitted.

### 4.4 Incremental equivalence

An observed materialized or maintained result MUST be observationally equivalent, at the observation point and source observation identified by its freshness contract, to evaluation of the defining logical computation from scratch.

Incremental data structures or dependency graphs are implementation mechanisms, not the semantic definition.

### 4.5 Security/hyperproperty obligations

Some confidentiality/integrity properties concern relationships among multiple executions or traces and cannot be reduced to ordinary trace inclusion. Security profiles therefore define separate obligations where required.

---

## 5. Determinism, ordering, and reproducibility

Runen distinguishes three concepts.

### 5.1 Semantic determinism

For the same explicit inputs and admitted external observations, all permitted executions produce observationally equivalent results.

### 5.2 Schedule independence

Changing the legal physical execution schedule does not change the observable result.

A computation may have unspecified physical execution order while remaining schedule-independent.

### 5.3 Heterogeneous reproducibility

Two distinct admitted realizations, for example CPU and GPU, satisfy the numerical/reproducibility relation required by the applicable numeric contract.

Heterogeneous reproducibility is not implied merely by semantic determinism.

### 5.4 Intentional nondeterminism

Source may explicitly admit multiple results, for example through random observation, external arrival order, race/arbitration constructs whose semantics permit alternatives, wall-clock observation, or explicit arbitrary selection.

Incidental implementation order MUST NOT silently become a nondeterminism source when source semantics do not admit it.

---

## 6. Language lifecycle

Runen distinguishes:

```text
Parse
  ↓
Language Validation
  ↓
Environment Admission
  ↓
Realization
  ↓
Execution
```

These phases are semantic distinctions; implementations need not literally use five compiler/runtime stages.

### 6.1 Language validation

Language validation determines whether a program satisfies the language rules supported by the claimed profile, including the applicable syntax, names, types, ownership, effects, resources, and unsafe preconditions that are statically checkable.

A program rejected because the environment lacks a required GPU feature is not therefore an ill-typed Core program.

### 6.2 Environment admission

Admission checks hard environment requirements such as an execution feature, authority, memory capability, ABI, realtime guarantee, or other profile-specific facility.

A hard requirement MUST either be admitted or rejected. It MUST NOT silently degrade into an optimization preference.

### 6.3 Realization

Realization chooses a legal physical implementation subject to language semantics and admitted environment contracts.

Placement, scheduling, transfer, layout, specialization, materialization, and incremental maintenance MAY be realization choices when semantics permit them.

### 6.4 Requirement versus preference

A **requirement** constrains correctness/admission.

A **preference** is an optimization request that an implementation MAY ignore. When practical, tooling SHOULD make ignored preferences explainable.

---

## 7. Core semantic stratum

Core owns ordinary language values and memory semantics.

### 7.1 Core responsibilities

Core includes the semantic foundations for:

- scalar and aggregate values;
- native structural types;
- local/static/storage places;
- initialization and destruction;
- move/copy ownership transfer;
- mutation;
- references/borrows;
- raw pointers and low-level memory;
- control flow and ordinary functions;
- effects and faults;
- generics/trait-like compile-time abstraction;
- const/static facilities;
- explicit external ABI and low-level architecture interfaces.

Not all of these are semantically closed yet.

### 7.2 Values and places

A value and a storage place are distinct semantic categories.

Accepted executable A0 rules for values, places, hierarchical initialization, move, copy, assignment, destruction, return, and fault cleanup are defined by [Annex A0](annex-a-memory.md).

Later memory/safety annexes MUST extend rather than silently contradict the accepted subset unless an explicit normative revision changes it.

### 7.3 Native structural types versus Model values

Core native structural types have ordinary language value/memory semantics.

A Model `record` is a logical value whose physical storage layout is not its semantic identity.

Crossing that boundary follows the bridge laws in Section 12.

### 7.4 Ownership

Runen targets affine ownership: a value has one logical owner unless its type and operation permit copy or shared access.

The exact lifetime, borrow, reborrow, interior-mutability, and safe aliasing rules remain P0-A work.

### 7.5 Shared and exclusive borrows

Illustrative forms such as `&T` and `&mut T` denote shared and exclusive borrowing concepts respectively.

Their exact validity, lifetime, reborrow, interior-mutability, and alias rules are not frozen by this document.

### 7.6 Address, pointer, provenance, and authority

Runen distinguishes:

- numeric address;
- language pointer/reference;
- provenance/validity information;
- resource permission;
- security authority.

A pointer is not semantically defined as merely an integer address.

Exact provenance rules are P0-A and remain open.

### 7.7 Unsafe code

Safe Runen MUST NOT require the caller to satisfy hidden undefined-behavior preconditions that are not represented by safe contracts.

An unsafe operation may expose proof obligations that the compiler cannot establish automatically.

A safe abstraction implemented using unsafe operations MUST discharge those obligations for **all** uses permitted by its safe public contract. An unsafe implementation detail is not allowed to transfer an undisclosed proof obligation to safe callers.

The complete unsafe operation list, validity model, pointer provenance rules, and UB taxonomy remain P0-A/P0-D work.

### 7.8 Integer arithmetic

Fixed-width integer arithmetic MUST have language-defined semantics; signed overflow MUST NOT become undefined behavior merely because a backend uses machine integers; and debug/release mode MUST NOT change language meaning.

Checked, wrapping, and saturating operations are part of the intended arithmetic model.

The default overflow behavior of plain arithmetic remains **open**.

### 7.9 Floating point

Runen retains three conceptual numeric modes:

- **standard** — portable baseline without arbitrary fast-math transformations;
- **reproducible** — stronger cross-realization repeatability, with emulation or rejection when required;
- **fast** — explicit documented numerical relaxations.

Exact operation accuracy, contraction, transcendental, NaN, subnormal, reduction, and cross-device rules remain P0-C.

### 7.10 Layout and ABI

Default native layout and ABI are not implicitly stable.

Stable/external layout and ABI require explicit mechanisms whose complete semantics remain P0-D.

---

## 8. Exec semantic stratum

Exec owns execution-visible work and logical executable resources.

### 8.1 `fn` and `task`

A normal function executes in its current execution context.

A `task` denotes computation visible to realization as an independent execution unit. A task may legally realize as a direct call, vectorized work, multicore work, GPU dispatch, accelerator operation, or another admitted realization when its contracts allow it.

Being a `task` does not itself guarantee asynchrony or GPU execution.

### 8.2 Structured concurrency

Runen supports structured task lifetimes as an architectural invariant: borrowed resources used by child work MUST NOT silently outlive the scope that makes those borrows valid.

Detached work must own or otherwise independently retain all state it requires.

Exact spawn/await/cancellation/fault propagation semantics remain P0-B/P0-D.

### 8.3 `for`

Ordered sequential iteration preserves source-defined relative iteration order.

### 8.4 `each`

`each` removes source-defined relative order among iterations.

Safe inter-iteration interaction is legal only through contracts that establish a valid interaction model, for example:

- disjoint mutation;
- shared reads;
- atomics;
- explicit reductions;
- commutative accumulation;
- explicit collectives.

`each` is not synonymous with nondeterminism. If participating operations are schedule-independent, an unordered physical schedule may still have one deterministic observable result.

### 8.5 Parallel patterns

Exec may standardize higher-level patterns such as map, reduce, scan, partition, and tile when their algebraic contracts permit stronger realization reasoning.

Reduction implementations MAY exploit only algebraic laws actually guaranteed by the reduction operator/contract.

### 8.6 Algorithm, schedule, specialization

Runen distinguishes:

- **algorithm/implementation** — defines the computation;
- **schedule** — changes physical arrangement without changing permitted behavior;
- **specialization** — alternative implementation of the same public semantic operation, potentially under stronger feature assumptions.

A scheduling transformation MUST have a legality/refinement contract. Specializations create equivalence/conformance obligations even when full automatic proof is unavailable.

### 8.7 Allocation

An allocation is physical storage in a specified/selected memory space. It is not transparently migratable merely because another device can access the logical data.

### 8.8 Buffer

`Buffer<T>` is a **logical coherent Exec resource**. Its logical identity is distinct from any one backing allocation or raw address.

A conforming realization may maintain, migrate, or replicate physical backing only according to the Buffer coherence contract.

`View` and `ViewMut` are logical permission-bearing views, not promises of permanently stable raw physical addresses.

Exposing a raw physical address requires an explicit mapped/pinned realization that constrains relocation for the validity duration of that address.

The complete Buffer coherence state machine is P0-B and remains open.

### 8.9 Transfers and placement

Physical transfer may be automatic when semantically legal, but MUST remain visible to realization policy, diagnostics, and cost/explainability tooling. Transparency in source does not make transfer cost semantically nonexistent.

### 8.10 Memory model and atomics

Safe Runen does not permit conflicting ordinary non-atomic accesses without the required ordering/permission relationship.

The formal CPU/GPU memory model, atomic order vocabulary, and atomic scope lattice remain P0-B. No host memory model is normative by default.

### 8.11 Hierarchical execution

Group/subgroup operations, group-local memory, barriers, broadcast/shuffle, and other hardware-shaped facilities belong to progressive Exec exposure rather than a separate GPU language.

Exact APIs and portable guarantees remain open.

---

## 9. Model semantic stratum

Model is an optional semantic stratum for logical state and derivation. A conforming Core-only implementation need not implement Model.

### 9.1 State domain

A **state domain** is the unit that controls a coherent set of authoritative logical state, invariants, revisions, admission, and commits.

The term **authority** is reserved for security/capability meaning and MUST NOT be used as the primary Model ownership term.

A state domain may additionally define observation/isolation, durability, failure, replication, or maintenance contracts when applicable.

No state domain is implicitly process-global.

### 9.2 Logical declarations

The intended Model vocabulary includes concepts such as:

- `record` — logical structured value;
- `property` — typed subject-to-value fact;
- `tag` — nominal unary fact;
- `predicate` — derived logical fact;
- `relation` — typed n-ary relationship;
- `query` — pure logical derivation;
- `rule` — declarative/reactive state transition;
- `observe` — request logical change observation;
- `materialize` — request retained realization;
- `maintain` — request ongoing correspondence under an explicit target contract.

Exact surface syntax is illustrative.

Graph and Field are **not** fundamental universal Model value categories. Specialized graph, spatial-field, ECS, rendering, UI, or domain systems may be built as libraries/profiles over the more general semantics.

### 9.3 Relation, multiplicity, and order

A relation is unordered unless an operation establishes order.

Operations whose result depends on an element being "first" therefore require an ordered input/contract.

Explicit arbitrary selection may exist, but MUST make the arbitrariness/nondeterministic allowance semantically visible.

Bag and sequence semantics, complete join/group/aggregation rules, and cardinality inference remain P0-E.

When an ordering key does not fully distinguish elements, equal-key order is not silently stable by physical storage order. A deterministic tie order requires an explicit semantic tie-breaker or stronger ordered-source contract.

### 9.4 Query purity

Queries are pure logical derivations unless an explicitly defined future operation/profile says otherwise.

A query plan, index, ECS archetype, database table, GPU representation, differential-dataflow graph, or other physical structure is not the logical query semantics.

### 9.5 ObservationSet

A Model evaluation spanning state domains is evaluated relative to an explicit immutable **`ObservationSet`** identifying the admitted observations for that evaluation/reaction wave.

An `ObservationSet` is immutable for that reaction wave.

It does **not** imply one globally synchronized distributed snapshot and MUST NOT be described as such unless a stronger profile explicitly guarantees one.

Compatibility rules for composing observations from multiple state domains remain P0-E.

### 9.6 Revisions and causal frontiers

A state revision identifies state-domain version/progress in that domain's state semantics.

A causal frontier describes causal knowledge/order where a profile requires it.

Neither concept is a clock domain by definition.

### 9.7 Rule reaction wave

Conceptually, a rule reaction wave follows:

```text
admitted triggers/events
        ↓
immutable ObservationSet
        ↓
match/query
        ↓
pure derivation
        ↓
staged effect proposals
        ↓
state-domain admission
        ↓
commit
        ↓
committed logical events become available to later waves
```

A reaction wave is a logical instant; physical execution may take nonzero wall time.

### 9.8 Staged effects and commit-coupled events

Rule mutation proposals such as set/insert/remove are staged. They are not immediately visible to other matching/derivation in the same reaction wave unless an explicit future semantic rule says otherwise.

For a successful state-domain commit, the admitted state changes and logical events defined as part of that commit become committed together. An implementation MUST NOT expose the event as committed while the corresponding state transition is not, or vice versa, unless the event's contract explicitly defines different semantics.

### 9.9 One mutable state domain

An ordinary rule may read observations from multiple state domains but mutates at most one state domain per commit/reaction transition.

This restriction prevents ordinary Rule semantics from silently becoming a universal distributed transaction model.

Cross-domain coordination requires explicit contracts/profiles.

### 9.10 Conflicts and accumulation

Conflicting proposals MUST NOT be resolved by incidental worker/scheduler order.

Resolution must be explicit: rejection, arbitration, accumulation, or a state-domain-defined conflict rule whose semantics are published.

Multi-source accumulation requires the algebraic laws needed by legal parallel realization.

### 9.11 Observe and materialize

`observe` requests observation semantics; it does not mandate one incremental implementation.

`materialize` requests retained realization. Full recomputation, caching, indexes, incremental views, GPU representations, or dependency graphs may all be legal implementations when they preserve the defining semantics.

### 9.12 Maintain

`maintain` requests ongoing semantic correspondence between a defining source computation and a target.

Every maintenance target MUST publish the applicable:

- source observation/revision relationship;
- freshness contract;
- progress expectation;
- failure behavior;
- target commit/update semantics.

`maintain` MUST NOT imply universal reliable synchronization, distributed transactions, or zero-latency propagation.

### 9.13 Incremental equivalence

At every observation point permitted by its freshness contract, an observed materialized or maintained result MUST be observationally equivalent to evaluating the defining logical computation from scratch over the corresponding admitted source observations.

### 9.14 Non-quiescent rule systems

Type/memory safety does not prove that a rule system reaches quiescence. Implementations and profiles may need budgets, diagnostics, quotas, cycle/causal reporting, or progress controls.

Those mechanisms do not change the logical rule meaning merely by existing.

---

## 10. Time, revision, freshness, and progress

Runen does not define one implicit universal clock.

### 10.1 Clock domain

A clock domain defines when temporal values/events are active or sampled. It may represent simulation ticks, frames, audio samples, monotonic time, or another temporal coordinate.

Moving information between clock domains requires an explicit semantic operation/contract such as sampling, holding, buffering, synchronization, interpolation, or resampling. Exact standard APIs remain open.

### 10.2 State revision

State revision belongs to state-domain observation/commit semantics. It is not automatically wall time or a clock tick.

### 10.3 Causal frontier

A causal frontier belongs to causal/distributed ordering semantics when a profile defines it. It is not automatically a state revision or clock.

### 10.4 Freshness

Freshness identifies how current an observation/materialization/maintenance result must be relative to its source observations.

Freshness is distinct from correctness and distinct from propagation progress.

### 10.5 Progress

Progress guarantees require explicit assumptions. `await`, observation, maintenance, or a valid task does not by itself prove eventual completion.

Realtime/deadline guarantees belong to a profile whose environment admission can establish the required assumptions. A hard deadline MUST NOT silently degrade to best effort.

---

## 11. Remote boundaries

Remote interaction is **not shared memory**.

A Network or distributed profile must define message/protocol, failure, ordering, identity, serialization, observation, authority, and consistency contracts appropriate to that boundary.

Ordinary references, borrows, raw pointers, or Buffer physical addresses MUST NOT silently acquire remote-shared-memory meaning merely because a runtime can communicate with another machine.

Network protocols, CRDTs, replication strategies, RPC systems, and distributed transactions are optional mechanisms/profiles rather than universal Core/Model semantics.

---

## 12. Cross-stratum bridge laws

Core, Exec, and Model are coherent only if values crossing their boundaries have explicit meaning.

### 12.1 Model to Core

A Model observation reifies into either:

- an ordinary immutable Core value/snapshot representation; or
- an explicit logical handle whose operations are defined by its contract.

Observing Model state MUST NOT implicitly create a lexical Core borrow directly into arbitrary state-domain internal storage.

A hidden revision/observation identity MAY accompany an observation even when ordinary source does not expose it.

### 12.2 Core to Model

Ordinary lexical mutation MUST NOT directly mutate arbitrary state-domain internals through a Core reference.

State-domain mutation crosses through explicit state-domain operations, transactions, rule proposals, or another defined bridge contract.

### 12.3 Core to Exec

Exec tasks receive Core values, owned resources, or permission-bearing borrows/views according to explicit ownership/resource rules.

Execution placement does not change the logical ownership contract.

A raw pointer valid in one physical realization MUST NOT be assumed valid after migration/relocation unless a mapping/pinning contract preserves that validity.

### 12.4 Exec to Core

Exec completion reifies results into Core values/resources according to the task/resource contract. A physical device result is not permitted to bypass Core validity/ownership rules merely because it was produced by a GPU or accelerator.

### 12.5 Model to Exec

A live Model query/state-domain observation is not silently captured as mutable live state by an Exec task.

Execution over Model-derived data requires an explicit bridge such as:

- immutable Core snapshot values;
- materialization;
- Buffer/resource realization;
- an explicit logical handle whose execution semantics are defined.

### 12.6 Exec to Model

Exec computation does not gain state-domain commit authority merely by holding a physical resource or by having computed candidate values.

Changes to Model state re-enter through the applicable state-domain admission/commit contract.

### 12.7 Bridge non-leakage law

An implementation MAY optimize a bridge away physically when it can prove the same semantics, but it MUST NOT expose an otherwise-forbidden borrow, address, mutation, observation, ordering, or authority merely because two strata happen to share one runtime representation.

---

## 13. Realization transparency and explainability

Runen allows implementation freedom but rejects semantic opacity at important automatic boundaries.

Automatic placement, transfer, specialization, scheduling, materialization, or incremental maintenance SHOULD be inspectable by tooling.

Inspection is not itself language behavior unless a profile explicitly makes it so, but an implementation should be able to explain why a requirement was rejected, a preference was ignored, a transfer occurred, or a specialization was/was not admitted.

No runtime cost is required merely because a semantic concept exists. Static situations MAY erase to direct calls, static layouts, fixed schedules, or direct instructions when behavior is preserved.

---

## 14. Conformance profiles

Runen conformance is profile-based.

The initial profile taxonomy is:

- **Runen Core** — base language semantics required by all conforming Runen implementations;
- **Runen Exec** — execution-visible tasks/resources and heterogeneous realization;
- **Runen Model** — logical/declarative state semantics;
- **Runen Hosted** — hosted standard-environment facilities;
- **Runen Network** — remote/distributed protocol facilities;
- **Runen Security** — additional authority/information-flow guarantees;
- **Runen Realtime** — realtime/progress admission and guarantee facilities.

Profiles may compose. A freestanding implementation may conform to Runen Core without implementing Model, GPU, network, or hosted facilities.

An implementation MUST state which profiles and profile versions it claims.

A profile claim does not permit weakening Core semantics.

Detailed profile conformance rules are in [Conformance & Assurance](conformance.md).

---

## 15. Source complexity boundary

Runen's semantic rigor is intended to support ordinary source, not dominate it.

Routine source should primarily be able to use problem-level concepts such as functions, tasks, ordinary iteration, structured parallel iteration, records/relations/queries/rules, observation, materialization, and maintenance.

If routine code requires explicit provenance annotations, effect-row plumbing, clock/revision bookkeeping, physical placement, region proofs, coherence-state manipulation, or scheduler controls merely to express ordinary application logic, the language design SHOULD be simplified rather than adding another abstraction layer.

Advanced annotations and explicit machinery belong at genuine semantic/optimization/unsafe boundaries.

---

## 16. Explicitly unresolved semantics

This document freezes the architecture above, not every language rule.

The following remain open and MUST NOT be inferred from implementation behavior or examples:

### P0-A — value, memory, and safety completion

- memory objects/allocations beyond A0 locals;
- lifetimes and deallocation;
- complete borrow/reborrow rules;
- interior mutability;
- raw pointer construction/dereference;
- provenance;
- address stability/pinning;
- validity/invariants;
- full UB taxonomy;
- complete unsafe operation list;
- safe-abstraction soundness formalization.

### P0-B — Exec resource/concurrency completion

- formal CPU/GPU memory model;
- atomics/order/scope;
- Buffer coherence/mapping/relocation state machine;
- async task cancellation and fault propagation;
- hierarchical group/subgroup contract;
- any schedule/transformation authoring surface.

### P0-C — numeric baseline

- default integer overflow behavior;
- complete `standard`, `reproducible`, and `fast` floating-point contracts;
- operation accuracy;
- NaN/subnormal behavior;
- contraction/transcendentals;
- reduction equivalence.

### P0-D — language completion

- lexical grammar;
- parser grammar;
- modules/import/name resolution;
- trait coherence;
- closure/capture details;
- const-evaluation restrictions;
- stable layout/ABI mechanisms;
- complete panic/fault semantics.

### P0-E — minimal Model algebra

- exact logical typing;
- absence/null semantics;
- Relation/Bag/Sequence completion;
- joins;
- grouping/aggregation;
- identity/keys;
- query cardinality/order inference;
- state-domain interface;
- ObservationSet compatibility;
- exact freshness representation;
- maintenance target contracts.

### P0-F — integration/conformance

- cross-stratum proving workloads;
- CPU/GPU semantic/numeric differential tests;
- from-scratch versus incremental differential tests;
- optimizer/refinement validation;
- profile composition tests.

The dependency-ordered program is maintained in [Semantic Closure](semantic-closure.md).

---

## 17. Revision discipline

Before Runen 1.0, provisional normative semantics may be revised deliberately when implementation proving falsifies an assumption.

Such revision MUST:

1. identify the affected normative rule;
2. explain why the old rule is unsound, incoherent, or unnecessarily constraining;
3. update dependent normative text;
4. update executable conformance artifacts for already-implemented subsets;
5. preserve the distinction between semantic change and implementation optimization.

A compiler's existing behavior is evidence about an implementation. It is not sufficient justification for changing the language specification by itself.
