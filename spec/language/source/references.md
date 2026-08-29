# Source Shared References

Status: **provisional normative; incomplete**

This document owns the first represented source-language safe-reference relation: Shared reference type/value semantics, whole-binding root borrow targets, Shared reference authority and carrier lifetime, root Shared-borrow formation, bounded Shared dereference/copy production, implicit lexical lifetime validity, Shared-reference parameter transfer consequences, and the source-to-Core refinement obligations of this bounded slice.

It consumes source type identity and owned-value duplicability from [Source type foundation](types.md); function-local binding identity, scope, lookup, lifecycle, assignment mutability, structural-root lifecycle, and assignment from [Source function-local bindings](local-bindings.md); structural root availability from [Source structural ownership](structural-ownership.md); function entity/parameter structure from [Source callables](callables.md); direct-call argument evaluation, activation lifetime, lexical/activation cleanup, return, defined-fault propagation, and divergence from [Source function execution](function-execution.md); and represented concrete reference spellings from [Source concrete syntax](concrete-syntax.md). It does not redefine those owners.

The lower refinement target is the accepted Shared safe-reference relation in [Core references](../core/references.md) and the parameter-transfer relation in [Core functions and direct calls](../core/functions.md). Core reference identity, `StorageRegion`, reference-authority identity, Core liveness, and proving representation are not source-language authority.

This first source slice is intentionally Shared-only. It does not expose Core `Exclusive` or `ExclusiveReplace`, source reborrow, source reference mutation, raw pointers, source `unsafe`, reference results, named lifetimes, or a general source place/lvalue/address category.

## Shared reference source type

For every first-slice admissible referent source type `T`, the represented source type domain contains one **Shared reference type** `SharedRef(T)`.

Its concrete spelling is `&T` under `concrete-syntax.md`.

Two Shared reference source types are equal exactly when their referent source types are equal under `types.md`.

The referent type is therefore a semantic type-identity dimension. Lifetime, target binding identity, dynamic authority identity, lexical source location, storage identity, physical address, ABI representation, and lower Core type identifier are not source type-identity dimensions in this slice.

A Shared reference type is source-duplicable. Duplicating one Shared reference value creates another source reference carrier for the same source Shared authority and target; it does not form a second root borrow or a child authority.

The `SharedRef(T)` referent edge is semantic indirection rather than direct structural record containment. This first slice nevertheless forbids Shared reference record fields and nested Shared reference referents, so it does not yet alter the acyclic represented nominal-record containment relation from `types.md`.

## Referent admission

A source type `T` is **first-slice Shared-referent-admissible** exactly when all of the following hold:

1. `T` is a represented source value type under `types.md`;
2. `T` is duplicable under the source-semantic duplicability relation from `types.md`; and
3. the structural source value shape of `T` contains no Shared reference type.

Because this source revision defines no raw-pointer source type, no source raw-pointer clause is required by this predicate. The lower refinement of every admitted referent MUST still satisfy the current Core reference-parameter referent-safety boundary.

A Shared reference type is represented in this slice only when its referent is first-slice Shared-referent-admissible.

This duplicable-referent restriction is a bounded source-language admission rule, not a claim that safe Shared references fundamentally require duplicable referents. Later source revisions may widen the source operation set and referent domain only under an accepted owner.

## Contextual type admission

A represented `SharedRef(T)` is admitted directly only in these source declaration contexts:

- one function parameter type; or
- one immutable ordinary local binding type.

It is source-invalid in this first slice as:

- a function result type;
- a nominal record field type;
- the declared type of a mutable ordinary local binding; or
- the referent of another Shared reference type.

Pattern-introduced local bindings cannot acquire a Shared reference type because first-slice nominal record fields cannot have Shared reference type.

These are source contextual-admission restrictions. They do not redefine Core type legality and do not imply that later source revisions must preserve the same restricted contexts.

## Source reference value

A represented source Shared reference value contains exactly the semantic facts required by this source relation:

- one dynamic **source borrow target** selected as described below; and
- one opaque dynamic **source Shared-authority identity** authorizing shared access to that target.

Its source type separately fixes the exact referent source type.

Neither target nor authority identity is program-observable data. They cannot be compared, converted to integers, serialized, named by source syntax, or used as physical addresses.

A source Shared reference is not a raw pointer, Core `ReferenceAuthorityId`, Core `StorageRegion`, Core `LoanId`, static Core `Place`, physical address, lifetime name, module binding, or source structural-ownership path by itself.

Source reference values arise only from root Shared-borrow formation or source duplication/transport of an existing valid Shared reference value. No literal or record construction fabricates one.

## Borrowable binding root

The first source slice defines exactly one borrow-target category: a **borrowable binding root**.

A borrowable binding root is one active parameter binding or ordinary local binding in the current source function activation, selected through the unqualified function-local lookup relation from `local-bindings.md`.

The target is the complete empty structural path of that binding root. This first slice does not admit:

- a non-empty field path;
- a pattern path independently of its root binding;
- a producer-backed transient;
- a direct-call result;
- a record-construction transient;
- a field-receiver transient;
- a pattern-scrutinee transient;
- a dereference result;
- an arbitrary temporary; or
- a general source expression/place/lvalue

as a root borrow target.

The dynamic target identity is the selected binding instance in the current source function activation. This source-semantic identity exists only to define the safe reference/lifetime relation. It does not expose a physical storage location or require the source binding to have a stable physical address.

A later source owner may introduce structural borrow targets or another source place relation only explicitly. Existing structural source paths from `structural-ownership.md` do not silently become addressable places because this root relation exists.

## Root Shared-borrow formation

The concrete `&x` form from `concrete-syntax.md` requests one root Shared borrow.

Let `x` resolve to one active parameter/local binding whose declared source type is `T`, and let the surrounding receiving position require exact source type `SharedRef(U)`.

Formation is source-valid only when:

1. `T` and `U` are exactly equal source types;
2. `T` is first-slice Shared-referent-admissible;
3. the complete structural root of `x` is fully available under `structural-ownership.md` immediately before formation;
4. the dynamic target binding extent is active; and
5. Shared alias admission succeeds against every active source reference authority targeting that same binding root.

Because this first slice contains only Shared source authorities, overlapping Shared authorities are mutually compatible. Formation therefore fails no alias check merely because another Shared root authority for the same target is active.

Successful formation:

1. creates one fresh source Shared-authority identity over the complete dynamic binding root;
2. creates one source reference carrier naming that authority;
3. produces one owned Shared reference value carrying that target/authority relation; and
4. leaves the target binding's structural ownership state and semantic value unchanged.

Formation does not duplicate, consume, move, mutate, replace, destroy, or otherwise read out the target value.

## Shared authority and reference carriers

Every live source Shared reference scalar value is one **source reference carrier** for its Shared-authority identity.

A source Shared authority remains active exactly while at least one live source carrier names it. This first slice has no source child/reborrow authority relation, so no child-only extension rule is required.

Carrier consequences are:

- root formation creates one carrier;
- source duplication of a Shared reference creates one additional carrier naming the same target and authority;
- transfer of an already produced reference value into a receiving local/parameter transfers that produced carrier and does not create another authority;
- lexical/activation cleanup of a live Shared reference value removes that carrier; and
- the Shared authority ends when its final carrier ends.

This is source-semantic alias/lifetime bookkeeping. It does not require physical reference counting, garbage collection, hidden heap allocation, runtime counters, or a custom destructor.

Independent `&x` root formations create independent Shared authorities even when they target the same binding root. Those authorities may coexist because the first slice admits Shared/Shared aliasing.

## Consequences for the borrowed target

While one or more source Shared authorities target binding root `x`, source operations on `x` remain governed by their existing owners plus this Shared-alias restriction.

The following remain permitted when otherwise source-valid:

- non-consuming ordinary whole-binding duplicate use;
- non-consuming binding-root field-value production; and
- non-consuming direct-root pattern binding production.

Whole-binding assignment or reinitialization of `x` is source-invalid while any Shared authority for `x` remains active.

No first-slice source field assignment, partial-field reinitialization, interior mutation, reference assignment, ownership-moving dereference, or explicit drop operation exists.

The first-slice referent restriction is significant here. If `T` is first-slice Shared-referent-admissible, then `T` is source-duplicable. For a positively duplicable nominal record, every structurally contained field source type is likewise duplicable under `types.md`. Consequently the currently represented ordinary value, field-value, and record-pattern uses reachable within a first-slice borrowed target do not consume a structural path merely because they produce a value.

This reference slice therefore adds no second consumed-path set, borrow mark inside `structural-ownership.md`, control-flow union/intersection, or source alias-state join. Shared authority is a distinct dynamic source validity relation whose lexical lifetime is established below.

## Shared reference duplication and ordinary use

Because `SharedRef(T)` is source-duplicable, ordinary whole-binding use of one Shared-reference parameter/local binding follows the existing duplicable-value relation from `local-bindings.md`.

Successful use:

1. requires the reference binding's complete structural root to be fully available;
2. produces another owned Shared reference value with the same target and authority identity;
3. creates one additional carrier for that authority; and
4. leaves the original reference binding owned and available.

An ordinary source use of a first-slice Shared reference therefore never moves the original carrier.

The produced carrier is then transferred into the applicable receiving position under `function-execution.md`.

## Shared dereference/copy producer

The concrete `*r` form from `concrete-syntax.md` is one bounded source owned-value producer.

`r` resolves through the existing unqualified function-local lookup relation and MUST denote one active parameter/local binding of exact source type `SharedRef(T)`.

The operation obtains the stored Shared reference value non-consumingly. It does not duplicate or consume the reference carrier merely to select its target.

The dereference producer is source-valid only when:

- the source reference carrier is live;
- its source Shared authority remains active;
- its target binding extent remains active; and
- the surrounding receiving position accepts exact source type `T`.

`T` is already duplicable by Shared-referent admission.

Successful `*r`:

1. produces one duplicate owned source value of exact type `T` through the existing source duplicability capability;
2. leaves the referenced target semantic value/ownership state unchanged;
3. leaves the stored reference carrier unchanged; and
4. creates no new source reference carrier or authority.

This is a duplicate-value dereference only. The first slice defines no source dereference Move, Drop, Assign, InteriorAssign, structural field-relative reference access, or reborrow.

## Implicit lexical lifetime validity

This slice introduces no source lifetime identifier, lifetime parameter, explicit outlives clause, lifetime type-identity dimension, or lifetime annotation syntax.

Reference validity instead follows the source target/authority relation plus bounded lexical carrier extents.

### Reference locals

A Shared-reference ordinary local is immutable in this slice.

Its initializer is evaluated before the local enters scope under `local-bindings.md` and `function-execution.md`. Therefore a direct target selected by `&x` is already active before the new reference binding begins.

A reference local may also initialize from ordinary duplication of an already active Shared-reference parameter/local binding. Because the source reference is duplicable, that use does not end the source carrier.

The accepted lexical-scope/declaration/cleanup rules guarantee the new local cannot outlive the target/source relation from which it was created:

- a local declared later in the same continuing lexical scope is cleaned before an earlier local;
- a local in a descendant lexical scope is cleaned before its ancestor scope ends; and
- ordinary locals are cleaned before parameter binding ownership ends at activation termination.

Reference locals cannot be rebound, stored in record fields, nested inside another reference, or returned in this slice. Therefore no accepted operation can extend a stored reference local beyond the target/source extent already valid when initialization succeeds.

A stored Shared authority remains active until lexical cleanup of its last carrier. This slice does **not** infer an earlier authority end merely because a reference binding is no longer textually used. Non-lexical authority shortening is deliberately absent.

### Temporary root borrow transferred to a call

A root `&x` produced directly as a direct-call argument creates one transient reference carrier during argument evaluation.

That produced carrier remains live while the direct-call relation holds the successful argument value and is transferred into the corresponding callee parameter if call admission succeeds.

The target belongs to the still-live caller activation. Caller suspension does not end that binding extent.

- on normal callee return, callee cleanup ends its remaining parameter/local carriers before the caller resumes;
- on defined fault, source activation cleanup proceeds from the faulting callee outward and ends reference carriers before an applicable target binding extent may end;
- on divergence, the caller remains suspended, so its target storage/binding extent and every outstanding Shared authority/carrier remain live;
- nested or recursive calls repeat the same relation.

No named lifetime is needed to express this bounded call relation.

### Shared-reference parameters

A Shared-reference parameter receives one valid carrier targeting storage in a still-live suspended ancestor activation.

That external target extent is valid for the complete callee activation under the direct-call relation. Parameter bindings are immutable. Any local Shared-reference duplicate established from the parameter has an extent contained within the same activation and is cleaned before the parameter ends.

The first-slice source operation set cannot move, drop, assign, or interior-mutate the external referent through the Shared reference. Therefore every normally returning source-valid callee leaves that external referent fully owned/live, satisfying the lower Core normal-return referent requirement by construction.

## Direct-call transfer

Shared-reference parameters remain ordinary source parameter slots under `callables.md`. There is no source borrowed-parameter pass mode.

Argument count/order, exact type matching, left-to-right argument evaluation, held successful argument values, activation creation, parameter initialization, normal result handling, fault propagation, and divergence remain owned by `function-execution.md`.

Reference-specific consequences are:

- ordinary use of an existing Shared-reference binding duplicates the source reference and creates a produced carrier while retaining the caller binding carrier;
- direct `&x` creates a new root Shared authority/carrier and produces that carrier;
- the successfully produced carrier is transferred into the parameter binding without creating another root authority;
- source reference parameter types refine only to Core parameter-transfer-safe Shared reference types;
- reference-containing source results are not represented.

No source reborrow is implied at a call boundary.

## Source-to-Core refinement

A faithful lowering of this represented source slice MUST preserve these semantic facts:

- source `SharedRef(T)` maps to the canonical Core safe-reference type whose referent is the faithful Core lowering of `T` and whose permission is `Shared`;
- root source `&x` maps to Core Shared root-reference formation from the direct Core storage representing binding `x`;
- source duplication of a Shared reference maps to ordinary Core `Copy` of the stored Shared-reference value and therefore to the existing Core carrier-duplication consequence;
- source `*r` maps to Core reference-relative `Copy` of the complete referent region;
- a source Shared-reference parameter maps to one ordinary Core parameter slot of the corresponding Core Shared-reference type; and
- source reference local/parameter cleanup maps to ordinary Core stored-value cleanup whose reference-carrier consequences are already owned by Core references.

This mapping creates no new Core semantics and MUST remain inside the current Core parameter-transfer-safe boundary.

The following facts remain source authority and MUST NOT be reconstructed from lower representation:

- source binding identity and lookup;
- borrowable-binding-root eligibility;
- source referent duplicability/reference-free admission;
- contextual restriction to parameter/immutable-local types;
- lexical source lifetime validity;
- source assignment prohibition while Shared authority is active; and
- source type/accessibility validity.

A frontend/lowerer may choose HIR/Core identifiers and other implementation structures freely only when they preserve these source facts. Core `StorageRegion`, reference-authority identifiers, local numbering, path liveness, or machine behavior do not retroactively define source semantics.

## Raw-pointer and unsafe separation

Every operation represented here is safe by construction and refines only to accepted safe-reference Core operations.

This document defines no:

- raw-pointer source type or value;
- address exposure or numeric address;
- reference-to-raw conversion;
- pointer arithmetic, cast, null, or provenance manipulation;
- source `unsafe` lexical/callable form;
- unsafe-operation admission;
- safe-public-contract wrapper validation;
- ABI/layout/FFI/linkage relation;
- representation validity, address stability, relocation, or pinning rule.

Those relations remain independently deferred until an accepted concrete consumer requires them.

## Explicit first-slice exclusions

This revision does not define:

- source `Exclusive` or `ExclusiveReplace` reference types;
- `&mut` or another mutable/exclusive source reference spelling;
- source reborrow or child reference authority;
- Shared references to non-duplicable referents;
- reference-containing nominal record fields/aggregates;
- recursive nominal types through reference fields;
- mutable/rebindable reference locals;
- nested Shared reference referents;
- field/path borrow targets;
- reference-relative field access;
- reference Move, Drop, Assign, or InteriorAssign;
- reference-containing function results or callable borrow-origin/result contracts;
- lifetime names, parameters, explicit outlives constraints, or non-lexical shortening;
- closures/captures, generics/traits/coherence, const/static storage, async/tasks, or package behavior.

The absence of those relations does not infer them from Core or another language. Each requires its own accepted source owner or explicit extension of this owner.