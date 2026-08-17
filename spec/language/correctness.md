# Correctness Relations

Status: **provisional normative**

Runen does not collapse distinct correctness questions into one universal relation.

## Behavior refinement

A correctness-preserving lowering or transformation MUST NOT introduce an observable behavior forbidden by its source semantics.

Conceptually:

```text
Behaviors(lowered) ⊆ Behaviors(source)
```

under the applicable abstraction mapping.

## Progress and liveness

Safety and typing do not imply eventual completion. Fairness, deadlines, bounded response, eventual propagation, and similar progress properties require explicit assumptions and guarantees.

## Numeric equivalence

Two otherwise legal realizations can differ numerically. The applicable numeric contract determines which differences are permitted.

## Incremental equivalence

Incremental equivalence is distinct from ordinary behavior refinement. Model maintenance semantics define the concrete relation between an observed maintained result and evaluation of its defining logical computation.

## Security properties

Confidentiality and integrity properties can relate multiple executions or traces. A contract that defines such hyperproperties MUST state obligations not reducible to single-trace behavior refinement.

## Determinism

**Semantic determinism** means that the same explicit inputs and admitted external observations yield observationally equivalent results across permitted executions.

**Schedule independence** means that changing the legal physical execution schedule does not change the observable result.

**Heterogeneous reproducibility** means that distinct admitted realizations satisfy the relation required by the applicable numeric contract.

These are distinct properties. None implies the others without an explicit rule.

## Intentional nondeterminism

Source MAY explicitly admit multiple results through operations whose contracts expose nondeterminism.

Incidental implementation order MUST NOT create nondeterminism when source semantics do not admit it.
