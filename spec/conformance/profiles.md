# Conformance Profiles

Status: **provisional normative**

A conforming implementation MUST implement the normative semantics of every profile and profile version it claims.

Extensions MUST NOT silently weaken or redefine a claimed Runen profile.

Unsupported profile facilities must be identified rather than treated as unspecified language behavior.

## Core

Runen Core is the base profile required by every conforming implementation.

A freestanding implementation may claim Core without Hosted, Exec, Model, Network, Security, or Realtime.

## Exec

Runen Exec depends on Core and adds execution-visible tasks/resources, structured parallelism, Buffer/resources, heterogeneous realization contracts, synchronization, and the Exec memory model.

Exec does not imply that a GPU is present.

## Model

Runen Model depends on Core and on any explicit bridge facilities it uses. It adds logical data/query/state-domain/rule/observation/maintenance semantics.

Model does not require one database, ECS, incremental engine, or storage architecture.

## Hosted

Runen Hosted identifies a Standard Environment contract. Hosted is a profile, not a fourth semantic stratum.

## Network

Runen Network defines the remote communication, failure, ordering, identity, serialization, observation, authority, and consistency contracts it claims.

Network MUST NOT reinterpret Core references or pointers as remote shared-memory references by default.

## Security

Runen Security defines additional authority, confidentiality/integrity, information-flow, isolation, sandbox, or related contracts.

## Realtime

Runen Realtime defines the environment assumptions, admission requirements, scheduling/progress guarantees, deadlines, and failure behavior of realtime claims.

A hard guarantee MUST be rejected when the environment cannot establish it rather than silently becoming a preference.

## Composition

Profiles compose only when their contracts are mutually satisfiable.

A profile implementation MUST preserve Core semantics.

An interaction between profiles requires an explicit normative interaction rule; the absence of one does not authorize implementation-defined semantics.

## Claims and versions

An implementation MUST state the profiles and profile versions it claims.

Language and optional profile versions may evolve independently.

The compatibility/stability policy of future stable releases is not specified by this provisional version.