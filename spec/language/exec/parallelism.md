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

Safe inter-iteration interaction requires an explicit legal interaction model. Ordinary non-atomic inter-iteration access is governed by the conflict and unordered-access rules in [Exec memory model](memory-model.md). The identity-bearing unordered reduction defined below is a separate structured interaction model. Atomics, commutative accumulation, and collectives require their own defined contracts; listing them as interaction categories does not itself authorize behavior that those contracts have not yet defined.

The structured entry/completion boundary does not legalize a conflicting ordinary pair of sibling-iteration accesses. Conversely, disjoint or otherwise legally interacting sibling work need not be physically serialized merely because normal continuation waits for the structured operation to complete.

Buffer logical coherence consumes the semantic ordering relationships established here according to [Exec Buffers](resources/buffers.md); this document does not redefine Buffer visibility or coherence.

Unordered physical scheduling does not by itself imply semantic nondeterminism.

## Identity-bearing unordered reduction

An **unordered reduction** is a structured interaction that combines semantic contributions produced by source-unordered participating iterations into one reduction result.

A participating iteration may produce zero or more semantic reduction contributions when the enclosing reduction operation permits that cardinality. Contributions are distinct occurrences even when two or more contributions carry semantically equal values.

A reduction contribution belongs to the reduction interaction itself. Producing or combining a contribution is not an ordinary non-atomic read or state-changing access to a shared accumulator region. Ordinary accesses performed by an iteration outside the reduction interaction remain governed by [Exec memory model](memory-model.md), and participating in a reduction does not legalize an otherwise-conflicting ordinary access.

The reduction defined by this revision has an explicit semantic identity value `e`. The identity defines the result of an empty contribution collection and the neutral element used by the operator contract below.

### Operator contract

The reduction combination operator MUST guarantee all of the following under the semantic equivalence relation applicable to the reduction result:

- **two-sided identity:** `combine(e, x)` and `combine(x, e)` are equivalent to `x`;
- **associativity:** `combine(combine(a, b), c)` is equivalent to `combine(a, combine(b, c))`;
- **commutativity:** `combine(a, b)` is equivalent to `combine(b, a)`.

These laws are semantic operator-contract obligations. An implementation MUST NOT infer them merely from operator spelling, host-language traits, backend instructions, or observed test values.

Where an applicable numeric contract defines the result equivalence or transformation freedom, that numeric contract remains authoritative. This Exec rule does not define integer overflow, floating-point reassociation, NaN behavior, contraction, approximation, or another numeric policy.

This revision defines only unordered reductions whose operator contract guarantees all three obligations above. Ordered reductions, intentionally nondeterministic reductions, reductions without an explicit identity, and weaker-law reduction forms are not defined by this revision.

### Contributions and result

A normally completed reduction incorporates every semantic contribution produced by its participating iterations exactly once. It MUST NOT omit, duplicate, or invent a semantic contribution.

The identity value defines the empty result and may participate as a neutral operand in the semantic combination. A realization MAY additionally initialize or combine physical partial results with the identity value any finite number of times when the two-sided identity law guarantees that doing so is semantically neutral. Such identity-valued physical initialization is realization state, is not a semantic contribution, and does not count as contribution duplication or invention.

Because sibling contributions have no source-defined relative order, a legal realization MAY choose any permutation of the contributions and any binary combination tree only where the operator and applicable result contract guarantee that the resulting value is semantically equivalent.

Physical worker, lane, chunk, queue, partial-accumulator, or tree order is not additional semantic input. A realization MAY use such physical structure only as an implementation technique preserving the reduction contract.

For a normally completed `each` carrying the reduction, normal continuation is reached only after every required iteration has completed normally and every contribution produced by those iterations has been incorporated. The reduction result is then available to the normal continuation.

This reduction interaction establishes no general ordering relation among sibling iterations and no atomic, fence, barrier, or other synchronization semantics for ordinary accesses.

The reduction result and partial-result behavior when an iteration faults, is cancelled, diverges, or otherwise fails to complete normally are not defined by this revision.

Hierarchical execution concepts such as groups, subgroups, group-local storage, barriers, broadcast, and shuffle belong to Exec. Their precise portable semantics are not defined by this revision.
