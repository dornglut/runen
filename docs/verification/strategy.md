# Verification Strategy

Status: **non-normative assurance guidance**

This document owns methods for gaining confidence that normative Runen revisions are internally consistent and that implementations satisfy normative Runen semantics. It does not define Runen semantics, conformance profiles, or repository CI.

## Normative specification change assurance

A normative specification change requires assurance appropriate to its semantic risk and impact. Use the lightest review and verification evidence that adequately establishes the following obligations.

### Acceptance completeness

The exact-head semantic review must account for every acceptance criterion in the owning issue.

When a criterion requires executable, oracle, corpus, differential, model-checking, proof, or other verification evidence, the review must identify the exact evidence that establishes the criterion. The existence of an implementation capable of exercising a behavior is not by itself evidence that the required semantic property has been verified.

### Semantic composition

Review the proposed semantic delta against the current accepted normative rules needed to determine its meaning.

The review must establish the resulting semantics from current normative authority rather than from the issue description, pull-request description, implementation structure, host-language behavior, or test expectations.

When separately owned normative rules compose to determine an outcome, review their combined consequence rather than treating each rule as isolated merely because it has a separate canonical owner.

### Semantic impact reconciliation

When a change defines, narrows, extends, transfers, or closes a semantic concern, inspect current normative statements whose truth or ownership is directly affected by that change.

Before acceptance, reconcile any directly affected:

- ownership or responsibility boundary;
- relationship between separately owned semantic concepts;
- statement that a concern is not defined by the current revision; or
- verification obligation required by the changed semantic contract.

Do not broaden review into an unrestricted whole-specification audit merely because other documents mention the same terminology. Broaden further when the bounded review exposes a concrete dependency, ownership conflict, cross-stratum consequence, or phase-boundary question.

Semantic assurance is not established while a blocking contradiction, stale ownership statement, or stale open-item boundary that would make acceptance untruthful remains unresolved.

### Risk-scaled assurance

Review and verification depth should scale with the semantic risk and composition radius of the change.

Examples include:

- editorial or navigation-only changes requiring ordinary exact-head review;
- isolated semantic clarification requiring review of its canonical owner and the accepted rules needed to interpret it;
- a new semantic relation requiring composition and affected-boundary reconciliation;
- memory-model, ownership, safety, numeric, or other high-risk semantic work benefiting from adversarial or executable evidence where available; and
- phase or milestone closure requiring broader completeness review against the closure contract.

No fixed file count, document count, pull-request rate, or proof technology substitutes for review proportional to the actual semantic risk.

### Exact-head review evidence

The semantic assurance verdict applies to one exact feature head.

Exact-head semantic assurance evidence should identify the reviewed revision and enough information to establish:

- the owning issue;
- acceptance-criterion completeness;
- required verification evidence;
- material semantic findings and their disposition; and
- the final semantic-consistency verdict.

The verdict establishes assurance only for the reviewed revision; it does not establish assurance for a materially changed head.

The review evidence is assurance evidence, not normative authority. Do not copy exact-head review state into a second durable ledger or generated specification database.

## Evidence classes

Use the lightest evidence that adequately addresses the risk:

- focused positive/negative conformance examples;
- property-based and adversarial testing;
- executable reference semantics;
- differential execution between independent realizations;
- translation validation;
- model checking for protocol/state-machine properties;
- theorem proving for narrow high-risk kernels;
- manual semantic review.

No single proof technology is mandatory for all Runen implementations.

## Negative evidence

Tests should verify rejection at the correct boundary for invalid state transitions, unsafe operations, races, inadmissible realizations, and profile violations.

Test expectations must not derive from accidental host-language ownership, destruction, pointer-address, allocation, hash-order, or scheduler behavior.

## Differential assurance

When several realizations implement one semantic operation, compare them against the strongest available oracle. Important pairs include reference versus lowered execution, scalar versus parallel CPU, CPU versus GPU under a numeric contract, from-scratch versus incremental Model evaluation, and pre- versus post-optimization execution.

A mismatch is evidence of an implementation defect or specification gap; running first does not make one side authoritative.
