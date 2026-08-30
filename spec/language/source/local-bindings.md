# Source Function-Local Bindings

Status: **provisional normative; incomplete**

This document owns the represented source semantics for function-local binding identity, lexical scope and lookup precedence, binding assignment mutability, binding lifecycle, ordinary whole-binding owned-value use, whole-binding assignment legality, first-slice Shared-reference/raw-pointer local contextual integration, and the points at which a binding's structural ownership state begins, persists, resets, or ends.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module lookup from [Source names and modules](names-modules.md), source value types and owned-value duplicability from [Source type foundation](types.md), structural paths, structural ownership state, path availability, consumption, and remaining-ownership frontiers from [Source structural ownership](structural-ownership.md), callable parameter-slot types from [Source callables](callables.md), Shared-reference target/authority/carrier/lifetime rules from [Source Shared references](references.md), and raw-pointer contextual admission, pointer-origin provenance, lexical target validity, and raw pointee operations from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md). It does not redefine those owners.

Represented binding-rooted field-path selection, direct field accessibility, and final-field duplicate-or-consume value production are owned by [Source field-value access](field-access.md). Represented recursive record-pattern selection, including bounded node-local rest/omission, and pattern-specific binding production are owned by [Source patterns](patterns.md). Represented source body attachment, dynamic activations, direct calls, owned argument/result transfer including reference carriers, local initialization, assignment replacement ordering, normal-continuation presence, lexical-scope and activation cleanup, return, recursion, divergence, defined-fault propagation, and raw-operation execution ordering are owned by [Source function execution](function-execution.md). Represented conditional selection, zero/one/two normal-outcome composition, bounded `while` condition/body selection, structural-state joins/backedges, and raw-pointer-origin joins/backedges are owned by [Source control flow](control-flow.md). Concrete parameter/local/pattern/value/call/field-value/assignment/block/conditional/while/return/reference/raw-pointer/unsafe spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define structural ownership mathematics, Shared reference formation/dereference/authority semantics, raw-pointer formation/pointee access/unsafe admission semantics, normal-continuation presence, conditional or loop selection/successor composition, field lookup, pattern structure, general expression evaluation, traits, ABI, Core liveness, or an implementation representation.

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

A represented binding identity is independent of original identifier spelling, token/source offset, parser node, physical address, compiler collection index, HIR/Core identifier choice, runtime storage identity, source Shared-authority identity, or source raw-pointer origin provenance.

For the function form represented by `concrete-syntax.md`, concrete parameter source order maps to callable parameter-slot order and each parameter identifier supplies the lexical key for its corresponding parameter binding. Every represented concrete parameter binding is immutable for assignment purposes, including a parameter whose type is `SharedRef(T)`. `RawPtr(T)` is not parameter-admissible under `callables.md` in this slice.

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

For first-slice Shared-reference types, contextual local admission is stricter:

- `let name: &T = Value;` may establish an immutable ordinary local when `SharedRef(T)` is valid under `references.md`; and
- `let mut name: &T = Value;` is source-invalid in this slice.

This immutable-only rule is a source reference-lifetime boundary. It does not redefine the general assignment-mutability classification of non-reference locals and does not imply a hidden `const` or type-level mutability dimension.

For first-slice raw-pointer types, both ordinary local mutability classes are admitted when `RawPtr(T)` is valid under `raw-pointers-unsafe.md`:

- `let name: raw T = Value;` establishes an immutable raw-pointer local; and
- `let mut name: raw T = Value;` establishes a mutable raw-pointer local whose stored pointer value may later be replaced by ordinary whole-binding assignment.

Raw-pointer local mutability applies only to the stored pointer value. It does not grant pointee mutation authority. Every raw-pointer initializer additionally MUST satisfy the pointer-origin lexical target-validity relation from `raw-pointers-unsafe.md` for the complete static extent of the receiving local.

After successful initializer transfer, the new local begins with one complete structural owned-value root of its declared type and the initial empty consumed-path state from `structural-ownership.md`. When the initialized value is a Shared reference, the local additionally stores the produced reference carrier whose authority/lifetime consequence is owned by `references.md`; when it is a raw pointer, source validation additionally retains the exact pointer-origin provenance owned by `raw-pointers-unsafe.md`. Neither relation introduces a second structural ownership state.

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

First-slice nominal record fields cannot have `SharedRef(T)` or `RawPtr(T)` type, so the represented record pattern relation cannot introduce a Shared-reference or raw-pointer binding in this slice. No borrow-binding or pointer-binding pattern mode is implied.

A pattern with no binding leaves introduces no function-local binding and therefore does not change the lookup environment by itself.

## Abstract lexical scopes

A represented function body has one root lexical scope. A represented nested block establishes one child lexical scope of its containing lexical scope. The resulting lexical scopes form a finite rooted tree.

The root body braces in `concrete-syntax.md` delimit the root lexical scope. Each concrete `BlockStatement` establishes exactly one child lexical scope containing its enclosed `BodyStatement` sequence and optional terminal `ReturnStatement`, and ending at that block's closing boundary. Recursively nested block statements therefore establish descendant lexical scopes. An `unsafe` block is one such ordinary child lexical block plus the separate unsafe-admission fact owned by `raw-pointers-unsafe.md`.

Each explicit represented conditional arm is one ordinary `BlockStatement` and therefore one child lexical scope. A then arm and explicit else arm of the same conditional are sibling scopes. An omitted else introduces no synthetic lexical scope under `control-flow.md`. Each represented `while` body is likewise one ordinary `BlockStatement` child scope; repeated dynamic iterations re-enter that same static source scope rather than creating new source binding identities.

The semantic scope tree does not prescribe parser nodes, source ranges, HIR scope identifiers, Core blocks, physical storage lifetime, or a physical address for a borrow/raw-pointer target.

A parameter binding belongs to the function root scope and is in scope throughout the represented function body, including descendant lexical scopes while those scopes are active.

An ordinary local or pattern-introduced local binding is in scope from immediately after its successful declaration/initialization boundary through the end of its containing lexical scope, including descendant lexical scopes while execution remains in that activation. A return terminates the activation under `function-execution.md` rather than creating a later point in the ended scope.

These existing containment/cleanup relations are consumed by `references.md` to prove the first-slice implicit lexical reference lifetime and by `raw-pointers-unsafe.md` to require that every pointer local's target extent contain that pointer local's complete extent. This binding owner does not add lifetime names, pointer-origin sets, or a second scope tree.

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

The concrete whole-binding value use, binding-rooted `FieldValueUse` root, direct binding-root pattern scrutinee, whole-binding assignment target, unqualified direct-call target, root Shared-borrow operand `&x`, bounded Shared-dereference operand `*r`, raw-address target, raw-move pointer operand, and raw-assign pointer operand consume this precedence. A wrong-category selected entity is rejected rather than bypassed.

For `&x`, `references.md` additionally requires the resolved entity to be one active parameter or ordinary local binding and selects only its complete root. For `*r`, `references.md` additionally requires the resolved binding to have exact type `SharedRef(T)`.

For raw address formation, `raw-pointers-unsafe.md` additionally requires the resolved target to be one active parameter or ordinary local binding of a first-slice raw-pointee-admissible type and selects only its complete root. For raw move/assign, that owner requires the pointer operand binding to have exact type `RawPtr(T)` and consumes its retained exact pointer origin.

A nominal record-pattern head is not a function-local value-binding lookup. `patterns.md` defines each represented record-pattern head through same-module nominal-record declaration lookup independently of active local bindings with equal keys.

Source-unit module aliases remain the distinct qualified-lookup mechanism owned by `names-modules.md`. The concrete `alias::member` direct-call target resolves through that mechanism rather than this unqualified lookup.

Beyond the represented two-part module alias/member qualification, operation-specific field selectors, bounded record-pattern field selection, bounded Shared reference root/dereference lookup, and bounded raw-pointer root/pointer-operand lookup above, this revision defines no arbitrary member lookup, nested module paths, labels, generic parameters, lifetime names, methods, associated items, or another future name domain.

## Binding assignment mutability

Every represented parameter/local binding is exactly one of:

- **immutable**; or
- **mutable**.

Assignment mutability is a binding property independent of source type identity, structural ownership state, source owned-value duplicability, callable-signature identity/equality, Shared alias authority, and raw-pointer origin provenance.

Consuming an owned value from an immutable binding, including a represented structural subvalue when `field-access.md` or `patterns.md` permits that consumption, is valid. Immutability restricts ordinary binding assignment/reinitialization; it does not require the binding to retain ownership of every subvalue and it is not raw target-access authority.

Represented parameters are immutable. Ordinary locals are immutable unless their concrete declaration carries `mut`. Every binding introduced by the represented record pattern is immutable. No parameter-mutability or pattern-binding-mutability form is represented.

A first-slice local whose declared type is `SharedRef(T)` MUST be immutable; the otherwise represented mutable-local form is invalid for that type.

A first-slice local whose declared type is `RawPtr(T)` MAY be immutable or mutable. A mutable raw-pointer local may be ordinarily assigned another exact `RawPtr(T)` value only when the incoming pointer origin satisfies the lexical target-validity rule from `raw-pointers-unsafe.md` for the complete receiving-local extent.

Assignment to any immutable binding is source-invalid regardless of whether its complete structural root is fully available, partially available, or unavailable.

Binding mutability does not itself replace a value or restore ownership. Replacement is an explicit ordinary assignment operation under the rules below. Separately, unsafe raw replacement through a pointer may replace an immutable **pointee target** because `raw-pointers-unsafe.md` deliberately does not consume ordinary target-binding assignment mutability as a precondition; that operation is not whole-binding assignment through this owner.

## Binding structural ownership state

Every in-scope represented parameter/local binding owns exactly one structural owned-value root under `structural-ownership.md` whose root type is the binding's declared source type.

This document owns only the binding lifecycle around that structural state:

- successful parameter transfer establishes the parameter with complete initial ownership;
- successful ordinary local initialization establishes the local with complete initial ownership;
- successful pattern binding production establishes each new pattern binding with complete initial ownership;
- represented consuming/duplicating operations act on the binding's structural state only through their canonical operation owners and `structural-ownership.md`;
- successful whole-binding replacement establishes a fresh complete structural ownership state for the replacement value; and
- lexical/activation termination ends whatever binding ownership remains according to `function-execution.md`.

Shared-reference authority/carrier state and raw-pointer origin provenance are deliberately distinct from this consumed-path state. `references.md` owns the authority relation and `raw-pointers-unsafe.md` owns pointer origin. Root Shared-borrow and raw address formation both leave the target binding's structural ownership state unchanged. Raw ownership move and raw replacement alter the target structural state only through the explicit transitions consumed by their canonical raw-pointer owner.

Entering or normally exiting a child lexical scope does not itself change the structural ownership state or pointer-origin provenance of an ancestor binding. Valid ownership transitions, pointer retargeting, or assignment affecting an ancestor inside the child remain in force at the following parent-scope program point when the applicable control-flow relation admits that normal continuation.

Structural source paths, prefix-free consumed-path state, fully/partially/unavailable classification, path consumption, and recursive remaining-frontier selection are defined only by `structural-ownership.md`. They are not redefined here.

For the represented statement-level conditional, `control-flow.md` owns normal-continuation composition for enclosing binding states:

- when two normal outcomes meet, every enclosing binding must have exactly equal structural ownership state and continues with that common state;
- when exactly one normal outcome exists, every enclosing binding continues with exactly the state from that sole normal outcome, without comparison against a returning outcome; and
- when zero normal outcomes exist, there is no following binding state because the conditional has no normal continuation.

That same control-flow owner additionally consumes `raw-pointers-unsafe.md` to require exact continuing pointer-origin equality for every enclosing raw-pointer binding across two normal outcomes; a sole normal outcome carries its exact origin forward without comparison.

For the represented bounded `while`, `control-flow.md` likewise owns the complete structural and raw-pointer-origin state relation. Let `H` be the enclosing binding environment immediately before condition evaluation and let successful condition validation produce `C`:

- the false loop outcome continues normally with exactly `C`;
- the true outcome validates one ordinary child block from `C`;
- if that body has a normal continuation, then after ordinary child-scope cleanup every enclosing binding identity from `H` MUST have exactly the same structural ownership state as in `H` before a backedge is admitted;
- every continuing enclosing raw-pointer binding from `H` MUST likewise have exactly the same pointer origin as in `H` before that backedge is admitted;
- a body with no normal continuation contributes no backedge state and requires no backedge-state equality check; and
- assignment mutability is unchanged: only explicit accepted ordinary assignment to a mutable binding may restore complete binding ownership or retarget a raw-pointer local before a backedge, while immutable bindings receive no implicit restoration/retargeting.

The Shared-reference relation adds no source authority state to those exact structural/pointer-origin comparisons. Shared reference carriers stored in immutable reference bindings have lexical lifetime and cannot be moved/rebound by ordinary source use; target-side permitted safe uses are non-consuming. The reference validity relation therefore requires no union, intersection, widening, or second control-flow join lattice here.

Raw-pointer origin is an additional exact provenance fact, not a structural ownership path set. The raw-pointer/control-flow relation likewise adds no origin union, set, maybe-state, widening, fixed point, or NLL lattice.

This binding owner does not derive a successor or backedge state by union, intersection, normalization, widening, lower Core path state, fixed-point iteration, or another merge rule.

Future refutable matches, catch/recovery forms, additional loop forms, or other control-flow forms require their own accepted definite-state relations; this document adds none beyond the bounded `while` relation owned by `control-flow.md`.

## Ordinary whole-binding owned-value use

A represented **ordinary whole-binding owned-value use** applies to the empty structural path of one selected parameter/local binding.

The complete root path MUST be fully available under `structural-ownership.md` immediately before the use.

If the binding's source type is duplicable under `types.md`:

1. produce another owned source value of that complete type through the accepted duplicability capability; and
2. leave the binding's structural ownership state unchanged.

If the binding's source type is non-duplicable:

1. transfer/consume the complete owned value through the empty structural path; and
2. apply the canonical successful-consumption transition from `structural-ownership.md`.

For a binding of exact type `SharedRef(T)`, the type is duplicable. The successful duplicate therefore has the reference-carrier consequence owned by `references.md`: it creates another carrier naming the same Shared authority/target while retaining the stored source carrier. This is not a reborrow or new root authority.

For a binding of exact type `RawPtr(T)`, the type is duplicable. The successful duplicate therefore preserves the exact raw-pointer value and `PointerOrigin(binding)` provenance owned by `raw-pointers-unsafe.md`; it does not access the pointee or create any reference authority.

Ordinary whole-binding use of a partially available or unavailable complete root is source-invalid, not a defined runtime moved-state fault.

The concrete `IdentifierUse` value form maps to this operation after lookup resolves one parameter/local binding.

This relation does not define field-value production, record-pattern ownership, Shared dereference, raw address formation, or raw pointee access. Those owners may use their own bounded receiving relations without first applying ordinary whole-binding use to the complete target root.

## Whole-binding assignment and reinitialization

A represented whole-binding assignment target MUST resolve through the function-local lookup relation above and MUST denote one represented parameter/local binding. The binding MUST be mutable.

Additionally, the target binding root MUST NOT currently be targeted by any active source Shared authority under `references.md`.

The RHS MUST produce exactly one owned source value whose type is exactly equal under `types.md` to the target binding's declared source type.

When the target binding has type `RawPtr(T)`, the produced RHS raw-pointer value additionally MUST carry an exact pointer origin whose target binding extent contains the complete receiving pointer-local extent under `raw-pointers-unsafe.md`. This source validity requirement applies before the new pointer value/origin becomes the target binding's continuing state.

The target may have a fully available, partially available, or unavailable complete structural root when assignment begins. Successful assignment always replaces/reinitializes the complete binding value. An active Shared borrow is a separate rejection condition even when the current structural root would otherwise be replaceable.

The target remains in scope during RHS evaluation. Every RHS use observes the target's current structural ownership and Shared-authority state. A consuming RHS may therefore change structural ownership state before replacement completes. A Shared authority established or retained during RHS evaluation likewise remains controlling at the replacement point. For a raw-pointer assignment, RHS production similarly determines the exact incoming pointer origin before replacement commits.

After successful RHS production, satisfaction of any raw-pointer lexical target-validity requirement, and satisfaction of the active-Shared-authority prohibition, `function-execution.md` owns source-first replacement ordering:

1. select and end ownership of the target's then-current remaining old-value frontier through `structural-ownership.md`;
2. transfer the successfully produced replacement value into the target; and
3. establish a fresh complete structural ownership state with an empty consumed-path set.

For a `RawPtr(T)` target, the successful transfer also replaces the stored pointer-origin provenance with the incoming exact origin. Ending the old pointer value has no pointee effect under `raw-pointers-unsafe.md`.

Thus a mutable binding may be reinitialized from any represented structural ownership state only when no active Shared reference authority currently targets its complete root, while an immutable binding may not be ordinarily assigned in any state.

A first-slice Shared-reference local itself cannot be an assignment target because reference locals are immutable. A raw-pointer local may be an ordinary assignment target only when it is mutable. Unsafe raw replacement of the **pointee** remains a separate operation and does not use this target-binding mutability rule.

A defined fault or divergence during RHS evaluation performs no replacement/reset merely because assignment was intended. Ownership, Shared-authority, and pointer-origin transitions that completed while evaluating the RHS remain in force under their existing owners.

This assignment relation defines no field assignment, partial-field reinitialization, general source place/lvalue, reference-relative assignment, interior mutability, raw pointee replacement, or destructuring assignment.

## Binding cleanup and discard boundary

When represented execution ends a binding's ownership, its remaining owned source subvalues are exactly the complete-root remaining ownership frontier selected by `structural-ownership.md` from the binding's then-current state.

`function-execution.md` owns when that frontier is selected and the ordering between bindings, scopes, parameters, activations, assignment replacement, raw replacement, normal return, and defined-fault cleanup.

When a binding's remaining owned value is a first-slice Shared reference, ending that value additionally removes its source reference carrier at that existing cleanup point under `references.md`. The reference-specific consequence adds no custom cleanup body and does not access the referent.

When a binding's remaining owned value is a raw pointer, ending that value has no pointer-specific pointee effect under `raw-pointers-unsafe.md`. It neither changes target structural ownership nor ends a Shared authority.

The existing reverse local/declaration and activation cleanup ordering is part of the first-slice lexical lifetime proof in `references.md` and lexical pointer-target validity in `raw-pointers-unsafe.md`: reference/pointer locals end before the earlier or ancestor target/source extents whose validity their initialization consumed.

A binding is not source-invalid solely because one or more remaining owned subvalues are non-duplicable when its scope or activation terminates. This revision defines no source `drop` ability, must-consume classification, custom destructor, or unused-value prohibition.

Zero-field and recursively zero-leaf frontier members remain source-owned values even when faithful Core refinement emits no scalar destruction operation.

## Function, call, assignment, pattern, control-flow, reference, raw-pointer, and fault boundary

This document defines body-local binding identity, scope, lookup, assignment mutability, binding lifecycle around structural ownership, first-slice Shared-reference/raw-pointer local integration, ordinary whole-binding use, and ordinary assignment legality/reset.

It does not redefine the execution relation owned by `function-execution.md`, including:

- function body execution and normal-continuation presence;
- direct-call argument evaluation and parameter transfer;
- Shared-reference produced-carrier transfer/caller suspension consequences;
- ordinary assignment RHS evaluation, old-value cleanup, and replacement transfer;
- raw replacement source-first execution ordering;
- result production and return transfer;
- dynamic activation identity or recursion;
- lexical-scope/caller/callee cleanup sequencing; or
- defined-fault propagation across activations.

It does not redefine Shared root-borrow formation, reference authority/carrier identity, dereference/copy, lexical reference lifetime validity, or source-to-Core reference refinement from `references.md`.

It does not redefine raw address formation, raw pointee move/replacement, unsafe admission, pointer-origin validity, or source-to-Core raw refinement from `raw-pointers-unsafe.md`.

It does not redefine represented conditional or bounded-`while` condition/body selection, normal-successor composition, structural-state equality, raw-pointer-origin equality, or loop backedge admission from `control-flow.md`.

It likewise does not redefine field-path selection/production from `field-access.md`, pattern structure/ownership from `patterns.md`, or structural ownership mathematics from `structural-ownership.md`.

Indirect calls, function values, closures, mutable/exclusive references, reference pass modes, lifetime names, unsafe callable contracts, broader panic/catch forms, and other future execution relations remain outside this owner.

## Implementation boundary

This revision does not add or require parser, lossless-syntax, HIR, Core MIR production, runtime, or backend representation.

A faithful implementation MAY retain structural ownership for bindings using resolved field indices or another implementation identity after source field resolution, but those representations are not source semantic identity. It MAY separately retain exact raw-pointer origin provenance for raw-pointer locals as required by `raw-pointers-unsafe.md`. Core path state, scalar liveness, reference-authority IDs, Core pointer-target metadata, or runtime storage identities MUST NOT become the binding's source ownership/reference/pointer-origin authority, including when a represented conditional establishes a normal successor or a represented `while` validates one backedge.

## Further boundaries

Beyond the represented concrete subset, this revision does not define type inference, assignment expressions, uninitialized locals, precedence/general expressions, field assignment or partial-field reinitialization, arbitrary member/method lookup, additional refutable/shorthand pattern forms, unequal-state/path-dependent ownership after a two-normal-outcome conditional join, additional loop forms or general loop fixed-point inference, catch/recovery joins, mutable/exclusive references, reference reborrow, field/path reference targets, reference-containing aggregates/results, lifetime names/parameters/non-lexical shortening, raw-pointer call transfer or pointer-containing aggregates, unsafe callable contracts, closures/captures, generics, traits/coherence, methods/overloads, explicit clone/copy operators, custom destructors, must-consume/drop abilities, const/static semantics, ABI/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR production code, or backend behavior.

Activation-local raw pointers and lexical unsafe admission are represented by `raw-pointers-unsafe.md`; their existence does not create the excluded broader pointer/call/unsafe relations here.