# Program Behavior

Status: **provisional normative**

A valid Runen program denotes a set of permitted observable behaviors.

Conceptually:

```text
Behavior = outcome + observable trace
```

## Outcomes

Where applicable, Core distinguishes:

- normal return;
- defined fault;
- divergence.

Cancellation and environment failure may be defined by optional profiles.

Recoverable domain/application failure should ordinarily be represented as an ordinary value when it is part of the program contract.

## Observable behavior

Observations may include, when the applicable language/profile contract admits them:

- externally visible I/O;
- volatile or MMIO operations;
- state-domain commits;
- public logical events;
- network-visible actions;
- explicit host/environment effects.

Register allocation, temporary storage, cache policy, CPU-core choice, GPU lane numbering, query-plan shape, and physical data layout are not observable merely because tooling can inspect them.

## Refinement

A correctness-preserving lowering or transformation MUST NOT introduce an observable behavior forbidden by its source semantics.

Conceptually:

```text
Behaviors(lowered) ⊆ Behaviors(source)
```

under the applicable abstraction mapping.

This obligation applies whenever lowering, optimization, scheduling, physical layout, placement, query planning, specialization, incremental realization, or migration changes implementation form without changing source meaning.