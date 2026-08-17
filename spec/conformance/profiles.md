# Conformance Profiles

Status: **provisional normative**

This document owns profile taxonomy, composition, and claim rules. It does not restate the semantics owned by the language specification.

A conforming implementation MUST implement every normative rule included by each profile and profile version it claims.

Extensions MUST NOT silently weaken or redefine a claimed profile.

## Core

Runen Core is the base profile required by every conforming Runen implementation.

A Core claim includes the general language semantics, Core semantics, and bridge rules applicable to Core values and interactions.

Core is freestanding: claiming Core alone MUST NOT require an operating system, heap allocation, filesystem, networking, threads, an async runtime, a GPU runtime, a Model runtime, or hosted application frameworks.

## Exec

Runen Exec extends Core with the normative Exec semantics and applicable bridge rules.

An Exec claim does not imply that any particular accelerator is present; environment requirements are handled by admission contracts.

## Model

Runen Model extends Core with the normative Model semantics and applicable bridge rules.

A Model claim does not imply one physical storage or incremental realization.

## Additional profile families

Hosted, Network, Security, and Realtime are reserved profile families in the provisional architecture.

This revision does not define complete claimable contracts for those profile families. A standardized conformance claim requires a normative profile contract that identifies the rules it adds.

## Composition

Profiles compose only when their contracts are mutually satisfiable.

A profile implementation MUST preserve Core semantics.

An interaction between profiles requires an explicit normative interaction rule; the absence of one does not authorize implementation-defined semantics.

## Claims and versions

An implementation MUST state the profiles and profile versions it claims.

Language and optional profile versions may evolve independently.

The compatibility policy for future stable releases is not defined by this revision.