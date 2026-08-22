# Source Function-Local Bindings

Status: **provisional normative; incomplete**

This document owns the represented source semantics for function-local binding identity, lexical scope and lookup precedence, binding assignment mutability, binding lifecycle, ordinary whole-binding owned-value use, whole-binding assignment legality, and the points at which a binding's structural ownership state begins, persists, resets, or ends.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module lookup from [Source names and modules](names-modules.md), source value types and owned-value duplicability from [Source type foundation](types.md), structural paths, structural ownership state, path availability, consumption, and remaining-ownership frontiers from [Source structural ownership](structural-ownership.md), and callable parameter-slot types from [Source callables](callables.md). It does not redefine those owners.

Represented binding-rooted field-path selection, direct field accessibility, and final-field duplicate-or-consume value production are owned by [Source field-value access](field-access.md). Represented exhaustive record-pattern selection and pattern-specific binding production are owned by [Source patterns](patterns.md). Represented source body attachment, dynamic activations, direct calls, owned argument/result transfer, local initialization, assignment replacement ordering, lexical-scope and activation cleanup, return, recursion, divergence, and defined-fault propagation are owned by [Source function execution](function-execution.md). Represented conditional selection and definite normal successor ownership are owned by [Source control flow](control-flow.md). Concrete parameter/local/pattern/value/call/field-value/assignment/block/conditional spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define structural ownership mathematics, conditional selection or joins, field lookup, pattern structure, general expression evaluation, references, traits, ABI, Core liveness, or an implementation representation.

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

A represented binding identity is independent of original identifier spelling, token/source offset, parser node, physical address, compiler collection index, HIR/Core identifier choice, and runtime storage identity.

For the function form represented by `concrete-syntax.md`, concrete parameter source order maps to callable parameter-slot order and each parameter identifier supplies the lexical key for its corresponding parameter binding. Every represented concrete parameter binding is immutable for assignment purposes.

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

After successful initializer transfer, the new local begins with one complete structural owned-value root of its declared type and the initial empty consumed-path state from `structural-ownership.md`.

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

A pattern with no binding leaves introduces no function-local binding and therefore does not change the lookup environment by itself.

## Abstract lexical scopes

A represented function body has one root lexical scope. A represented nested block establishes one child lexical scope of its containing lexical scope. The resulting lexical scopes form a finite rooted tree.

The root body braces in `concrete-syntax.md` delimit the root lexical scope. Each concrete `BlockStatement` establishes exactly one child lexical scope containing its enclosed `BodyStatement` sequence and ending at that block's closing boundary. Recursively nested block statements therefore establish descendant lexical scopes.

Each explicit represented conditional arm is one ordinary `BlockStatement` and therefore one child lexical scope. A then arm and explicit else arm of the same conditional are sibling scopes. An omitted else introduces no synthetic lexical scope under `control-flow.md`.

The semantic scope tree does not prescribe parser nodes, source ranges, HIR scope identifiers, Core blocks, or physical storage lifetime.

A parameter binding belongs to the function root scope and is in scope throughout the represented function body, including descendant lexical scopes.

An ordinary local or pattern-introduced local binding is in scope from immediately after its successful declaration/initialization boundary through the end of its containing lexical scope, including descendant lexical scopes.

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

The concrete whole-binding value use, binding-rooted `FieldValueUse` root, direct binding-root pattern scrutinee, whole-binding assignment target, and unqualified direct-call target consume this precedence. A wrong-category selected entity is rejected rather than bypassed.

A nominal record-pattern head is not a function-local value-binding lookup. `patterns.md` defines each represented record-pattern head through same-module nominal-record declaration lookup independently of active local bindings with equal keys.

Source-unit module aliases remain the distinct qualified-lookup mechanism owned by `names-modules.md`. The concrete `alias::member` direct-call target resolves through that mechanism rather than this unqualified lookup.

Beyond the represented two-part module alias/member qualification, operation-specific field selectors, and bounded record-pattern field selection, this revision defines no arbitrary member lookup, nested module paths, labels, generic parameters, lifetime names, methods, associated items, or another future name domain.

## Binding assignment mutability

Every represented parameter/local binding is exactly one of:

- **immutable**; or
- **mutable**.

Assignment mutability is a binding property independent of source type identity, structural ownership state, source owned-value duplicability, callable-signature identity/equality, and future alias/borrow authority.

Consuming an owned value from an immutable binding, including a represented structural subvalue when `field-access.md` or `patterns.md` permits that consumption, is valid. Immutability restricts assignment/reinitialization; it does not require the binding to retain ownership of every subvalue.

Represented parameters are immutable. Ordinary locals are immutable unless their concrete declaration carries `mut`. Every binding introduced by the represented record pattern is immutable. No parameter-mutability or pattern-binding-mutability form is represented.

Assignment to an immutable binding is source-invalid regardless of whether its complete structural root is fully available, partially available, or unavailable.

Binding mutability does not itself replace a value or restore ownership. Replacement is an explicit assignment operation under the rules below.

## Binding structural ownership state

Every in-scope represented parameter/local binding owns exactly one structural owned-value root under `structural-ownership.md` whose root type is the binding's declared source type.

This document owns only the binding lifecycle around that structural state:

- successful parameter transfer establishes the parameter with complete initial ownership;
- successful ordinary local initialization establishes the local with complete initial ownership;
- successful pattern binding production establishes each new pattern binding with complete initial ownership;
- represented consuming/duplicating operations act on the binding's structural state only through their canonical operation owners and `structural-ownership.md`;
- successful whole-binding replacement establishes a fresh complete structural ownership state for the replacement value; and
- lexical/activation termination ends whatever binding ownership remains according to `function-execution.md`.

Entering or normally exiting a child lexical scope does not itself change the structural ownership state of an ancestor binding. Valid ownership transitions or assignment affecting an ancestor inside the child remain in force at the following parent-scope program point when the applicable control-flow relation admits that continuation.

Structural source paths, prefix-free consumed-path state, fully/partially/unavailable classification, path consumption, and recursive remaining-frontier selection are defined only by `structural-ownership.md`. They are not redefined here.

For the represented statement-level conditional, `control-flow.md` owns definite normal successor validity by comparing the structural ownership state of every enclosing binding across normal arm outcomes. When that owner admits the join, each binding continues with exactly one common structural state. This binding owner does not derive that state by union, intersection, normalization, or another merge rule.

Future loops, refutable matches, catch/recovery forms, early returns, or other control-flow forms require their own accepted definite-state relations; this document adds none.

## Ordinary whole-binding owned-value use

A represented **ordinary whole-binding owned-value use** applies to the empty structural path of one selected parameter/local binding.

The complete root path MUST be fully available under `structural-ownership.md` immediately before the use.

If the binding's source type is duplicable under `types.md`:

1. produce another owned source value of that complete type through the accepted duplicability capability; and
2. leave the binding's structural ownership state unchanged.

If the binding's source type is non-duplicable:

1. transfer/consume the complete owned value through the empty structural path; and
2. apply the canonical successful-consumption transition from `structural-ownership.md`.

Ordinary whole-binding use of a partially available or unavailable complete root is source-invalid, not a defined runtime moved-state fault.

The concrete `IdentifierUse` value form maps to this operation after lookup resolves one parameter/local binding.

This relation does not define field-value production or record-pattern ownership. Those owners may use non-empty structural paths without first applying ordinary whole-binding use to the complete root.

## Whole-binding assignment and reinitialization

A represented whole-binding assignment target MUST resolve through the function-local lookup relation above and MUST denote one represented parameter/local binding. The binding MUST be mutable.

The RHS MUST produce exactly one owned source value whose type is exactly equal under `types.md` to the target binding's declared source type.

The target may have a fully available, partially available, or unavailable complete structural root when assignment begins. Successful assignment always replaces/reinitializes the complete binding value.

The target remains in scope during RHS evaluation. Every RHS use observes the target's current structural ownership state. A consuming RHS may therefore change that state before replacement completes.

After successful RHS production, `function-execution.md` owns source-first replacement ordering:

1. select and end ownership of the target's then-current remaining old-value frontier through `structural-ownership.md`;
2. transfer the successfully produced replacement value into the target; and
3. establish a fresh complete structural ownership state with an empty consumed-path set.

Thus a mutable binding may be reinitialized from any represented structural ownership state, while an immutable binding may not be assigned in any state.

A defined fault or divergence during RHS evaluation performs no replacement/reset merely because assignment was intended. Ownership transitions that completed while evaluating the RHS remain in force under their existing owners.

This assignment relation defines no field assignment, partial-field reinitialization, general source place/lvalue, borrow/reference target, interior mutability, or destructuring assignment.

## Binding cleanup and discard boundary

When represented execution ends a binding's ownership, its remaining owned source subvalues are exactly the complete-root remaining ownership frontier selected by `structural-ownership.md` from the binding's then-current state.

`function-execution.md` owns when that frontier is selected and the ordering between bindings, scopes, parameters, activations, assignment replacement, normal return, and defined-fault cleanup.

A binding is not source-invalid solely because one or more remaining owned subvalues are non-duplicable when its scope or activation terminates. This revision defines no source `drop` ability, must-consume classification, custom destructor, or unused-value prohibition.

Zero-field and recursively zero-leaf frontier members remain source-owned values even when faithful Core refinement emits no scalar destruction operation.

## Function, call, assignment, pattern, control-flow, and fault boundary

This document defines body-local binding identity, scope, lookup, assignment mutability, binding lifecycle around structural ownership, ordinary whole-binding use, and assignment legality/reset.

It does not redefine the execution relation owned by `function-execution.md`, including:

- function body execution;
- direct-call argument evaluation and parameter transfer;
- assignment RHS evaluation, old-value cleanup, and replacement transfer;
- result production and return transfer;
- dynamic activation identity or recursion;
- lexical-scope/caller/callee cleanup sequencing; or
- defined-fault propagation across activations.

It does not redefine represented conditional condition/arm selection or definite normal ownership joins from `control-flow.md`.

It likewise does not redefine field-path selection/production from `field-access.md`, pattern structure/ownership from `patterns.md`, or structural ownership mathematics from `structural-ownership.md`.

Indirect calls, function values, closures, references/pass modes, broader panic/catch forms, and other future execution relations remain outside this owner.

## Implementation boundary

This revision does not add or require parser, lossless-syntax, HIR, Core MIR production, runtime, or backend representation.

A faithful implementation MAY retain structural ownership for bindings using resolved field indices or another implementation identity after source field resolution, but those representations are not source semantic identity. Core path state or scalar liveness MUST NOT become the binding's source ownership authority, including at a represented conditional join.

## Further boundaries

Beyond the represented concrete subset, this revision does not define type inference, assignment expressions, uninitialized locals, precedence/general expressions, field assignment or partial-field reinitialization, arbitrary member/method lookup, additional refutable/rest/shorthand pattern forms, unequal-state/path-dependent conditional ownership, loops or their fixed points, early-return joins, catch/recovery joins, references/borrows/lifetime inference, closures/captures, generics, traits/coherence, methods/overloads, explicit clone/copy operators, custom destructors, must-consume/drop abilities, const/static semantics, ABI/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR production code, or backend behavior.
