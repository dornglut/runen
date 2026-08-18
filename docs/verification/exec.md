# Exec Verification Contract

Status: **non-normative conformance-obligation documentation**

This document records focused assurance obligations for defined Exec semantic slices. It does not define Runen semantics, conformance profiles, compiler architecture, or repository CI.

The normative ordinary-access and structured-iteration rules exercised here are owned by `spec/language/exec/memory-model.md` and `spec/language/exec/parallelism.md`. Structured task lifetime and detachment are owned by `spec/language/exec/tasks.md`. Buffer-specific identity, region, view-access, and logical-coherence facts are owned by `spec/language/exec/resources/buffers.md`. Core storage, overlap, borrowing, and interior-mutability facts remain owned by their Core specifications.

`crates/runen-exec-oracle` is the verification-only executable conformance model for the currently represented Exec relations below. It is not source or compiler Exec IR, a runtime, a scheduler, a backend, or a normative owner. Future compiler/runtime realizations must remain independently accountable to the normative specification rather than treating the oracle representation as language semantics.

## Ordinary unordered-access boundary

In the cases below, region identity and overlap come from the canonical owner of the accessed storage or resource. `unordered` means that the language semantics provide no relative order between the two pieces of work; incidental backend execution order is ignored. A case described as non-conflicting passes only the ordinary Exec conflict relation; every other applicable semantic contract still has to permit the accesses.

Required cases:

- unordered `read(A)` with `read(A)` is non-conflicting under the ordinary-access relation;
- unordered state-changing access to `A` with state-changing access to disjoint `B` is non-conflicting under the ordinary-access relation;
- unordered `read(A)` with a state-changing access to overlapping `A` is not a legal ordinary safe interaction when no separately defined mechanism applies;
- unordered state-changing access to `A` with another state-changing access to overlapping `A` is not a legal ordinary safe interaction when no separately defined mechanism applies;
- semantically ordered sequential accesses to the same region are not rejected merely because the corresponding unordered pair would conflict;
- a backend that serializes source-unordered conflicting accesses does not make the interaction legal;
- physically parallel execution of a non-conflicting pair does not by itself make that pair conflicting;
- Core interior mutability under shared aliases does not by itself legalize overlapping state-changing accesses from source-unordered Exec work;
- for Core-origin structural storage, root/descendant overlap and sibling-field disjointness are consumed as Core-owned facts rather than redefined by Exec.

These obligations do not authorize atomics, commutative accumulation, collectives, or additional synchronization mechanisms whose normative contracts remain open. The structured barrier and identity-bearing unordered reduction are covered separately below.

## Buffer logical-region boundary

Buffer cases consume the Buffer owner's logical identity and overlap relation and the memory-model owner's ordinary conflict relation. They must not infer semantics from physical storage arrangement.

Required cases:

- regions belonging to distinct Buffer identities are disjoint under the Buffer region relation even if a realization can co-locate, reuse, or otherwise relate their physical backing;
- two regions of one Buffer with disjoint logical element-position sets are disjoint;
- two regions of one Buffer whose logical element-position sets intersect overlap;
- overlapping `View` read/read access is non-conflicting under the ordinary-access relation when every other applicable contract permits both accesses;
- source-unordered overlapping `View` read with a state-changing `ViewMut` access conflicts when no separately defined interaction mechanism applies;
- source-unordered overlapping state-changing `ViewMut` accesses conflict when no separately defined interaction mechanism applies;
- source-unordered state-changing accesses through `ViewMut` to disjoint Buffer regions are non-conflicting under the ordinary-access relation;
- migration, replication, or replacement of physical backing does not change the logical Buffer identity or Buffer region selected by a continuing logical view;
- physical address, allocation identity, host slice aliasing, host borrow checking, and host scheduler behavior are not semantic oracles for Buffer identity, overlap, view permission, or conflict.

These obligations do not define view construction/lifetime rules, mapping, raw-address exposure, version representation, synchronization, or a physical coherence protocol.

## Buffer ordered-coherence boundary

These cases exercise only the Buffer visibility consequence after some applicable semantic owner has already established an order between accesses. They do not infer semantic order from physical execution or backend submission.

Required cases:

- when a state-changing access to region `R` is semantically ordered before a permitted read of overlapping `R`, that read cannot observe the pre-change logical state solely because a stale physical replica exists;
- when several state-changing accesses to the same logical element positions are semantically ordered before a later permitted read, the read observes the logical state after those changes in their semantic order rather than an older physical copy;
- migration, replication, caching, or replacement of backing between an ordered state change and read does not change the logical result required by the established order;
- state changes and coherence work for region `A` do not require a disjoint region `B` to migrate, synchronize, or serialize unless another applicable contract requires it;
- Buffer coherence does not legalize source-unordered conflicting `View`/`ViewMut` or `ViewMut`/`ViewMut` ordinary accesses;
- selecting one physical replica rather than another is not program-observable when both realize the same required logical Buffer state;
- replica identity, implementation version metadata, physical address, allocation identity, backend queue order, host cache behavior, and transfer implementation are not semantic oracles for ordered Buffer visibility.

These obligations do not define version counters, replica ownership, transfer completion, future atomic/synchronization visibility, mapping, or a coherence implementation algorithm.

## Structured `each` normal-completion boundary

These cases exercise only normal structured entry/completion. They do not define abnormal iteration completion or infer sibling order from a physical schedule.

Required cases:

- a permitted state change sequenced before entry to `each` is semantically before a later permitted overlapping read performed by an iteration when both belong to the same defined continuation;
- sibling iterations remain source-unordered even when one legal realization executes them sequentially;
- source-unordered overlapping ordinary sibling read/write or write/write access remains conflicting under the accepted memory-model rule;
- source-unordered state-changing accesses to disjoint Buffer regions are non-conflicting under that rule and may execute physically concurrently;
- a normally completed `each` has no normal continuation until every required iteration has completed normally;
- after normal completion, a permitted continuation read of a Buffer region changed by an iteration receives logical state after that semantically ordered iteration change;
- after normal completion, the continuation may consume the combined effects of several disjoint sibling state changes without imposing a relative order among those sibling iterations;
- the normal join boundary makes no claim about an iteration that faults, is cancelled, diverges, or otherwise fails to complete normally;
- backend queue order, worker order, host thread timing, lane order, chunk order, and physical serialization are not semantic oracles for sibling iteration order.

These obligations do not define iteration construction, task semantics, fault aggregation, cancellation, early exit, atomics, or collectives. The hierarchy and full-`each` structured barrier contracts are covered separately below.

## Group and subgroup hierarchy boundary

These cases exercise only the nested logical participant hierarchy owned by `spec/language/exec/parallelism.md`. They do not make hierarchy membership a synchronization, scheduling, or hardware-topology mechanism.

Required cases:

- an empty required `each` iteration set admits exactly an empty hierarchy membership set and does not fabricate empty groups or subgroups;
- for a non-empty `each`, every required iteration has exactly one hierarchy membership and no invented iteration receives one;
- duplicate required fixture iteration identities, duplicate membership for one iteration, missing required iterations, and invented iterations are rejected;
- groups form a disjoint and exhaustive partition of the required `each` iteration set;
- subgroups within one group form a disjoint and exhaustive partition of that group;
- subgroup identity is scoped by the containing group, so equal private subgroup tokens under distinct groups denote distinct subgroup identities;
- same-subgroup membership implies same-group membership;
- reordering the private finite membership storage does not change membership, same-group, or same-subgroup results;
- group and subgroup identities expose equality only and do not become numeric indices, coordinates, lanes, hardware cohorts, or scheduling order;
- hierarchy membership supplies no sibling iteration order and does not legalize an otherwise-conflicting ordinary sibling access;
- the existing full-`each` structured barrier remains rooted at the complete required iteration set and is unaffected by hierarchy membership alone.

`GroupId`, group-scoped `SubgroupId`, `HierarchyMembership`, and the finite hierarchy fixture are verification representation only. They do not define source hierarchy syntax, hierarchy selection/admission, group or subgroup sizes, dimensions, coordinates, enumeration order, launch geometry, runtime worker topology, or hardware subgroup identity.

These obligations do not define group/subgroup barriers, partial-cohort synchronization, atomic or fence scope, collectives, group-local storage, broadcast, shuffle, scans, or any other hierarchy-sensitive operation. Such operations require their own normative contracts before executable evidence is extended to cover them.

## Full-`each` structured barrier boundary

These cases exercise the structured phase boundary owned by `spec/language/exec/parallelism.md` and the ordinary-access synchronization relation owned by `spec/language/exec/memory-model.md`. The barrier is modeled as a full-`each` phase cut rather than an imperative lane-level operation.

Required cases:

- the required participants of one barrier fixture are exactly the iterations required by the enclosing `each` fixture;
- normal barrier completion requires exact before-phase completion by every required iteration, independent of participant completion-list order;
- missing, duplicate, invented, or ambiguous duplicate fixture iteration identities reject exact before-phase completion coverage;
- every before-barrier participant is ordered before every after-barrier participant of the same barrier instance;
- sibling before phases receive no relative order from the barrier;
- sibling after phases receive no relative order from the barrier;
- distinct barrier fixture identities create no order by identity alone;
- an overlapping ordinary read/write or write/write pair remains a conflict even when the same barrier orders the before-phase access before the after-phase access;
- same-phase overlapping sibling state-changing accesses remain unordered and conflicting absent another interaction contract;
- a permitted logical Buffer state change in a before phase is present in the logical state read by an ordered after-phase fixture;
- physical arrival, release, worker, lane, chunk, queue, cache-fence, and rendezvous order are not semantic oracles.

These obligations do not define source barrier syntax, dynamic divergent-barrier validation, hierarchy-sensitive or partial-cohort barriers, atomics, fences, collectives, or physical barrier implementation.

## Identity-bearing unordered reduction boundary

These cases exercise the reduction interaction owned by `spec/language/exec/parallelism.md`. Reduction contributions are not modeled as ordinary accesses to a shared accumulator, while ordinary accesses and other semantic actions outside the reduction interaction remain subject to their own applicable contracts.

Required cases:

- unordered reduction is admitted only when the represented combination contract establishes normal closed combination, result-only combination, two-sided identity, associativity, and commutativity;
- failure to establish any one of those obligations rejects this unordered reduction form;
- the empty semantic contribution collection yields the explicit identity value;
- every semantic contribution occurrence is incorporated exactly once, including distinct contributions that carry semantically equal values;
- contribution coverage is insensitive to contribution ordering but rejects omitted, duplicated, invented, or ambiguous duplicate fixture identities;
- lawful test-local exact combination produces the same result across distinct contribution permutations and binary tree shapes;
- additional identity-valued physical partial initialization is permitted only as neutral realization state and does not count as a semantic contribution;
- permitted physical regrouping cannot change combination outcome or observable trace because the admitted combination contract requires normal result-only behavior;
- physical worker, lane, chunk, queue, partial-accumulator, and tree order are not semantic input;
- reduction admission does not legalize an overlapping ordinary sibling read/write or write/write conflict;
- a normally completed `each` carrying a reduction exposes its result to normal continuation only after all required iterations complete normally and all produced semantic contributions are incorporated;
- no result or partial-result contract is inferred for iteration fault, cancellation, divergence, or other abnormal completion.

Arithmetic used in an executable fixture is test-local evidence only. It must use values that avoid overflow or representation questions and does not define Runen integer, floating-point, or reduction-operator semantics.

## Structured task lifetime and detachment boundary

These cases exercise only structured task-scope lifetime/order and state-retention relations owned by `spec/language/exec/tasks.md`. They do not model task creation, execution, waiting, cancellation, fault propagation, or scheduling.

Required cases:

- normal completion of one structured task scope requires every child attached to that scope's normal-completion set to have completed normally;
- attached-child completion coverage is insensitive to completion-list order but rejects missing, duplicate, invented, or ambiguous duplicate fixture task identities;
- the empty attached-child set permits normal completion without inventing a child task;
- actions of an attached child are ordered before the originating scope's normal continuation;
- two children attached to the same scope receive no relative order from membership alone;
- a task detached from the originating scope receives no ordering to that scope's normal continuation from detachment alone;
- attachment or detachment does not extend, renew, copy, or upgrade a scope-bounded borrow/view permission;
- a scope-bounded state dependency is not safe to keep using after detachment once the originating scope may complete;
- owned and independently retained state dependencies are detach-safe under this lifetime relation;
- detached work is detach-safe only when every state dependency it still requires is owned or independently retained;
- task-scope membership does not legalize an otherwise-conflicting ordinary sibling access;
- no fault/cancellation/result behavior is inferred when an attached child does not complete normally.

`TaskId`, task-scope phases, and `TaskStateRetention` are verification-only tokens/classifications. `IndependentlyRetained` does not prescribe reference counting, allocation ownership, a runtime handle, or another retention implementation.

## Executable oracle coverage

The current `runen-exec-oracle` executable subset covers only relations already defined above:

- Buffer identity, finite logical-region overlap, and distinct-Buffer disjointness;
- ordinary read/state-change conflict classification;
- the cross-phase `each` normal entry/completion ordering relation, with no sibling or intra-iteration order;
- nested group/subgroup hierarchy membership, group-scoped subgroup identity, and order-neutral same-group/same-subgroup relations;
- full-`each` structured barrier phase ordering and exact before-phase participant completion coverage;
- a finite logical Buffer-state fixture for ordered state changes and reads, independent of physical replicas;
- complete unordered-reduction contract admission evidence and exact unordered semantic-contribution coverage;
- structured task-scope attached-child ordering/completion coverage and detachment state-retention admissibility.

Its `BufferId`, `PositionId`, `ValueToken`, iteration tokens, hierarchy tokens and memberships, barrier tokens, contribution tokens, task tokens, finite collections, reduction-contract evidence flags, and task-retention classifications are verification representation only. They do not freeze language values, source syntax, indexing, dimensional shape, compiler IR identities, hierarchy enumeration order, barrier order/topology, contribution order, operator traits, task handles, task parentage, retention mechanisms, versioning, physical allocation, scheduling, or backend representation.

The private generic exact-coverage helper used by hierarchy, barrier, reduction, and task fixtures is mechanical oracle implementation. It owns no Runen semantic concept.

## Future executable evidence

As additional Exec semantics acquire concrete executable consumers, extend reference/conformance evidence only after their normative owners are closed enough to state the expected behavior. Compiler IR, runtime, CPU/GPU realizations, and other production mechanisms should be checked against the strongest applicable semantic oracle without making the oracle representation authoritative over the specification.
