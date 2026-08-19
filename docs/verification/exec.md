# Exec Verification Contract

Status: **non-normative conformance-obligation documentation**

This document records focused assurance obligations for defined Exec semantic slices. It does not define Runen semantics, conformance profiles, compiler architecture, or repository CI.

The normative ordinary-access and structured-iteration rules exercised here are owned by `spec/language/exec/memory-model.md` and `spec/language/exec/parallelism.md`. Structured task lifetime, detachment, dynamic task-scope identity, explicit normal task join, and cooperative cancellation observation are owned by `spec/language/exec/tasks.md`. Buffer-specific identity, region, view-access, logical-coherence, and address-free typed-mapping facts are owned by `spec/language/exec/resources/buffers.md`; physical allocation identity and allocation extent are owned by `spec/language/exec/resources/allocations.md`. Core storage, overlap, borrowing, interior-mutability, and function-termination cleanup facts remain owned by their Core specifications.

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

These obligations do not authorize atomic behavior beyond the exchange and direct release/acquire relations covered separately below, commutative accumulation, collectives, or additional synchronization mechanisms whose normative contracts remain open. The structured barrier and identity-bearing unordered reduction are also covered separately below.

## Atomic exchange and direct release/acquire boundary

These cases exercise the atomic-exchange indivisibility, location-local modification-order, direct release/acquire synchronization, and root/group/subgroup-cohort scope relations owned by `spec/language/exec/memory-model.md`. They do not define which source storage forms support atomic access, and they consume location identity plus value/replacement admissibility as facts supplied by the applicable storage/resource owner. Dynamic `each`, iteration identity, semantic hierarchy establishment/binding, hierarchy/group/subgroup identity, and participant membership are consumed from `spec/language/exec/parallelism.md` without redefining them.

Required cases:

- an empty represented exchange set leaves the initial semantic location value unchanged;
- one unscoped base exchange returns the initial value and leaves its desired value as the final location value;
- two source-unordered unscoped base exchanges with distinct desired values admit both candidate modification-order permutations, with each exchange returning exactly the value installed immediately before it in that permutation;
- equal desired values do not collapse distinct exchange occurrences;
- duplicate represented exchange identities are rejected regardless of their represented synchronization scope;
- a represented exchange identity belonging to another semantic location is rejected by the fixture;
- candidate modification order requires exact unique coverage and rejects missing, duplicated, invented, or foreign-location exchange identities;
- a verification-only semantic-order constraint must refer to represented exchanges on the same location;
- a candidate modification order that violates such a constraint is rejected, while a satisfying order yields the corresponding prior and final values;
- exchange identity is structurally scoped by atomic-location identity, so equal private exchange tokens under distinct locations denote distinct occurrences;
- distinct locations have independent modification-order fixtures and no cross-location order is inferred;
- a root-scoped exchange records the `IterationId` performing that exchange, and its selected root-cohort identity is derived from that iteration's containing `EachId`; no independent root label can disagree with the producer;
- distinct sibling `IterationId`s belonging to the same dynamic `each` therefore select the same root-cohort scope without becoming the same iteration;
- a group-scoped exchange is admitted only from a represented exact established `HierarchyFixture` containing its producer iteration, and its selected `GroupId` is derived from that producer's validated membership rather than accepted as an independent label;
- an absent producer or producer from another dynamic `each` cannot obtain group-scope admission from that hierarchy fixture merely because a private iteration token matches;
- distinct sibling producers in the same exact established group select compatible group scope and may synchronize when the other direct release/acquire requirements hold;
- producers in distinct groups of one hierarchy select incompatible group scope and do not synchronize through that pair;
- equal private group tokens under distinct hierarchy identities remain distinct semantic groups and therefore yield incompatible group scopes, including distinct hierarchy fixtures under the same `EachId`;
- a subgroup-scoped exchange is admitted only from a represented exact established `HierarchyFixture` containing its producer iteration, and its selected `SubgroupId` is derived from that producer's validated membership rather than accepted as an independent label;
- an absent producer or producer from another dynamic `each` cannot obtain subgroup-scope admission from that hierarchy fixture merely because a private iteration token matches;
- distinct sibling producers in the same exact established subgroup select compatible subgroup scope and may synchronize when the other direct release/acquire requirements hold;
- producers in distinct subgroups select incompatible subgroup scope and do not synchronize through that pair, including equal private subgroup tokens under distinct groups;
- equal private subgroup tokens under distinct hierarchy identities remain distinct semantic subgroups and therefore yield incompatible subgroup scopes, including distinct hierarchy fixtures under the same `EachId`;
- unscoped, root-cohort-scoped, group-cohort-scoped, and subgroup-cohort-scoped exchanges represented on one atomic location participate in the same location-local modification order rather than creating scope-specific atomic locations or order partitions;
- scope classification does not change candidate modification-order coverage, applicable semantic-order constraints, prior-value observation, or final-value computation;
- private immediate-predecessor evidence is consumed only by the focused release/acquire relation and is not exposed as a modification-order query or enumeration surface;
- a base exchange does not create release/acquire synchronization;
- an unscoped release exchange that is the immediate modification-order predecessor of an unscoped acquire exchange on the same location synchronizes with that acquire, preserving the accepted unscoped relation;
- reversing those two exchanges in modification order removes that release-to-acquire synchronization;
- an acquire-release exchange is acquire-capable, so a directly preceding scope-compatible release or acquire-release exchange may synchronize with it;
- an acquire-release exchange is release-capable, so it may synchronize with a directly following scope-compatible acquire or acquire-release exchange;
- two directly adjacent acquire-release exchanges synchronize through the same direct-predecessor relation when their scopes are compatible;
- a base exchange between a release-capable and an acquire-capable exchange prevents the earlier exchange from synchronizing with the later one under this direct-predecessor relation;
- base immediately before acquire does not synchronize, and release immediately before base does not synchronize;
- root-cohort-scoped release/acquire occurrences performed by iterations belonging to the same dynamic `each` have a defined compatible scope relation and may synchronize when the other direct-relation requirements hold;
- otherwise identical directly adjacent root-cohort-scoped exchanges performed by iterations from distinct dynamic `each` identities have a defined incompatible scope relation and do not synchronize, while still participating in one modification order for the location;
- a mixed pair drawn from different represented scope forms is reported as an open scope interaction rather than a defined incompatible pair; this includes unscoped/root, unscoped/group, unscoped/subgroup, root/group, root/subgroup, and group/subgroup pairs, and the focused synchronization predicate supplies no current edge for such an open pair without proving future incompatibility;
- root-, group-, or subgroup-cohort scope identity does not infer sibling `each` order or legalize an otherwise-conflicting ordinary sibling access;
- equal desired or prior values do not infer synchronization; predecessor exchange identity, exchange semantics, and the defined scope relation control the relation;
- exchange identities from different atomic locations do not synchronize through this relation even when their represented cohort scopes would otherwise be compatible;
- release/acquire synchronization does not change the ordinary-access conflict predicate, even when that synchronization supplies semantic order around a conflicting ordinary pair;
- validating atomic exchange, scope, or release/acquire evidence does not infer sibling `each` order from physical scheduling.

`AtomicLocationId`, location-scoped `AtomicExchangeId`, `AtomicValueToken`, `AtomicExchangeSemantics`, `AtomicGroupScope`, `AtomicSubgroupScope`, `AtomicExchangeScope`, `AtomicScopeRelation`, `AtomicExchange`, `AtomicExchangeFixture`, and `AtomicExchangeRealization` are verification representation only. `Base`, `Release`, `Acquire`, and `AcquireRelease` are verification classifications of the normative semantic classes rather than a frozen source memory-order enumeration. `Unscoped`, `Root(IterationId)`, `Group(AtomicGroupScope)`, and `Subgroup(AtomicSubgroupScope)` are verification classifications of the currently represented synchronization-scope forms. Root scope derives root-cohort identity from the scoped exchange's verification-only producer iteration. `AtomicGroupScope` and `AtomicSubgroupScope` are constructed only from a represented exact established hierarchy fixture and the producer's validated membership; neither can publicly pair a producer with an unrelated free cohort label. They are not source hierarchy handles, hierarchy-establishment mechanisms, or reusable participant-domain abstractions. These forms are not a frozen source memory-scope enumeration or backend scope lattice. `Compatible`, `Incompatible`, and `Open` are verification evidence for the currently defined-or-open pairwise scope relationship, not source values or a general scope lattice. Candidate modification-order slices are supplied as realization evidence and are not exposed by an accepted realization as semantic scheduler order. Verification-only `(before, after)` constraint pairs stand for order facts already owned elsewhere; they do not define a generic Runen execution graph. Exact immediate-predecessor and per-exchange scope metadata remain private after realization; the accepted realization exposes only the focused scope-relation and synchronization predicates required by this verification boundary.

These obligations do not define atomic load/store, compare-exchange, fetch operations, release sequences, sequentially-consistent semantics, fences, broader atomic scope, mixed interoperability among unscoped/root/group/subgroup scope forms, source/lowering syntax for identifying the current iteration or establishing/selecting a hierarchy, mixed atomic/non-atomic race legality for the atomic location itself, source atomic syntax/types/order/scope enums, storage layout, addresses, progress guarantees, or backend atomic instructions.

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

These obligations do not define view construction/lifetime rules, raw-address or byte-level mapping, raw-address exposure, version representation, synchronization, or a physical coherence protocol.

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

These obligations do not define version counters, replica ownership, transfer completion, atomic access to Buffer storage, mixed atomic/non-atomic Buffer visibility rules, raw-address or byte-level mapping, or a coherence implementation algorithm.

## Address-free typed Buffer mapping and allocation-extent boundary

These cases exercise only the typed physical-accessibility relation owned by `spec/language/exec/resources/buffers.md` and the physical allocation identity/extent relation owned by `spec/language/exec/resources/allocations.md`. Mapping does not become logical access permission, synchronization, or an address/representation model.

Required cases:

- one allocation fixture has one equality-only physical allocation identity and one represented live extent;
- one typed mapping occurrence is structurally bound to exactly one allocation identity and exactly one logical Buffer region;
- equal private mapping tokens under distinct allocation identities denote distinct mapping occurrences;
- a mapping cannot begin after its backing allocation extent has ended;
- an allocation extent cannot end while a represented mapping still depends on it;
- ending the exact mapping releases only that extent dependency, after which the allocation extent may end;
- attempting to end a mapping through another allocation identity is rejected without ending the mapping;
- an ended mapping cannot supply further mapped-access evidence;
- duplicate mapping occurrence identity within one allocation fixture is rejected;
- one mapping admits typed access evidence only for logical positions contained in its selected Buffer region; a disjoint same-Buffer region is not made physically accessible by that mapping;
- mapping the same logical Buffer region through distinct allocation identities does not change Buffer identity, region equality, or region overlap semantics;
- a mapped state-changing access updates the same `LogicalBufferState` fixture used by ordinary ordered-coherence evidence rather than a second physical/staging state;
- a later mapped read through another allocation identity observes the current logical Buffer state after an already-established logical change, not an allocation-local stale copy;
- ending a mapping does not mutate logical Buffer state;
- existence of a mapping does not change `Access::conflicts_with`: overlapping source-unordered read/change and change/change pairs remain conflicting under the ordinary relation;
- equal numeric fixture fields used by `AllocationId` and `BufferId` do not merge those distinct identity domains;
- no raw address, pointer, byte sequence, layout, alignment, stride, contiguity, allocation-space class, version/dirty state, transfer queue, scheduler order, or backend operation is semantic evidence for the mapping relation.

`AllocationId`, `AllocationFixture`, `AllocationError`, `BufferMappingId`, `BufferMappingFixture`, and `BufferMappingError` are verification representation only. Allocation-fixture begin/end and private active-mapping bookkeeping are mechanical lifetime evidence, not allocator or source mapping operations. `mapped_read` and `mapped_change` stand only for typed accesses whose independent logical permission/order/legality obligations are already satisfied; they do not grant access authority. The fixture stores no physical value copy, address, byte state, version, transfer state, or mapping-derived order.

These obligations do not define allocation creation/destruction APIs, allocation spaces or interoperability, raw-address/pinned/byte mappings, relocation, address stability, pointer provenance, representation validity, Buffer source API spelling, flush/invalidate semantics, atomic Buffer access, mixed atomic/non-atomic Buffer access, or a physical coherence protocol.

## Structured `each` normal-completion boundary

These cases exercise only normal structured entry/completion. They do not define abnormal iteration completion or infer sibling order from a physical schedule.

Required cases:

- each dynamic `each` fixture has an equality-only execution identity, and required iteration identity is structurally scoped by that execution identity;
- equal private iteration tokens under distinct dynamic `each` identities denote distinct iteration identities;
- a permitted state change sequenced before entry to `each` is semantically before a later permitted overlapping read performed by an iteration when both belong to the same defined continuation of the same dynamic `each`;
- sibling iterations remain source-unordered even when one legal realization executes them sequentially;
- entry, iteration, and normal-continuation phase points from distinct dynamic `each` identities receive no order from the structured-`each` relation even when their private tokens otherwise match;
- source-unordered overlapping ordinary sibling read/write or write/write access remains conflicting under the accepted memory-model rule;
- source-unordered state-changing accesses to disjoint Buffer regions are non-conflicting under that rule and may execute physically concurrently;
- a normally completed `each` has no normal continuation until every required iteration has completed normally;
- after normal completion, a permitted continuation read of a Buffer region changed by an iteration receives logical state after that semantically ordered iteration change;
- after normal completion, the continuation may consume the combined effects of several disjoint sibling state changes without imposing a relative order among those sibling iterations;
- the normal join boundary makes no claim about an iteration that faults, is cancelled, diverges, or otherwise fails to complete normally;
- backend queue order, worker order, host thread timing, lane order, chunk order, and physical serialization are not semantic oracles for sibling iteration order.

`EachId`, `IterationId`, and `EachPhase` are verification representation only. Their private tokens are not source iteration handles, launch identifiers, indices, worker/lane identities, scheduler tokens, or execution order.

These obligations do not define iteration construction, task semantics, fault aggregation, cancellation, early exit, atomic exchange semantics, or collectives. The atomic-exchange, hierarchy, structured-barrier, and unordered-reduction contracts are covered separately.

## Group and subgroup hierarchy boundary

These cases exercise the nested logical participant hierarchy and semantic establishment/binding relation owned by `spec/language/exec/parallelism.md`. They do not make hierarchy establishment or membership a synchronization, scheduling, source, or hardware-topology mechanism.

Required cases:

- an empty required `each` iteration set admits exactly an empty hierarchy membership set while retaining the containing dynamic `each` identity, and does not fabricate empty groups or subgroups;
- for a non-empty `each`, every required iteration has exactly one hierarchy membership and no invented iteration receives one;
- duplicate required fixture iteration identities, duplicate membership for one iteration, missing required iterations, and invented iterations are rejected;
- a required iteration or membership iteration from another dynamic `each` is rejected even when its private iteration token matches a local token;
- membership whose group/subgroup identity belongs to another hierarchy fixture is rejected;
- hierarchy identity is scoped by the containing dynamic `each`, so equal private hierarchy tokens under distinct `EachId`s denote distinct hierarchy identities;
- two distinct verification hierarchy fixtures may use distinct `HierarchyId`s under the same `EachId`; representing that case does not assert that source semantics always permit multiple hierarchy instances;
- equal private group/subgroup tokens under two distinct same-`each` hierarchy identities remain distinct scoped identities;
- groups form a disjoint and exhaustive partition of the required `each` iteration set;
- subgroups within one group form a disjoint and exhaustive partition of that group;
- group identity is scoped by the containing hierarchy and therefore transitively by the dynamic `each`, so equal private group tokens under distinct hierarchy IDs denote distinct group identities;
- subgroup identity is scoped by the containing group and therefore transitively by the hierarchy and dynamic `each`, so equal private subgroup tokens under distinct groups or hierarchies denote distinct subgroup identities;
- same-subgroup membership implies same-group membership;
- reordering the private finite membership storage for the same hierarchy identity does not change membership, same-group, or same-subgroup results;
- hierarchy, group, and subgroup identities expose equality only and do not become numeric indices, coordinates, lanes, hardware cohorts, or scheduling order;
- a group/subgroup barrier fixture bound to hierarchy A rejects an otherwise equal-token selector belonging to distinct hierarchy B even when both hierarchy fixtures use the same `EachId`;
- a group/subgroup reduction fixture bound to hierarchy A likewise rejects a hierarchy-B selector under the same dynamic `each`;
- hierarchy establishment, identity, and membership supply no sibling iteration order and do not legalize an otherwise-conflicting ordinary sibling access;
- root barrier and root reduction participation each remain the complete required `each` set independent of hierarchy membership, while narrower forms require an explicit group or subgroup selection from one exact established hierarchy instance.

`HierarchyId`, hierarchy-scoped `GroupId`, group-scoped `SubgroupId`, `HierarchyMembership`, and the finite hierarchy fixture are verification representation only. A `HierarchyFixture` stands for one already-established semantic hierarchy instance for the focused verification question; representing multiple fixtures under one `EachId` does not define source-level hierarchy multiplicity. Their identity scoping consumes `EachId`; it does not define source hierarchy or iteration handles, hierarchy construction, selection, observation, admission, multiplicity, group or subgroup sizes, dimensions, coordinates, enumeration order, launch geometry, runtime worker topology, or hardware subgroup identity.

These obligations do not define broader atomic or fence scope, collectives beyond the unordered reduction covered separately below, group-local storage, broadcast, shuffle, scans, or another hierarchy-sensitive operation. Root-, group-, and subgroup-cohort atomic exchange scope are covered separately above. Group and subgroup scope consume one exact established hierarchy binding and the producer's membership without redefining hierarchy establishment. Other hierarchy-sensitive operations may consume the normative establishment/binding rule but still require their own operation-specific contracts before executable evidence is extended to cover them.

## Cohort-scoped structured barrier boundary

These cases exercise the selected-cohort phase boundary owned by `spec/language/exec/parallelism.md` and the participant ordinary-access synchronization relation owned by `spec/language/exec/memory-model.md`. The barrier remains a structured phase cut rather than an imperative lane-level operation.

Required cases:

- a root barrier is explicitly bound to one dynamic `EachId`, selects exactly that execution's complete required iteration set without requiring a hierarchy, and preserves the identity even for an empty root cohort;
- duplicate required root fixture iteration identities are rejected;
- a root barrier rejects a required iteration from another dynamic `each` even when its private token matches a local participant token;
- a group barrier selects exactly one existing group from one represented established hierarchy instance and rejects an unknown group;
- a subgroup barrier selects exactly one existing group-scoped subgroup from that exact hierarchy instance and rejects an unknown subgroup;
- a group barrier rejects a group identity from another hierarchy even when the private group token matches a local group, including across distinct hierarchy identities under the same `EachId`;
- a subgroup barrier rejects a subgroup identity from another hierarchy even when the private group/subgroup tokens match a local subgroup, including across distinct hierarchy identities under the same `EachId`;
- equal private subgroup tokens under distinct groups select distinct subgroup cohorts;
- a group or subgroup barrier does not admit an iteration from another dynamic `each` as a participant merely because its private iteration token matches a local participant;
- a group barrier excludes iterations belonging to other groups;
- a subgroup barrier excludes same-group iterations belonging to other subgroups and all iterations in other groups;
- the exact participant set is private verification state and supplies no public enumeration order;
- barrier before/after phase points can be obtained only for actual participants; nonparticipants cannot fabricate phase participation through the public oracle API;
- normal barrier completion requires exact before-phase completion by every participant, independent of completion-list order;
- missing, duplicate, invented, or foreign-`each` completion identities reject exact before-phase completion coverage;
- every participant before-barrier phase is ordered before every participant after-barrier phase of the same barrier instance;
- sibling participant before phases receive no relative order from the barrier;
- sibling participant after phases receive no relative order from the barrier;
- a nonparticipant receives no phase, completion obligation, ordinary-access order, or Buffer visibility consequence from the barrier;
- distinct barrier fixture identities create no order by identity or cohort relationship alone;
- an overlapping ordinary participant read/write or write/write pair remains a conflict even when the same barrier orders the before-phase access before the after-phase access;
- same-phase overlapping participant state-changing accesses remain unordered and conflicting absent another interaction contract;
- a permitted logical Buffer state change by a participant in a before phase is present in the logical state read by an ordered participant after-phase fixture;
- hierarchy membership without an explicit barrier creates no synchronization;
- physical arrival, release, worker, lane, chunk, queue, cache-fence, and rendezvous order are not semantic oracles.

`BarrierFixture`, `BarrierId`, `BarrierPhase`, `EachId`, scoped iteration identities, and finite participant collections are verification representation only. The fixture is not a source barrier API, runtime rendezvous object, hardware scope, atomic memory-scope model, or source-visible dynamic-`each` handle. The root case replaces the prior root-only free barrier helpers; no compatibility barrier oracle is retained.

These obligations do not define source barrier syntax, dynamic divergent-barrier validation, atomic order semantics beyond direct release/acquire, broader atomic scope or fence semantics, additional collectives, group-local storage, or physical barrier implementation. Root-, group-, and subgroup-cohort atomic exchange scope are covered separately above.

## Cohort-scoped identity-bearing unordered reduction boundary

These cases exercise the selected-cohort reduction interaction owned by `spec/language/exec/parallelism.md`. Reduction contributions are not modeled as ordinary accesses to a shared accumulator, while ordinary accesses and other semantic actions outside the reduction interaction remain subject to their own applicable contracts.

Required cases:

- unordered reduction is admitted only when the represented combination contract establishes normal closed combination, result-only combination, two-sided identity, associativity, and commutativity;
- failure to establish any one of those obligations rejects this unordered reduction form;
- a root reduction is explicitly bound to one dynamic `EachId`, selects exactly that execution's complete required iteration set without requiring a hierarchy, and preserves the identity even for an empty root cohort;
- duplicate required root fixture iteration identities are rejected;
- a root reduction rejects a required iteration from another dynamic `each` even when its private token matches a local participant token;
- a group reduction selects exactly one existing group from one represented established hierarchy instance and rejects an unknown group;
- a subgroup reduction selects exactly one existing group-scoped subgroup from that exact hierarchy instance and rejects an unknown subgroup;
- a group reduction rejects a group identity from another hierarchy even when the private group token matches a local group, including across distinct hierarchy identities under the same `EachId`;
- a subgroup reduction rejects a subgroup identity from another hierarchy even when the private group/subgroup tokens match a local subgroup, including across distinct hierarchy identities under the same `EachId`;
- equal private subgroup tokens under distinct groups select distinct reduction cohorts;
- a group or subgroup reduction does not admit an iteration from another dynamic `each` as a participant or contribution producer merely because its private iteration token matches a local participant;
- the exact participant set is private verification state and supplies no public enumeration order;
- a semantic contribution token can be created only for an actual reduction participant; nonparticipants cannot fabricate contribution membership through the public oracle API;
- one participant may produce multiple distinct semantic contribution occurrences when the enclosing reduction operation permits that cardinality;
- a non-empty reduction cohort may produce zero semantic contributions, in which case the explicit identity remains the result;
- every semantic contribution occurrence is incorporated exactly once, including distinct contributions that carry semantically equal values;
- contribution coverage is insensitive to incorporation ordering but rejects omitted, duplicated, invented, cross-reduction, foreign-`each`, or ambiguous duplicate occurrence identities;
- duplicate required `ContributionId`s are invalid even when the duplicate tokens record different participant producers;
- lawful test-local exact combination produces the same result across distinct contribution permutations and binary tree shapes;
- additional identity-valued physical partial initialization is permitted only as neutral realization state and does not count as a semantic contribution;
- permitted physical regrouping cannot change combination outcome or observable trace because the admitted combination contract requires normal result-only behavior;
- physical worker, lane, chunk, queue, partial-accumulator, and tree order are not semantic input;
- reduction cohort membership and reduction admission do not order sibling iterations or legalize an overlapping ordinary sibling read/write or write/write conflict;
- a normally completed `each` carrying a reduction exposes the one reduction result only at its normal continuation after every required `each` iteration completes normally and every semantic contribution produced by reduction participants is incorporated;
- no cohort-local continuation, participant-local result, leader/lane result, or automatic result distribution is inferred from selecting a group or subgroup cohort;
- no result or partial-result contract is inferred for iteration fault, cancellation, divergence, or other abnormal completion.

`ReductionFixture`, `ReductionId`, `ContributionId`, `ReductionContribution`, `EachId`, scoped iteration identities, and finite participant/contribution collections are verification representation only. They do not define source reduction syntax, a runtime reduction object, a source-visible dynamic-`each` handle, collective result distribution, physical accumulator identity, a reduction tree, or participant enumeration order. The fixture replaces the prior free contribution-coverage helper; no compatibility reduction oracle is retained.

Arithmetic used in an executable fixture is test-local evidence only. It must use values that avoid overflow or representation questions and does not define Runen integer, floating-point, or reduction-operator semantics.

## Structured task lifetime and detachment boundary

These cases exercise structured task-scope lifetime/order and state-retention relations owned by `spec/language/exec/tasks.md`. They do not model task creation, execution, fault propagation, or scheduling.

Required cases:

- each represented dynamic structured task scope has an equality-only identity used solely to scope its attachment/detachment/normal-continuation ordering evidence;
- normal completion of one structured task scope requires every child attached to that scope's normal-completion set to have completed normally;
- attached-child completion coverage is insensitive to completion-list order but rejects missing, duplicate, invented, or ambiguous duplicate fixture task identities;
- the empty attached-child set permits normal completion without inventing a child task;
- actions of an attached child are ordered before the normal continuation of that same dynamic structured task scope;
- an attached-child phase associated with one dynamic task scope is not ordered before the normal continuation of a distinct dynamic task scope, including when the test phases reuse the same `TaskId` token;
- two children attached to the same scope receive no relative order from membership alone;
- a task detached from the originating scope receives no ordering to that scope's normal continuation from detachment alone;
- attachment or detachment does not extend, renew, copy, or upgrade a scope-bounded borrow/view permission;
- a scope-bounded state dependency is not safe to keep using after detachment once the originating scope may complete;
- owned and independently retained state dependencies are detach-safe under this lifetime relation;
- detached work is detach-safe only when every state dependency it still requires is owned or independently retained;
- task-scope identity or membership does not legalize an otherwise-conflicting ordinary sibling access;
- no fault/result behavior is inferred when an attached child does not complete normally.

`TaskScopeId`, `TaskId`, task-scope phases, and `TaskStateRetention` are verification-only tokens/classifications. `TaskScopeId` is not a source scope handle, runtime parent object, nesting level, executor identity, scheduler identity, or observable ordering token. `IndependentlyRetained` does not prescribe reference counting, allocation ownership, a runtime handle, or another retention implementation.

## Explicit normal task join boundary

These cases exercise only the explicit normal task-join target/completion/ordering relation owned by `spec/language/exec/tasks.md`. The oracle does not model source task handles, task results, physical waiting, or abnormal join behavior.

Required cases:

- normal join completion evidence succeeds only for `Normal` completion of the exact target task;
- normal completion evidence for another task does not satisfy the join target;
- cancelled completion evidence for the target task does not satisfy normal join completion, and no cancelled/faulted join result is inferred;
- actions of the exact target task are ordered before the post-join normal continuation;
- the reverse post-join-to-target direction is not inferred;
- an unrelated task receives no order to that post-join continuation;
- two distinct join occurrences do not order their post-join continuations merely by join identity, including when both target the same task;
- a detached task remains unordered to its originating structured scope's continuation from detachment alone, while a later explicit normal join may independently order that exact task to its post-join continuation;
- later join ordering does not change the existing detach-safe classification: scope-bounded state remains unsafe to require after detachment, while owned or independently retained state remains detach-safe;
- join targeting does not create sibling-task order or legalize an otherwise-conflicting unrelated ordinary access;
- target-to-post-join semantic order may be consumed by the existing Buffer ordered-coherence contract without creating a second Buffer visibility rule;
- no progress, fairness, physical blocking, scheduler, result transport, fault propagation, cancellation propagation, or source task-handle semantics are inferred.

`TaskJoinId`, `TaskJoinPhase`, `task_join_orders`, and `task_join_can_complete_normally` are verification representation only. `TaskJoinId` is not a source join handle, task handle, runtime wait object, scheduler event, progress token, or generic dependency-graph node. `TargetTask` and `After` are focused verification phase classifications, not source or runtime task states.

These obligations do not define source `spawn`/`await`/`join` syntax, task-handle acquisition or representation, task results, fault/cancellation behavior of a joining context, eligibility or multiplicity of joins, progress/fairness, physical suspension/wakeup, executor machinery, or task-scope parentage.

## Cooperative task cancellation observation boundary

These cases exercise only the explicitly sequenced cooperative cancellation relation owned by `spec/language/exec/tasks.md`. The terminal cancellation transition consumes the cleanup consequence owned by `spec/language/core/value-storage.md`; the task oracle does not add a Core `Cancel` instruction or duplicate Core destruction-domain semantics.

Required cases:

- a new one-task cancellation fixture starts running with no pending request;
- explicit cancellation observation with no pending request yields `Continue` and leaves the represented task running;
- an explicitly sequenced valid request changes only the cancellation state to pending and does not by itself make the task terminal;
- request followed by explicit observation yields terminal `Cancelled`;
- repeated valid requests while cancellation is pending are idempotent;
- a request or observation naming another fixture task is rejected without changing the represented task's state;
- the terminal fixture admits no further cancellation observation transition;
- cancellation pending state, request, and observation do not create sibling task order or legalize an otherwise-conflicting ordinary sibling access;
- a cancelled attached child does not count as a normally completed child for the existing exact attached-completion relation;
- detachment and detach-safe state-retention evidence remain unchanged by cancellation state;
- fixture transition call order stands only for semantic sequencing already supplied by an applicable contract and is not evidence for source-unordered request/observation races or host-timing order.

`TaskCancellationFixture`, `TaskCancellationState`, `TaskCancellationObservation`, and `TaskCancellationError` are verification representation only. `Running`, `CancellationPending`, `Cancelled`, and `Continue` are fixture classifications/results, not frozen source task states, runtime task handles, cancellation tokens, scheduler states, or a general outcome API. `TerminalTask` means the focused fixture has no further represented cancellation-state transition; it does not define source-handle validity or a post-completion request API.

The current Core MIR/reference machine is intentionally unchanged. Existing Core cleanup semantics remain authoritative for the reverse-local destruction-domain procedure that the normative cancellation contract consumes once cancellation termination has been selected.

These obligations do not define cancellation-request authority, source spawn/await/cancel forms, source-unordered request/observation interaction, implicit/asynchronous preemption, polling, timers, deadlines, fairness, masking, containing-scope or sibling propagation, task results, fault aggregation, catch/unwind policy, custom destructors, runtime executor machinery, or a Core cancellation instruction.

## Executable oracle coverage

The current `runen-exec-oracle` executable subset covers only relations already defined above:

- Buffer identity, finite logical-region overlap, and distinct-Buffer disjointness;
- physical allocation identity/extent plus active typed-mapping lifetime nesting, exact Buffer-region mapping coverage, and mapped typed access through the one logical Buffer-state fixture;
- ordinary read/state-change conflict classification;
- validated atomic-exchange occurrence identity, exchange semantics and producer-derived root/group/subgroup-scope classification, exact candidate modification-order coverage across scope forms, location-local semantic-order constraints, prior-value observation, private immediate-predecessor/exact-scope evidence, focused compatible/incompatible/open scope-relation evidence, direct scope-compatible release/acquire synchronization, and final-value computation;
- dynamic-`each`-scoped iteration identity plus the instance-local cross-phase `each` normal entry/completion ordering relation, with no sibling, intra-iteration, or cross-`each` order;
- dynamic-`each`-scoped hierarchy identity, exact established-hierarchy fixture binding, hierarchy-instance-scoped group identity, group-scoped subgroup identity, nested hierarchy membership, foreign-`each`/foreign-hierarchy rejection including same-`each` distinct-hierarchy selectors, and order-neutral same-group/same-subgroup relations;
- validated root/group/subgroup structured-barrier cohorts, explicit root `EachId`, exact established-hierarchy binding for narrower cohorts, foreign-`each`/foreign-hierarchy rejection, participant-only phase construction, cross-phase ordering, and exact before-phase completion coverage;
- a finite logical Buffer-state fixture for ordered state changes and reads, independent of physical replicas;
- complete unordered-reduction contract admission evidence plus validated root/group/subgroup reduction cohorts, explicit root `EachId`, exact established-hierarchy binding for narrower cohorts, foreign-`each`/foreign-hierarchy rejection, participant-only contribution construction, and exact unordered semantic-contribution coverage;
- dynamic-task-scope-local attached-child ordering, outcome-aware normal-completion coverage, detachment state-retention admissibility, explicit normal task-join target-to-continuation ordering plus exact normal-target completion evidence, and explicitly sequenced cooperative cancellation request/observation transitions.

Its atomic-location/exchange/value tokens, exchange-semantics/scope/scope-relation classifications, validated atomic group/subgroup-scope admission tokens, private predecessor/exact-scope evidence and fixtures, `AllocationId`, allocation extent fixture/error classifications, `BufferId`, `PositionId`, `ValueToken`, `BufferMappingId`, mapping fixture/error classifications, `EachId`, scoped iteration tokens, hierarchy tokens and memberships, barrier tokens and validated fixtures, reduction/contribution tokens and validated fixtures, `TaskScopeId`, task tokens, `TaskJoinId`, task-join phase classifications, cancellation fixture/state/result/error classifications, finite collections, reduction-contract evidence flags, and task-retention classifications are verification representation only. They do not freeze language values, source syntax, indexing, dimensional shape, compiler IR identities, source iteration or hierarchy handles, source hierarchy construction/observation/multiplicity, atomic storage forms, source memory-order or memory-scope enumerations, modification-order representation, hierarchy enumeration order, barrier participant order/topology, reduction participant or contribution order, operator traits, task handles, source join handles or runtime waits, task-scope handles or nesting, cancellation handles, task parentage, retention mechanisms, allocation APIs or spaces, physical or numeric addresses, mapping source handles, layout or representation, versioning, physical-copy state, scheduling, or backend representation.

The private generic exact-coverage helper used by atomic, hierarchy, barrier, reduction, and task fixtures, the crate-private dynamic-`each` identity relation used by structured/hierarchy/barrier/reduction/atomic fixtures, the crate-private hierarchy cohort collection used by barrier and reduction fixtures, and the allocation fixture's private active-mapping-token bookkeeping are mechanical oracle implementation. They own no Runen semantic concept.

## Future executable evidence

As additional Exec semantics acquire concrete executable consumers, extend reference/conformance evidence only after their normative owners are closed enough to state the expected behavior. Compiler IR, runtime, CPU/GPU realizations, and other production mechanisms should be checked against the strongest applicable semantic oracle without making the oracle representation authoritative over the specification.
