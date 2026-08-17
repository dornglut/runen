# Runen Conformance & Assurance

Status: **provisional normative conformance boundary**

This document defines what an implementation may claim and how conformance evidence relates to the normative language specification.

It does not require one implementation architecture or one test harness.

## 1. Conformance principle

A conforming implementation MUST implement the normative semantics of every profile and profile version it claims.

An implementation MAY support additional extensions. Extensions MUST NOT silently weaken or redefine the semantics of a claimed Runen profile.

An implementation MUST identify unsupported profile facilities rather than treating them as unspecified language behavior.

## 2. Initial profiles

### Runen Core

Base profile. Required by every conforming Runen implementation.

Core covers ordinary language values, control, memory/safety, functions, effects, faults, and low-level facilities as those semantics become normatively closed.

A freestanding implementation may claim Core without Hosted, Exec, Model, Network, Security, or Realtime.

### Runen Exec

Depends on Core.

Adds execution-visible tasks/resources, structured concurrency, parallel iteration/patterns, Buffer/resources, heterogeneous realization contracts, synchronization, and the Exec memory model.

Exec does not imply a GPU is available. A particular program may carry admission requirements that require one.

### Runen Model

Depends on Core and on any explicit bridge facilities it uses.

Adds logical records/facts/relations, queries, state domains, ObservationSets, rules, observation/materialization/maintenance, and incremental-equivalence obligations.

Model does not require one database, ECS, incremental engine, or storage architecture.

### Runen Hosted

Defines the hosted Standard Environment facilities required by the claimed Hosted version.

Hosted is an environment/profile contract, not a new semantic stratum.

### Runen Network

Defines remote communication/protocol facilities, serialization/identity/failure/ordering contracts, and any distributed observation/replication guarantees claimed by that profile.

Network MUST NOT reinterpret Core references or pointers as remote shared-memory references by default.

### Runen Security

Defines additional authority, confidentiality/integrity, information-flow, declassification/endorsement, isolation, or sandbox contracts.

Security properties that are hyperproperties require assurance stronger than ordinary single-trace tests where applicable.

### Runen Realtime

Defines environment assumptions, admission requirements, scheduling/progress guarantees, deadlines, and failure behavior for realtime claims.

A Realtime implementation MUST reject a hard guarantee it cannot establish rather than silently treating it as a preference.

## 3. Profile composition

Profiles compose only when their contracts are mutually satisfiable.

A profile implementation MUST preserve Core semantics.

Where two profiles introduce an interacting boundary, the specification must define the interaction explicitly. Absence of an interaction rule is an open specification gap, not permission for implementation-defined behavior.

## 4. Language version versus profile version

Runen may version the base language and optional profiles independently when that avoids forcing unrelated facilities into one compatibility schedule.

Before a stable compatibility policy is adopted, versions in this repository are provisional design identifiers rather than ecosystem stability promises.

## 5. Program validity and environment admission

Conformance distinguishes:

- **language-invalid** — violates rules of the claimed language/profile;
- **valid but not admitted** — program is valid but the target environment cannot satisfy a hard requirement;
- **admitted but realization-constrained** — only a subset of legal realizations satisfy the program/environment contract;
- **executed** — one admitted realization runs according to the normative behavior.

An implementation MUST NOT report an environment capability absence as if it proved the source language ill-typed when the distinction matters.

## 6. Reference semantics

A repository reference machine or evaluator is a conformance oracle only for the semantic subset its governing normative annex says it implements.

Reference code is subordinate to normative text. If code and accepted normative semantics disagree, the implementation is defective unless a deliberate normative revision changes the specification.

Current reference coverage:

- A0 Core value/place/init/move/copy/assignment/drop/return/fault semantics.

## 7. Conformance tests

A conformance suite SHOULD include positive and negative cases.

Positive cases prove required permitted behavior.

Negative cases prove that invalid programs, forbidden state transitions, unsafe uses, race patterns, profile-incompatible operations, or inadmissible realizations are rejected at the correct semantic boundary.

Tests MUST NOT use accidental host-language/runtime properties as the expected Runen result.

## 8. Differential assurance

Where multiple realizations implement one semantic operation, differential testing SHOULD compare them against the strongest available oracle.

Expected categories include:

- interpreter/reference versus lowering/backend;
- scalar CPU versus SIMD/multicore;
- CPU versus GPU under the applicable numeric contract;
- from-scratch Model evaluation versus incremental/materialized realization;
- generic implementation versus specialization;
- pre-optimization versus post-optimization observable behavior.

A differential mismatch is evidence of an implementation defect or a specification gap. It is not automatically resolved by choosing whichever implementation happened to run first.

## 9. Refinement assurance

Transformations that change representation, schedule, lowering, query plan, materialization strategy, or specialization require evidence appropriate to their risk.

Evidence may include:

- construction that is correct by a small local rule;
- executable equivalence tests;
- property-based/adversarial tests;
- translation validation;
- formal proof for high-risk kernels;
- manual semantic review.

No single proof technology is mandated by the language specification.

## 10. Diagnostic conformance

Exact wording of ordinary diagnostics is not currently normative.

However, an implementation SHOULD distinguish at least:

- language validation failure;
- unsafe/validity failure detected statically;
- environment admission failure;
- realization failure;
- defined runtime fault;
- profile requirement failure.

Tooling SHOULD expose enough structured context that automated and human users can tell which semantic boundary rejected a program.

## 11. Current repository validation

`cargo validate` validates the Rust repository implementation and conformance suite. It is **not** itself the Runen language conformance definition.

The repository validation gate currently checks formatting, Rust tests, Clippy, diff hygiene, and checkout-state integrity. The semantic meaning of those tests comes from the applicable normative specification/annex.

## 12. Claim discipline

Until every P0 closure gate required by a profile is complete, documentation MUST use qualified language such as:

- provisional specification;
- executable subset;
- architecture candidate;
- profile incomplete;
- open semantic issue.

The project MUST NOT claim a complete independent-implementation-ready Runen language merely because the reference machine or compiler can execute a subset.