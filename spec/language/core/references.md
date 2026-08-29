# Core References

Status: **provisional normative; incomplete**

This document owns the currently represented Core semantics for first-class safe-reference types and values, reference permission classes, reference-backed alias authority, reference-carrier lifetime, root reference formation, reference reborrowing, reference-relative storage access, cross-activation safe-reference value-transport consequences, and the validity requirements that keep represented safe references non-dangling.

It consumes structural storage regions, storage extent, stored-value lifetime, initialization state, ordinary replacement permission, interior mutability, destruction, and cleanup from [Core value and storage semantics](value-storage.md); structural overlap and the shared/exclusive alias-authority law from [Core borrowing](borrowing.md); and parameter/result transfer boundaries and activation lifetime from [Core functions and direct calls](functions.md). Raw-pointer values and provenance remain separately owned by [Core pointers and provenance](pointers.md).

A safe reference is not a raw pointer, a proving-MIR `LoanId`, a static `Place`, a dynamic storage-instance identity, a physical address, an ABI representation, or a provenance token. This document does not define source-language reference or lifetime syntax.

## Reference type

The represented Core type domain contains safe-reference types parameterized by exactly:

1. one exact referent Core type identity; and
2. one reference permission class.

The represented permission classes are:

- **Shared**;
- **Exclusive**; and
- **ExclusiveReplace**.

Two reference types are equal exactly when both their referent type identities and permission classes are equal. Equality of referent scalar kind, structural shape, or another lower classification does not make distinct referent type identities interchangeable.

A safe-reference pointee edge is semantic indirection, not structural value containment. A reference to type `T` therefore does not recursively contain a value of `T`, and a type graph cycle that passes through a safe-reference pointee edge does not by that fact create an infinitely recursive structural value.

This type relation does not define variance, subtyping, coercion, lifetime parameters, nullability, fat-pointer metadata, layout, size, alignment, ABI representation, or source spelling.

## Permission classes

Reference permission is an explicit capability independent of the current stored-value lifetime occupying the target storage.

### Shared

A Shared reference carries shared alias authority over its target region.

Subject to the independently owned operation requirements, it may authorize:

- non-consuming `Read`;
- `Copy` when the selected target type is copyable;
- formation of a Shared reference reborrow; and
- `InteriorAssign` when the selected target independently lies within an interior-mutable region.

It does not authorize ownership-moving `Move`, ordinary `Assign`, explicit `Drop`, Exclusive reborrow, or ExclusiveReplace reborrow.

### Exclusive

An Exclusive reference carries exclusive alias authority but does not carry ordinary replacement permission.

Subject to the independently owned operation requirements, it may authorize every Shared operation and additionally:

- ownership-moving `Move`;
- explicit `Drop`;
- Shared reborrow; and
- Exclusive reborrow.

It does not authorize ordinary `Assign` and cannot form an ExclusiveReplace reborrow.

### ExclusiveReplace

An ExclusiveReplace reference carries exclusive alias authority plus ordinary replacement permission for its target region.

Subject to the independently owned operation requirements, it may authorize every Exclusive operation and additionally:

- ordinary source-first `Assign` or reinitialization; and
- Shared, Exclusive, or ExclusiveReplace reborrow.

The replacement capability is explicit because [Core value and storage semantics](value-storage.md) separates ordinary assignment permission from alias exclusivity. Exclusive alias authority alone never implies ordinary replacement permission.

Interior mutability remains separate from all three permission classes. An interior write still requires the selected target to lie within an interior-mutable region even when the reference is Exclusive or ExclusiveReplace.

## Reference value

A represented safe-reference value contains exactly the semantic facts required by this relation:

- one target `StorageRegion`, using the dynamic storage-instance identity and structural projection relation from [Core value and storage semantics](value-storage.md); and
- one opaque dynamic **reference-authority identity** selecting the reference-backed authority interval that authorizes access to that target.

The value's Core reference type separately fixes its exact referent type and permission class.

The target region is semantic structural identity. It is not a physical address, byte offset, allocation address, relocation token, ABI handle, or exposed provenance object.

The reference-authority identity is also not program-observable data. It is distinct from `LoanId`, storage-instance identity, static place identity, raw-pointer provenance, activation identity, and any implementation counter used by a validator or oracle.

A represented Core constant cannot fabricate a safe-reference value. Safe-reference values arise only from the accepted formation or reborrow relations below and are then transported by ordinary value operations.

## Reference carrier

Every live safe-reference scalar value is one **reference carrier** for its reference-authority identity.

Carrier identity is semantic bookkeeping for authority lifetime. It is not a Core value component that a program may inspect, compare, hash, serialize, or convert to an integer.

A reference-backed authority interval remains active exactly while at least one of the following holds:

- at least one live carrier names that authority; or
- at least one active child reference-backed authority is directly or indirectly descended from it.

Consequently an authority may remain active with no carrier of its own while a child reborrow remains active. When an authority has no carrier and no active child, that authority interval ends. If its parent is also carrierless and has no other active child, termination proceeds transitively toward the root until reaching an authority that still has a carrier or active child, or until the complete reference-authority branch ends.

This carrier rule is semantic alias-authority bookkeeping. It does not require physical reference counting, garbage collection, a hidden heap allocation, or a user-visible/custom destructor.

## Reference value transport

Safe-reference values participate in ordinary Core stored-value lifecycle, subject to their permission-specific copyability.

### Move

Moving a safe-reference value transfers its existing carrier to the produced owned value. The moved-from reference storage becomes Dead under the ordinary value/storage ownership-transfer relation. Move creates no new carrier, does not end the referenced authority, does not change the target region, and does not change the authority identity.

Moving a reference-containing aggregate applies the same rule recursively to every contained safe-reference carrier.

### Copy

A Shared reference type is copyable. Copying one Shared reference creates one additional carrier naming the same reference-authority identity and target region.

Exclusive and ExclusiveReplace reference types are not copyable.

An aggregate containing safe references is copyable only when its ordinary structural copyability relation succeeds; consequently an aggregate containing an Exclusive or ExclusiveReplace reference is non-copyable, while a reference field contributes positively only when that field is Shared and every other structural field is copyable.

Copying a reference-containing aggregate recursively creates one additional carrier for each copied Shared reference leaf. It creates no new reference authority and does not reborrow.

### Storage and assignment

Storing, initializing, or transferring a safe-reference value into another Core storage place preserves its target and authority identity and transfers the produced carrier into that stored value.

Source-first replacement evaluates the replacement value before destroying the old destination contents. Reference carriers in the old destruction domain therefore remain active through source evaluation. Destroying those old reference values then removes their carriers before the replacement value is written, exactly at the ordinary destruction point selected by [Core value and storage semantics](value-storage.md).

No assignment or initialization operation duplicates a carrier merely because a value changes storage location.

### Destruction

Destroying a live safe-reference scalar removes exactly that carrier. It performs no read, move, destruction, assignment, or other access to the referenced target.

After carrier removal, the referenced authority ends only if the authority has no remaining carrier and no active child. Authority termination may therefore occur as a consequence of ordinary explicit destruction, replacement destruction, aggregate destruction, or function-termination cleanup, without adding a custom destructor body.

Destroying a reference-containing aggregate follows the ordinary reverse structural destruction order. Reference-specific authority termination occurs only when each reference leaf is reached by that existing order.

## Reference-backed authority

A **reference-backed authority interval** is a dynamic shared or exclusive alias-access authority over exactly one target storage region.

- Shared references have shared authority.
- Exclusive and ExclusiveReplace references have exclusive authority.

The replacement capability of ExclusiveReplace is an additional operation permission, not a third alias kind.

Reference-backed authorities participate in the structural overlap/conflict and delegation law owned by [Core borrowing](borrowing.md). Existing explicit loans and reference-backed authorities therefore constrain one another when their concrete target regions overlap, even though they have distinct identity and lifecycle mechanisms.

A reference-backed authority is not declared by a body-local `LoanId`. It is created dynamically by reference formation or reborrow and ends through the carrier/child rule above. Existing explicit `Borrow` and `EndBorrow(LoanId)` remain separate proving-MIR operations; `EndBorrow` does not name or end a reference-backed authority.

## Root reference formation

The first represented reference slice permits root safe-reference formation only from **direct Core storage** in the current activation.

Root formation conceptually produces one owned safe-reference value of exact reference type `R` from one direct source place `p`.

Let `T` be the exact type reached by `p`, and let `R` have referent type `U` and permission `K`. Formation is language-valid only when:

1. `T` and `U` are the same exact Core type identity;
2. the complete target place `p` is fully Live at formation;
3. the target storage extent exists;
4. alias admission succeeds against every currently active explicit loan and reference-backed authority under the shared/exclusive conflict law consumed from [Core borrowing](borrowing.md); and
5. when `K` is ExclusiveReplace, `p` additionally has the ordinary direct-storage assignment permission required by [Core value and storage semantics](value-storage.md).

For Shared formation, alias admission uses the shared requirement. For Exclusive or ExclusiveReplace formation, alias admission uses the exclusive requirement.

Successful formation:

1. resolves `p` to its semantic target `StorageRegion`;
2. creates one fresh root reference-authority identity with the selected shared/exclusive authority over that target;
3. creates one carrier naming that authority; and
4. produces the resulting safe-reference value without reading, copying, moving, mutating, destroying, or replacing the target value.

Formation requires the target to be fully Live because a safe reference initially denotes a valid currently owned value region. After formation, later operation-specific liveness rules remain controlling as described below.

Root reference formation from `PlaceAccess::Loan`, from another reference, or from a raw pointer is not represented by this root relation. Reference-to-reference derivation uses the explicit reborrow relation below.

## Reference access

A **reference access** reaches target storage through one live stored safe-reference carrier plus zero or more structural projections relative to that reference's target region.

The operation first obtains the stored safe-reference value non-consumingly through an otherwise legal ordinary Core access to the storage containing the reference value. Obtaining the carrier value for dereference does not copy, move, destroy, or replace that carrier.

The reference value's target region is then extended by the requested relative structural projections. The resulting selected target type must be structurally valid under the ordinary type-projection relation.

Reference access is distinct from `PlaceAccess::Loan`:

- it is authorized by the dynamic reference-authority identity carried by the stored reference value;
- it may resolve to storage in a still-live suspended ancestor activation;
- it does not require the current function body to declare the authority as a `LoanId`; and
- it does not convert the reference into a raw pointer or physical address.

A reference access is valid only while its reference-backed authority remains active and while the target storage extent continues. Valid Core construction and lifecycle rules guarantee these facts; failure to maintain them is language-invalid Core rather than a defined runtime dangling-reference state.

## Reference access operations

After a reference access resolves its concrete target region, the selected reference permission supplies alias/replacement authority while every other operation-specific requirement remains unchanged.

### Read and Copy

Shared, Exclusive, and ExclusiveReplace reference access may perform `Read` when the selected target is fully Live.

They may perform non-consuming `Copy` when the selected target is fully Live and its selected target type is copyable.

Copying the pointee does not copy the reference carrier unless the pointee value itself contains safe-reference leaves, in which case the ordinary recursive reference-copy rules apply to those copied value leaves.

### Move and Drop

Only Exclusive and ExclusiveReplace reference access may perform ownership-moving `Move` or explicit `Drop`, subject to ordinary target liveness/destruction requirements.

Moving or destroying the current pointee ends stored-value lifetimes but does not by itself end the target storage extent or the reference-backed authority. The reference may therefore continue to denote the same vacant structural storage region while its authority remains active.

### Ordinary replacement

Only ExclusiveReplace reference access may perform ordinary `Assign` or reinitialization. The operation uses the same source-first replacement lifecycle, exact type requirement, destruction-domain ordering, and resulting stored-value lifetime rules owned by [Core value and storage semantics](value-storage.md).

The caller-local assignment-mutability fact that permitted root ExclusiveReplace formation is not re-queried in a callee. The explicit replacement capability carried by the reference type/authority is the interprocedural permission consumed by the operation.

Exclusive reference access without the Replace capability cannot perform ordinary `Assign` even though it has exclusive alias authority.

### Interior replacement

Shared, Exclusive, and ExclusiveReplace reference access may perform `InteriorAssign` only when the selected target independently lies within an interior-mutable region. The existing source-first replacement lifecycle applies.

Interior mutability does not strengthen reference permission, create a new authority, or permit ownership-moving operations through Shared authority.

## Reference reborrow

A reference reborrow derives one fresh child reference-backed authority and one new reference carrier from an existing reference access.

Let the parent reference permission be `P`, the requested child permission be `C`, and the selected reference-access target have exact type `T`. The produced child reference type must have referent type exactly `T`.

Permission may never strengthen:

- Shared parent permits only Shared child;
- Exclusive parent permits Shared or Exclusive child;
- ExclusiveReplace parent permits Shared, Exclusive, or ExclusiveReplace child.

For the selected target, child admission additionally follows the existing delegation law:

- a Shared child requires the parent to retain shared authority at that target;
- an Exclusive or ExclusiveReplace child requires the parent to retain exclusive authority at that target;
- overlapping active child authorities restrict the parent and sibling derivations exactly under the shared/exclusive structural overlap rules;
- disjoint child authorities do not constrain access to disjoint parent subregions.

Successful reborrow:

1. creates one fresh child reference-authority identity targeting the selected structural region;
2. records the current reference authority as its parent;
3. delegates the applicable shared/exclusive authority from parent to child for the complete child interval;
4. creates one carrier naming the child authority; and
5. produces the child safe-reference value.

Reborrow does not copy or move the parent carrier. A parent may therefore remain stored while authority to an overlapping region is partially or completely delegated to its active children.

A child authority with no carrier remains active while it has an active descendant, and parent authority is restored over the delegated region only after the applicable child branch ends.

## Interaction with explicit loans

Explicit loans from [Core borrowing](borrowing.md) and reference-backed authorities are distinct semantic identities but share one alias-conflict domain over concrete structural storage regions.

Consequently:

- direct access must account for both active explicit loans and active reference-backed authorities;
- explicit root-borrow creation must account for overlapping reference-backed authorities;
- reference root formation must account for overlapping explicit loans;
- reference and explicit-loan accesses are each constrained by their own active descendants/delegation state; and
- unsafe raw-pointer target compatibility must account for every overlapping active explicit loan and reference-backed authority, without treating the raw pointer as carrying either authority identity.

This document does not create a generic observable alias node or unify `LoanId` and reference-authority identity. [Core borrowing](borrowing.md) owns the common overlap/conflict consequences.

## Raw-pointer separation

The first reference slice does not widen raw-pointer formation or raw-pointer target operations.

In particular, reference access is not an accepted source for the currently represented raw-pointer `AddressOf` operand, and `RawRead`, `RawMove`, and `RawAssign` do not consume a safe reference as a raw pointer.

A reference value carries no raw-pointer provenance. A raw pointer carries no reference-authority identity. Ending a reference authority does not mutate an independently existing raw-pointer value, and no raw-pointer value is formed merely by moving, copying, storing, dereferencing, passing, or returning a safe reference.

The call-transfer restrictions in [Core functions and direct calls](functions.md) additionally prohibit safe-reference parameters from exposing raw-pointer-containing referent storage across activations. No cross-activation raw-pointer relation is created by safe-reference parameter or result transfer.

## Target stored-value lifetime

A safe reference targets a continuing structural storage region, not one frozen stored-value lifetime.

Root formation requires the complete initial target to be Live. After formation:

- Read and Copy continue to require the selected target value to be Live;
- Move and Drop through Exclusive or ExclusiveReplace authority may end the selected stored-value lifetime and leave the target storage vacant;
- ExclusiveReplace ordinary assignment may later initialize or replace the continuing target region under its ordinary lifecycle;
- legal InteriorAssign may replace the current stored value when the independent interior-mutability requirement holds; and
- none of those stored-value transitions changes the target storage-instance identity or reference-authority identity while the storage extent and authority remain active.

A Shared reference therefore does not mean one immutable stored-value lifetime exists forever; interior mutation may legally replace a value under shared authority where the type/storage owner permits it.

## Storage-extent validity

A safe-reference authority and every carrier/descendant authority derived from it MUST NOT outlive the storage extent containing its target region.

When one local storage extent is about to end under function-termination cleanup, no live reference carrier and no active reference-authority descendant anywhere in the represented active-call stack may still target that local storage instance.

This requirement applies even when no later dereference would execute. Valid Core does not contain a temporarily or permanently dangling safe-reference value.

Because [Core value and storage semantics](value-storage.md) ends each local storage extent only after that local's cleanup completes, ordinary reverse local/field destruction may itself remove reference carriers before the target extent ends. A valid program may therefore rely on the already defined cleanup order when that order provably destroys all target-dependent carriers before the target local's extent ends.

The requirement also permits a callee reference parameter to target storage belonging to a still-live suspended caller or another still-live ancestor activation. Suspension does not end those storage extents.

A reference to a callee-local storage region cannot escape into a normally resumed caller through the bounded result contract owned by [Core functions and direct calls](functions.md): an admitted safe-reference result preserves the exact target and reference-authority identity of its designated incoming Shared-reference parameter origin, whose target belongs to still-live external storage rather than a callee local. Ordinary result forms remain reference-free, and storage reachable through a transferred safe-reference parameter still contains no safe-reference leaf. Passing a callee-local reference further into a nested call does not extend the callee-local target beyond its owning activation unless that owning activation itself remains live; every nested activation must terminate before the owning activation can return normally, unless it diverges or faults instead.

Violation of the storage-extent validity requirement is a Core language-validation failure. It is not a defined `Fault`, not undefined behavior selected by a dereference, and not a runtime recovery case.

## Zero-leaf targets

A safe reference to a zero-leaf structural value is meaningful.

Its target `StorageRegion`, storage extent, authority, carrier lifetime, overlap relation, and structural identity remain semantic facts even when the target has no scalar storage leaf and therefore contributes no scalar destruction event.

Reference validity and alias authority MUST NOT be reconstructed from byte size, scalar-leaf count, or physical addressability.

## Function parameter transfer boundary

[Core functions and direct calls](functions.md) owns cross-activation value transfer, including the exact parameter-transfer predicate and referent-state contract. This reference owner supplies only the reference-value and authority facts consumed there.

A safe-reference value transferred into a parameter remains the same semantic target and authority carrier after transfer. Parameter transfer does not copy an Exclusive or ExclusiveReplace reference and does not create a new authority merely because the carrier moves into another activation.

A Shared argument produced by ordinary Copy has a new carrier for the same authority before transfer; a Shared argument produced by Move transfers its existing carrier.

Call admission additionally requires the transferred carrier to retain the complete authority promised by its permission and its complete target to be fully Live after all argument evaluation. Those call-entry requirements do not change the general rule that a safe reference may continue to denote vacant storage inside one activation after a legal Move or Drop.

The first transfer slice admits only safe-reference parameters whose exact referent structural value contains neither a raw-pointer leaf nor another safe-reference leaf, under the predicate owned by `functions.md`. This is a call boundary, not a restriction on general Core reference type formation or intra-activation use.

A temporary borrowed-call pattern may be represented without a special parameter pass mode:

1. create a child reference reborrow in the caller;
2. restore/retain a fully-Live complete child target and complete child authority for call admission;
3. move that child reference value as an ordinary call argument;
4. keep the parent reference carrier in the suspended caller while the child authority delegates the overlapping target;
5. execute the callee using its ordinary reference parameter value; and
6. on a normal return, require the child target to be fully Live before callee cleanup; if the function result does not preserve that exact child authority under the callable result contract owned by `functions.md`, cleanup removes its final callee-owned carrier so the child authority ends and the parent's delegated authority is restored before caller continuation; when an admitted result does preserve that exact child authority, the preserved result carrier instead keeps the child active and the parent remains delegated until that returned carrier later ends.

On normal return, `functions.md` requires every external referent domain introduced through safe-reference parameters to be fully Live after return-operand effects and before cleanup. The actual target value may have changed; the postcondition preserves structural liveness, not pre-call value equality.

Defined fault and divergence consequences remain owned by [Core functions and direct calls](functions.md) and do not synthesize this normal-return fully-Live postcondition on paths with no normal continuation.

## Function result transport boundary

[Core functions and direct calls](functions.md) owns callable result admissibility, selection of any result-origin parameter slot, independent function validation of that origin contract, Return ordering, caller continuation summaries, and the exact result-transfer predicate. This reference owner does not redefine those policies.

When `functions.md` admits a safe-reference result, the preserved result value obeys the ordinary reference-value transport relation from this document:

- the result carrier retains exactly the same semantic target `StorageRegion` and reference-authority identity as the produced result operand;
- preserving and transferring that carrier across activation cleanup creates no new target, reference authority, reborrow, or additional carrier merely because the value crosses the result boundary;
- the preserved carrier is outside the terminating activation's local cleanup set and therefore is not removed when other callee-local carriers are destroyed; and
- the referenced authority remains active after callee cleanup whenever that preserved carrier or an active descendant keeps it live under the existing carrier/child lifecycle.

If the preserved authority is itself a child authority created in an earlier still-live activation, its parent/ancestor delegation consequences remain exactly those already defined by the carrier/child relation. Result transfer does not detach, re-root, widen, narrow, or otherwise rewrite that authority branch.

These are reference transport and lifetime consequences only. Which result values are permitted to survive, which input origin they must match, and which result shapes or permission classes are admitted remain owned by `functions.md`.

## Parameter referent-state boundary

The exact parameter-transfer predicate is owned by [Core functions and direct calls](functions.md).

For this first transfer slice, the structural value stored in the target of each transferred safe-reference carrier contains neither:

- a raw-pointer leaf; nor
- another safe-reference leaf.

The raw-pointer exclusion prevents extraction of a raw-pointer value from suspended ancestor storage. The safe-reference exclusion prevents a callee from creating, destroying, moving, copying, or replacing nested reference carriers/authority identities in suspended ancestor storage without a callable authority/effect contract, and prevents storing a newly formed callee-local reference into transferred caller storage.

Richer reference parameters whose referents contain safe references require a later callable contract capable of describing nested authority origins/effects and caller continuation state. This revision does not infer that contract from recursive callee analysis.

## Defined fault, return, and divergence

Reference carrier cleanup uses the ordinary destruction points selected by the existing storage/function owners.

On normal return, the function owner first applies its external-referent postcondition and, when a safe-reference result is admitted, preserves that produced result carrier outside activation-local cleanup. Callee local cleanup then removes every remaining callee-owned reference carrier in ordinary reverse local/field order. A preserved result carrier is not part of that destruction set; it keeps its existing authority active according to the ordinary carrier/child lifecycle while other carrier removals may end sibling or descendant branches and restore delegated ancestor authority as applicable.

On defined fault, no result carrier is preserved. Each terminating activation performs the same ordinary cleanup as the fault propagates outward. Reference carriers and reference-authority branches therefore end according to each activation's cleanup before that activation's local storage extents end. The normal-return fully-Live external-referent postcondition does not apply because no caller normal continuation is selected.

If a callee diverges, no result carrier is produced by a completed Return and no termination cleanup occurs merely because execution continues indefinitely. The caller remains suspended, its storage extents continue, and reference authorities/carriers retained in any still-live activation continue according to this document. No normal-return referent-state postcondition is synthesized on a diverging path.

Undefined behavior remains distinct. This document adds no unsafe safe-reference operation and no safe-reference UB category.

## Determinism and verification

For a fixed valid Core execution, reference target selection, fresh reference-authority distinction, carrier transfer/copy/removal, parent-child delegation, authority termination, and reference-access resolution are deterministic from the accepted storage/value state, call stack, and alias-authority state.

A reference oracle MAY assign opaque numeric identities to reference authorities or carriers for verification, but those numbers are not Core-observable and do not prescribe implementation representation.

Reference-backed authority and carrier state are semantic execution state and therefore participate where a proving implementation needs them to distinguish path states. This does not make implementation collection order or identifier numbering semantic.

## Representation boundary

The represented reference relation requires stable semantic `StorageRegion` identity for the duration of a target storage extent. It does not require physical address stability.

No operation in this slice exposes:

- a numeric address;
- bytes or invalid bit patterns;
- reference size/alignment;
- field offsets;
- ABI representation;
- serialization identity;
- relocation observation; or
- pinning.

A realization may preserve the semantic reference relation using physical addresses, handles, relocation tables, indirection, or another mechanism. Representation validity, physical relocation/address-stability, and pinning remain deferred until an accepted consumer actually makes them observable or necessary.

## Separate semantic owners and deliberate exclusions

[Core borrowing](borrowing.md) owns structural overlap and the common alias-conflict consequences shared by explicit loans and reference-backed authorities. [Core value and storage semantics](value-storage.md) owns storage extent, stored-value lifecycle, ordinary replacement, interior mutability, destruction domain/order, and function-local cleanup. [Core functions and direct calls](functions.md) owns parameter/result transfer predicates, callable safe-reference result-origin policy, bounded transferred-referent entry/normal-return state, and activation/call behavior. [Core pointers and provenance](pointers.md) owns raw-pointer values and provenance. [Core unsafe semantics](unsafe.md) owns unsafe-operation preconditions and undefined-behavior classification.

This revision deliberately does not define:

- source reference, borrow, lifetime, or `unsafe` syntax;
- named or inferred source lifetimes;
- source places/lvalues;
- safe-reference result admissibility or callable result-origin selection beyond the transport consequences consumed from `functions.md`;
- safe-reference-containing aggregate results, derived/subregion result authorities, authority detachment/re-rooting, or Exclusive/ExclusiveReplace result transfer;
- cross-activation transfer through safe references whose referent structural values contain safe-reference or raw-pointer leaves;
- callable borrowed-effect summaries beyond the bounded fully-Live entry/normal-return contract owned by `functions.md`;
- raw-pointer transfer across calls;
- raw-pointer formation through reference access;
- pointer/reference casts, arithmetic, null, or numeric addresses;
- physical reference layout, ABI, FFI, or linkage;
- representation validity, physical address stability, relocation guarantees, or pinning;
- generics, traits, variance, subtyping, or coercion;
- closures or captures;
- const/static semantics;
- safe-public-contract source checking;
- heap allocation/deallocation;
- custom destructor bodies; or
- concurrency and memory-ordering semantics.

Those concerns require their own accepted canonical owners or later consumer-driven extensions.