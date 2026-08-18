# Exec Parallelism

Status: **provisional normative; incomplete**

Ordered sequential iteration preserves source-defined relative iteration order.

## Structured unordered iteration

`each`-style structured parallel iteration removes source-defined relative order among sibling iterations of one execution.

Semantic actions sequenced before entry to an `each` in its containing execution context occur before actions performed by its iterations when those preceding actions belong to the same defined continuation.

An `each` execution completes normally only after every iteration required by that execution has completed normally. Actions sequenced in the normal continuation after that completed `each` occur after the actions performed by every completed iteration of the `each`.

These entry and normal-completion relationships do not impose a relative order among sibling iterations. In particular, a backend that happens to execute sibling iterations serially does not thereby create source-defined inter-iteration order.

A legal realization MAY execute sibling iterations serially, concurrently, vectorized, in chunks, on an accelerator, or using another permitted physical schedule, but it MUST preserve the same entry, sibling-unordered, and normal-completion relationships.

This revision defines only normal structured completion. The behavior of an `each` execution when an iteration faults, is cancelled, diverges, or otherwise does not complete normally is not defined by this revision.

## Inter-iteration interaction

Safe inter-iteration interaction requires an explicit legal interaction model. Ordinary non-atomic inter-iteration access is governed by the conflict and unordered-access rules in [Exec memory model](memory-model.md). The structured barrier and identity-bearing unordered reduction defined below are separate structured interaction models. Atomics, commutative accumulation, and collectives require their own defined contracts; listing them as interaction categories does not itself authorize behavior that those contracts have not yet defined.

The structured entry/completion boundary does not legalize a conflicting ordinary pair of sibling-iteration accesses. Conversely, disjoint or otherwise legally interacting sibling work need not be physically serialized merely because normal continuation waits for the structured operation to complete.

Buffer logical coherence consumes the semantic ordering relationships established here according to [Exec Buffers](resources/buffers.md); this document does not redefine Buffer visibility or coherence.

Unordered physical scheduling does not by itself imply semantic nondeterminism.

## Group and subgroup hierarchy

A **hierarchy instance** belongs to one dynamic `each` execution and supplies nested semantic participant cohorts for hierarchy-sensitive Exec operations.

The complete required iteration set of the `each` is the root cohort.

- If that required iteration set is empty, the hierarchy has no groups or subgroups.
- Otherwise, the required iterations are partitioned into one or more non-empty **groups**.
- Within every non-empty group, that group's iterations are partitioned into one or more non-empty **subgroups**.

Therefore every required iteration belongs to exactly one group and exactly one subgroup within that group. Distinct groups are disjoint and exhaust the root cohort. Distinct subgroups within one group are disjoint and exhaust that group. A subgroup cannot contain iterations from two groups.

No empty group or subgroup is introduced merely as hierarchy metadata. This hierarchy contract does not otherwise constrain the cardinality of an `each` execution.

### Semantic identity and stability

Group and subgroup membership is semantic structure once a hierarchy instance has been established for an execution. Membership is stable for the duration of that hierarchy instance and MUST NOT silently change merely because a realization changes physical scheduling or placement.

Group and subgroup identities are opaque cohort identities. They are not numeric indices, coordinates, addresses, queue identities, worker identities, lane identities, hardware wave identities, or scheduling order.

Subgroup identity is scoped by its containing group. An implementation or verification representation that uses equal subgroup tokens under distinct groups does not thereby identify one subgroup spanning those groups.

Hierarchy membership by itself establishes no execution order, synchronization, memory visibility, progress guarantee, physical concurrency, temporal contiguity, or scheduling relationship. Sibling iterations remain source-unordered regardless of whether they share a group or subgroup.

### Establishment boundary

This revision does not define how source code requests, constrains, observes, or obtains a hierarchy instance.

A future hierarchy-sensitive operation MUST define how its relevant hierarchy is established or admitted and what hierarchy variation, if any, its contract permits. A hierarchy choice that can affect program semantics MUST NOT be treated as an arbitrary hidden scheduling choice merely because several physical realizations are available.

This revision defines no fixed group or subgroup sizes, dimensions, coordinates, local or global indices, ordering, contiguity, uniform-size requirement, launch geometry, or hardware topology.

The existing full-`each` normal-completion and structured-barrier contracts continue to use the complete root cohort and are not reinterpreted as group- or subgroup-scoped operations by the existence of a hierarchy instance.

## Full-`each` structured barrier

A **structured barrier** in this revision is a phase boundary belonging to one dynamic `each` execution. It partitions every iteration required by that execution into one before-barrier phase and one after-barrier phase for that barrier instance.

The required participants of the barrier are exactly the iterations required by the enclosing `each` execution. This first barrier form does not define a subgroup, partial cohort, workgroup, lane set, or independently selected participant set.

For one structured barrier instance:

1. every required iteration has one before-barrier phase and one after-barrier phase;
2. the barrier boundary completes normally only after every required iteration has completed its before-barrier phase normally;
3. no required iteration begins its after-barrier phase before the barrier boundary has completed normally;
4. the boundary introduces no relative order among sibling before-barrier phases and no relative order among sibling after-barrier phases.

The memory-ordering consequence of this completed phase boundary is owned by [Exec memory model](memory-model.md).

Physical arrival order, release order, worker assignment, lane identity, queue order, chunking, and rendezvous implementation are not semantic input. A realization MAY implement the boundary using any mechanism that preserves the defined phase structure and applicable memory semantics.

Different dynamic barrier instances are distinct semantic boundaries. Barrier identity alone does not order actions around two different barriers; any such order must follow from their placement and other applicable semantics in the enclosing execution.

This revision defines the structured phase form rather than an imperative barrier call. Source syntax, lowering, and validation for any future imperative spelling are not defined here; such a future form requires its own rules establishing the structured participation represented by this barrier boundary.

If a required iteration faults, is cancelled, diverges, or otherwise fails to complete its before-barrier phase normally, the barrier and enclosing `each` abnormal-completion behavior are not defined by this revision.

## Identity-bearing unordered reduction

An **unordered reduction** is a structured interaction that combines semantic contributions produced by source-unordered participating iterations into one reduction result.

A participating iteration may produce zero or more semantic reduction contributions when the enclosing reduction operation permits that cardinality. Contributions are distinct occurrences even when two or more contributions carry semantically equal values.

A reduction contribution belongs to the reduction interaction itself. Producing or combining a contribution is not an ordinary non-atomic read or state-changing access to a shared accumulator region. Ordinary accesses performed by an iteration outside the reduction interaction remain governed by [Exec memory model](memory-model.md). Other semantic actions used while producing a contribution remain governed by their own applicable contracts. Participating in a reduction does not legalize, reorder, or otherwise weaken those independent obligations.

The reduction defined by this revision has an explicit semantic identity value `e`. The identity defines the result of an empty contribution collection and the neutral element used by the operator contract below.

### Combination contract

The identity and every semantic contribution are values admitted by the reduction's combination contract.

For every pair of values that can arise by recursively combining the identity and admitted contributions, `combine` MUST complete normally and yield another value admitted by the same reduction. Combination MUST have no Runen-observable behavior other than producing that result value.

These requirements make permitted regrouping and permutation a result-combination question rather than a choice that can change defined outcome or observable trace through the combination operation itself.

The combination operator MUST additionally guarantee all of the following under the semantic equivalence relation applicable to the reduction result:

- **two-sided identity:** `combine(e, x)` and `combine(x, e)` are equivalent to `x`;
- **associativity:** `combine(combine(a, b), c)` is equivalent to `combine(a, combine(b, c))`;
- **commutativity:** `combine(a, b)` is equivalent to `combine(b, a)`.

These are semantic operator-contract obligations. An implementation MUST NOT infer them merely from operator spelling, host-language traits, backend instructions, or observed test values.

Where an applicable numeric contract defines the result equivalence or transformation freedom, that numeric contract remains authoritative. This Exec rule does not define integer overflow, floating-point reassociation, NaN behavior, contraction, approximation, or another numeric policy.

This revision defines only unordered reductions whose combination contract guarantees normal closed result-only combination plus all three algebraic obligations above. Ordered reductions, intentionally nondeterministic reductions, reductions without an explicit identity, effectful or partial combination, and weaker-law reduction forms are not defined by this revision.

### Contributions and result

A normally completed reduction incorporates every semantic contribution produced by its participating iterations exactly once. It MUST NOT omit, duplicate, or invent a semantic contribution.

The identity value defines the empty result and may participate as a neutral operand in the semantic combination. A realization MAY additionally initialize or combine physical partial results with the identity value any finite number of times when the two-sided identity law guarantees that doing so is semantically neutral. Such identity-valued physical initialization is realization state, is not a semantic contribution, and does not count as contribution duplication or invention.

Because sibling contributions have no source-defined relative order, a legal realization MAY choose any permutation of the contributions and any binary combination tree only where the complete combination and applicable result contracts guarantee that the resulting behavior and value are semantically equivalent.

Physical worker, lane, chunk, queue, partial-accumulator, or tree order is not additional semantic input. A realization MAY use such physical structure only as an implementation technique preserving the reduction contract.

For a normally completed `each` carrying the reduction, normal continuation is reached only after every required iteration has completed normally and every contribution produced by those iterations has been incorporated. The reduction result is then available to the normal continuation.

An unordered reduction is not an implicit structured barrier. It establishes no general ordering relation among sibling iterations and no atomic, fence, barrier, or other synchronization semantics for ordinary accesses unless a separate contract explicitly supplies such a relationship.

The reduction result and partial-result behavior when an iteration faults, is cancelled, diverges, or otherwise fails to complete normally are not defined by this revision.

Group-local storage, group/subgroup barriers, group/subgroup reductions or collectives, broadcast, shuffle, scans, atomics, and atomic/fence scopes remain not defined by this revision. The hierarchy above defines only their shared participant-domain foundation.
