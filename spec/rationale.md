# Runen Rationale

Status: **non-normative**

This document records why the current semantic architecture exists. It helps reviewers understand the design, but it does not define Runen behavior.

External research is evidence and pressure testing only. Normative Runen semantics live in `language.md` and accepted annexes.

## 1. Why Core · Exec · Model

Repeated design review kept converging on three distinct semantic responsibilities:

- ordinary values/memory and low-level control;
- execution-visible heterogeneous work/resources;
- logical state/query/reaction semantics.

Attempts to collapse these concerns tended to make one domain's implementation mechanism leak into another domain's source model.

The three-stratum design is therefore not a claim that an implementation needs three runtimes. It is a discipline for assigning semantic ownership.

## 2. Why explicit bridge laws

A language can have individually coherent subsystems and still be incoherent at their boundaries.

The key questions are semantic:

- does a Model observation become a Core value, borrow, or logical handle?
- can a task capture a live query?
- does a Buffer address remain valid after migration?
- does physical ownership grant Model commit authority?

Leaving these questions to runtime convenience would make independent implementations disagree in precisely the cases Runen is intended to unify. The bridge laws therefore belong in the language specification.

## 3. Why Model observations do not expose ordinary borrows

A Model state domain may be backed by an ECS, relational store, replicated service, incremental engine, or another representation.

If ordinary observation implicitly produced `&T`/`&mut T` into state-domain storage, Core lexical lifetime/alias semantics would become coupled to one physical storage realization.

The architecture instead reifies immutable Core values/snapshots or explicit logical handles. Mutation re-enters through a state-domain commit/proposal API.

## 4. Why `state domain`, not `owner` or `state authority`

`owner` was overloaded with memory ownership, runtime ownership, and Model state responsibility.

`state authority` then collided with the separate security meaning of authority/capability.

`state domain` is narrower: it is the unit responsible for a coherent set of logical state, invariants, revisions, admission, and commit.

Security authority remains a distinct concept.

## 5. Why no universal implementation mechanism

The design explicitly rejects making any of these universal language mechanisms:

- MVCC;
- CRDTs;
- ECS storage;
- render graphs;
- Virtual DOMs;
- transactional memory;
- self-adjusting dependency graphs;
- differential dataflow;
- one graph algebra;
- one field/spatial algebra;
- one scheduler;
- one physical query plan.

These can be excellent realizations of more general contracts, but freezing them as universal semantics would make Runen narrower and more complex at the same time.

## 6. Why Graph and Field are not fundamental Model value families

Graph/path and sampled/spatial field semantics each bring substantial specialized algebra.

The base Model contract does not need those algebras to define records, facts, relations, queries, rules, observations, or maintenance.

They therefore belong in later standard-environment/domain modules unless proving shows a genuinely universal missing semantic primitive.

## 7. Why there is no universal `index` language type

Indexing needs differ across native arrays, collections, tensor dimensions, logical relations, GPU execution domains, and external ABIs.

A universal fundamental `index` type would conflate logical dimensions with persistent/native representation.

The current architecture leaves indexing operations/types to the relevant Core/container/Exec/Model contracts rather than creating one universal semantic integer family member.

## 8. Why determinism is separated from schedule independence

An unordered physical schedule does not necessarily imply multiple observable results.

For example, parallel work over disjoint elements can be physically unordered while producing the same result under every legal schedule.

Conversely, a deterministic execution on one device does not automatically imply numerically reproducible results across heterogeneous realizations.

Keeping deterministic behavior, schedule independence, and heterogeneous reproducibility separate gives the compiler stronger freedom without weakening source meaning.

## 9. Why clock, revision, and causal frontier are distinct

Earlier designs risked treating every notion of ordering/version as a clock.

That creates false equivalences:

- a simulation tick is not a database/state revision;
- a state revision is not necessarily wall time;
- a causal frontier is not necessarily a revision;
- network arrival order is not automatically logical time.

The current vocabulary keeps them separate and requires explicit bridges when a system relates them.

## 10. Why ObservationSet is fundamental

Rules/queries reading multiple state domains need a precise answer to "which observations did this evaluation use?"

`ObservationSet` captures the immutable admitted observations for a reaction/evaluation wave without pretending the whole distributed system has one global snapshot.

This is strong enough to define repeatable logical evaluation and weak enough to admit different state-domain/storage/distributed implementations.

## 11. Why Buffer is logical

Exec needs one source-level resource identity even when realization may stage or move data between memory spaces.

Treating `Buffer<T>` as permanently identical to one address/allocation would prevent transparent legal movement; treating addresses as freely stable across movement would be unsafe.

The solution is to separate logical Buffer identity/coherence from physical allocations. Raw physical addresses require explicit mapping/pinning constraints.

## 12. Why staged rule effects commit with logical events

If a rule's state mutation and its logical event can become visible independently, observers can see an event describing state that is not committed, or committed state whose defining event is missing.

For events defined as part of a state-domain commit, the language therefore couples their commit visibility.

This does not require one global/distributed transaction protocol.

## 13. Why `maintain` needs a target contract

"Keep these things synchronized" hides several different questions:

- which source revision/observation does the target represent?
- how fresh must it be?
- is progress best-effort or guaranteed under assumptions?
- what happens on failure?
- when is the target update committed/visible?

Without those answers, `maintain` would promise more than any portable runtime can guarantee.

## 14. Research pressure used by the design

The active research spine has been used for specific pressure, not copied wholesale:

- **CompCert** — observable behavior and refinement discipline;
- **Iris / RustBelt** — unsafe abstraction and resource-soundness pressure;
- **CHERI** — evidence that address, provenance, bounds, permission, and pointer authority are separable concepts;
- **Koka** — effect inference pressure;
- **Deterministic Parallel Java (DPJ)** — region/effect noninterference and deterministic parallelism pressure;
- **Regent / Legion** — logical resources versus physical instances and privileges/coherence;
- **WGSL / SPIR-V** — real heterogeneous numeric and memory/scope constraints;
- **Lustre / Vélus** — logical-clock semantics and verified lowering;
- **self-adjusting computation** — from-scratch/incremental equivalence;
- **Jif** — distinction between access authority and information flow;
- **TLA+** — assurance pressure for state/progress/fairness protocols;
- **Alive2** — translation-validation precedent;
- **WebAssembly Core Specification** — specification structure separating validation, execution, and embedding concerns.

No item above is normative Runen authority.

## 15. Why source complexity is a kill gate

Runen's differentiated value is intended to be:

```text
safe systems Core
+
deterministic heterogeneous Exec
+
logical declarative Model
```

That synthesis fails if ordinary source must routinely manipulate the machinery introduced to prove or realize it.

Provenance, effects, regions, placement, clocks, revisions, coherence, numeric modes, and scheduling should surface when they represent real semantic choices, not merely because the compiler internally reasons about them.

If that boundary cannot be maintained, simplification is preferred over additional abstraction layers.