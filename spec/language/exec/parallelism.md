# Exec Parallelism

Status: **provisional normative; incomplete**

Ordered sequential iteration preserves source-defined relative iteration order.

## Structured unordered iteration

`each`-style structured parallel iteration removes source-defined relative order among sibling iterations of one execution.

One dynamic `each` execution has opaque semantic identity only as needed to scope the required iteration identities and other semantic structures that belong to that execution. Each required iteration has opaque semantic identity scoped by its containing dynamic `each` execution. Equal implementation or debug iteration tokens used in distinct dynamic `each` executions do not thereby identify one iteration shared by those executions.

Dynamic `each` identity and iteration identity are not numeric indices, source handles, launch identifiers, worker or lane identities, queue identities, scheduler tokens, hardware topology identities, or ordering relations.

Semantic actions sequenced before entry to an `each` in its containing execution context occur before actions performed by its iterations when those preceding actions belong to the same defined continuation.

An `each` execution completes normally only after every iteration required by that execution has completed normally. Actions sequenced in the normal continuation after that completed `each` occur after the actions performed by every completed iteration of the `each`.

These entry and normal-completion relationships belong to that dynamic `each` execution. Equal implementation or debug identities in a distinct dynamic `each` execution do not create an entry, sibling, or continuation ordering relationship between the executions.

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

A hierarchy instance has opaque semantic identity only as needed to scope the cohort identities it contains, and that hierarchy identity is itself scoped by the dynamic `each` execution to which the hierarchy belongs. Equal implementation or debug hierarchy tokens used in distinct dynamic `each` executions do not thereby identify one hierarchy shared by those executions. Hierarchy identity is not a numeric index, source handle, launch coordinate, queue identity, worker identity, hardware topology identity, or scheduling order.

Group and subgroup identities are opaque cohort identities. They are not numeric indices, coordinates, addresses, queue identities, worker identities, lane identities, hardware wave identities, or scheduling order.

Group identity is scoped by its containing hierarchy instance and therefore transitively by the containing dynamic `each` execution. Equal implementation or debug group tokens used in distinct hierarchy instances do not thereby identify one group shared by those hierarchies.

Subgroup identity is scoped by its containing group and therefore transitively by the containing hierarchy instance and dynamic `each` execution. Equal implementation or debug subgroup tokens used under distinct groups or distinct hierarchy instances do not thereby identify one subgroup spanning those cohorts.

Hierarchy membership and cohort participation consume these scoped identities. An iteration identity belonging to another dynamic `each` execution cannot retarget to an iteration in this hierarchy, barrier cohort, or reduction cohort merely because an implementation or debug iteration token is equal.

Hierarchy membership by itself establishes no execution order, synchronization, memory visibility, progress guarantee, physical concurrency, temporal contiguity, or scheduling relationship. Sibling iterations remain source-unordered regardless of whether they share a group or subgroup.

### Semantic establishment and binding

A hierarchy instance is **semantically established** for a dynamic `each` execution when an applicable Runen semantic context fixes one specific hierarchy identity and its valid group/subgroup membership partition as semantic structure before a hierarchy-sensitive operation consumes that hierarchy.

Establishment is a semantic binding. It is not a physical scheduling choice, worker topology, launch geometry, backend discovery result, or permission for a realization to derive hierarchy membership from incidental execution placement.

Every hierarchy-sensitive operation that consumes a group or subgroup MUST be bound to the exact established hierarchy instance containing the selected cohort identity. A group or subgroup identity from another hierarchy instance MUST NOT retarget to an otherwise equal-looking cohort, including when both hierarchy instances belong to the same dynamic `each` execution and use equal implementation or debug cohort tokens.

Establishment itself creates no execution order, synchronization, memory visibility, progress guarantee, physical concurrency, or scheduling relationship. Those effects exist only where another applicable semantic contract supplies them.

This revision does not define whether a source construct may establish one hierarchy instance or multiple hierarchy instances for one dynamic `each` execution, and there is no implicit globally current hierarchy. If an applicable future contract permits multiple hierarchy instances for one dynamic `each`, their semantic identities remain distinct and each hierarchy-sensitive operation still binds to one exact instance.

A realization MUST preserve the established hierarchy identity and membership relation consumed by an operation; it MAY realize that semantic hierarchy using any physical topology or schedule that preserves those facts. A realization MUST NOT silently substitute a different semantic hierarchy instance. If a future source, profile, or operation contract permits hierarchy choice or variation for one semantic operation, that contract MUST explicitly define the permitted variation or behavior set; each resulting operation instance then binds to the exact hierarchy selected by that semantic contract.

This revision does not define how source code requests, constrains, observes, constructs, names, counts, or otherwise obtains hierarchy instances, and it does not define a hierarchy-specific environment admission protocol. An implementation's inability to realize required semantic hierarchy structure is not permission to substitute a different semantics-affecting partition.

This revision defines no fixed group or subgroup sizes, dimensions, coordinates, local or global indices, ordering, contiguity, uniform-size requirement, launch geometry, or hardware topology.

The normal-completion contract of the enclosing `each` continues to use the complete root cohort. The structured barrier below may explicitly select the root cohort, one established group, or one established subgroup; hierarchy membership by itself does not reinterpret normal completion or create a barrier.

## Cohort-scoped structured barrier

A **structured barrier** is a phase boundary belonging to one dynamic `each` execution and selecting exactly one participant cohort for that barrier instance.

The selected **barrier cohort** is exactly one of:

- the complete root cohort of the enclosing `each` execution;
- one group from a semantically established hierarchy instance for that `each` execution; or
- one subgroup from that semantically established hierarchy instance.

A root-cohort barrier does not require a hierarchy instance. A group- or subgroup-cohort barrier is bound to the exact semantically established hierarchy instance containing the selected group or subgroup, and the selected cohort MUST exist in that hierarchy instance. A group or subgroup identity from another hierarchy instance does not retarget to a cohort in this hierarchy merely because an implementation or debug token is equal. This revision does not define source syntax or another source-level mechanism for selecting or obtaining that hierarchy.

The participant set is exactly the selected barrier cohort and is fixed for the barrier instance. The root cohort is empty exactly when the enclosing `each` has no required iterations; such a root barrier has no participants. Group and subgroup cohorts are non-empty by the hierarchy contract above.

For one structured barrier instance:

1. every participant has one before-barrier phase and one after-barrier phase;
2. the barrier boundary completes normally only after every participant has completed its before-barrier phase normally;
3. no participant begins its after-barrier phase before the barrier boundary has completed normally;
4. actions in every participant's before-barrier phase are therefore before actions in every participant's after-barrier phase;
5. the boundary introduces no relative order among sibling participant before-barrier phases and no relative order among sibling participant after-barrier phases;
6. an iteration outside the selected cohort is a nonparticipant: it has no phase or completion obligation for this barrier and receives no execution order from this barrier.

The ordinary-access memory-ordering consequence of this completed phase boundary is owned by [Exec memory model](memory-model.md).

Selecting a group or subgroup for a barrier does not change hierarchy membership. Hierarchy membership still provides no synchronization by itself; the order above arises from the explicit structured barrier instance.

Physical arrival order, release order, worker assignment, lane identity, queue order, chunking, and rendezvous implementation are not semantic input. A realization MAY implement the boundary using any mechanism that preserves the defined participant and phase structure plus all applicable memory semantics.

Different dynamic barrier instances are distinct semantic boundaries. Barrier identity, cohort kind, or a relationship between their selected cohorts does not by itself order actions around two different barriers; any such order must follow from their placement and other applicable semantics in the enclosing execution.

This revision defines the structured phase form rather than an imperative barrier call. Source syntax, lowering, and validation for any future imperative spelling are not defined here; such a future form requires its own rules establishing the structured participation represented by this barrier boundary.

If a participant faults, is cancelled, diverges, or otherwise fails to complete its before-barrier phase normally, the barrier and enclosing `each` abnormal-completion behavior are not defined by this revision.

## Cohort-scoped identity-bearing unordered reduction

An **unordered reduction** is a structured interaction belonging to one dynamic `each` execution that combines semantic contributions from one selected participant cohort into one reduction result.

The selected **reduction cohort** is exactly one of:

- the complete root cohort of the enclosing `each` execution;
- one group from a semantically established hierarchy instance for that `each` execution; or
- one subgroup from that semantically established hierarchy instance.

A root-cohort reduction does not require a hierarchy instance. A group- or subgroup-cohort reduction is bound to the exact semantically established hierarchy instance containing the selected group or subgroup, and the selected cohort MUST exist in that hierarchy instance. A group or subgroup identity from another hierarchy instance does not retarget to a cohort in this hierarchy merely because an implementation or debug token is equal. This revision does not define source syntax or another source-level mechanism for selecting or obtaining that hierarchy.

The participant set is exactly the selected reduction cohort and is fixed for the reduction instance. The root cohort is empty exactly when the enclosing `each` has no required iterations. Group and subgroup cohorts are non-empty by the hierarchy contract above.

An iteration outside the selected reduction cohort is a nonparticipant and MUST NOT produce a semantic contribution belonging to that reduction instance.

A participant may produce zero or more semantic reduction contributions when the enclosing reduction operation permits that cardinality. Contributions are distinct occurrences even when two or more contributions carry semantically equal values. Therefore a non-empty reduction cohort may still produce no semantic contributions.

A reduction contribution belongs to the reduction interaction itself. Producing or combining a contribution is not an ordinary non-atomic read or state-changing access to a shared accumulator region. Ordinary accesses performed by an iteration outside the reduction interaction remain governed by [Exec memory model](memory-model.md). Other semantic actions used while producing a contribution remain governed by their own applicable contracts. Participating in a reduction does not legalize, reorder, synchronize, or otherwise weaken those independent obligations.

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

A normally completed reduction incorporates every semantic contribution produced by its participants exactly once. It MUST NOT omit, duplicate, or invent a semantic contribution.

The identity value defines the result when the participant cohort produces no semantic contributions and may participate as a neutral operand in the semantic combination. A realization MAY additionally initialize or combine physical partial results with the identity value any finite number of times when the two-sided identity law guarantees that doing so is semantically neutral. Such identity-valued physical initialization is realization state, is not a semantic contribution, and does not count as contribution duplication or invention.

Because sibling contributions have no source-defined relative order, a legal realization MAY choose any permutation of the contributions and any binary combination tree only where the complete combination and applicable result contracts guarantee that the resulting behavior and value are semantically equivalent.

Physical worker, lane, chunk, queue, partial-accumulator, or tree order is not additional semantic input. A realization MAY use such physical structure only as an implementation technique preserving the reduction contract.

For a normally completed `each` carrying the reduction, normal continuation is reached only after every required iteration of the complete `each` execution has completed normally and every semantic contribution produced by the reduction's participants has been incorporated. The one reduction result is then available to that normal continuation.

This revision does not define a cohort-local continuation, a result visible to participants before the enclosing `each` normal continuation, a leader or lane result, or automatic distribution of the result to the selected cohort.

An unordered reduction is not an implicit structured barrier. Selecting a root, group, or subgroup cohort for a reduction establishes no general ordering relation among sibling iterations and no atomic, fence, barrier, ordinary-access visibility, or other synchronization semantics unless a separate contract explicitly supplies such a relationship.

The reduction result and partial-result behavior when an iteration faults, is cancelled, diverges, or otherwise fails to complete normally are not defined by this revision.

Group-local storage, cohort-local reduction result distribution, group/subgroup collectives beyond the unordered reduction defined above, broadcast, shuffle, scans, subgroup-cohort and broader atomic scope semantics, and hierarchy-sensitive fence scope semantics remain not defined by this revision. Atomic interaction and scope semantics are owned by [Exec memory model](memory-model.md); the hierarchy above defines participant-domain structure where an applicable operation explicitly consumes it, while the structured barrier and unordered reduction define only their own cohort-scoped contracts.
