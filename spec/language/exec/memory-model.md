# Exec Memory Model

Status: **provisional normative; incomplete**

This document owns Exec access-conflict, data-race, memory-ordering, and synchronization relations. Resource-specific identity, region, and state semantics remain owned by the semantic owner of the storage or resource being accessed.

## Ordinary non-atomic access

An **ordinary non-atomic access** is a semantic access to one storage or resource region that is not defined by an atomic operation.

The canonical owner of the accessed storage or resource defines its region identity, overlap relation, and state transitions. Exec consumes those facts; it does not reinterpret them from host addresses, allocator identity, or backend memory operations.

For the conflict relation, an ordinary access is either:

- a **non-state-changing read**, which observes the selected region without changing its semantic value or storage state; or
- a **state-changing access**, which changes the selected region's semantic value or storage state, including applicable initialization, reinitialization, replacement, destruction, or ownership-consuming transfer.

Two ordinary non-atomic accesses **conflict** exactly when their semantic regions overlap and at least one of the accesses is state-changing.

This conflict relation does not erase additional effect, ordering, authority, validity, or resource rules owned elsewhere. An access that does not conflict under this relation is legal only when every other applicable semantic contract also permits it.

For Core-origin storage crossing into Exec, Core remains the owner of storage extent, stored-value state, structural region facts, place overlap, and alias authority. In particular, Core interior mutability under shared aliases is not by itself an Exec synchronization mechanism or permission for conflicting unordered execution.

## Unordered ordinary access

Access legality is determined by semantic ordering and interaction contracts, not by incidental physical scheduler order.

For source-unordered work with no separately defined synchronization or interaction mechanism:

- accesses to disjoint semantic regions do not conflict under the ordinary-access relation;
- overlapping non-state-changing reads do not conflict under the ordinary-access relation;
- a conflicting pair of ordinary non-atomic accesses is not a legal safe interaction.

Therefore a realization that happens to serialize source-unordered conflicting accesses does not make them legal, and physically parallel execution does not by itself make an otherwise permitted non-conflicting interaction illegal.

This is a language/profile semantic legality rule. It is not environment admission and is not an optimization preference. The source-language or compiler mechanism by which a program establishes the required interaction contract is not defined by this revision.

## Mixed ordinary non-atomic and atomic-exchange conflict

For an admitted atomic exchange, the canonical owner of the targeted storage or resource may define a logical access footprint for the exchange's semantic location under that resource's region and overlap relation.

An ordinary non-atomic access and that atomic exchange **conflict** exactly when the ordinary access region overlaps that resource-defined atomic-location footprint. Because the atomic exchange defined by this revision is state-changing, both an overlapping ordinary read and an overlapping ordinary state-changing access conflict with the exchange. An ordinary access whose region is disjoint from the atomic-location footprint does not conflict with that exchange under this mixed relation.

For source-unordered work with no separately defined synchronization or interaction mechanism, such a mixed conflicting pair is not a legal safe interaction. A realization that happens to serialize the ordinary access and atomic exchange physically does not make the source-unordered pair legal.

This conflict relation does not make the ordinary access atomic and does not insert it into the atomic location's modification order. The modification order defined below still contains only atomic exchanges governed by that contract.

If another semantic contract establishes order between an ordinary access and an atomic exchange, the pair is not source-unordered solely with respect to that order. This mixed-conflict rule does not thereby grant either access, define atomic capability, or by itself define the value/visibility consequence of every ordered mixed interaction. Every applicable authority, resource-state, ordering, validity, effect, and operation-specific contract still has to permit and define the interaction.

The first currently defined resource footprint consumed by this relation is the singleton logical Buffer region of a Buffer atomic location defined by [Exec Buffers](resources/buffers.md). This document does not duplicate Buffer location identity, region overlap, or state semantics.

## Atomic exchange base relation

An **atomic exchange** is a normally completing read-modify-write interaction with exactly one semantic location.

The canonical owner of the targeted storage or resource defines that location's semantic identity, stored-value state, and whether the desired replacement value and replacement operation are admitted. This rule does not infer those facts from an address, allocation, backend atomic type, or machine instruction, and it does not define which source storage forms support atomic access.

For one admitted exchange targeting location `L` with desired value `d`, the operation indivisibly observes the value immediately preceding its replacement under the modification-order rule below, replaces the value of `L` with `d`, and returns that preceding value. The observe-and-replace step is indivisible with respect to every other atomic exchange governed by this contract on the same semantic location.

### Location-local modification order

All normally completing atomic exchanges governed by this contract that modify one semantic location occur in one total **modification order** for that location.

Each exchange returns the desired value installed by its immediately preceding exchange in that location's modification order, if such a predecessor exists. The first exchange instead returns the location value immediately before the first exchange. After any finite non-empty prefix of the modification order, the location value is the desired value installed by the last exchange in that prefix.

Any applicable Runen semantic ordering already established between two exchanges constrains their modification order. If exchange `A` is semantically ordered before exchange `B`, then `A` MUST precede `B` in the modification order of their common location.

For source-unordered exchanges on one location, any total modification order that extends all applicable established semantic-order constraints is permitted. The prior values and location values resulting from every such order are explicitly permitted behaviors of this operation under [Correctness Relations](../correctness.md).

Incidental physical worker order, scheduler order, queue order, cache behavior, or instruction choice is not additional semantic input. A realization MUST NOT produce an exchange result that cannot arise from a permitted location-local modification order.

Modification orders of distinct semantic locations are independent under this base relation. This rule does not create one global order across atomic locations.

### Exchange synchronization classes

For the synchronization relation defined by this revision, each atomic exchange has exactly one of these semantic classes:

- a **base exchange** has only the atomicity and modification-order semantics above;
- a **release exchange** has those base semantics plus the release behavior below;
- an **acquire exchange** has those base semantics plus the acquire behavior below;
- an **acquire-release exchange** has those base semantics plus both the release and acquire behaviors below.

These classes are semantic operation distinctions, not frozen source spellings or a complete future memory-order enumeration.

A **release-capable exchange** is a release exchange or an acquire-release exchange. An **acquire-capable exchange** is an acquire exchange or an acquire-release exchange.

A release exchange does not gain acquire behavior merely because the exchange also returns a prior value. An acquire exchange does not gain release behavior merely because the exchange also installs a desired value. An acquire-release exchange has both behaviors by definition. A base exchange supplies neither release nor acquire synchronization.

### Exchange synchronization scope

For the direct synchronization relation defined by this revision, an atomic exchange may use any of these currently represented scope forms:

- an **unscoped exchange** participates in the previously defined unscoped direct synchronization relation when paired with another unscoped exchange;
- a **root-cohort-scoped exchange** is performed by one iteration of a dynamic `each` execution and selects the complete root cohort of that iteration's containing dynamic `each`, as defined by [Exec parallelism](parallelism.md);
- a **group-cohort-scoped exchange** is performed by one iteration of a dynamic `each` execution, binds to one exact hierarchy instance already established for that execution, and selects exactly the producer iteration's group in that bound hierarchy instance; or
- a **subgroup-cohort-scoped exchange** is performed by one iteration of a dynamic `each` execution, binds to one exact hierarchy instance already established for that execution, and selects exactly the producer iteration's subgroup in that bound hierarchy instance.

A root-cohort scope does not require a hierarchy instance. The scoped exchange's executing iteration identity determines the selected dynamic `each` identity; there is no independent root-cohort identity supplied to the operation that can disagree with that producer. The iteration and dynamic-`each` identities remain the opaque semantic identities owned by Exec parallelism, not source handles, numeric indices, launch identifiers, worker or lane identities, queue identities, scheduler tokens, or hardware memory-scope enumerations.

A group-cohort-scoped exchange requires one exact hierarchy instance already established for the producer iteration's dynamic `each` under [Exec parallelism](parallelism.md). The exchange binds to that semantic hierarchy identity; neither the operation nor a realization may establish a substitute hierarchy or silently replace the bound hierarchy with another same-`each` hierarchy merely because implementation/debug cohort tokens or physical topology happen to match. The selected group is the producer iteration's stable group membership in that exact bound hierarchy. There is no independently supplied group identity that can disagree with the producer's membership.

A subgroup-cohort-scoped exchange likewise requires one exact hierarchy instance already established for the producer iteration's dynamic `each`. The exchange binds to that semantic hierarchy identity, and the selected subgroup is the producer iteration's stable subgroup membership in that exact bound hierarchy. There is no independently supplied subgroup identity that can disagree with the producer's membership. Because subgroup identity is scoped by its containing group, equal implementation or debug subgroup tokens under distinct groups or hierarchy identities do not denote the same subgroup scope.

The source or lowering mechanism that identifies the current iteration and requests a scoped exchange form is not defined by this revision. The source mechanism that causes or selects an established hierarchy is likewise not defined here and remains owned by Exec parallelism's establishment boundary. Those open source/lowering boundaries do not make semantic scope a free-floating label: a root-cohort-scoped exchange belongs to its recorded producer iteration, and a group- or subgroup-cohort-scoped exchange additionally consumes that producer's exact established hierarchy binding and membership.

Scope does not partition an atomic location or its modification order. Unscoped, root-cohort-scoped, group-cohort-scoped, and subgroup-cohort-scoped exchanges on one semantic location remain atomic exchanges on that same location and participate in the one location-local modification order above. Scope changes only the synchronization relationship that may be established between exchange occurrences.

The **synchronization-scope relationship** for two exchanges is defined by this revision in these cases:

- two unscoped exchanges are **scope-compatible**, preserving the accepted unscoped relation;
- two root-cohort-scoped exchanges whose executing iterations belong to the same dynamic `each` are **scope-compatible**;
- two root-cohort-scoped exchanges whose executing iterations belong to distinct dynamic `each` identities are **scope-incompatible**;
- two group-cohort-scoped exchanges whose selected groups have the same semantic group identity are **scope-compatible**;
- two group-cohort-scoped exchanges whose selected groups have distinct semantic group identities are **scope-incompatible**, including groups with equal implementation or debug group tokens scoped by different hierarchy identities;
- two subgroup-cohort-scoped exchanges whose selected subgroups have the same semantic subgroup identity are **scope-compatible**;
- two subgroup-cohort-scoped exchanges whose selected subgroups have distinct semantic subgroup identities are **scope-incompatible**, including equal implementation or debug subgroup tokens scoped by distinct groups or hierarchy identities;
- one root-cohort-scoped exchange and one group-cohort-scoped exchange are **scope-compatible** exactly when the root-scoped producer belongs to the exact group selected by the group-scoped exchange in that group's exact established hierarchy; otherwise that valid root/group pair is **scope-incompatible**;
- one root-cohort-scoped exchange and one subgroup-cohort-scoped exchange are **scope-compatible** exactly when the root-scoped producer belongs to the exact subgroup selected by the subgroup-scoped exchange in that subgroup's exact established hierarchy; otherwise that valid root/subgroup pair is **scope-incompatible**;
- one group-cohort-scoped exchange and one subgroup-cohort-scoped exchange are **scope-compatible** exactly when the subgroup's containing group has the same semantic group identity as the group selected by the group-scoped exchange and the group-scoped producer's stable subgroup membership has the same semantic subgroup identity as the subgroup selected by the subgroup-scoped exchange; and
- otherwise, that group-cohort/subgroup-cohort pair is **scope-incompatible**.

The root/group and root/subgroup rules are symmetric. The narrower-scoped exchange supplies the exact established hierarchy whose selected group or subgroup is being compared; that hierarchy is used only to determine whether the root-scoped producer belongs to the selected narrower cohort. This does not bind root scope itself to a hierarchy instance, create an implicit globally current hierarchy, or permit a different same-`each` hierarchy to retarget the narrower scope. If the producers belong to distinct dynamic `each` identities, they cannot satisfy the required narrower-cohort membership and the pair is scope-incompatible.

The group-cohort/subgroup-cohort rule is symmetric. Requiring the same semantic group identity also requires the scopes to agree on the exact hierarchy-scoped group rather than merely belonging to the same dynamic `each`; equal implementation or debug group/subgroup tokens from a distinct hierarchy instance do not satisfy that requirement. Within that exact group, the additional subgroup-membership requirement proves that the group-scoped producer belongs to the subgroup selected by the subgroup-scoped exchange. The subgroup-scoped producer belongs to the selected group by construction. These mixed rules do not define a more general cross-hierarchy participant-inclusion relation or scope lattice.

The synchronization-scope relationship of the remaining mixed pairs is **not defined by this revision**. Those open pairs are unscoped/root-cohort, unscoped/group-cohort, and unscoped/subgroup-cohort. In particular, absence of a synchronization edge from those open relations is not a normative statement that a future mixed-scope interaction must be incompatible. Defining those interactions requires a semantic participant-domain relationship for unscoped exchange that this revision does not establish.

Root-, group-, or subgroup-cohort scope identity by itself establishes no sibling-iteration order, barrier, ordinary-access visibility, hierarchy establishment, progress guarantee, physical concurrency, or other synchronization relationship. The scope condition matters only when every other requirement of the explicit direct release/acquire relation below is also satisfied.

Broader atomic scope is not defined by this revision. Any broader form requires its own semantic participant-domain and scope-relationship contract before it can affect synchronization.

### Direct release/acquire synchronization

Let `R` and `A` be two normally completing atomic exchanges on the same semantic location for which the synchronization-scope relationship above is defined. `R` **synchronizes with** `A` under this revision exactly when:

- `R` is release-capable;
- `A` is acquire-capable;
- `R` and `A` are scope-compatible; and
- `R` is the immediate predecessor of `A` in that location's modification order.

Only that direct predecessor relation carries release/acquire synchronization in this revision. If another exchange occurs between an earlier release-capable exchange and an acquire-capable exchange in modification order, that earlier exchange does not synchronize with the acquire-capable exchange merely because it precedes the intervening exchange. Release-sequence semantics are not defined by this revision.

When `R` synchronizes with `A`, every semantic action sequenced before `R` in `R`'s execution context is semantically ordered before every semantic action sequenced after `A` in `A`'s execution context, when those actions belong to the applicable defined continuations.

For ordinary non-atomic accesses, this synchronization consumes the existing conflict and unordered-access rules rather than replacing them. A conflicting ordinary pair remains conflicting under the conflict predicate. When one such access is sequenced before `R` and the other is sequenced after a synchronizing `A`, the synchronization supplies semantic order between those accesses, so they are not a source-unordered conflicting pair solely with respect to that relation. Every other applicable authority, validity, effect, resource, and operation-specific contract still has to permit both accesses.

An acquire-capable exchange is not required to synchronize with a release-capable exchange in every permitted execution. Where the pair's synchronization-scope relationship is defined, whether synchronization occurs depends on the permitted modification order selected for that execution, the class of its immediate predecessor, and the defined scope compatibility of that pair.

### Deliberate synchronization boundary

This revision defines only base, release, acquire, and acquire-release exchange classes; unscoped/unscoped, root-cohort/root-cohort, group-cohort/group-cohort, subgroup-cohort/subgroup-cohort, root-cohort/group-cohort, root-cohort/subgroup-cohort, and group-cohort/subgroup-cohort synchronization-scope relationships; and the direct release-to-acquire relation above for pairs whose scope relationship is defined.

Release sequences, sequential consistency, fences, broader atomic memory scopes, the remaining mixed interoperability between unscoped exchange and root/group/subgroup cohort-scoped forms, mixed ordinary/atomic value and visibility semantics beyond the conflict/source-unordered legality relation above, the complete data-race rule involving atomics, other atomic operations, progress guarantees, and source atomic syntax or types are not defined by this revision. Their absence does not authorize behavior beyond rules already established by their canonical owners.

## Structured barrier synchronization

The cohort-scoped structured barrier defined by [Exec parallelism](parallelism.md) is an explicit synchronization mechanism for ordinary accesses among its selected participants.

The phase relation owned by that construct induces this memory-model ordering for one normally completed barrier instance: every ordinary non-atomic access by every participant in its before-barrier phase is ordered before every ordinary non-atomic access by every participant in its after-barrier phase.

An iteration outside the selected barrier cohort is a nonparticipant. An ordinary access performed by a nonparticipant receives no ordering or visibility guarantee from that barrier alone.

This relationship is semantic. It does not derive from physical arrival order, release order, worker scheduling, cache operations, backend queue behavior, or the fact that two iterations share a hierarchy group or subgroup.

The barrier does not change the ordinary conflict relation. Two overlapping ordinary accesses with at least one state-changing access still conflict. When one such participant access is in a before-barrier phase and the other participant access is in an after-barrier phase of the same normally completed barrier instance, the barrier supplies semantic order between them, so they are not a source-unordered conflicting pair under the rule above. Every other applicable authority, validity, effect, resource, and operation-specific contract still has to permit both accesses.

Conflicting ordinary accesses performed by sibling participants within the same before-barrier phase or within the same after-barrier phase remain source-unordered unless another semantic contract orders them.

Barrier identity alone does not order distinct barrier instances. Neither overlap/nesting between two selected barrier cohorts nor hierarchy membership alone supplies memory order. Ordering between ordinary accesses around distinct barriers must arise from their placement or another applicable semantic relationship in the enclosing execution.

Buffer logical coherence consumes this barrier-established participant access order through its own canonical contract; this document does not redefine Buffer state or visibility.

No host, hardware, or backend memory model is normative by default.

## Open memory-model rules

The complete cross-realization memory model, the complete data-race definition involving synchronization or atomic operations beyond the atomic-exchange, mixed ordinary/atomic conflict, and structured-barrier relations above, atomic order semantics beyond the base/release/acquire/acquire-release exchange classes, release sequences, broader atomic scope and the remaining mixed interoperability between unscoped exchange and represented structured scope forms, mixed ordinary/atomic value and visibility semantics beyond the conflict/source-unordered legality relation above, and additional synchronization relations are not defined by this revision. Their absence does not authorize additional conflicting ordinary accesses.
