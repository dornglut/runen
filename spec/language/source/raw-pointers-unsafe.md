# Source Raw Pointers and Unsafe Admission

Status: **provisional normative; incomplete**

This document owns the first represented source-language raw-pointer and unsafe-admission relation: raw-pointer type/value semantics, exact binding-root pointer-origin provenance, lexical pointer validity, bounded raw address formation, one unsafe ownership-moving raw access, one unsafe source-first raw replacement, lexical unsafe admission, automatic discharge of the represented unsafe preconditions, and source-to-Core refinement for this bounded slice.

It consumes represented source type identity and owned-value duplicability from [Source type foundation](types.md); binding identity, lexical scope, lookup, assignment mutability, binding lifecycle, ordinary value use, and ordinary assignment from [Source function-local bindings](local-bindings.md); structural ownership state, path availability, consumption, remaining-ownership frontier, and root reset from [Source structural ownership](structural-ownership.md); Shared reference authority and carrier state from [Source Shared references](references.md); callable admission from [Source callable signatures](callables.md); execution, cleanup, return, defined fault, and divergence from [Source function execution](function-execution.md); represented control-flow joins and loop backedges from [Source control flow](control-flow.md); and represented spellings from [Source concrete syntax](concrete-syntax.md). It does not redefine those owners.

The lower refinement target is the accepted raw-pointer, unsafe-operation, storage/lifecycle, alias-compatibility, and direct-call relation in [Core pointers and provenance](../core/pointers.md), [Core unsafe semantics](../core/unsafe.md), [Core value and storage semantics](../core/value-storage.md), [Core borrowing](../core/borrowing.md), and [Core functions and direct calls](../core/functions.md).

This first source slice is intentionally activation-local. It does not expose raw-pointer parameters or results, pointer-containing records, pointer-to-pointer or pointer-to-reference types, a general source place/lvalue category, raw field addresses, null or fabricated pointers, numeric addresses, pointer arithmetic, pointer comparison, pointer/integer conversion, physical layout, ABI, relocation/address-stability, pinning, unsafe callable contracts, or transferred unsafe proof obligations.

## Raw-pointer source type

For every first-slice admissible pointee source type `T`, the represented source type domain contains one raw-pointer type `RawPtr(T)`.

A source type is **first-slice raw-pointee-admissible** exactly when it is either:

- one represented intrinsic scalar source type from `types.md`; or
- one represented nominal record source type from `types.md`.

`RawPtr(T)` and `SharedRef(T)` are not first-slice raw-pointee-admissible. The raw-pointer constructor is therefore nonrecursive in this slice and does not create a raw-pointer-to-reference relation.

Two raw-pointer source types `RawPtr(A)` and `RawPtr(B)` are equal exactly when `A` and `B` are equal under the source type-equality relation from `types.md`.

Every represented `RawPtr(T)` is source-duplicable. Duplicating a raw-pointer value preserves the exact raw-pointer value and pointer-origin provenance defined below; it does not duplicate, copy, move, borrow, initialize, or otherwise access the target value.

Raw-pointer type identity contains no source mutability qualifier, target binding identity, pointer-origin identity, numeric or physical address, size, alignment, layout, representation, provenance encoding, ABI property, relocation guarantee, address-stability guarantee, or pinning property.

## Contextual admission

A represented `RawPtr(T)` is admitted directly only as the declared type of an ordinary function-local binding in this slice.

Such a local may be immutable or mutable under the ordinary assignment-mutability relation from `local-bindings.md`. Mutability controls only ordinary replacement of the stored raw-pointer value; it grants no authority to mutate the pointee.

`RawPtr(T)` is source-invalid in this slice as:

- a function parameter type;
- a function result type;
- a nominal record field type;
- a Shared-reference referent;
- a raw-pointer pointee; or
- a pattern-introduced binding type, because first-slice record fields cannot contain raw pointers.

These are bounded source admission rules. They do not redefine Core raw-pointer type legality and do not imply that later source revisions must preserve the same contexts.

## Raw-pointer value and pointer-origin provenance

Every represented source raw-pointer value denotes one existing complete source binding root in the current function activation. The pointer does not carry source Shared-reference authority, does not create a source loan, and does not keep any Shared authority active.

Source validation additionally tracks one non-observable **pointer-origin provenance** for every raw-pointer value flow:

```text
PointerOrigin(binding)
```

where `binding` is the exact source-semantic parameter/local binding identity whose complete root was selected by raw address formation.

Pointer-origin provenance is a validation fact. It is not program-observable data, source type identity, a lifetime name, a numeric address, physical storage identity, Core `Place`, Core storage-instance identity, Core pointer-provenance representation, or an alias-authority token.

Pointer origin is established and transported exactly as follows:

- raw address formation of binding `x` produces `PointerOrigin(x)`;
- ordinary duplication of a raw-pointer value preserves the origin unchanged;
- initialization of a raw-pointer local from an existing raw-pointer value preserves the origin unchanged; and
- ordinary assignment of a raw-pointer value to a mutable raw-pointer local replaces that local's stored origin with the incoming value's exact origin after successful assignment.

No operation in this slice merges, widens, forgets, reconstructs, or fabricates pointer origins.

## Lexical target validity

Every raw-pointer local has one **lexically valid target requirement**.

When a raw-pointer local is initialized, the target binding named by the incoming pointer origin MUST have a lexical extent containing the complete lexical extent of the receiving raw-pointer local.

When a mutable raw-pointer local is later assigned another raw-pointer value, the target binding named by the incoming origin MUST likewise have a lexical extent containing the complete receiving raw-pointer local extent.

Consequently a pointer local cannot receive a pointer to a binding introduced in a shorter-lived descendant scope, a later loop-body-local dynamic instance, or another binding whose extent may end while the pointer local remains active.

A parameter binding's extent contains every ordinary local extent in its function activation. An earlier local in the same lexical scope may contain a later local's extent under the binding-lifecycle relation from `local-bindings.md`. Descendant-local extents do not contain an ancestor local's extent.

This lexical rule is deliberately stronger than treating a later dangling access as admitted unsafe behavior. Current Core semantics do not define dangling access outside represented local-storage extents, so this source slice never lowers a source-valid raw-pointer value whose target storage extent may already have ended.

The rule is also significant for represented loops: one static binding identity may correspond to distinct dynamic storage instances on separate dynamic entries into a loop body. A raw-pointer local whose extent survives such an entry therefore cannot retain or receive an origin naming that shorter-lived loop-body binding.

This slice introduces no named lifetime, outlives syntax, non-lexical lifetime inference, origin set, maybe-origin state, or dynamic dangling check.

## Raw address formation

The represented raw-address producer selects exactly one complete active parameter or ordinary local binding root `x` and produces `RawPtr(T)`, where `T` is exactly the declared source type of `x`.

Formation is source-valid only when:

1. `x` resolves through the existing function-local lookup relation;
2. `x` is an active parameter or ordinary local binding;
3. its declared type `T` is first-slice raw-pointee-admissible; and
4. the surrounding receiving position requires exact source type `RawPtr(T)`.

Formation does **not** require the complete target root to be fully available under structural ownership. It may select a fully available, partially available, or unavailable binding root while that binding's lexical/storage extent remains active.

Successful formation:

- produces one owned raw-pointer value with `PointerOrigin(x)`;
- leaves the target structural ownership state unchanged;
- creates no Shared reference authority, reference carrier, loan, or borrow interval;
- does not read, duplicate, consume, move, mutate, initialize, destroy, replace, or otherwise access the target value; and
- is not itself an unsafe-admission-required source operation.

The first slice admits only the complete empty structural path of a binding root. It does not admit field paths, producer transients, dereference results, call results, arbitrary values, pattern paths independently of their binding root, static/global storage, or another general source place as an address-formation target.

A faithful lowering maps successful raw address formation to Core `AddressOf` of the complete lowered local place. The source target need not be structurally available because Core `AddressOf` likewise selects continuing storage independently of stored-value liveness.

## Unsafe raw ownership move

The represented **raw ownership move** obtains one stored `RawPtr(T)` value non-consumingly and transfers the complete current target value out of the binding root named by that pointer's exact origin.

A raw ownership move is source-valid only when all of the following hold:

1. evaluation occurs within an active unsafe-admission region defined below;
2. the raw-pointer operand resolves to one active binding of exact type `RawPtr(T)`;
3. its stored pointer-origin provenance is `PointerOrigin(x)` for one still-active target binding `x`;
4. the complete structural root of `x` is fully available immediately before the move;
5. no active first-slice source Shared authority targets `x`; and
6. the surrounding receiving position requires exact source type `T`.

Successful raw ownership move:

- obtains the pointer value without consuming or retargeting the pointer binding;
- consumes/transfers exactly the complete structural root of `x` through the existing structural-ownership transition;
- produces one owned value of exact type `T`; and
- leaves the pointer origin unchanged.

The operation is ownership-moving even when `T` is source-duplicable. Raw access does not silently select ordinary duplicate semantics.

The complete-availability requirement discharges the represented Core target-liveness precondition. The absence of a source Shared authority targeting the root discharges the represented Core exclusive raw-target alias requirement for the currently represented source authority set. Therefore a source-valid raw ownership move refines directly to Core `RawMove` without transferring an unproven unsafe obligation to a caller.

A partially available or unavailable target fails source validation for raw ownership move. It is not admitted as source-level undefined behavior merely because the operation appears inside an unsafe block.

The current Core `RawRead` operation is not exposed as a source value operation by this slice. It discards its semantic read result and therefore does not provide a useful owned source value. A future non-consuming owned raw load/copy requires a separate accepted source/Core relation.

## Unsafe raw source-first replacement

The represented **raw replacement** targets one stored `RawPtr(T)` value and one source producer of exact type `T`.

Raw replacement is source-first. Its selected target is determined before source evaluation, while the target replacement itself occurs only after successful source production.

A represented raw replacement proceeds as follows:

1. require an active unsafe-admission region;
2. resolve the raw-pointer operand to one active binding of exact type `RawPtr(T)` and snapshot its exact `PointerOrigin(x)`;
3. require `x` to remain an active lexically valid target under this document;
4. validate and evaluate the complete source producer according to its canonical owner;
5. if source evaluation faults or diverges, perform no raw target destruction, replacement, structural reset, or pointer retargeting;
6. after successful source evaluation, require that no active first-slice source Shared authority targets `x`;
7. select the then-current remaining ownership frontier of the complete target root through `structural-ownership.md`;
8. end that remaining target ownership in the canonical frontier order through `function-execution.md`; and
9. install the already produced exact-`T` source value as the new complete target value, establishing the existing fresh empty consumed-path state for that root.

The target root need not be fully available before raw replacement. A fully available, partially available, or unavailable target may be replaced. Source evaluation may itself change the target structural ownership state before step 7; the frontier selected for replacement is the state that exists after successful source evaluation.

Ordinary assignment mutability of target binding `x` is not a raw-replacement precondition. Raw replacement is a distinct explicitly unsafe operation and does not become ordinary assignment merely because it reuses the structural replacement lifecycle.

Pointer-binding mutability is independently relevant only when ordinary assignment retargets the stored raw-pointer value. Raw replacement does not change the pointer value or origin.

The post-source Shared-authority check discharges the currently represented Core exclusive raw-target alias requirement. The source-first ordering, remaining-value destruction, and complete-root reset refine to Core `RawAssign` and its consumed value/storage replacement lifecycle.

## Unsafe admission

The first source unsafe boundary is one lexical **unsafe-admission block**. It is an ordinary child lexical scope for binding lifetime, cleanup, control flow, return, fault, and divergence; it additionally marks its body and descendant lexical blocks as an active unsafe-admission region.

Raw ownership move and raw replacement require an active unsafe-admission region. Raw address formation, raw-pointer duplication, raw-pointer local initialization, and ordinary raw-pointer assignment do not.

Nested unsafe-admission blocks are semantically idempotent with respect to unsafe admission. Entering or leaving such a block does not itself change structural ownership state, pointer origin, Shared authority, callable identity, function safety classification, or runtime state.

Unsafe admission is **not** a trust boundary. It does not waive, assume, assert, defer, or transfer an unsafe precondition.

All represented source functions remain safe callable entities under `callables.md`. This slice defines no unsafe function type, unsafe callable declaration, unsafe call, unsafe parameter/result contract, hidden caller precondition, body-derived safety effect, user-written proof contract, or caller obligation.

Accordingly, every represented Core undefined-behavior precondition consumed by one admitted source raw operation MUST be discharged by source validation from the exact facts defined by the applicable canonical source owners. For this slice those facts are limited to exact pointer origin, target lexical validity, target structural ownership state, and active source Shared-authority state.

If an unsafe raw operation's represented Core preconditions cannot be discharged, the source program is invalid. Failure is not a defined `Fault`, permitted undefined behavior, implementation-defined behavior, or an obligation silently inherited by a safe caller.

This is the first bounded source realization of the safe-public-contract law from Core unsafe semantics. It does not define a general theorem language, effect system, proof annotation, unsafe trait, or future unsafe-callable policy.

## Control-flow provenance consequences

Pointer-origin provenance is part of the definite source-validation state consumed by the represented control-flow owner.

When two normal conditional outcomes meet, every continuing enclosing raw-pointer binding MUST hold exactly the same `PointerOrigin(binding)` on both outcomes before one normal successor may be established.

When exactly one conditional outcome continues normally, that outcome's exact pointer-origin state continues without comparison against a non-returning outcome, analogously to the existing structural-ownership rule.

For a represented `while` backedge, every continuing enclosing raw-pointer binding from the pre-condition/header environment MUST have exactly the same origin before the backedge is admitted as it had in that pre-condition/header environment.

The loop body may temporarily retarget a mutable raw-pointer local when ordinary assignment is otherwise source-valid, but every normally continuing backedge must restore the exact required origin. A body with no normal continuation contributes no backedge origin requirement.

These rules add no union, intersection, origin set, maybe-origin state, widening, normalization, fixed-point inference, dynamic tag, or non-lexical lifetime relation.

Structural ownership and pointer-origin equality are distinct required states. A normal control-flow edge is admitted only when every independently applicable state requirement from its canonical owner succeeds.

## Cleanup, fault, divergence, and target lifecycle

A raw-pointer value is an ordinary owned source value with no pointer-specific destruction effect in this slice. Ending ownership of a raw-pointer local removes only that pointer value. It does not destroy, move, mutate, restore, retarget, or otherwise affect the pointee.

The lexical target-validity rule ensures that ordinary cleanup of a pointer local occurs before any target binding whose extent must contain that local may end.

A defined fault follows the existing source cleanup relation. Pointer-local cleanup has no target-specific effect. If a raw replacement source producer faults, the target replacement has not occurred because source evaluation precedes target replacement.

Divergence likewise creates no synthetic target replacement, cleanup, result, or pointer-origin transition. A diverging raw-replacement source never reaches the target replacement steps.

Raw ownership move and raw replacement affect target structural ownership exactly at their defined successful transitions. Later ordinary source use of the target observes that updated structural state through the existing owners; no runtime moved-state repair or second raw-specific ownership state exists.

## Source-to-Core refinement

A faithful typed frontend and lowerer MUST preserve the source relations above rather than reconstructing them from Core behavior after lowering.

For this first slice:

- `RawPtr(T)` lowers to the existing Core raw-pointer type whose pointee is the exact lowering of `T`;
- raw address formation of complete binding root `x` lowers to Core `AddressOf` of the complete Core local place representing `x`;
- ordinary raw-pointer duplication lowers through the existing Core copyable raw-pointer value relation;
- ordinary raw-pointer owned transfer/assignment uses existing Core owned-value initialization/replacement operations as applicable to the receiving pointer local;
- source raw ownership move lowers to Core `RawMove` of the stored pointer value;
- source raw replacement lowers to Core `RawAssign`, preserving source-first evaluation order;
- raw-pointer cleanup requires no pointer-specific Core operation beyond ordinary value/storage cleanup; and
- pointer-origin provenance is retained by source validation/lowering only as needed to select and justify the correct Core target effects; it does not require a second runtime pointer-origin object.

The source-to-Core refinement MUST preserve the target binding's source structural-ownership effects. A lowerer MUST NOT use Core liveness after the fact to decide whether source RawMove was valid or to reconstruct the source remaining-ownership frontier for RawAssign.

Likewise, Core pointer targets/provenance, storage-instance identities, numeric verifier identifiers, static Core places, or runtime pointer representations MUST NOT become source-observable facts merely because the lower implementation retains them.

No physical address, pointer representation, layout, relocation, stability, pinning, ABI, or target policy is required by this refinement.

## Determinism and verification

For one fixed source-valid program and fixed ordinary source inputs, pointer-origin production/transport, target selection, structural state transitions of successful raw move/replacement, and unsafe-admission validity are deterministic from the canonical source facts consumed above.

An implementation MAY retain additional non-observable provenance or Core verification metadata, but such metadata MUST NOT widen source validity, create pointer equality, distinguish two source pointer values with the same defined source facts, or add observable behavior.

## Not yet defined

This revision deliberately does **not** define:

- raw-pointer parameters, results, or other cross-activation pointer transfer;
- raw-pointer-containing record fields or aggregates;
- pointer-to-pointer or pointer-to-Shared-reference types;
- Shared references to raw-pointer values;
- null pointers, integer/fabricated pointers, or pointer constants;
- source exposure of Core `RawRead`;
- a non-consuming owned raw load/copy;
- raw field/path address formation or a general source place/lvalue category;
- pointer arithmetic, offsets, one-past rules, equality, ordering, hashing, identity observation, or pointer/integer conversion;
- target-sized integers;
- physical addresses, layout, alignment, endian rules, representation validity, relocation, address stability, or pinning;
- heap allocation, deallocation, global/static storage, or allocation APIs;
- unsafe functions, unsafe calls, unsafe callable/effect signatures, caller proof obligations, or user-written proof contracts;
- source Exclusive/ExclusiveReplace references, reborrow, mutable-reference syntax, derived/multiple reference-result origins, lifetime names, or non-lexical lifetimes;
- atomics, concurrency, memory ordering, data races, ABI, FFI, or linkage; or
- a general undefined-behavior taxonomy beyond the represented Core raw-operation preconditions consumed by this slice.
