# Source Structural Ownership

Status: **provisional normative; incomplete**

This document owns the represented source-semantic relation for structural owned-value roots, structural source paths, structural ownership state, path availability, consuming and non-consuming path use requirements, and deterministic remaining-ownership frontiers.

It consumes represented source type identity, nominal record identity, record field identity and source structural field order from [Source type foundation](types.md). It does not redefine those owners.

[Source function-local bindings](local-bindings.md) instantiates this relation for each represented parameter/local binding and owns binding identity, lexical scope, lookup, assignment mutability, declaration lifecycle, ordinary whole-binding owned-value use, assignment legality, and reset points. [Source field-value access](field-access.md) consumes this relation for binding-rooted selected paths. [Source patterns](patterns.md) consumes this relation for direct binding-root pattern paths and for the structural ownership state of a producer-backed pattern scrutinee transient. [Source function execution](function-execution.md) consumes remaining-ownership frontiers when represented binding or transient ownership ends.

This document does not define lexical bindings, names, mutability, field lookup/accessibility, pattern syntax, source type duplicability selection, custom destruction, references/borrows, a source place/lvalue category, physical storage, layout, Core MIR liveness, or an implementation representation.

## Structural owned-value roots

A **structural owned-value root** consists of:

- exactly one represented source value type, the **root type**; and
- exactly one structural ownership state defined below.

A structural owned-value root is semantic ownership of one complete source value. The relation does not require the root to have a lexical identifier, source binding identity, physical address, storage identity, HIR local, Core local, or another source-observable identity.

Represented parameter/local bindings instantiate persistent structural owned-value roots through `local-bindings.md`. A successful producer-backed record-pattern scrutinee instantiates one non-binding transient structural owned-value root through `patterns.md` and `function-execution.md`.

The root relation itself does not decide how a value is produced, when a binding or transient begins or ends, or which source operation is permitted to use a path. Those are owned by the applicable consuming source operation.

## Structural source paths

A **structural source path** is a finite sequence of resolved nominal-record field identities beginning at one structural owned-value root's root type.

The empty path `[]` denotes the complete root value.

A non-empty path `[f0, f1, ..., fn]` is structurally valid exactly when:

1. the root type is a nominal record type containing field identity `f0`;
2. for every later field `fi`, the source type reached by the preceding path prefix is a nominal record type containing `fi`; and
3. each path step uses that selected field's declared source type from `types.md` as the type reached by the extended prefix.

For every structurally valid path `p`, define `type(p)` as the represented source type reached by the complete path. `type([])` is the root type.

Structural paths use source-semantic nominal record and field identity. They are not identifier spellings, parser nodes, compiler field indices, Core projections, byte offsets, addresses, physical storage regions, or ABI layout.

A path `a` is an **ancestor** of path `b` exactly when `a` is a proper prefix of `b`. Paths are **structurally disjoint** exactly when neither path is equal to nor an ancestor of the other.

The empty path is therefore an ancestor of every non-empty path.

## Structural ownership state

Each structural owned-value root has one finite **consumed-path set** containing structurally valid paths rooted at that value.

The consumed-path set MUST be prefix-free: no two distinct members may be equal, ancestor/descendant, or otherwise prefix-comparable.

An empty consumed-path set denotes complete initial ownership of the root value. The relation does not imply that every root is always initialized with that state; the applicable lifecycle owner establishes when one complete owned value begins. `local-bindings.md` and `patterns.md` define the represented initial-state boundaries that consume this relation.

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

This document does not define which source types are duplicable or what preserves their semantic value. [Source type foundation](types.md) owns represented owned-value duplicability, and the operation-specific owner determines whether it consumes that capability.

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

The structural frontier is source-semantic cleanup selection. The applicable lifecycle/execution owner decides when the frontier is selected and when each member's ownership ends. This document fixes the frontier member order but does not define lexical-scope ordering between distinct bindings or ordering between different transient owners.

## Complete, mixed, and empty frontiers

The recursive frontier relation yields these general consequences without a second special-case algorithm:

- when the consumed-path set is empty, the complete-root frontier contains exactly `[]`;
- when one or more nested paths are consumed, the frontier contains exactly the maximal still-owned disjoint subvalues reached by recursively descending partially available ancestors in reverse record declaration order;
- when the complete root path `[]` is consumed, the frontier is empty;
- when separate exhaustive operations consume every direct or nested owned leaf without consuming `[]` itself, the complete-root frontier may nevertheless be empty; an empty frontier does not imply or synthesize a whole-root consumption event.

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

Ending ownership of one zero-leaf source frontier member may faithfully refine to no lower scalar destruction operation when the applicable Core destruction domain is empty. That lower erasure does not retroactively remove or change the source ownership state.

## Structural replacement boundary

This structural owner does not define assignment or another replacement operation.

A consuming owner may replace a structural root only when its own semantics explicitly authorize replacement. When such an owner establishes a new complete owned root value, it MAY establish a fresh empty consumed-path set as part of that owner's accepted lifecycle transition.

`local-bindings.md` uses this boundary for successful whole-binding assignment/reinitialization. A pattern scrutinee transient is not replaceable under the represented pattern relation.

## Definite source validity and future control flow

Structural availability is statically required source validity for operations that consume or duplicate a path through this relation. An invalid use is not converted into a defined runtime use-after-consumption fault merely because a physical implementation could track moves dynamically.

A future branch, loop, refutable-match, catch, or other multi-path control-flow owner MUST define how structural ownership states are made definite at successor program points and how path-dependent remaining ownership is cleaned. This document does not define a control-flow join by union, intersection, runtime flag, or another mechanism.

In particular, a consumed-path union is not silently established as a general branch-join rule by the existence of this straight-line structural relation.

## Direct consumers

### Function-local bindings

`local-bindings.md` associates one structural owned-value root with every in-scope represented parameter/local binding.

Binding lifecycle establishes when that state begins, persists, resets, or ends. This owner supplies only the structural mathematics applied to that state.

### Binding-rooted field-value access

`field-access.md` resolves one non-empty structural path from a binding root, requires its final selected path to be fully available, and selects duplicate or consume according to the final field type's duplicability. Field lookup/accessibility and producer semantics remain owned there.

### Record patterns

`patterns.md` resolves structural binding-leaf paths.

For a direct binding-root pattern, those paths use the root binding's structural ownership state.

For a successful producer-backed pattern, the produced transient begins as one complete structural owned-value root with an empty consumed-path set, pattern leaf production applies selected duplicate/consume transitions to that transient, and its remaining ownership frontier is selected through this document before the transient ends.

### Function execution and cleanup

`function-execution.md` decides when represented binding or transient ownership ends and consumes the applicable remaining frontier. It also owns ordering between distinct bindings, lexical scopes, activations, and producer transients.

## Source/Core separation

Source structural ownership is independent of Core proving representation.

Core path state, `Live`/`Dead`, Never-initialized state, scalar copyability, destruction domains, local identifiers, and projections are not source structural ownership authority.

A faithful lowering MAY map resolved source paths to Core structural projections after source validation and MAY omit lower destruction for source-owned zero-leaf frontier members where Core has no scalar destruction domain. It MUST NOT use lower liveness or copyability to reconstruct source path availability, duplicate-versus-consume selection, or remaining-frontier membership.

## Further boundaries

This revision does not define general source places/lvalues, field assignment, partial-field reinitialization, references, borrows, lifetimes, pointer provenance, interior mutability, custom destructors, must-consume policy, arbitrary temporary lifetime extension, branch/loop joins, refutable matching, exception/catch state merges, ABI/layout, parser/HIR/Core MIR representation, runtime moved-state flags, or backend storage.

Those concerns require their own accepted owners and may consume this structural relation only when their canonical semantics explicitly say so.