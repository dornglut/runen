# Correctness Relations

Status: **provisional normative**

Runen does not collapse distinct correctness questions into one universal relation.

## Behavior refinement

A realization or transformation must not add forbidden observable behaviors. See [Program Behavior](behavior.md).

## Progress and liveness

Safety and typing do not imply eventual completion. Fairness, deadlines, bounded response, eventual propagation, and similar progress properties require explicit assumptions and guarantees.

## Numeric equivalence

Two otherwise legal realizations may differ numerically. The applicable numeric contract determines which differences are permitted.

## Incremental equivalence

At an observation point admitted by a freshness contract, a materialized or maintained result MUST be observationally equivalent to evaluating its defining logical computation from scratch over the corresponding admitted source observation.

## Security properties

Confidentiality and integrity properties may relate multiple executions or traces. Profiles that define such hyperproperties must state obligations that are not reducible to single-trace behavior refinement.

## Determinism

**Semantic determinism** means that the same explicit inputs and admitted external observations yield observationally equivalent results across permitted executions.

**Schedule independence** means that changing the legal physical execution schedule does not change the observable result.

**Heterogeneous reproducibility** means that distinct admitted realizations, such as CPU and GPU, satisfy the relation required by the applicable numeric contract.

These are distinct properties. None implies the others without an explicit rule.

## Intentional nondeterminism

Source may explicitly admit multiple results through operations whose contracts expose nondeterminism, such as random observation, external arrival order, or explicit arbitrary selection.

Incidental implementation order MUST NOT create nondeterminism when source semantics do not admit it.