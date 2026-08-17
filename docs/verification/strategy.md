# Verification Strategy

Status: **non-normative assurance guidance**

This document owns methods for gaining confidence that implementations satisfy normative Runen semantics. It does not define conformance profiles or repository CI.

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