# Source Safe References

Status: **provisional normative; incomplete**

This document owns the represented source-language safe-reference relation: Shared and replacement-capable exclusive reference types and values, complete-root and bounded Shared/replacement-capable binding-field safe-reference targets, bounded Shared field-relative child targets, source safe-authority compatibility, reference authority/carrier lifetime, source-validation result provenance, root formation, bounded complete-referent dereference, explicit bounded reborrow, replacement through a replacement-capable reference, parameter/result transfer consequences, external-referent structural ownership, bounded safe-reference result validity, implicit lexical lifetime validity, and source-to-Core refinement for this bounded slice.

It consumes source type identity and owned-value duplicability from [Source type foundation](types.md); function-local binding identity, scope, lookup, lifecycle, assignment mutability, structural-root lifecycle, ordinary value use, and assignment from [Source function-local bindings](local-bindings.md); structural root/path availability, consumption, replacement reset, bounded non-empty subpath installation, and remaining ownership frontiers from [Source structural ownership](structural-ownership.md); nominal record field identity and direct field accessibility for bounded Shared/replacement-capable binding-field root and Shared field-relative reborrow selection from [Source field-value access](field-access.md); function entity/parameter/result-contract callable structure from [Source callables](callables.md); direct-call argument/result evaluation, activation lifetime, lexical/activation cleanup, return, defined-fault propagation, and divergence from [Source function execution](function-execution.md); represented control-flow state composition from [Source control flow](control-flow.md); the independently owned raw-pointer/unsafe relation from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md); and represented concrete reference spellings from [Source concrete syntax](concrete-syntax.md). It does not redefine those owners.

The lower refinement target is the accepted safe-reference relation in [Core references](../core/references.md) and the parameter/result-transfer plus safe-reference result-contract relation in [Core functions and direct calls](../core/functions.md). Core reference identity, `StorageRegion`, reference-authority identity, Core liveness, and proving representation are not source-language authority.

This source relation represents Shared references and one replacement-capable exclusive class. It exposes bounded Shared and replacement-capable root formation over a binding root or one resolved binding field path, bounded Shared field-relative child reborrow through an existing safe reference, exact Shared-identity results, and one complete-referent Shared direct-child result contract. It does not expose plain Core `Exclusive`, reference-containing aggregate fields/results, replacement-capable field-relative children, direct reference-relative field/subregion value access, projected/subregion result origins, arbitrary descendant or multiple-origin result contracts, named lifetimes, non-lexical lifetime shortening, or a general source place/lvalue/address category.

## Source reference types

For every admissible referent source type `T`, the represented source type domain may contain:

- `SharedRef(T)`, concretely spelled `&T`; and
- `ExclusiveReplaceRef(T)`, concretely spelled `&mut T`.

A safe-reference source type is identified exactly by its referent source type and permission class. Two safe-reference source types are equal exactly when both dimensions are equal under the source type-equality relation from `types.md`.

Lifetime, target binding identity, dynamic authority identity, lexical source location, storage identity, physical address, ABI representation, and lower Core type identifier are not source type-identity dimensions.

`SharedRef(T)` is source-duplicable. Duplicating one Shared reference value creates another carrier for the same source Shared authority and target; it does not form a new root or child authority.

`ExclusiveReplaceRef(T)` is source-non-duplicable. Ordinary owned transfer moves its single carrier. No source Copy, implicit duplication, or compatibility alias exists for this class.

Safe-reference referent edges are semantic indirection rather than direct nominal-record containment. This slice still forbids safe-reference record fields and nested safe-reference referents.

## Referent admission

A source type `T` is **Shared-referent-admissible** exactly when all of the following hold:

1. `T` is a represented source value type under `types.md`;
2. `T` is duplicable under the source-semantic duplicability relation from `types.md`; and
3. the structural source value shape of `T` contains neither a safe-reference type nor a raw-pointer type.

This preserves the previously accepted Shared-reference referent domain.

A source type `T` is **replacement-reference-referent-admissible** exactly when all of the following hold:

1. `T` is one represented intrinsic scalar or nominal record source type under `types.md`; and
2. the structural source value shape of `T` contains neither a safe-reference type nor a raw-pointer type.

Replacement-reference referents may therefore be duplicable or non-duplicable nominal values. `SharedRef`, `ExclusiveReplaceRef`, and `RawPtr` are not admissible replacement-reference referents.

These bounded restrictions preserve the current Core transferable-referent safety boundary. They do not claim that future safe references fundamentally require the same exclusions.

## Contextual type admission

A represented `SharedRef(T)` is admitted directly only as:

- one function parameter type;
- one immutable ordinary local binding type; or
- one function result type satisfying the bounded safe-reference result contract from `callables.md` and this document.

A represented `ExclusiveReplaceRef(T)` is admitted directly only as:

- one function parameter type; or
- one immutable ordinary local binding type.

A safe-reference type is source-invalid in this slice as:

- a nominal record field type;
- the declared type of a mutable/rebindable reference local;
- the referent of another safe-reference type; or
- a function result when its permission class is `ExclusiveReplace`.

A `SharedRef(T)` result remains invalid when its callable safe-reference result contract is missing or ambiguous. `RawPtr(T)` is not an admissible safe-reference referent.

Pattern-introduced local bindings cannot acquire a safe-reference type because represented nominal record fields cannot contain safe references.

## Source reference values

Every represented source safe-reference value contains exactly the dynamic semantic facts required by this relation:

- one dynamic source reference target;
- one opaque dynamic source reference-authority identity; and
- the permission class fixed by its source type.

Neither target nor authority identity is program-observable data. They cannot be compared, converted to integers, serialized, named by source syntax, or used as physical addresses.

A source safe reference is not a raw pointer, Core `ReferenceAuthorityId`, Core `StorageRegion`, Core `LoanId`, static Core `Place`, physical address, lifetime name, module binding, or source structural-ownership path by itself.

Source safe-reference values arise only from root formation, explicit reborrow, ordinary transport of an existing valid safe-reference value, or a successful direct call whose advertised result contract summarizes an authority already created by valid callee execution. No literal, raw-pointer operation, record construction, or implementation convenience fabricates one.

## Source-validation result provenance and activation origins

Source validation tracks non-observable **reference-result provenance** for represented Shared-reference value flow where result validity can depend on identity. It also retains the exact incoming authority/target selected by a contract-bearing activation independently of the current carrier location of the designated parameter binding.

These facts are not runtime data, source type-identity dimensions, or lifetime names. They exist to distinguish exact incoming authority identity from freshly formed or freshly reborrowed authority and to validate direct-parent ancestry for the bounded derived-result contract.

Within validation of one function body:

- a Shared-reference parameter in callable slot `i` begins with provenance **ParameterOrigin(i)**;
- root Shared formation `&x` or `&x.field...` begins with **RootOrigin(x)** for the selected binding identity while the produced authority independently retains its exact structural target path;
- ordinary Shared-reference duplication preserves provenance unchanged;
- transfer into an immutable Shared-reference local preserves provenance unchanged;
- ordinary use of such a local or parameter preserves provenance unchanged;
- every explicit reborrow creates a fresh authority and therefore fresh **ReborrowOrigin** provenance distinct from the parent's provenance, whether its relative field path is empty or non-empty;
- a successful `SharedIdentity(j)` direct call produces caller-side provenance exactly equal to the caller argument value supplied to slot `j`; and
- a successful `SharedDirectChild(j)` direct call produces one fresh caller-side derived-child provenance paired with the fresh summarized child authority whose direct parent is the exact caller authority supplied to slot `j`.

At activation entry for `SharedIdentity(i)`, the incoming Shared authority/target from slot `i` is the exact activation identity-result origin.

At activation entry for `SharedDirectChild(i)`, the incoming `ExclusiveReplaceRef(T)` authority/target from slot `i` is the exact **activation direct-child parent origin**. That origin fact persists for the activation even if ordinary non-copyable carrier transport later moves the designated parameter carrier into another local, through a nested call, or out of the parameter binding. The callable contract names the incoming authority, not the parameter binding's then-current carrier slot.

Distinct parameter slots establish distinct activation origin facts even when dynamic arguments happen to alias or carry related authorities.

A caller-created Shared child passed to an identity-preserving Shared function may be returned when the callee preserves that exact incoming child authority: inside the callee it is `ParameterOrigin(i)`, while the callable summary maps the successful result back to the caller-side provenance unchanged.

A fresh direct-child result is never identity-equivalent to its parent. Its caller-side provenance remains fresh even though its exact parent/target relation is known. Subsequent `SharedIdentity` calls may preserve that child unchanged.

A faithful typed frontend MAY encode provenance and activation-origin evidence differently, but it MUST preserve exact authority identity, target, direct-parent distinctions, and callable contract selection and MUST NOT reconstruct an advertised result contract from body dataflow after callable validation.

## Safe-reference target domains

A source safe-reference target is one dynamic structural region. Root formation can establish such a region from storage owned by the current activation; reference transport can carry that same region into another activation; and explicit reborrow can derive a child region by composing the parent target with a bounded relative structural field path. None of these target relations exposes a physical address, Core place, source binding name, or implementation storage token as program data.

### Local binding structural target

A local safe-reference root target begins at one active parameter or ordinary local binding selected through the unqualified function-local lookup relation from `local-bindings.md`.

For represented root formation, the target path is either:

- the complete empty structural path of the selected binding; or
- one non-empty structural field path selected by the bounded binding-field root relation below.

For every non-empty binding-field root path, selection begins from the root binding's declared source type and resolves each concrete field selector in order. Every step requires the current type to be one nominal record, resolves exactly one declared field identity, requires the existing direct field-accessibility relation from `field-access.md`, appends that field identity to the structural path from `structural-ownership.md`, and continues from that field's exact declared source type. Unknown, wrong-category, or inaccessible steps reject the complete formation before any fresh authority/carrier is created.

The final selected field/root type, not the outer record type, is the candidate referent type. Shared root formation applies Shared-referent admission to that exact type; replacement-capable root formation applies replacement-reference-referent admission to that exact type. The containing binding storage extent remains the target extent for every selected descendant structural region.

Replacement-capable root formation additionally requires the selected binding to be one mutable ordinary local as defined below. Parameters remain eligible only for Shared root formation because represented parameters do not establish ordinary local replacement permission.

No root formation selects a pattern path independently of its root binding, producer transient, direct-call result, record-construction transient, dereference result, arbitrary temporary, grouped value, or general source expression/place/lvalue. No source qualification syntax is introduced inside a field path.

### Replacement-capable external referent structural root

Every `ExclusiveReplaceRef(T)` parameter establishes exactly one non-binding **external referent structural root** of exact root type `T` for the callee activation.

That root consumes the structural ownership relation from `structural-ownership.md`:

- it begins fully available with an empty consumed-path set at successful call entry;
- `*r` ownership Move consumes its complete root;
- `*r = value` replacement restores complete ownership with an empty consumed-path set;
- an explicit child reborrow may select the complete root or, for a Shared child, one bounded relative structural descendant path within this same root; and
- normal completion must satisfy the restoration law below.

This structural root is validation state for storage owned by a still-live suspended ancestor activation. It is not a local binding, does not introduce a lexical identifier, and does not duplicate the caller's binding structural state. A bounded Shared relative reborrow through this parameter observes path availability at the exact selected external-root path; it does not create another structural ownership domain for that child.

A `SharedRef(T)` parameter does **not** establish a second mutable external structural ownership root. Successful call entry already requires its complete target region to be fully available, and the represented Shared-reference operations available to the callee do not consume, replace, or otherwise change referent structural ownership. A structurally valid descendant selected by bounded Shared relative reborrow therefore remains ownership-available through that Shared parameter while its target extent and authority are valid. This distinction does not weaken the independent target-authority/delegation requirements below.

For a local-root or local-field reference target in the current activation, operations use that existing binding's structural ownership state instead of creating a second domain.

### Reference-relative structural target selection

Let a stored parent safe-reference value carry authority `A`, exact dynamic target region `R`, and exact referent source type `P`. A bounded relative field selector sequence resolves from `P` without re-resolving a source root binding.

For a non-empty selector sequence, every step:

1. requires the current selected source type to be one nominal record;
2. resolves exactly the declared field identity matching the selector's lexical key;
3. requires the existing direct field-accessibility relation from `field-access.md` relative to the containing source module;
4. appends that field identity to one relative structural path `p` under `structural-ownership.md`; and
5. continues from the selected field's exact declared source type.

Unknown, wrong-category, or inaccessible selector steps reject the complete reborrow before any fresh child authority/carrier is created. Qualification is not introduced inside the relative field path.

The empty relative path `[]` selects exactly `R`. A non-empty relative path `p` selects exactly the structural descendant `compose(R, p)`. Composition is semantic structural target identity: it neither discovers nor reconstructs an originating source binding, and it does not create a physical address or expose a lower Core projection identity as source data.

When `R` is backed by a current-activation local binding target at structural path `q`, selected ownership availability is the existing binding structural state at `q + p`. When `R` belongs to a replacement-capable parameter's external referent root at path `q`, selected ownership availability is that same external structural state at `q + p`. Equal or ancestor consumption rejects the selected path, a consumed descendant that makes it partially available rejects it, and a structurally disjoint consumed sibling remains compatible under `structural-ownership.md`.

For a transported Shared parameter target, the no-mutable-external-root rule above applies: successful call entry established the complete parent target fully available, and represented Shared operations cannot consume or replace it. Relative target selection still performs the exact field-resolution, accessibility, type, target-extent, and authority/delegation checks; it does not infer a mutable external structural state merely to represent availability.

## Canonical source safe-authority compatibility

This document owns one canonical **source safe-authority compatibility** relation. Direct/root operation owners consume it rather than defining parallel alias rules.

For one target structural region/path:

- a **Shared requirement** is satisfied exactly when no overlapping active source exclusive safe authority exists;
- an **Exclusive requirement** is satisfied exactly when no overlapping active source safe authority of either kind exists.

Two local-binding structural targets overlap exactly when their roots are the same binding and their structural paths are equal or one path is an ancestor of the other under `structural-ownership.md`. Distinct roots and structurally disjoint sibling paths do not overlap. Two regions within one replacement-capable external referent root overlap by the same equal-or-ancestor structural-path relation; the existing complete external target is its empty path. Transported safe-reference targets preserve their exact dynamic structural region identity, and child composition preserves the same structural overlap relation without converting it into source-name or physical-address identity.

`SharedRef` authorities are shared. `ExclusiveReplaceRef` authorities are exclusive; replacement capability is an additional operation permission, not a third alias kind.

For direct operations on an original local root or selected descendant path, every active overlapping safe-reference branch is considered, including a root replacement-capable authority whose own retained reference-relative capability has been reduced by a child. A Shared child does not legalize unrelated direct access to an overlapping original region. A field-root authority constrains exactly overlapping equal/ancestor/descendant regions and does not constrain a structurally disjoint sibling merely because both lie in one containing record. A live replacement-capable root authority continues to block direct operations requiring Shared compatibility throughout its exact target and overlapping descendants or ancestors until that authority branch actually ends.

For reference-relative operations through a parent authority `A`, retained capability is evaluated at the exact selected target region. An active Shared child branch is compatible with a Shared requirement over an overlapping selected region. An active replacement-capable child branch suspends parent capability over every selected region that overlaps that child branch. A child branch targeting a structurally disjoint sibling does not constrain the parent's capability at the selected region. Ancestor/descendant overlap is the same structural overlap relation above. Consequently a Shared field-relative child downgrades an `ExclusiveReplace` parent's retained capability only over its overlapping structural region; it does not spuriously remove replacement-capable authority over disjoint siblings. Full complete-referent capability, when another rule requires it, still fails whenever any active child branch delegates part of that complete referent in a way incompatible with that required complete capability.

The represented direct/root operations consume the compatibility classes as follows:

- non-consuming ordinary whole-binding duplicate use: Shared requirement;
- non-consuming binding-root field-value production: Shared requirement;
- non-consuming direct-root pattern production: Shared requirement;
- Shared root formation, including bounded binding-field root formation: Shared requirement;
- raw address formation: Shared requirement;
- consuming ordinary whole-binding use: Exclusive requirement;
- consuming binding-root field-value production: Exclusive requirement;
- consuming direct-root pattern production: Exclusive requirement;
- whole-binding assignment/reinitialization: Exclusive requirement;
- replacement-capable root formation, including bounded binding-field root formation: Exclusive requirement;
- raw ownership move: Exclusive requirement; and
- raw replacement: Exclusive requirement at its post-source commit point.

Source `unsafe` does not weaken this relation.

## Root Shared-reference formation

Concrete `&x` requests one complete-root Shared borrow. Concrete `&x.field...` requests one bounded Shared borrow of the exact selected binding field path.

Let `x` resolve to one active parameter or ordinary local binding with declared source type `R`. Let the zero-or-more concrete field selectors resolve under the local binding structural-target relation above to exact structural path `p` with final selected source type `T`. Let the surrounding receiving position require exact source type `SharedRef(U)`.

Formation is source-valid only when:

1. `T` and `U` are exactly equal source types;
2. `T` is Shared-referent-admissible;
3. the selected structural path `p` is fully available immediately before formation;
4. the containing target binding extent is active; and
5. the canonical Shared requirement succeeds for exactly the selected target region `p` against every overlapping active safe authority.

For the zero-selector case, `p` is the complete empty path and these rules are exactly the existing `&x` rules. For a non-empty path, a consumed equal or ancestor path rejects formation, a consumed descendant that makes `p` only partially available rejects formation, and a consumed structurally disjoint sibling does not reject formation. Shared-referent admission applies to the exact final type `T`; the containing root type `R` need not itself be Shared-referent-admissible when a selected descendant `T` is.

Successful formation creates one fresh Shared root authority and one carrier with `RootOrigin(x)`, targets exactly the selected structural path `p`, leaves target ownership/value unchanged, and performs no target read, copy, move, mutation, destruction, replacement, restoration, or normalization.

Independent Shared root formations create independent authorities when compatibility permits them, including two compatible Shared roots over overlapping regions. A field-root authority is a fresh root authority, not a child/reborrow of another reference merely because another authority targets an ancestor, descendant, or equal structural region.

## Root replacement-capable formation

Concrete `&mut x` requests one complete-root replacement-capable exclusive reference. Concrete `&mut x.field...` requests one bounded replacement-capable exclusive reference to the exact selected binding field path.

Let `x` resolve to one active ordinary local binding with declared source type `R`. Let the zero-or-more concrete field selectors resolve under the local binding structural-target relation above to exact structural path `p` with final selected source type `T`. Let the receiving position require exact source type `ExclusiveReplaceRef(U)`.

Formation is source-valid only when:

1. `T` and `U` are exactly equal source types;
2. `T` is replacement-reference-referent-admissible;
3. `x` is an ordinary local binding, not a parameter;
4. `x` is mutable under `local-bindings.md`;
5. the selected structural path `p` is fully available immediately before formation;
6. the containing target binding extent is active; and
7. the canonical Exclusive requirement succeeds for exactly the selected target region `p` against every overlapping active safe authority.

For the zero-selector case, `p` is the complete empty path and these rules are exactly the existing `&mut x` rules. For a non-empty path, a consumed equal or ancestor path rejects formation, a consumed descendant that makes `p` only partially available rejects formation, and a consumed structurally disjoint sibling does not reject formation. Replacement-reference-referent admission applies to the exact final type `T`; the containing root type `R` need not itself be replacement-reference-referent-admissible when a selected descendant `T` is.

The broader bounded non-empty subpath-installation relation from `structural-ownership.md` does not widen formation admission. Exactly consumed or partially available `p` remains invalid for root formation even though an already valid replacement-capable reference may later reinitialize or reconstruct its selected target through the separate replacement relation below. Formation always requires a currently fully available referent value.

Parameters are not replacement-capable root targets because represented parameters do not establish ordinary local replacement permission. No independent field-level mutability property is introduced: the selected root binding's existing assignment mutability supplies replacement permission for every source-valid descendant target selected here.

Successful formation creates one fresh replacement-capable exclusive root authority and one carrier targeting exactly structural path `p`, leaves target ownership/value unchanged, and performs no target read, copy, move, mutation, destruction, replacement, restoration, or normalization. A projected replacement-capable field-root authority is a fresh root authority, not a child/reborrow of another safe reference merely because another authority targets an ancestor, descendant, or equal structural region.

## Authority and carrier lifecycle

Every live source safe-reference scalar value is one **source reference carrier** for its authority identity.

A source reference authority remains active while either:

- it has at least one live carrier; or
- it has an active child authority, directly or transitively.

When an authority has no carrier and no active child, it ends. Carrierless ancestors terminate transitively when their last descendant branch ends.

Carrier consequences are:

- root formation creates one carrier and one fresh root authority;
- Shared duplication creates one additional carrier for the same authority;
- replacement-capable references are non-copyable and ordinary whole-binding use moves the existing carrier;
- initialization, parameter transfer, Return transfer, and ordinary owned-value transport transfer an existing produced carrier rather than creating an authority;
- explicit reborrow creates one fresh child authority and one child carrier without moving/copying the parent carrier;
- caller-side validation of a successful `SharedDirectChild` call summarizes the one fresh child authority already required to have been created by valid callee execution; that summary is not a second runtime reborrow;
- lexical/activation cleanup of a live reference removes that carrier; and
- cleanup of a reference value does not access its referent.

Reference locals are immutable. This slice performs no non-lexical authority shortening merely because a carrier is no longer textually used.

## Direct target consequences

While active safe authority overlaps a local binding structural region, direct operations on that region remain governed by their existing operation owners plus the canonical compatibility relation above.

A direct non-consuming duplicate/path operation may proceed only under the Shared requirement. A direct ownership-consuming path operation or whole-binding assignment may proceed only under the Exclusive requirement.

Thus an active replacement-capable root authority blocks direct access to every region overlapping its exact target even when that authority currently has only a Shared child and the parent's retained reference-relative capability is Shared on the overlap. Ending the child alone does not end the root authority; overlapping direct access becomes eligible only after the complete conflicting authority branch ends. A projected replacement-capable authority over `x.left` does not constrain direct access to disjoint `x.right`, just as a Shared field-root authority for `x.left` does not constrain a direct operation on disjoint `x.right` merely because both share binding root `x`.

No separate borrow mark is added to `structural-ownership.md`. Safe authority and structural consumed-path state are distinct validation relations.

## Ordinary safe-reference binding use

Ordinary whole-binding use of `SharedRef(T)` follows the existing duplicable-value relation:

1. the reference binding root must be fully available;
2. one additional carrier for the same target/authority is produced;
3. provenance is preserved; and
4. the stored reference binding remains owned and available.

Ordinary whole-binding use of `ExclusiveReplaceRef(T)` follows the existing non-duplicable ownership-transfer relation:

1. the reference binding root must be fully available;
2. the stored carrier is moved into the produced value without creating another carrier or authority;
3. the moved-from reference binding root becomes consumed under `structural-ownership.md`; and
4. target, authority identity, permission, and any applicable validation provenance are preserved.

Moving the carrier itself is not a target access. Active child authorities may keep the parent authority alive even when a parent carrier is temporarily absent.

## Complete-referent dereference

Concrete `*r` is one bounded complete-referent owned-value producer.

`r` resolves to one active parameter/local binding whose exact type is either `SharedRef(T)` or `ExclusiveReplaceRef(T)`. The stored reference carrier is obtained non-consumingly; dereference does not move/copy the carrier merely to select its target.

For `SharedRef(T)`, successful `*r` requires a live carrier/authority, active target extent, fully available complete target, retained Shared authority, and an exact surrounding required type `T`. It produces one duplicate owned `T`, leaves target ownership unchanged, and creates no reference carrier or authority.

For `ExclusiveReplaceRef(T)`:

- when `T` is duplicable, `*r` requires retained Shared reference-relative authority plus a fully available complete target and produces one non-consuming duplicate of `T`;
- when `T` is non-duplicable, `*r` requires retained Exclusive reference-relative authority plus a fully available complete target and ownership-moves the complete referent `T`, consuming exactly the structural region denoted by that reference while target storage and reference authority remain live.

For a reference into a current-activation local binding, the complete target structural state is the existing structural ownership state of that containing binding observed at the exact target path selected by root formation; a non-duplicable Move consumes exactly that path. For a replacement-capable parameter, the complete target is its external referent structural root and a non-duplicable Move consumes that root. `*r` always addresses the complete referent of the stored reference, even when that complete referent is one projected binding-field region selected when a root authority was formed.

No explicit source safe-reference Drop, interior assignment, field-relative dereference, or general dereference place is represented.

## Complete-referent replacement

Concrete statement `*r = Value;` is represented only when `r` has exact type `ExclusiveReplaceRef(T)`.

Replacement is source-first:

1. resolve `r` and identify its current complete referent domain without consuming the destination carrier;
2. before RHS producer consequences may commit, require the destination reference carrier to be live and its authority branch to retain full replacement-capable exclusive authority over the complete referent;
3. validate and evaluate the RHS producer under exact required type `T`;
4. if RHS evaluation faults or diverges, perform no outer referent replacement;
5. after successful RHS production, require the destination reference carrier still to be live and its authority branch still to retain full replacement-capable exclusive authority over the complete referent;
6. for a complete external-referent root or a local-binding target at empty path, select and end the then-current complete-root remaining ownership frontier; for a local-binding target at non-empty path `p`, require the canonical bounded non-empty subpath-installation admission and select and end exactly the then-current `frontier(p)`;
7. install the produced exact-`T` value into the complete referent; and
8. for a complete external-referent root or local-binding empty path, establish fresh complete ownership with an empty consumed-path set; for a local-binding target at non-empty path `p`, apply the canonical successful bounded subpath-installation transition, removing exactly consumed paths equal to or below `p` while preserving every structurally disjoint consumed path.

The pre-RHS authority requirement prevents an already active incompatible delegation from being bypassed merely because RHS evaluation might later change authority state. The post-RHS requirement revalidates the actual replacement point after all successful RHS consequences. Both checks apply to the reference's exact complete referent target.

The structural admission in step 6 is evaluated on the successful post-RHS state at the actual replacement point. For a non-empty local-binding target `p`, replacement therefore admits the canonical fully available, exactly consumed, or partially available descendant-consumed cases and rejects a state containing a consumed strict ancestor of `p`. This does not split a consumed ancestor, add a runtime moved-state check, or create a second structural ownership relation.

The RHS may itself move from the referent through the same reference when otherwise valid. The outer replacement therefore selects the applicable remaining frontier only after successful RHS evaluation. An already valid projected replacement-capable root may consequently reinitialize its exactly consumed target; ending the reference authority alone would not restore that ownership.

The destination reference carrier/authority MUST remain usable through the commit check. This slice does not permit an RHS to consume or disable that destination carrier and then commit through a snapshotted stale dereference destination.

No Shared-reference replacement or plain-Exclusive replacement source form exists. This relation remains complete-referent relative to the stored reference and does not add a concrete `*r.field... = Value;` spelling.

## Explicit bounded reborrow

This slice represents one bounded Shared reborrow family and the existing complete-referent replacement-capable reborrow:

```text
&*r
&*r.field
&*r.outer.inner
&mut *r
```

For a Shared reborrow, `r` resolves to one active parameter/local binding whose exact type is `SharedRef(P)` or `ExclusiveReplaceRef(P)`. The zero-or-more field selectors after `r` resolve under the reference-relative structural-target relation above to relative path `p` and exact final selected source type `T`. The surrounding receiving position must require exact `SharedRef(T)`, and `T` must be Shared-referent-admissible. The parent referent `P` need not itself be Shared-referent-admissible merely because selected descendant `T` is; in particular, a non-duplicable replacement-reference-admissible record may contain a selected duplicable Shared-referent-admissible field.

Before Shared child creation:

1. the parent reference binding root is fully available and owns one live carrier for parent authority `A`;
2. the parent target extent is active;
3. the relative selector path is structurally valid and every selected field satisfies the canonical direct accessibility relation;
4. the exact selected target `compose(A.target, p)` is fully available under the applicable local or replacement-capable external structural ownership state, or under the transported-Shared availability invariant described above; and
5. `A` retains Shared reference-relative capability over exactly that selected target after accounting for structurally overlapping child branches.

For `p = []`, these requirements are exactly the previously represented complete-referent `&*r` relation.

`&mut *r` remains a complete-referent-only request for one fresh `ExclusiveReplaceRef(T)` child. The parent must itself be replacement-capable, its complete referent must satisfy the existing full-availability and replacement-reference-admission requirements, and it must retain full replacement-capable exclusive authority over that complete referent. No field selector is represented after the parent identifier in this branch.

Successful reborrow:

1. creates exactly one fresh child authority;
2. records the existing parent authority `A` as its direct parent;
3. for Shared reborrow, targets exactly `compose(A.target, p)` with Shared permission; for `&mut *r`, targets exactly the unchanged complete parent target with `ExclusiveReplace` permission;
4. creates exactly one child carrier without moving or copying the parent carrier;
5. produces fresh `ReborrowOrigin` provenance; and
6. delegates authority only over the child's exact structural target until that child branch ends.

Permission never strengthens. A Shared parent cannot produce `&mut *r`.

Delegation is target-relative. An overlapping replacement-capable child suspends parent reference-relative access on that overlap. An overlapping Shared child leaves Shared parent capability on the overlap. Structurally disjoint child branches do not constrain parent access or another child derivation on a disjoint sibling. Parent capability over a delegated region is restored only after the applicable child branch ends.

Reborrow performs no target read, duplicate, move, mutation, destruction, replacement, restoration, or structural ownership normalization. It changes no referent structural ownership state.

Stored child locals are permitted only as immutable reference locals. This slice defines no replacement-capable field/subregion child, direct field-relative dereference/value access, plain Exclusive child, implicit call-site reborrow, or reference field.

## Implicit lexical lifetime validity

This slice introduces no source lifetime identifier, lifetime parameter, explicit outlives clause, lifetime type-identity dimension, lifetime annotation syntax, or non-lexical shortening.

Reference validity follows target extent, authority/carrier lifecycle, lexical carrier extents, explicit child derivation, and the advertised safe-reference result contract.

### Reference locals

Every safe-reference ordinary local is immutable. Its initializer is evaluated before the local enters scope.

A local may initialize from root formation, ordinary transport of an existing reference, explicit reborrow, or a valid Shared contract-bearing direct-call result as applicable to its exact type.

Reverse lexical cleanup and activation cleanup guarantee that a non-escaping local reference ends before an earlier same-scope or ancestor local target extent. A binding-field root authority, whether Shared or replacement-capable, uses the containing root binding's storage extent; it creates no independently longer-lived field extent. A Shared field-relative child likewise uses the continuing parent target's storage extent and creates no separate field lifetime. Replacement-capable child locals likewise end before their parent/target extent under the represented lexical rules.

A Shared-reference local may be returned only when its exact authority/provenance relation satisfies the enclosing callable's advertised safe-reference result contract. A replacement-capable reference is never result-admissible.

### Parameters

A Shared-reference parameter receives one valid Shared carrier targeting storage in a still-live suspended ancestor activation. It remains governed by its exact authority and may be explicitly reborrowed where allowed. Its target may already be one projected structural field region because parameter transfer preserves the exact target of the caller-produced Shared value. A bounded Shared relative reborrow composes its selector path from that exact transported target; it does not rediscover the caller's source root.

A replacement-capable parameter receives one moved replacement-capable carrier plus one external referent structural root that begins fully available. Its transported target may likewise be one caller-selected projected binding-field region; the callee treats that exact transported region as the complete referent represented by its external structural root rather than rediscovering or widening the caller's source binding path. The target extent remains live while the caller activation is suspended. Bounded Shared relative reborrow through such a parameter selects one path within that existing external structural root without creating another ownership domain.

Parameter reference bindings are immutable as bindings. Their permission class governs referent operations independently of parameter binding mutability.

## Direct-call transfer and call-entry authority

Safe-reference parameters remain ordinary owned parameter values. There is no borrowed-call pass mode and no implicit reborrow at a call boundary.

Arguments are evaluated left-to-right and successful produced values are held by `function-execution.md`. After all argument production succeeds and before callee entry, every held safe-reference argument must satisfy both:

1. its complete target/referent structural root is fully available; and
2. its authority retains the complete capability promised by its exact source reference type.

Here "complete target/referent" means the complete structural region denoted by that safe-reference authority. A Shared or replacement-capable field-root reference, or a Shared field-relative child value, therefore carries its selected structural region as its complete referent; call transfer does not widen that target to an ancestor region.

A held `SharedRef(T)` requires full Shared capability. A held `ExclusiveReplaceRef(T)` requires full replacement-capable exclusive capability. A parent with an active child that reduces or suspends the required complete capability cannot satisfy call entry.

Ordinary use of an existing Shared binding duplicates a carrier. Ordinary use of an existing replacement-capable binding moves its carrier. A caller that wants to retain a replacement-capable parent across a nested call must pass an explicit child `&mut *parent`; a Shared child, including a bounded field-relative child when the parameter type matches its selected field, is used when only Shared permission is required.

At successful call entry, every replacement-capable parameter establishes its external referent structural root fully available. The callee may move its complete referent and later restore it through replacement.

On normal callee continuation, every transferred replacement-capable external referent MUST be fully available before activation cleanup. A valid safe-reference result carrier is preserved according to the result contract below before normal call completion can end any still-live carrier for its designated origin authority. The origin authority may already be carrierless when earlier ordinary transport or nested-call cleanup ended that carrier. On defined fault there is no normal restoration obligation; ordinary fault cleanup applies to carriers/locals and produces no result. On divergence no synthetic restoration or cleanup occurs and the caller remains suspended.

Nested and recursive calls repeat the same relation.

## Normal-completion restoration law

For every incoming `ExclusiveReplaceRef(T)` parameter, every explicit normal Return and every normal no-result fallthrough MUST, after result-value effects and before activation cleanup, leave that parameter's external referent structural root fully available.

Failure is source-invalid normal completion. No implicit edge repair, replacement, reset, or cleanup operation is inserted to satisfy this law.

Defined fault and divergence have no normal restoration obligation.

## Safe-reference result validity

Only `SharedRef(T)` has a represented reference result form. Its callable contract is exactly the `SharedIdentity(i)` or `SharedDirectChild(i)` descriptor established by `callables.md` before body validation.

### Shared identity result

Let the contract be `SharedIdentity(i)`.

Every source-valid normal result-bearing Return MUST produce one `SharedRef(T)` value satisfying both:

1. source-validation provenance is exactly the body-local `ParameterOrigin(i)` established for the incoming Shared value in slot `i`; and
2. its carrier names the exact same dynamic target and exact same Shared-authority identity established by that incoming value.

Identity-preserving transport may use ordinary Shared duplication, immutable locals, nested identity-preserving calls, or recursion. The exact incoming target may be a projected field region formed by the caller, including a caller-created field-relative child; identity preservation keeps that target unchanged.

Fresh root formation, including fresh field-root formation, and every fresh reborrow, including bounded field-relative reborrow, create a new authority identity and therefore cannot satisfy an identity contract merely because they reach the same or an overlapping target or descend from the designated authority. Another parameter slot likewise cannot satisfy slot `i` merely because one dynamic caller aliases them.

A caller-created Shared child may round-trip through a callee with `SharedIdentity(i)` when the callee preserves that exact incoming child authority. Caller-side provenance after return is exactly the argument provenance supplied to slot `i`.

### Shared direct-child result

Let the contract be `SharedDirectChild(i)`. At successful activation entry, let `A` be the exact incoming replacement-capable authority supplied through slot `i`, and let `R` be its exact complete target/referent domain. `A` and `R` are activation result-origin facts even if the parameter carrier is later moved away from its original binding.

Every source-valid normal result-bearing Return MUST produce one `SharedRef(T)` value whose authority `C` satisfies all of the following after result production and before activation cleanup:

1. `C` is active and the produced value owns one live carrier for `C`;
2. `C` has exact Shared permission and retains complete Shared reference-relative capability over its complete target;
3. `C` targets exactly `R`, with no structural projection/subregion difference;
4. `C` has direct parent exactly `A`; and
5. the ordinary normal-completion restoration law succeeds for every incoming replacement-capable external referent, including the designated origin referent.

The result may be produced directly by zero-selector `&*r`, transported through immutable Shared locals/duplication, forwarded through `SharedIdentity`, or produced by a nested `SharedDirectChild` call whose summarized child still has direct parent `A`.

Moving the `ExclusiveReplaceRef(T)` origin carrier through an immutable local or into a nested call does not change the activation origin authority `A`. A valid result is therefore defined by authority ancestry, not by whether the original parameter binding still stores a carrier.

The following do not satisfy `SharedDirectChild(i)`:

- a fresh Shared root, including a Shared field root;
- a Shared authority belonging to another parameter/root;
- a child whose direct parent is not `A` even when its target equals `R`;
- a grandchild or deeper descendant of `A`;
- a non-empty field-relative reborrow whose target is a projected/subregion descendant of `R`, even when its direct parent is `A`; or
- any replacement-capable result permission.

The exact complete-target requirement for `SharedDirectChild` remains normative. Bounded Shared field-relative reborrow therefore does not add a projected result contract: only its zero-selector `&*r` case can satisfy the existing direct-child target requirement when every other requirement holds.

### Caller-side result summary

A successful result-bearing direct call validates from the callee's advertised callable contract without expanding the callee body.

For `SharedIdentity(i)`, caller-side result authority/provenance is exactly the authority/provenance carried by the successful argument supplied to slot `i`, preserving existing identity behavior and any projected Shared target unchanged.

For `SharedDirectChild(i)`, let caller authority `A` and target `R` be carried by the successful held `ExclusiveReplaceRef(T)` argument supplied to slot `i`. Independent validation of the successful call first accounts for the one callee-preserved Shared child authority `C` and direct edge `A -> C` before applying ordinary end-of-call carrier consequences. `C` has:

- target exactly `R`;
- parent exactly `A`;
- one surviving result carrier;
- fresh derived-child provenance; and
- no replacement capability.

Before control resumes in the caller, ordinary callee execution and cleanup have ended the transferred parent carrier: if that carrier is still owned by a callee binding at activation cleanup, cleanup releases it there; it may instead have been moved and ended earlier by ordinary transport or nested-call completion. The normal caller continuation therefore exposes the already-preserved result child `C`; it does not create `C` after parent-carrier release. `A` remains active carrierlessly while `C` or any descendant of `C` remains active. Every pre-existing ancestor of `A` remains unchanged. When the final descendant branch ends, ordinary authority lifecycle recursively ends any eligible carrierless ancestors.

This caller-side validation summary denotes the one callee-created child authority required by valid execution. It does not add another runtime reborrow, duplicate the result carrier, detach/re-root the child, or recreate a parent carrier.

Passing an already-derived Shared child through `SharedIdentity` preserves it unchanged. If a replacement-capable child authority `B` of an earlier authority `A` is instead supplied as the origin of another `SharedDirectChild` call, the resulting Shared child has direct parent `B`; it is therefore a grandchild relative to `A` and cannot satisfy an outer direct-child contract naming `A`.

On defined fault, no safe-reference result authority/carrier/provenance is produced. On divergence, no safe-reference result authority/carrier/provenance, cleanup, restoration, or normal continuation state is synthesized.

Replacement-capable results, projected/subregion result origins, arbitrary descendant contracts, multiple origins, and reference-containing aggregate results remain invalid.

## Control-flow consequences

Reference authority parent/child state, including each exact structural target region for root and field-relative child authorities, is tracked sequentially by source validation according to carrier ownership, call summaries, result escape, and lexical cleanup. The represented relation compares exact continuing semantic state; it does not introduce a generic authority-graph join, lattice, fixed point, or non-lexical lifetime inference. Persistent returned children remain ordinary authority/carrier state governed by the same exact parent/target/lifecycle rules.

External referent structural roots are definite structural ownership state. `control-flow.md` consumes them alongside binding structural state:

- when two conditional outcomes both continue normally, each external referent root must have exactly equal structural ownership state on both outcomes;
- when exactly one outcome continues normally, that outcome's exact external referent state continues;
- for `while`, let `H` be the complete enclosing state before condition validation and `C` the state after successful condition validation; a normal backedge and `continue` must restore the applicable external referent state exactly to `H`, `break` must carry the exact `C` state required by the loop-exit target, and the false/post-loop state is `C`;
- no implicit repair/reset occurs at an edge.

The normal-completion restoration law remains an additional terminal requirement after represented control-flow composition.

## Source-to-Core refinement

A faithful lowering MUST preserve these semantic facts:

- `SharedRef(T)` maps to the canonical Core safe-reference type with permission `Shared` and exact lowered referent `T`;
- `ExclusiveReplaceRef(T)` maps to the canonical Core safe-reference type with permission `ExclusiveReplace` and exact lowered referent `T`;
- root `&x` maps to Core Shared root-reference formation from the direct Core storage for `x`;
- bounded Shared root `&x.field...` maps to that same Core Shared root-reference operation with one direct place rooted at the lowered storage for `x` plus the exact resolved structural field-projection sequence;
- root `&mut x` maps to Core `ExclusiveReplace` root-reference formation for the source-valid mutable ordinary local complete-root target, and bounded `&mut x.field...` maps to that same Core operation with the exact resolved structural field-projection sequence on the direct local place;
- Shared source duplication maps to ordinary Core Copy of the Shared reference value;
- replacement-capable reference transport maps to ordinary Core Move/owned transfer and never to Copy;
- source `*r` through Shared or duplicable replacement-capable referents maps to Core reference-relative Copy of the complete referent;
- source `*r` through a non-duplicable replacement-capable referent maps to Core reference-relative Move of the complete referent;
- source `*r = value` maps to Core reference-relative ordinary Assign through `ExclusiveReplace`, preserving source-first evaluation and the source destination-carrier validity boundary;
- zero-selector `&*r` maps to the existing Core Shared complete-referent reborrow; bounded `&*r.field...` maps to the same Core Shared reborrow with the exact resolved field projections appended to its `ReferenceAccess`; and `&mut *r` maps to the existing Core ExclusiveReplace complete-referent reborrow;
- safe-reference parameters map to ordinary Core parameter slots of the corresponding exact safe-reference type;
- each replacement-capable parameter maps to the Core external-referent validation/postcondition relation;
- a source callable with no safe-reference result contract maps to Core `SafeReferenceResultContract::None`;
- `SharedIdentity(i)` maps to Core `SafeReferenceResultContract::SharedIdentity { origin: i }`;
- `SharedDirectChild(i)` maps to Core `SafeReferenceResultContract::SharedDirectChild { origin: i }`; and
- lexical/activation cleanup maps to ordinary Core carrier-aware cleanup.

A bounded Shared or replacement-capable binding-field root path maps to Core structural projections on the direct root place without producing an intermediate field value. A bounded Shared field-relative reborrow path maps to Core structural projections on the existing `ReferenceAccess` for the stored parent carrier. Zero relative selectors preserve the existing zero-projection reborrow. A faithful lowering does not reconstruct an originating root place from parent provenance and does not insert a synthetic target load/copy/move, root borrow, parent carrier Copy/Move, raw-pointer/address operation, authority detachment, or result-contract operation merely to realize either projected target.

Source result provenance, activation result-origin authority facts, structural target paths, and external referent structural state are validation/refinement evidence. They need not become source-observable runtime objects, but a frontend/lowerer MUST retain enough accepted semantic information to validate exact target overlap, target-relative delegation, identity/direct-parent result relations, call entry, control flow, caller summaries, and normal restoration without reconstructing those rules from host-language behavior or lower implementation convenience.

The following remain source authority and MUST NOT be reconstructed from lower representation:

- source binding identity and lookup;
- source referent admission/contextual restrictions;
- source Shared/replacement-capable binding-field path resolution/accessibility for root selection, Shared field-relative reborrow selection, and complete-referent-only `&mut *r` eligibility;
- source safe-authority compatibility for direct operations and target-relative delegation through parent references;
- result provenance, activation result-origin authority, and advertised safe-reference result-contract selection;
- lexical lifetime validity;
- external referent structural ownership state; and
- source type/accessibility validity.

## Raw-pointer and unsafe interaction

Raw-pointer values, raw address formation, raw ownership move/replacement, lexical unsafe admission, pointer-origin provenance, and their source-to-Core refinement are owned by `raw-pointers-unsafe.md`.

Their safe-reference interaction is exactly:

- a raw pointer is not a safe reference and carries no safe authority;
- safe-reference and raw-pointer types are excluded from each other's represented referent/pointee structural domains as specified by their owners;
- raw address formation requires the canonical Shared compatibility requirement and therefore rejects an active overlapping replacement-capable exclusive safe authority while remaining compatible with Shared authority;
- raw ownership move and raw replacement require the canonical Exclusive compatibility requirement and therefore reject every active overlapping safe authority;
- lexical `unsafe` never weakens, ends, or bypasses safe authority; and
- no reference-to-raw or raw-to-reference conversion is defined.

The existing raw source forms remain complete-root-only. This bounded structural-reference delivery does not add `raw &x.field...`, a reference-relative raw address, or another raw-pointer field-target syntax.

## Explicit exclusions

This revision does not define:

- a source form for plain Core `Exclusive`;
- safe-reference record fields or reference-containing aggregates;
- mutable/rebindable reference locals;
- nested reference referents;
- replacement-capable field/subregion reborrow through an existing reference;
- direct reference-relative field/subregion value access or dereference such as `*r.field`;
- producer/transient reference targets or a general source place/lvalue/postfix category;
- explicit safe-reference Drop or InteriorAssign source forms;
- replacement-capable function results;
- projected/subregion, arbitrary-descendant, multiple-origin, or explicit-selector Shared result contracts;
- named lifetimes, lifetime parameters, explicit outlives constraints, or non-lexical shortening;
- implicit call-site reborrow or a borrowed-call pass mode;
- pointer/reference conversions;
- unsafe callable contracts;
- closures/captures, generics/traits/coherence, const/static storage, async/tasks, ABI/layout/FFI/linkage, or package behavior.

The bounded Shared and replacement-capable complete-root/binding-field-root formation, bounded Shared complete/field-relative child reborrow, exact-identity result, complete-referent direct-child Shared-result, and complete-referent replacement-capable reborrow relations above are the only represented safe-reference forms. The absence of broader relations does not permit them to be inferred from Core, host-language behavior, another language, parser convenience, or test expectations.
