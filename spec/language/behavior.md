# Program Behavior

Status: **provisional normative**

A valid Runen program denotes a set of permitted observable behaviors.

Conceptually:

```text
Behavior = outcome + observable trace
```

## Outcomes

Where applicable, Core distinguishes normal return, defined fault, and divergence.

Cancellation and environment failure may be defined by contracts that introduce those outcomes.

Recoverable domain or application failure should ordinarily be represented as an ordinary value when it is part of the program contract.

## Observable behavior

An observation exists only when the applicable language or profile contract makes it observable. Such observations may include externally visible I/O, volatile or MMIO operations, state-domain commits, public logical events, network-visible actions, or explicit host/environment effects.

Physical implementation state is not observable unless a normative contract explicitly exposes it.