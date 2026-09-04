# Source Function-Local Bindings

Status: **provisional normative; incomplete**

This document owns the represented source semantics for function-local binding identity, lexical scope and lookup precedence, binding assignment mutability, binding lifecycle, ordinary whole-binding owned-value use, whole-binding assignment legality, bounded binding-root field assignment/reinitialization legality, safe-reference/raw-pointer local contextual integration, and the points at which a binding's structural ownership state begins, persists, resets, or ends.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module lookup from [Source names and modules](names-modules.md), source value types and owned-value duplicability from [Source type foundation](types.md), structural paths, structural ownership state, path availability, consumption, remaining-ownership frontiers, complete-root replacement reset, and bounded non-empty subpath installation from [Source structural ownership](structural-ownership.md), callable parameter-slot types from [Source callables](callables.md), safe-reference target/authority/carrier/lifetime and direct safe-authority compatibility rules from [Source safe references](references.md), and raw-pointer contextual admission, pointer-origin provenance, lexical target validity, and raw pointee operations from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md). It does not redefine those owners.

Represented binding-rooted field-path selection, direct field accessibility, final-field duplicate-or-consume value production, and bounded assignment-target field-path resolution/accessibility are owned by [Source field-value access](field-access.md). Represented recursive record-pattern selection, including bounded node-local rest/omission, and pattern-specific binding production are owned by [Source patterns](patterns.md). Represented source body attachment, dynamic activations, direct calls, owned argument/result transfer including safe-reference carriers and replacement-capable external referents, local initialization, whole-binding and bounded field-assignment replacement ordering, reference-relative replacement ordering, normal-continuation presence, lexical-scope and activation cleanup, return, recursion, divergence, defined-fault propagation, and raw-operation execution ordering are owned by [Source function execution](function-execution.md). Represented conditional selection, zero/one/two normal-outcome composition, bounded `while` condition/body selection, structural-state joins/backedges including external referent roots, and raw-pointer-origin joins/backedges are owned by [Source control flow](control-flow.md). Concrete parameter/local/pattern/value/call/field-value/assignment/block/conditional/while/return/reference/raw-pointer/unsafe spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define structural ownership mathematics, safe-reference formation/dereference/reborrow/replacement/authority semantics, raw-pointer formation/pointee access/unsafe admission semantics, normal-continuation presence, conditional or loop selection/successor composition, field lookup/accessibility, pattern structure, general expression evaluation, traits, ABI, Core liveness, or an implementation representation.

## Function-local binding identity

When a represented source function entity has a body under `function-execution.md`, that body has exactly one **parameter binding** corresponding to each callable-signature parameter slot.

Each parameter binding has:

- exactly the source value type of its corresponding signature parameter slot;
- one lexical identifier key governed by `lexical.md`;
- one stable source-semantic binding identity; and
- one assignment-mutability classification defined below.

Parameter lexical keys MUST be unique within one function body.

Parameter lexical keys, binding identities, and assignment-mutability classifications are body-local facts. They are not callable-signature identity or equality dimensions.

Parameter bindings and every represented function-local binding occupy one **function-local value-binding domain**.

A represented binding identity is independent of original identifier spelling, token/source offset, parser node, physical address, compiler collection index, HIR/Core identifier choice, runtime storage identity, source safe-reference authority identity, or source raw-pointer origin provenance.

For the function form represented by `concrete-syntax.md`, concrete parameter source order maps to callable parameter-slot order and each parameter identifier supplies the lexical key for its corresponding parameter binding. Every represented concrete parameter binding is immutable for assignment purposes, including parameters whose types are `SharedRef(T)` or `ExclusiveReplaceRef(T)`. `RawPtr(T)` is not parameter-admissible under `callables.md` in this slice.

A replacement-capable reference parameter's binding mutability remains immutable even though its reference permission permits complete-referent replacement. Parameter binding assignment and referent replacement are distinct semantic operations.

## Ordinary local declarations

A represented ordinary local declaration:

- belongs to exactly one lexical scope;
- introduces exactly one lexical identifier key and one stable local binding identity;
- has exactly one represented source value type;
- has exactly one initializer; and
- classifies the binding as immutable or mutable for assignment purposes.

Uninitialized ordinary local declarations are not represented.

The initializer is resolved and typed in the lexical environment that exists before the new binding is introduced. `function-execution.md` owns initializer evaluation and transfer. The binding enters scope only after successful initialization completes and therefore cannot be selected by lookup from its own initializer.

The concrete forms in `concrete-syntax.md` establish immutable `let name: Type = Value;` and mutable `let mut name: Type = Value;` bindings. This revision defines no inferred local type or uninitialized local form.

Safe-reference local admission is deliberately immutable-only:

- `let name: &T = Value;` may establish an immutable ordinary local when `SharedRef(T)` is valid under `references.md`;
- `let name: &mut T = Value;` may establish an immutable ordinary local when `ExclusiveReplaceRef(T)` is valid under `references.md`;
- `let mut name: &T = Value;` is source-invalid; and
- `let mut name: &mut T = Value;` is source-invalid.

This immutable-only rule is a source reference-lifetime/authority boundary. It does not redefine the general assignment-mutability classification of non-reference locals and does not imply a hidden `const` or type-level mutability dimension. In particular, `&mut T` denotes replacement capability over the referent, not mutability/rebinding of the reference local that stores the carrier.

For represented raw-pointer types, both ordinary local mutability classes are admitted when `RawPtr(T)` is valid under `raw-pointers-unsafe.md`:

- `let name: raw T = Value;` establishes an immutable raw-pointer local; and
- `let mut name: raw T = Value;` establishes a mutable raw-pointer local whose stored pointer value may later be replaced by ordinary whole-binding assignment.

Raw-pointer local mutability applies only to the stored pointer value. It does not grant pointee mutation authority. Every raw-pointer initializer additionally MUST satisfy the pointer-origin lexical target-validity relation from `raw-pointers-unsafe.md` for the complete static extent of the receiving local.

After successful initializer transfer, the new local begins with one complete structural owned-value root of its declared type and the initial empty consumed-path state from `structural-ownership.md`. When the initialized value is a safe reference, the local additionally stores the produced reference carrier whose authority/lifetime consequence is owned by `references.md`; when it is a raw pointer, source validation additionally retains the exact pointer-origin provenance owned by `raw-pointers-unsafe.md`. Neither relation introduces a second structural ownership state for the reference or pointer binding itself.

A replacement-capable reference value may also target a structural root external to the current activation. That external referent root is separate non-binding validation state owned by `references.md`; it is not the structural ownership state of the parameter/local binding that stores the reference carrier.

## Pattern-introduced local bindings

One source-valid record-destructuring declaration under `patterns.md` may introduce zero or more ordinary function-local bindings as one grouped declaration boundary.

For every pattern binding leaf, `patterns.md` supplies:

- the introduced lexical key;
- the exact selected source type; and
- the duplicate-or-consume production consequence that yields the binding's initial owned value.

`patterns.md` also supplies the complete declaration's retained binding-leaf source order. This binding owner uses that order as the declaration order of the introduced bindings.

This binding owner supplies each introduced binding with one stable source-semantic binding identity and classifies it as immutable for assignment purposes.

Before any pattern binding is introduced:

- all introduced lexical keys MUST be pairwise distinct across the complete pattern tree;
- every introduced key MUST satisfy the overlapping-shadow prohibition below against the pre-declaration lexical environment; and
- the complete declaration MUST have passed the pattern structure/type/accessibility validation owned by `patterns.md`.

If any introduced key is invalid, the complete declaration is rejected. It introduces no subset of the intended bindings and does not create a partially extended lexical environment.

All bindings introduced by one successful record-destructuring declaration enter scope **together after the complete declaration finishes**, including any producer-backed transient completion required by `patterns.md` and `function-execution.md`. None participates in lookup while that same declaration is validating or producing its binding values.

The binding-leaf source order defined by `patterns.md` is the declaration order of the introduced bindings for lexical cleanup composition. Pattern structure does not change nominal record structural field order from `types.md`.

Each successfully established pattern binding begins with one complete structural owned-value root of its exact binding type and the initial empty consumed-path state from `structural-ownership.md`.

Represented nominal record fields cannot have `SharedRef(T)`, `ExclusiveReplaceRef(T)`, or `RawPtr(T)` type, so the represented record pattern relation cannot introduce a safe-reference or raw-pointer binding in this slice. No borrow-binding or pointer-binding pattern mode is implied.

A pattern with no binding leaves introduces no function-local binding and therefore does not change the lookup environment by itself.

## Abstract lexical scopes

A represented function body has one root lexical scope. A represented nested block establishes one child lexical scope of its containing lexical scope. The resulting lexical scopes form a finite rooted tree.

The root body braces in `concrete-syntax.md` delimit the root lexical scope. Each concrete `BlockStatement` establishes exactly one child lexical scope containing its enclosed `BodyStatement` sequence and optional terminal `ReturnStatement`, and ending at that block's closing boundary. Recursively nested block statements therefore establish descendant lexical scopes. An `unsafe` block is one such ordinary child lexical block plus the separate unsafe-admission fact owned by `raw-pointers-unsafe.md`.

Each explicit represented conditional arm is one ordinary `BlockStatement` and therefore one child lexical scope. A then arm and explicit else arm of the same conditional are sibling scopes. An omitted else introduces no synthetic lexical scope under `control-flow.md`. Each represented `while` body is likewise one ordinary `BlockStatement` child scope; repeated dynamic iterations re-enter that same static source scope rather than creating new source binding identities.

The semantic scope tree does not prescribe parser nodes, source ranges, HIR scope identifiers, Core blocks, physical storage lifetime, or a physical address for a borrow/raw-pointer target.

A parameter binding belongs to the function root scope and is in scope throughout the represented function body, including descendant lexical scopes while those scopes are active.

An ordinary local or pattern-introduced local binding is in scope from immediately after its successful declaration/initialization boundary through the end of its containing lexical scope, including descendant lexical scopes while execution remains in that activation. A return terminates the activation under `function-execution.md` rather than creating a later point in the ended scope.

These existing containment/cleanup relations are consumed by `references.md` to prove the represented implicit lexical safe-reference lifetime and by `raw-pointers-unsafe.md` to require that every pointer local's target extent contain that pointer local's complete extent. Child safe-reference locals end before their earlier parent/reference target extent. This binding owner does not add lifetime names, authority sets, pointer-origin sets, or a second scope tree.

## Function-local shadowing and key reuse

**Overlapping function-local shadowing is forbidden.**

A parameter/local declaration MUST NOT introduce a lexical identifier key equal to the key of another parameter/local binding whose lexical scope contains the declaration point.

For one grouped record-destructuring declaration, this requirement applies to every binding leaf against the pre-declaration lexical environment, and all binding-leaf keys in that declaration MUST also be pairwise distinct.

Consequently:

- a local cannot shadow a parameter;
- a nested local cannot shadow an enclosing local;
- two sequential locals in the same continuing lexical scope cannot reuse one key;
- two bindings introduced by one pattern cannot share a key; and
- disjoint sibling lexical scopes, including explicit sibling conditional arms, MAY independently introduce the same key because their binding scopes do not overlap.

This prohibition applies only inside the function-local value-binding domain. A function-local binding key MAY equal a module-level declaration key.

## Function-local lookup precedence

Within a represented function body, an **unqualified function-body identifier lookup** that participates in the function-local value-binding domain consults active parameter/local bindings first.

If exactly one active parameter/local binding has the requested lexical key, lookup resolves to that binding. The consuming source form then determines whether the selected binding is a valid entity category for that operation.

Only when no active parameter/local binding resolves the key does lookup fall through to the accepted same-module relation in `names-modules.md`.

Lookup MUST NOT skip an active function-local binding merely because the consuming context would prefer a module-level entity of another category.

The concrete whole-binding value use, binding-rooted `FieldValueUse` root, direct binding-root pattern scrutinee, whole-binding or bounded binding-root field assignment target, unqualified direct-call target, safe-reference root/reborrow/dereference/reference-replacement operands, raw-address target, raw-move pointer operand, and raw-assign pointer operand consume this precedence. A wrong-category selected entity is rejected rather than bypassed.

For a bounded binding-root field assignment target, the first identifier resolves through this relation to one parameter/local binding; `field-access.md` then resolves the one-or-more field selectors from that binding's declared type. The selected binding still MUST satisfy the assignment-mutability rule below. No module-level fallback, qualified assignment root, arbitrary receiver, or general lvalue lookup is introduced.

For Shared root `&x` or `&x.field...`, `references.md` additionally requires the root identifier to resolve to one active parameter or ordinary local binding and owns selection of the complete root or bounded structural field path plus the exact Shared referent/accessibility/availability requirements. For root `&mut x`, that owner additionally requires one active mutable ordinary local binding with a replacement-reference-admissible referent and selects only its complete root.

For `*r`, `&*r`, `&mut *r`, and the destination reference binding in `*r = Value;`, `references.md` additionally requires the resolved entity to be one active safe-reference parameter/local binding with the exact permission/referent required by that operation. These bounded forms do not perform a general dereference-place or arbitrary-value lookup.

For raw address formation, `raw-pointers-unsafe.md` additionally requires the resolved target to be one active parameter or ordinary local binding of a first-slice raw-pointee-admissible type and selects only its complete root. For raw move/assign, that owner requires the pointer operand binding to have exact type `RawPtr(T)` and consumes its retained exact pointer origin.

A nominal record-pattern head is not a function-local value-binding lookup. `patterns.md` defines each represented record-pattern head through same-module nominal-record declaration lookup independently of active local bindings with equal keys.

Source-unit module aliases remain the distinct qualified-lookup mechanism owned by `names-modules.md`. The concrete `alias::member` direct-call target resolves through that mechanism rather than this unqualified lookup.

Beyond the represented two-part module alias/member qualification, operation-specific field selectors, bounded record-pattern field selection, bounded binding-root field assignment, bounded safe-reference root/dereference/reborrow/replacement lookup, and bounded raw-pointer root/pointer-operand lookup above, this revision defines no arbitrary member lookup, nested module paths, labels, generic parameters, lifetime names, methods, associated items, or another future name domain.

## Binding assignment mutability

Every represented parameter/local binding is exactly one of:

- **immutable**; or
- **mutable**.

Assignment mutability is a binding property independent of source type identity, structural ownership state, source owned-value duplicability, callable-signature identity/equality, safe-reference alias authority/permission, and raw-pointer origin provenance.

Consuming an owned value from an immutable binding, including a represented structural subvalue when `field-access.md` or `patterns.md` permits that consumption, is valid when the applicable safe-authority compatibility requirement is also satisfied. Immutability restricts ordinary whole-binding and binding-root field assignment/reinitialization; it does not require the binding to retain ownership of every subvalue and it is not raw target-access authority.

Represented parameters are immutable. Ordinary locals are immutable unless their concrete declaration carries `mut`. Every binding introduced by the represented record pattern is immutable. No parameter-mutability or pattern-binding-mutability form is represented. Therefore the current concrete bounded field-assignment form can successfully target only a mutable ordinary local, although the semantic lookup relation remains parameter/local and rejects an immutable parameter through the ordinary mutability rule.

Every represented local whose declared type is `SharedRef(T)` or `ExclusiveReplaceRef(T)` MUST be immutable; the otherwise represented mutable-local form is invalid for either safe-reference type.

A local whose declared type is `RawPtr(T)` MAY be immutable or mutable. A mutable raw-pointer local may be ordinarily assigned another exact `RawPtr(T)` value only when the incoming pointer origin satisfies the lexical target-validity rule from `raw-pointers-unsafe.md` for the complete receiving-local extent.

Assignment to any immutable binding is source-invalid regardless of whether its complete structural root or a selected structural subpath is fully available, partially available, or unavailable.

Binding mutability does not itself replace a value or restore ownership. Replacement is an explicit ordinary assignment operation under the rules below. Replacement capability carried by `ExclusiveReplaceRef(T)` separately permits `*r = Value;` to replace the referent even though the reference binding `r` itself is immutable. Unsafe raw replacement through a pointer may likewise replace an immutable **pointee target** because `raw-pointers-unsafe.md` deliberately does not consume ordinary target-binding assignment mutability as a precondition. Neither operation is whole-binding or binding-root field assignment to the reference/pointer binding.

## Binding structural ownership state

Every in-scope represented parameter/local binding owns exactly one structural owned-value root under `structural-ownership.md` whose root type is the binding's declared source type.

This document owns only the binding lifecycle around that structural state:

- successful parameter transfer establishes the parameter with complete initial ownership;
- successful ordinary local initialization establishes the local with complete initial ownership;
- successful pattern binding production establishes each new pattern binding with complete initial ownership;
- represented consuming/duplicating operations act on the binding's structural state only through their canonical operation owners and `structural-ownership.md`;
- successful whole-binding replacement establishes a fresh complete structural ownership state for the replacement value;
- successful bounded binding-root field assignment applies the canonical non-empty subpath-installation transition from `structural-ownership.md` to the binding's existing root state; and
- lexical/activation termination ends whatever binding ownership remains according to `function-execution.md`.

Safe-reference authority/carrier state, replacement-capable external referent structural state, and raw-pointer origin provenance are deliberately distinct facts. `references.md` owns safe authority and external-referent state; `raw-pointers-unsafe.md` owns pointer origin. Root safe-reference formation and raw address formation leave the target binding's structural ownership state unchanged. Dereference Move through a replacement-capable reference updates the actual local-root structural state when the reference targets a local binding. Raw ownership move and raw replacement likewise alter the target structural state only through the explicit transitions consumed by their canonical owners.

Entering or normally exiting a child lexical scope does not itself change the structural ownership state, external-referent state, safe authority, or pointer-origin provenance of an ancestor/enclosing domain. Valid ownership transitions, reference child lifecycle, pointer retargeting, or assignment affecting an enclosing domain inside the child remain in force at the following parent-scope program point when the applicable control-flow relation admits that normal continuation. Child reference-local cleanup may end a child authority and thereby restore parent reference-relative authority before the enclosing normal outcome is formed.

Structural source paths, prefix-free consumed-path state, fully/partially/unavailable classification, path consumption, bounded subpath-installation state, and recursive remaining-frontier selection are defined only by `structural-ownership.md`. They are not redefined here.

For represented statement-level conditionals, `control-flow.md` owns normal-continuation composition for enclosing binding states and replacement-capable external referent states:

- when two normal outcomes meet, every continuing structural root must have exactly equal structural ownership state and continues with that common state;
- when exactly one normal outcome exists, every continuing structural root continues with exactly the state from that sole normal outcome, without comparison against a non-normal outcome; and
- when zero normal outcomes exist, there is no following structural state because the conditional has no normal continuation.

That same control-flow owner additionally consumes `raw-pointers-unsafe.md` to require exact continuing pointer-origin equality for every enclosing raw-pointer binding across two normal outcomes; a sole normal outcome carries its exact origin forward without comparison.

For represented bounded `while`, `control-flow.md` likewise owns the complete binding/external-referent structural and raw-pointer-origin state relation. Let `H` be the complete enclosing environment immediately before condition evaluation and let successful condition validation produce `C`:

- the false loop outcome continues normally with exactly `C`;
- the true outcome validates one ordinary child block from `C`;
- an ordinary normal body backedge and `continue` MUST restore every applicable structural root to exactly its `H` state and every continuing raw-pointer origin to exactly its `H` origin;
- `break` MUST carry every applicable structural root and raw-pointer origin exactly in the state required by `C`;
- a body/path with no applicable normal transfer contributes no corresponding state comparison; and
- assignment/replacement rules remain explicit: only an accepted source replacement may restore consumed structural ownership before an edge, and immutable bindings receive no implicit binding replacement or subpath reinitialization.

Safe-reference authority/delegation state does not introduce a general control-flow lattice. Immutable reference locals, non-copyable replacement-capable carrier movement, explicit reborrow, lexical child cleanup, and the no-field/no-result/no-rebinding restrictions make persistent carrier/authority consequences definite through existing binding ownership plus the sequential reference relation. `control-flow.md` owns the exact composition boundary.

Raw-pointer origin is an additional exact provenance fact, not a structural ownership path set. The raw-pointer/control-flow relation likewise adds no origin union, set, maybe-origin state, widening, fixed point, or NLL lattice.

This binding owner does not derive a successor or backedge state by union, intersection, normalization, widening, lower Core path state, fixed-point iteration, or another merge rule.

Future refutable matches, catch/recovery forms, additional loop forms, or other control-flow forms require their own accepted definite-state relations; this document adds none beyond the represented conditional/while relations owned by `control-flow.md`.

## Ordinary whole-binding owned-value use

A represented **ordinary whole-binding owned-value use** applies to the empty structural path of one selected parameter/local binding.

The complete root path MUST be fully available under `structural-ownership.md` immediately before the use.

If the binding's source type is duplicable under `types.md`:

1. require the canonical Shared safe-authority compatibility relation for direct access to that binding root;
2. produce another owned source value of that complete type through the accepted duplicability capability; and
3. leave the binding's structural ownership state unchanged.

If the binding's source type is non-duplicable:

1. require the canonical Exclusive safe-authority compatibility relation for direct access to that binding root;
2. transfer/consume the complete owned value through the empty structural path; and
3. apply the canonical successful-consumption transition from `structural-ownership.md`.

The safe-authority check concerns authorities targeting the binding being used. Moving or copying a safe-reference **carrier binding** is not direct access to that carrier's referent and does not conflict with the authority carried by its own value merely because that reference is active.

For a binding of exact type `SharedRef(T)`, the type is duplicable. The successful duplicate therefore has the carrier consequence owned by `references.md`: it creates another carrier naming the same Shared authority/target while retaining the stored source carrier. This is not a reborrow or new root authority.

For a binding of exact type `ExclusiveReplaceRef(T)`, the type is non-duplicable. Successful ordinary use moves the one stored carrier into the produced value and consumes the reference binding root, without copying the carrier, ending the authority when an active descendant still keeps it alive, or accessing the referent.

For a binding of exact type `RawPtr(T)`, the type is duplicable. The successful duplicate preserves the exact raw-pointer value and `PointerOrigin(binding)` provenance owned by `raw-pointers-unsafe.md`; it does not access the pointee or create any reference authority.

Ordinary whole-binding use of a partially available or unavailable complete root is source-invalid, not a defined runtime moved-state fault.

The concrete `IdentifierUse` value form maps to this operation after lookup resolves one parameter/local binding.

This relation does not define field-value production, record-pattern ownership, safe-reference dereference/reborrow, raw address formation, or raw pointee access. Those owners may use their own bounded receiving relations without first applying ordinary whole-binding use to the complete target root.

## Whole-binding assignment and reinitialization

A represented whole-binding assignment target MUST resolve through the function-local lookup relation above and MUST denote one represented parameter/local binding. The binding MUST be mutable.

Before RHS consequences can commit, the target must satisfy the canonical Exclusive safe-authority compatibility requirement from `references.md`: no active overlapping safe authority may target that complete root.

The RHS MUST produce exactly one owned source value whose type is exactly equal under `types.md` to the target binding's declared source type.

When the target binding has type `RawPtr(T)`, the produced RHS raw-pointer value additionally MUST carry an exact pointer origin whose target binding extent contains the complete receiving pointer-local extent under `raw-pointers-unsafe.md`. This source validity requirement applies before the new pointer value/origin becomes the target binding's continuing state.

The target may have a fully available, partially available, or unavailable complete structural root when assignment begins. Successful assignment always replaces/reinitializes the complete binding value. Safe authority is a separate rejection condition even when the current structural root would otherwise be replaceable.

The target remains in scope during RHS evaluation. Every RHS use observes the target's current structural ownership and safe-authority state. A consuming RHS may therefore change structural ownership state before replacement completes. A safe authority established or retained during RHS evaluation likewise remains controlling at the replacement point. For a raw-pointer assignment, RHS production similarly determines the exact incoming pointer origin before replacement commits.

After successful RHS production, satisfaction of any raw-pointer lexical target-validity requirement, and a second satisfaction of the canonical Exclusive safe-authority requirement at the actual replacement point, `function-execution.md` owns source-first replacement ordering:

1. select and end ownership of the target's then-current remaining old-value frontier through `structural-ownership.md`;
2. transfer the successfully produced replacement value into the target; and
3. establish a fresh complete structural ownership state with an empty consumed-path set.

For a `RawPtr(T)` target, the successful transfer also replaces the stored pointer-origin provenance with the incoming exact origin. Ending the old pointer value has no pointee effect under `raw-pointers-unsafe.md`.

Thus a mutable binding may be reinitialized from any represented structural ownership state only when the complete-root Exclusive safe-authority requirement succeeds both at admission and after RHS evaluation, while an immutable binding may not be ordinarily assigned in any state.

Safe-reference locals themselves cannot be whole-binding assignment targets because reference locals are immutable. Reference-relative `*r = Value;` remains a separate operation owned by `references.md`/`function-execution.md`; unsafe raw replacement of a pointee remains a separate operation and does not use this target-binding mutability rule.

A defined fault or divergence during RHS evaluation performs no replacement/reset merely because assignment was intended. Ownership, safe-reference authority/carrier, external-referent, and pointer-origin transitions that completed while evaluating the RHS remain in force under their existing owners.

This whole-binding relation defines no general source place/lvalue, plain-Exclusive replacement, interior mutability, raw pointee replacement, or destructuring assignment.

## Bounded binding-root field assignment and reinitialization

A represented **bounded binding-root field assignment** targets one non-empty structural field path `p` under one selected parameter/local binding root.

The root identifier MUST resolve through the same function-local lookup relation as whole-binding assignment and MUST denote one represented parameter/local binding. The binding MUST be mutable. Because represented parameters are immutable in the current concrete subset, a successful concrete field assignment currently targets a mutable ordinary local.

`field-access.md` resolves the one-or-more field selectors from the binding's exact declared type, requires every selector step to select one declared nominal-record field with the existing direct accessibility, and supplies the exact non-empty structural path `p` and final type `type(p)`. This operation does not first produce, duplicate, consume, or otherwise evaluate an intermediate field value merely to select the target.

The RHS MUST produce exactly one owned source value whose type is exactly equal under `types.md` to `type(p)`. No conversion, coercion, inferred target type, structural record equivalence, method/property setter, or independently mutable field relation is introduced.

Before RHS consequences can commit, the exact selected structural target `p` MUST satisfy the canonical Exclusive direct safe-authority compatibility requirement from `references.md`. An overlapping authority targeting an ancestor, equal path, or descendant blocks this direct replacement as that canonical relation requires; a structurally disjoint sibling authority does not spuriously block it.

The selected binding and field path remain statically identified while RHS evaluation proceeds. Every RHS use observes the binding's then-current structural ownership and safe-authority state. A consuming RHS may therefore consume the exact target, one or more descendants of the target, or structurally disjoint paths before replacement commits.

On the RHS producer's normal successful continuation immediately before replacement, let `C` be the binding root's resulting consumed-path set. The assignment MUST satisfy the bounded non-empty subpath-installation admission relation from `structural-ownership.md`: no member of `C` may be a strict ancestor of `p`.

Therefore the post-RHS target may be:

- fully available, for ordinary replacement;
- exactly consumed at `p`, for reinitialization; or
- partially available because strict descendants of `p` are consumed, for reconstruction.

If a strict ancestor of `p` remains consumed on that successful continuation, the assignment is source-invalid. It does not split that consumed ancestor into sibling/complement consumed paths, implicitly reconstruct the ancestor, or defer validity to a runtime moved-state check.

After successful RHS production, satisfaction of the post-RHS structural admission, and a second satisfaction of the canonical Exclusive safe-authority compatibility requirement at the actual replacement point, `function-execution.md` owns source-first replacement ordering:

1. select and end only the target's then-current `frontier(p)` through `structural-ownership.md`;
2. preserve already consumed descendants as already ended rather than destroying them again;
3. transfer the successfully produced complete replacement value into the exact selected target `p`; and
4. apply the canonical successful non-empty subpath-installation transition, removing from `C` exactly `p` and consumed descendants of `p` while preserving every structurally disjoint consumed path.

The resulting selected target `p` is fully available. The complete binding root may still be partially available because a structurally disjoint path remains consumed.

A defined fault or divergence during RHS evaluation performs no target frontier cleanup, value installation, or consumed-path reset merely because field assignment was intended. Structural, reference, external-referent, and pointer-origin transitions that completed while evaluating the RHS remain in force under their existing owners.

This relation introduces no qualified assignment root, arbitrary receiver, general place/lvalue, assignment expression, compound/destructuring assignment, reference-relative field assignment, projected safe-reference replacement, raw field/path replacement, interior-mutability rule, or structural state splitting beneath a consumed ancestor.

## Binding cleanup and discard boundary

When represented execution ends a binding's ownership, its remaining owned source subvalues are exactly the complete-root remaining ownership frontier selected by `structural-ownership.md` from the binding's then-current state.

`function-execution.md` owns when that frontier is selected and the ordering between bindings, scopes, parameters, activations, whole-binding assignment replacement, bounded binding-root field replacement, safe-reference referent replacement, raw replacement, normal return, and defined-fault cleanup.

When a binding's remaining owned value is a safe reference, ending that value additionally removes its source reference carrier at that existing cleanup point under `references.md`. Removing the final carrier may end its authority or may leave a carrierless ancestor authority alive while a descendant remains. The reference-specific consequence adds no custom cleanup body and does not access the referent.

When a binding's remaining owned value is a raw pointer, ending that value has no pointer-specific pointee effect under `raw-pointers-unsafe.md`. It neither changes target structural ownership nor ends any safe authority.

The existing reverse local/declaration and activation cleanup ordering is part of the represented lexical lifetime proof in `references.md` and lexical pointer-target validity in `raw-pointers-unsafe.md`: reference/pointer locals and child reference carriers end before the earlier or ancestor target/source extents whose validity their initialization/derivation consumed.

A binding is not source-invalid solely because one or more remaining owned subvalues are non-duplicable when its scope or activation terminates. This revision defines no source `drop` ability, must-consume classification, custom destructor, or unused-value prohibition.

Zero-field and recursively zero-leaf frontier members remain source-owned values even when faithful Core refinement emits no scalar destruction operation.

## Function, call, assignment, pattern, control-flow, reference, raw-pointer, and fault boundary

This document defines body-local binding identity, scope, lookup, assignment mutability, binding lifecycle around structural ownership, safe-reference/raw-pointer local integration, ordinary whole-binding use, whole-binding assignment legality/reset, and bounded binding-root field assignment legality/reset.

It does not redefine the execution relation owned by `function-execution.md`, including:

- function body execution and normal-continuation presence;
- direct-call argument evaluation and parameter transfer;
- safe-reference produced-carrier transfer/caller suspension/external-referent consequences;
- whole-binding and bounded binding-root field assignment RHS evaluation, old-value cleanup, and replacement transfer;
- safe-reference referent replacement ordering;
- raw replacement source-first execution ordering;
- result production and return transfer;
- dynamic activation identity or recursion;
- lexical-scope/caller/callee cleanup sequencing; or
- defined-fault propagation across activations.

It does not redefine safe-reference root formation, authority/carrier identity, dereference, reborrow, referent replacement, lexical lifetime validity, external-referent state, result provenance, or source-to-Core reference refinement from `references.md`.

It does not redefine raw address formation, raw pointee move/replacement, unsafe admission, pointer-origin validity, or source-to-Core raw refinement from `raw-pointers-unsafe.md`.

It does not redefine represented conditional or bounded-`while` condition/body selection, normal-successor composition, binding/external-referent structural-state equality, raw-pointer-origin equality, or loop backedge/transfer admission from `control-flow.md`.

It likewise does not redefine field identity/path resolution or accessibility from `field-access.md`, field-value production from that owner, pattern structure/ownership from `patterns.md`, or structural ownership mathematics from `structural-ownership.md`.

Indirect calls, function values, closures, plain-Exclusive source references, reference pass modes, lifetime names, unsafe callable contracts, broader panic/catch forms, and other future execution relations remain outside this owner.

## Implementation boundary

This revision does not add or require parser, lossless-syntax, HIR, Core MIR production, runtime, or backend representation.

A faithful implementation MAY retain structural ownership for bindings/external referents using resolved field indices or another implementation identity after source field resolution, but those representations are not source semantic identity. It MAY separately retain safe-reference authority/provenance and exact raw-pointer origin facts as required by their owners. Core path state, scalar liveness, Core reference-authority IDs, Core external regions, Core pointer-target metadata, or runtime storage identities MUST NOT become the source binding/external-referent ownership, reference-origin, or pointer-origin authority, including when a represented conditional establishes a normal successor or a represented `while` validates one transfer/backedge.

## Further boundaries

Beyond the represented concrete subset, this revision does not define type inference, assignment expressions, uninitialized locals, precedence/general expressions, arbitrary member/method lookup beyond the bounded assignment-target field path and existing field-value/reference selectors, additional refutable/shorthand pattern forms, unequal-state/path-dependent ownership after a two-normal-outcome conditional join, additional loop forms or general loop fixed-point inference, catch/recovery joins, plain-Exclusive source references, reference targets beyond the bounded Shared binding-root/field-path and complete-root replacement-capable forms owned by `references.md`, reference-containing aggregates/results beyond the existing bounded Shared result, lifetime names/parameters/non-lexical shortening, raw-pointer call transfer or pointer-containing aggregates, unsafe callable contracts, closures/captures, generics, traits/coherence, methods/overloads, explicit clone/copy operators, custom destructors, must-consume/drop abilities, structural state splitting beneath a consumed ancestor, const/static semantics, ABI/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR production code, or backend behavior.

Activation-local raw pointers and lexical unsafe admission are represented by `raw-pointers-unsafe.md`; their existence does not create the excluded broader pointer/call/unsafe relations here.