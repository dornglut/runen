# Source Structural Ownership

Status: **provisional normative; incomplete**

This document owns the represented source-semantic relation for structural owned-value roots, structural source paths, structural ownership state, path availability, consuming and non-consuming path-use requirements, structural root replacement reset, bounded structural subpath installation/replacement/reinitialization state, and deterministic remaining-ownership frontiers.

It consumes represented source type identity, nominal record identity, record field identity and source structural field order from [Source type foundation](types.md). It does not redefine those owners.

[Source function-local bindings](local-bindings.md) instantiates this relation for each represented parameter/local binding and owns binding identity, lexical scope, lookup, assignment mutability, declaration lifecycle, ordinary whole-binding owned-value use, whole-binding and bounded binding-root field assignment legality, and binding reset points. [Source safe references](references.md) instantiates this relation for each replacement-capable parameter's non-binding external referent root and owns the reference operations that consume or restore that root. [Source field-value access](field-access.md) consumes this relation for binding-rooted selected paths and for the structural ownership state/frontier of a producer-backed field-receiver transient. [Source patterns](patterns.md) consumes this relation for direct binding-root pattern paths and for the structural ownership state of a producer-backed pattern scrutinee transient. [Source function execution](function-execution.md) consumes remaining-ownership frontiers when represented binding, external-referent replacement, bounded binding-root field replacement, or transient ownership ends. [Source control flow](control-flow.md) consumes complete binding and external-referent structural ownership states to establish definite represented conditional successors and bounded-loop transfer/backedge states.

This document does not define lexical bindings, names, mutability, field lookup/accessibility, pattern syntax, source type duplicability selection, reference authority/reborrow/lifetime, conditional or loop selection, custom destruction, a source place/lvalue category, physical storage, layout, Core MIR liveness, or an implementation representation.

## Structural owned-value roots

A **structural owned-value root** consists of:

- exactly one represented source value type, the **root type**; and
- exactly one structural ownership state defined below.

A structural owned-value root is a source-semantic ownership domain over the structural subvalues of one value whose complete source type is the root type. When established with an empty consumed-path set it owns the complete root value; later valid consumption may leave only a proper subset of structural subvalues owned, or no remaining owned subvalue. The relation does not require the root to have a lexical identifier, source binding identity, physical address, storage identity, HIR local, Core local, or another source-observable identity.

Represented parameter/local bindings instantiate persistent structural owned-value roots through `local-bindings.md`. Every replacement-capable safe-reference parameter instantiates one persistent non-binding external referent structural root through `references.md`. A successful producer-backed field-value receiver instantiates one non-binding transient structural owned-value root through `field-access.md` and `function-execution.md`. A successful producer-backed record-pattern scrutinee instantiates one non-binding transient structural owned-value root through `patterns.md` and `function-execution.md`.

The root relation itself does not decide how a value is produced, when a binding/external-referent/transient root begins or ends, or which source operation is permitted to use a path. Those are owned by the applicable lifecycle or operation owner.

## Structural source paths

A **structural source path** is a finite sequence of resolved nominal-record field identities beginning at one structural owned-value root's root type.

The empty path `[]` denotes the complete root value.

A non-empty path `[f0, f1, ..., fn]` is structurally valid exactly when:

1. the root type is a nominal record type containing field identity `f0`;
2. for every later field `fi`, the source type reached by the preceding path prefix is a nominal record type containing `fi`; and
3. each path step uses that selected field's declared source type from `types.md` as the type reached by the extended prefix.

For every structurally valid path `p`, define `type(p)` as the represented source type reached by the complete path. `type([])` is the root type.

Structural paths use source-semantic nominal record and field identity. They are not identifier spellings, parser nodes, compiler field indices, Core projections, byte offsets, addresses, physical storage regions, or ABI layout.

A path `a` is an **ancestor** of path `b` exactly when `a` is a proper prefix of `b`. Two paths are **structurally disjoint** exactly when they are unequal and neither is an ancestor of the other.

The empty path is therefore an ancestor of every non-empty path.

## Structural ownership state

Each structural owned-value root has one finite **consumed-path set** containing structurally valid paths rooted at that value.

The consumed-path set MUST be prefix-free: no two members are ancestor/descendant or otherwise prefix-comparable.

An empty consumed-path set denotes complete initial ownership of the root value. The applicable lifecycle owner establishes when one complete owned value begins. `local-bindings.md`, `references.md`, `field-access.md`, and `patterns.md` define the represented initial-state boundaries that consume this relation.

Consumed paths are source-validation facts. They are not runtime moved-value flags, dynamic faults, Core liveness facts, physical destruction markers, storage occupancy, or implementation bookkeeping authority.

## Path availability

For one structurally valid path `p` and the current consumed-path set `C`, exactly one of these classifications holds.

`p` is **fully available** exactly when no consumed path in `C`:

- equals `p`;
- is an ancestor of `p`; or
- is a descendant of `p`.

`p` is **partially available** exactly when:

- no consumed path equals `p` or is an ancestor of `p`; and
- at least one consumed path is a strict descendant of `p`.

`p` is **unavailable** exactly when some consumed path equals `p` or is an ancestor of `p`.

These classifications are mutually exclusive and exhaustive for every structurally valid path.

Consequently, the complete root path `[]` is:

- fully available exactly when `C` is empty;
- unavailable exactly when `C` contains `[]`; and
- partially available otherwise.

A partially available record path may contain fully available descendants that are structurally disjoint from every consumed descendant. Static traversal through a partially available ancestor does not itself recreate or observe the complete ancestor value.

## Consuming one path

A source operation may **consume** the complete owned value at path `p` through this relation only when `p` is fully available immediately before the operation.

Successful consumption:

1. transfers or otherwise ends ownership of exactly the complete source value at `p` as defined by the consuming operation;
2. adds `p` to the consumed-path set; and
3. changes no structurally disjoint path merely because `p` was consumed.

Because `p` was fully available, the prior consumed-path set contains no path comparable with `p`. Adding `p` therefore preserves the prefix-free invariant.

After successful consumption:

- `p` and every descendant of `p` are unavailable;
- every proper ancestor of `p` is partially available unless another accepted operation later replaces or ends that structural root; and
- every structurally disjoint path retains its prior availability classification.

This relation does not decide when an operation chooses consumption. Owned-value duplicability and operation-specific duplicate-versus-consume selection are owned by `types.md` and the applicable operation owner.

## Non-consuming duplicate use

When another accepted source owner has selected a non-consuming owned-value duplicate of the complete value at path `p`, that use is source-valid through this structural relation only when `p` is fully available immediately before production.

Successful duplicate production leaves the consumed-path set unchanged.

This document does not define which source types are duplicable or what preserves their semantic value. `types.md` owns represented owned-value duplicability, and the operation-specific owner determines whether and how that capability applies.

A non-consuming duplicate of one path does not make an unavailable or partially available path available and does not change ownership of any ancestor, descendant, or sibling path.

## Remaining ownership frontier

For cleanup or ownership-ending selection, every structurally valid path `p` has one deterministic **remaining ownership frontier** derived only from:

- the root type and structural field identities/order from `types.md`; and
- the current consumed-path set.

The frontier `frontier(p)` is defined recursively:

1. if `p` is unavailable, `frontier(p)` is empty;
2. if `p` is fully available, `frontier(p)` contains exactly `p`;
3. if `p` is partially available, `type(p)` MUST be one nominal record type; visit that record's fields in **reverse record declaration order** and concatenate `frontier(p + [field])` for those child paths in that order.

The remaining ownership frontier of the complete structural root is `frontier([])`.

Every frontier member is a maximal fully available source subvalue under the selected path. Frontier members are pairwise structurally disjoint. No consumed subvalue is a frontier member, and every still-owned structural subvalue lies within exactly one frontier member.

The structural frontier is source-semantic cleanup/replacement selection. The applicable lifecycle/execution owner decides when the frontier is selected and when each member's ownership ends. This document fixes frontier member order but does not define lexical-scope ordering between distinct bindings or ordering between different transient/external owners.

## Complete, mixed, and empty frontiers

The recursive frontier relation yields these general consequences without a second special-case algorithm:

- when the consumed-path set is empty, the complete-root frontier contains exactly `[]`;
- when one or more nested paths are consumed, the frontier contains exactly the maximal still-owned disjoint subvalues reached by recursively descending partially available ancestors in reverse record declaration order;
- when the complete root path `[]` is consumed, the frontier is empty;
- when separate exhaustive operations consume every structurally owned subvalue below the root without consuming `[]` itself, the complete-root frontier may nevertheless be empty; an empty frontier does not imply or synthesize a whole-root consumption event.

Pattern presentation order, source lexical declaration order, and execution order do not replace record structural declaration order for recursive frontier selection.

## Zero-field and zero-leaf values

A fully available zero-field nominal record path is one complete owned source value and contributes that path as one remaining-frontier member.

A recursively zero-leaf record value likewise remains a source-owned value even when no lower scalar leaf exists.

Therefore a zero-field or recursively zero-leaf value may be:

- fully available;
- consumed and unavailable;
- contained by a partially available ancestor;
- selected as a remaining-frontier member; or
- have its source ownership ended by an applicable execution owner.

These source facts MUST NOT be reconstructed from whether a Core or backend representation has a scalar destruction effect.

Ending ownership of one zero-leaf source frontier member may faithfully refine to no lower scalar destruction operation when the applicable Core destruction domain is empty. That lower erasure does not retroactively remove or change source ownership state.

## Structural replacement and subpath installation boundary

This structural owner does not itself authorize assignment or another replacement operation. An operation owner may install a complete new value only when its own semantics explicitly authorize that replacement/reinitialization and define the commit point at which this structural relation is consumed.

### Complete-root replacement

A consuming owner may replace a structural root only when its own semantics explicitly authorize replacement. When such an owner establishes a new complete owned root value, it establishes a fresh empty consumed-path set for that root.

`local-bindings.md` uses this boundary for successful whole-binding assignment/reinitialization. `references.md` uses it for successful complete-referent `*r = value` replacement, whether the selected target is a local binding root or a replacement-capable parameter's external referent root. `raw-pointers-unsafe.md` uses it for successful raw replacement of a local-root pointee. Field-receiver and pattern-scrutinee transients are not replaceable under their represented relations.

Replacement selects the then-current remaining frontier at the commit point defined by the operation owner; this document does not move that selection earlier.

### Bounded non-empty subpath installation

For one structurally valid **non-empty** path `p`, an accepted operation owner may install one complete new value of exact source type `type(p)` at `p` only through this relation.

Let `C` be the root's consumed-path set on the operation's normal successful continuation immediately before replacement commits. The installation is structurally admitted exactly when no member of `C` is a strict ancestor of `p`.

Therefore:

- a fully available `p` is admitted for replacement;
- an exactly consumed `p`, where `p` itself is in `C`, is admitted for reinitialization;
- a partially available `p`, where one or more strict descendants of `p` are in `C` and no equal/ancestor path is consumed, is admitted for reconstruction; and
- a `p` enclosed by a consumed strict ancestor is rejected.

The rejected ancestor-consumed case does not implicitly split that consumed ancestor into consumed sibling/complement paths and does not reconstruct ownership from below. Such a structural state-splitting relation is not represented by this revision.

At the commit point the applicable execution owner selects and ends only the then-current `frontier(p)` before installing the preserved new complete `type(p)` value. Already consumed descendants therefore contribute no frontier member and are not ended twice.

After successful installation, the new consumed-path set is exactly:

```text
C' = { c in C | p is not equal to c and is not an ancestor of c }
```

Equivalently, remove from `C` exactly every consumed path equal to `p` or strictly below `p`; preserve every structurally disjoint consumed path unchanged.

Because the admitted pre-commit state contains no consumed strict ancestor of `p`, this transition preserves prefix-freedom. It makes `p` fully available, leaves a previously fully available target's consumed-path state unchanged, removes exactly `p` for exact-path reinitialization, removes exactly consumed descendants for partial reconstruction, and never normalizes a disjoint region.

The operation owner decides how the target path is selected, which mutation/replacement permission is required, how the new value is produced, and what fault/divergence or alias-authority checks occur before commit. This structural relation supplies only the target-state admission, remaining-frontier selection, and successful consumed-path transition.

## Definite source validity and control flow

Structural availability is statically required source validity for operations that consume or duplicate a path through this relation. Structural subpath installation likewise requires its operation owner to prove the exact admitted pre-commit state; an invalid installation is not converted into a defined runtime moved-state fault merely because a physical implementation could track moves dynamically.

[Source control flow](control-flow.md) owns represented multi-path and cyclic consumers. It treats every continuing binding structural root and every continuing replacement-capable external referent root as definite structural state.

For a normally completing represented conditional:

- when two outcomes both continue normally, each continuing root's consumed-path set MUST be exactly equal on both outcomes before one definite successor is established;
- when exactly one outcome continues normally, that outcome's exact state continues without comparison against a non-returning outcome; and
- when neither outcome continues normally, no following structural state exists.

For represented bounded `while`, let `H` be the complete enclosing state immediately before condition validation and `C` the state after successful condition validation:

- the false normal successor is exactly `C`;
- the true outcome validates the body from a copy of `C`;
- a normal body backedge or `continue` MUST restore every applicable continuing root to the exact state it had in `H` before the transfer is admitted;
- `break` MUST satisfy the exact loop-exit state required by `C`; and
- a body with no applicable normal transfer contributes no corresponding state comparison.

These control-flow rules consume this structural state relation; they do not add a union, intersection, normalization, maybe-owned state, runtime flag, automatic edge repair/reset, widening operation, or generic fixed-point inference.

Future refutable matches, catch/recovery forms, additional loop forms, or other multi-path control-flow owners MUST independently define how structural ownership is made definite at their applicable successors. Their rules are not implied by the represented conditional/loop relations.

## Direct consumers

### Function-local bindings

`local-bindings.md` associates one structural owned-value root with every in-scope represented parameter/local binding.

Binding lifecycle establishes when that state begins, persists, resets, or ends. Successful whole-binding replacement uses the complete-root replacement boundary above. Successful bounded direct binding-root field assignment uses the non-empty subpath-installation relation above at its operation-defined post-RHS commit point. This owner supplies only the structural mathematics applied to that state.

### Replacement-capable external referents

`references.md` associates one non-binding structural owned-value root with every incoming `ExclusiveReplaceRef(T)` parameter.

The root begins fully available at call entry, complete-referent Move may consume it, complete-referent replacement resets it to complete ownership, explicit child reborrows select that same domain, control flow carries its exact state, and normal completion requires it to be fully available. This structural owner supplies only the state mathematics. This revision does not by itself authorize projected replacement through a safe reference merely because the generic non-empty installation relation exists.

### Field-value access

For a binding-root receiver, `field-access.md` resolves one non-empty structural path from the selected binding root, requires its final selected path to be fully available, and selects duplicate or consume according to the final field type's duplicability. Field lookup/accessibility, safe-authority compatibility, and producer semantics remain owned there and in `references.md`.

For a producer-backed receiver, successful receiver production establishes one non-binding field-receiver transient structural root with an empty consumed-path set. `field-access.md` applies the source-selected duplicate-or-consume consequence to its resolved path and selects the transient's remaining frontier through this document; `function-execution.md` owns the transient's dynamic ending and cleanup order.

For bounded direct binding-root field assignment, `field-access.md` supplies the existing nominal field-resolution/accessibility relation while `local-bindings.md` and `function-execution.md` own assignment permission, RHS production, and commit ordering. This document supplies only the selected non-empty path's structural installation state.

### Record patterns

`patterns.md` resolves structural binding-leaf paths.

For a direct binding-root pattern, those paths use the root binding's structural ownership state plus the direct safe-authority compatibility requirement consumed from `references.md`.

For a successful producer-backed pattern, the produced transient begins as one complete structural owned-value root with an empty consumed-path set, pattern leaf production applies selected duplicate/consume transitions to that transient, and its remaining ownership frontier is selected through this document before the transient ends.

### Function execution and cleanup

`function-execution.md` decides when represented binding/transient ownership ends and when complete-root or bounded subpath replacement frontiers are ended. It also owns ordering between distinct bindings, lexical scopes, activations, producer transients, safe-reference replacement, raw replacement, and bounded binding-root field replacement.

### Conditional and bounded-loop control flow

`control-flow.md` compares complete structural ownership states of continuing binding and external-referent roots across normal conditional outcomes and validates bounded-loop backedge/continue/break targets against their exact required states.

This document supplies the state being compared. It does not select arms or loop outcomes, define lexical scopes, or decide control-flow validity beyond the structural relations consumed by `control-flow.md`.

## Source/Core separation

Source structural ownership is independent of Core proving representation.

Core path state, `Live`/`Dead`, Never-initialized state, scalar copyability, destruction domains, local identifiers, projections, and Core external-referent state are not source structural ownership authority.

A faithful lowering MAY map resolved source paths to Core structural projections after source validation and MAY omit lower destruction for source-owned zero-leaf frontier members where Core has no scalar destruction domain. For an accepted direct binding-root subpath assignment, it MAY refine the selected source path to an existing projected Core ordinary-assignment destination only after the source owner has proved the structural installation relation above. It MUST preserve replacement-capable external-referent Move/restore and normal-return obligations through the accepted Core reference/direct-call relation. It MUST NOT use lower liveness or copyability to reconstruct source path availability, subpath-installation admission, consumed-path reset, duplicate-versus-consume selection, remaining-frontier membership, represented control-flow validity, or source normal-completion restoration.

## Further boundaries

This revision does not define general source places/lvalues, field-relative safe-reference replacement/access/reborrow, named lifetimes, pointer provenance, interior mutability, custom destructors, must-consume policy, arbitrary temporary lifetime extension, structural state splitting beneath a consumed ancestor, unequal-state/path-dependent conditional joins, additional loop forms or general loop fixed-point inference, refutable-match joins, exception/catch state merges, ABI/layout, parser/HIR/Core MIR representation, runtime moved-state flags, or backend storage.

Those concerns require their own accepted owners and may consume this structural relation only when their canonical semantics explicitly say so.