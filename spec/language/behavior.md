# Program Behavior

Status: **provisional normative**

A valid Runen program denotes a set of permitted observable behaviors.

Conceptually:

```text
Behavior = outcome + observable trace
```

## Outcomes

Where applicable, Core distinguishes normal return, defined fault, and divergence.

Cancellation and environment failure MAY be defined by contracts that introduce those outcomes.

Recoverable domain or application failure represented as an ordinary value remains an ordinary program result; representing such failure does not by itself make it a defined Core fault.

## Observable behavior

An observation exists only when the applicable language or profile contract makes it observable. Examples include externally visible I/O, volatile or MMIO operations, state-domain commits, public logical events, network-visible actions, or explicit host/environment effects.

Physical implementation state is not observable unless a normative contract explicitly exposes it.
