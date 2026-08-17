# Model Rules

Status: **provisional normative**

A rule is a declarative or reactive state transition evaluated against admitted observations.

## Reaction wave

Conceptually:

```text
admitted triggers or events
        ↓
immutable ObservationSet
        ↓
match or query
        ↓
pure derivation
        ↓
staged proposals and events
        ↓
state-domain admission
        ↓
commit
        ↓
committed logical events become available to later waves
```

A reaction wave is a logical instant; physical execution can take nonzero wall time.

## Staging and commit

Mutation proposals are staged. They are not immediately visible to other matching or derivation in the same reaction wave unless a normative rule explicitly permits that behavior.

Before commit, rule evaluation MAY read admitted observations, perform pure calculation, create state proposals, and stage logical events.

Pre-commit rule evaluation MUST NOT perform arbitrary irreversible external effects as though the transition were already committed.

For a successful state-domain commit, the admitted state changes and logical events defined as part of that transition acquire logical existence together.

Later delivery of a committed event can have a separate failure or retry contract.

## Mutation scope

An ordinary rule can read observations from multiple state domains but MUST mutate at most one state domain per commit or reaction transition.

Cross-domain coordination requires an explicit contract.

## Conflicts

Conflicting proposals MUST NOT be resolved by incidental worker or scheduler order.

Resolution MUST be explicit: rejection, deterministic arbitration, algebraic accumulation, or a published state-domain conflict rule.

If arbitration intentionally permits nondeterminism, that nondeterminism MUST be part of the semantic contract.

## Progress

A well-typed and memory-safe rule system is not guaranteed to reach quiescence. Rule-system progress or termination guarantees require an explicit contract.
