# Source Callables

Status: **provisional normative; incomplete**

This document owns the represented source function entity identity, callable-signature structure and equality, contextual parameter/result type admission, the reusable concrete non-generic callable-interface relation, the bounded safe-reference result contract, and exported-signature source accessibility.

It consumes module binding identity and accessibility from [Source names and modules](names-modules.md), represented concrete source value type identity and equality from [Source type foundation](types.md), captureless concrete function-value type structure from [Source function values and indirect calls](function-values.md), first-slice function type-parameter identity, abstract parametric type expressions, exact substitution, generic-validation equality, and resolved per-slot marker-requirement sets from [Source generics](generics.md), marker trait entity identity from [Source marker traits](traits.md), safe-reference contextual/type/result-contract accessibility facts from [Source safe references](references.md), and the first-slice activation-local raw-pointer callable exclusion plus absence of an unsafe callable dimension from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md). [Source closures and explicit by-value capture](closures.md) consumes the concrete non-generic callable-interface relation defined here for each closure site's explicit parameter/result structure; closure identity, captures, opaque closure types, and invocation remain owned there. Marker implementation/coherence and obligation satisfaction remain owned by `traits.md`. It does not redefine those owners.

Represented source function body attachment, source-function activations, call validation/execution after target classification, owned argument/result transfer including safe-reference carrier/result/external-referent consequences, recursion, cleanup, return, divergence, and defined-fault propagation are owned by [Source function execution](function-execution.md). The function-value branch of bounded-indirect target classification and function-value formation are owned by `function-values.md`; the dedicated-closure-binding branch is owned by `closures.md`. The non-observable generic activation substitution context carried by a generic call and marker-obligation validation ordering are owned separately by [Source generics](generics.md). Function-local parameter binding identity, scope, mutability, availability, and ordinary owned use are owned by [Source function-local bindings](local-bindings.md). The represented concrete function-definition, closure declaration, generic parameter/application, parameter, result, safe-reference type, and raw-pointer type spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define function-value payload identity, function-value formation, bounded indirect or bounded closure target classification, closure type/capture semantics, effects, generic substitution/capability semantics beyond the callable integration stated here, trait implementation/coherence, ABI, source lifetime names, a separate reference pass mode, an unsafe callable contract, or an implementation representation.

## Reusable concrete non-generic callable interface

A represented **concrete non-generic callable interface** contains exactly:

1. one finite ordered sequence of concrete parameter source types;
2. either no result value or exactly one concrete result source type; and
3. one exact bounded safe-reference result contract derived from that ordered parameter/result structure by this document.

This relation is reusable semantic interface structure. A non-generic source function signature instantiates it from its ordinary parameter/result slots. A closure site under `closures.md` instantiates it from its **explicit** closure parameters and explicit optional result only. Closure capture slots are not callable parameter slots and never participate in result-origin selection.

The relation does not establish source function entity identity, closure-site identity, signature equality between closures and functions, structural closure type compatibility, module accessibility, generic binders, runtime dispatch, ABI, or implementation representation.

A concrete parameter type is admitted exactly when it is:

- one represented intrinsic scalar or nominal record source type admitted by `types.md`;
- one represented concrete function-value type admitted by `function-values.md`;
- one represented `SharedRef(T)` satisfying the Shared-reference restrictions from `references.md`; or
- one represented `ExclusiveReplaceRef(T)` satisfying the replacement-reference restrictions from `references.md`.

A concrete result type is admitted exactly when it is:

- one represented intrinsic scalar or nominal record source type already result-admissible;
- one represented concrete function-value type admitted by `function-values.md`; or
- one represented `SharedRef(T)` satisfying the bounded safe-reference result-contract relation below.

`ExclusiveReplaceRef(T)`, `RawPtr(T)`, and opaque closure-site types from `closures.md` are not concrete result types in this first relation. `RawPtr(T)` and opaque closure-site types are likewise not concrete parameter types. Opaque closure types therefore do not become generally first-class merely because closures consume this callable-interface relation.

For a concrete result `SharedRef(T)`, `T` MUST satisfy the Shared-referent-admission relation from `references.md`. Let `S` be the ordered set of parameter-slot indices whose exact source type is `SharedRef(T)`:

- if `S` contains exactly one slot `i`, the result contract is **SharedIdentity(i)**;
- if `S` contains two or more slots, the interface is invalid because the identity-preserving origin is ambiguous;
- only when `S` is empty, let `R` be the ordered set of parameter-slot indices whose exact source type is `ExclusiveReplaceRef(T)`;
- if `R` contains exactly one slot `i`, the result contract is **SharedDirectChild(i)**;
- if `R` contains two or more slots, the interface is invalid because the direct-child parent origin is ambiguous; and
- if both `S` and `R` are empty, the interface is invalid because no represented safe-reference result contract can be established.

A concrete interface whose result is not `SharedRef(T)`, including a no-result interface, has contract **None** subject to ordinary result admission.

The contract descriptor selects an ordered **explicit callable parameter slot**. For closures, capture slots are excluded from this sequence and can never be `SharedIdentity`/`SharedDirectChild` origins. The detailed target, authority, carrier, restoration, and result-validity laws remain owned by `references.md`.

## Source function entities

A **function declaration** is a module-level source declaration that introduces exactly one module binding under `names-modules.md`.

That binding denotes one **source function entity**. Function entity identity is the identity of that declaration/binding.

Distinct function declarations denote distinct source function entities even when their callable signatures are structurally equal.

The binding's module-private or exported accessibility is determined only by `names-modules.md`; this document does not redefine accessibility.

Different callable signatures, generic arities, marker requirements, or type-parameter spellings do not make duplicate module binding keys legal. This revision defines no function overloading or overload set.

The represented `fn` definition in `concrete-syntax.md` establishes one function declaration/entity, one callable signature as mapped below, and one represented source body attached under `function-execution.md`. A non-empty generic type-parameter list makes that existing function entity generic under `generics.md`; it does not create a second entity category or overload namespace. Without the concrete `export` modifier that function binding is module-private; with the modifier it is exported under `names-modules.md`. Concrete spelling does not redefine function identity or callable-signature structure.

A future concrete declaration-only or alternate definition form, if accepted, must map to these semantic entities without creating a competing identity relation.

## Callable signatures

Every represented source function entity has exactly one **callable signature** containing:

1. one finite ordered sequence of generic type-parameter slots, which may be empty, including each slot's exact finite marker-requirement set;
2. one finite ordered sequence of parameter slots;
3. one result specification; and
4. one **safe-reference result contract**.

The semantic identity, lexical keys, scope, first-slice admissibility, and marker-requirement attachment of generic type-parameter slots are owned by `generics.md`. An empty type-parameter sequence identifies a non-generic function; a non-empty sequence identifies a generic function under that owner.

Each generic type-parameter slot contributes its exact finite marker-requirement set established under `generics.md`. This is callable-interface structure but does not change the slot's semantic identity or create an overload dimension. Marker trait identity is owned by `traits.md`.

For a non-generic source function, the concrete parameter/result portion and result contract are exactly the reusable concrete non-generic callable-interface relation above.

For a generic source function, the safe-reference result contract is still exactly one of:

- **None**;
- **SharedIdentity(origin)**, designating one ordinary parameter-slot index; or
- **SharedDirectChild(origin)**, designating one ordinary parameter-slot index.

Each parameter slot contains exactly one **parameter type expression**. The parameter sequence may be empty.

A parameter type expression is admitted in this revision exactly when it is either:

- one concrete parameter type admitted by the reusable relation above; or
- inside a generic function, one bare in-scope abstract type parameter admitted by `generics.md`.

A represented `RawPtr(T)` is not parameter-admissible. The raw-pointer type may be syntactically present in a parameter position through the general `Type` grammar, but such a function declaration is source-invalid before body execution or lowering. This preserves the activation-local raw-pointer contract and the existing Core prohibition on raw-pointer-containing parameter transfer.

An opaque closure-site type is not parameter-admissible. Closure values therefore do not become ordinary function or closure explicit arguments under this callable owner.

An abstract type parameter is not a safe-reference or raw-pointer type expression and cannot contain either constructor in this slice. Its generic-body ownership/capability behavior and marker-evidence behavior are owned by `generics.md`; this callable owner does not classify it as a concrete duplicable or non-duplicable type or derive operations from marker requirements.

Safe-reference parameters remain ordinary source parameter slots. When a parameter type expression is a represented safe-reference type, that exact type expression is the slot's source type; no second borrowed/reference/pass-mode dimension is added to the callable signature.

Parameter slots have no lexical identifier key or source binding name under this signature contract. For a concrete function definition under `concrete-syntax.md`, concrete parameter order maps directly to parameter-slot order; `local-bindings.md` owns the corresponding parameter binding keys, identities, scope, mutability, and availability. Generic type-parameter lexical keys remain a distinct type-position lookup domain owned by `generics.md`; those keys do not become parameter-slot names or value bindings.

The ordered parameter sequence is semantic signature structure. `generics.md` owns generic-application list presence, arity, type-argument admission, exact substitution, and marker-obligation validation before any ordinary value-argument producer effects may commit. After the instantiated parameter/result type expressions are supplied, positional source-call validation, argument evaluation order, owned-value transfer, reference-carrier transfer, call-entry authority, and replacement-capable external-referent consequences are owned by `function-execution.md` and `references.md`; physical register, stack, ABI, storage-layout, or other calling mechanisms remain outside this owner.

The result specification is exactly one of:

- **no result value**; or
- **one result value** of exactly one **result type expression**.

A result type expression is admitted exactly when it is either:

- one concrete result type admitted by the reusable relation above; or
- inside a generic function, one bare in-scope abstract type parameter admitted by `generics.md`.

`ExclusiveReplaceRef(T)` is not result-admissible. A concrete `-> &mut T` spelling is therefore source-invalid. This slice defines no replacement-capable result escape or restoration contract.

`RawPtr(T)` is likewise not result-admissible. A concrete `-> raw T` spelling is source-invalid. An opaque closure-site type is not result-admissible.

An abstract result type parameter always contributes contract **None**. First-slice generic substitution cannot turn it into a safe-reference, raw-pointer, function-value, or opaque closure-site type outside the generic type-argument domain admitted by `generics.md`.

For a generic function result `SharedRef(T)`, `T` MUST satisfy the Shared-referent-admission relation from `references.md`. The callable contract is established deterministically from the concrete safe-reference parameter positions before body validation as follows. Abstract type-parameter positions are not candidates in either set below.

Let `S` be the ordered set of parameter-slot indices whose exact resolved concrete source type is `SharedRef(T)`.

- If `S` contains exactly one slot `i`, the callable contract is **SharedIdentity(i)**.
- If `S` contains two or more slots, the declaration is source-invalid because the identity-preserving origin is ambiguous.
- Only when `S` is empty, let `R` be the ordered set of parameter-slot indices whose exact resolved concrete source type is `ExclusiveReplaceRef(T)`.
- If `R` contains exactly one slot `i`, the callable contract is **SharedDirectChild(i)**.
- If `R` contains two or more slots, the declaration is source-invalid because the direct-child parent origin is ambiguous.
- If both `S` and `R` are empty, the declaration is source-invalid because no represented safe-reference result contract can be established.

Consequently:

- every currently represented identity-valid `SharedRef(T)` signature retains the exact same origin slot and identity-preserving contract;
- one exact `SharedRef(T)` parameter continues to establish identity even when one or more `ExclusiveReplaceRef(T)` parameters with the same referent also exist;
- a `SharedRef(T)` result with no exact Shared candidate may use one unique `ExclusiveReplaceRef(T)` parameter as its direct-child parent origin;
- a source `ExclusiveReplaceRef(T)` parameter never establishes `SharedIdentity` merely because its referent is `T`;
- source exposes no plain Core `ExclusiveRef(T)`, so the represented source direct-child parent class is exactly `ExclusiveReplaceRef(T)`; and
- an ordinary abstract-result, replacement-capable-result-invalid, raw-result-invalid, opaque-closure-result-invalid, or no-result callable has contract **None**.

A mixed signature containing one exact `SharedRef(T)` candidate and one or more `ExclusiveReplaceRef(T)` candidates therefore remains identity-contract-bearing. This slice defines no syntax or alternate elision that selects a replacement-capable candidate instead while an exact Shared candidate exists.

The contract descriptor selects an ordered parameter slot. It is not a source parameter name, generic type-parameter identity, marker trait identity, closure capture identity, body-local binding identity, lifetime name, implementation local identifier, dynamic activation identity, storage identity, physical address, or lower Core identifier.

This **bounded result-contract elision** is the represented concrete source way to establish safe-reference result behavior. It is not body-derived origin inference. The advertised contract is established from callable structure before body validation and therefore remains available to independent source-call validation, nested calls, direct or bounded-indirect recursion, mutual recursion, generic application, and bounded closure invocation without inspecting or expanding a callee body.

For **SharedIdentity(i)**, normal result validity preserves the exact incoming Shared authority/target identity selected by slot `i`.

For **SharedDirectChild(i)**, normal result validity requires one complete-referent Shared authority whose direct parent is the exact incoming replacement-capable authority selected by slot `i` and whose target is that parent's exact target. The detailed target, authority, carrier, restoration, and result validity laws are owned by `references.md`; return and caller-transfer ordering are owned by `function-execution.md` and `references.md`.

Under `concrete-syntax.md`, omission of a result clause maps to `no result value`, while an explicit result type maps to one result value only when that type expression satisfies this result-admission rule and, for an abstract parameter, the additional boundary from `generics.md`. Existing `-> &T` syntax requires no lifetime or origin-selector grammar for either represented contract. `&*r` already spells the explicit complete-referent Shared child producer used by a direct-child body. Generic type-position precedence provides no escape from the first-slice reference/raw boundary: if the `T` in `-> &T`, `-> &mut T`, or `-> raw T` selects an in-scope generic type parameter, that selected abstract parameter is rejected as an inadmissible referent/pointee under `generics.md`, and lookup does not fall through to a same-named nominal record. Concrete `-> &mut T` and `-> raw T` results remain rejected by the result-admission rules above as well.

`no result value` is callable-signature/interface structure. It does not introduce an intrinsic Unit, Void, or equivalent source value type.

A future Unit-like source value type, if accepted, would be an ordinary result-bearing source type unless its canonical owner explicitly defines a different relation.

## Callable-signature equality

Two represented **source function callable signatures** are structurally equal exactly when all of the following hold:

- they have the same number of generic type-parameter slots;
- after aligning generic type-parameter slots by their ordered positions, each corresponding pair of marker-requirement sets contains exactly the same marker trait entity identities;
- they have the same number of ordinary parameter slots;
- after aligning generic type-parameter slots by their ordered positions, each pair of corresponding ordinary parameter type expressions is equal under the rule below;
- either both specify no result value, or both specify one result value whose corresponding type expressions are equal under the rule below; and
- their safe-reference result contracts are equal, meaning both are `None`, both are `SharedIdentity` with the same origin ordinary parameter slot, or both are `SharedDirectChild` with the same origin ordinary parameter slot.

For this comparison, corresponding type expressions are equal exactly when:

- two concrete source type expressions are equal under `types.md`; or
- both are abstract type-parameter uses whose semantic slots occupy the same position in the aligned generic binder sequences.

A concrete type expression and an abstract type-parameter use are never equal in this comparison. Two different abstract binder positions are likewise unequal even when some concrete application could substitute equal concrete types for both.

This makes generic signature equality alpha-invariant: type-parameter lexical spelling is not an equality dimension. The binder structure, each binder's exact resolved marker-requirement set, and the positions at which each binder is used remain equality dimensions.

Marker-requirement lexical spelling and source order are not equality dimensions after resolution. Exact resolved trait identity and set membership are equality dimensions. Reordering the same requirements or changing only qualification/spelling that resolves to the same trait entity does not change callable-signature equality.

A parameter whose concrete type is `SharedRef(T)` or `ExclusiveReplaceRef(T)` participates in signature equality through that exact source type identity. A Shared-reference result additionally participates through its result-contract variant and advertised ordinary parameter origin slot. No hidden lifetime/pass-mode, implementation witness, capture slot, or generic-substitution dimension is compared.

Raw-pointer and opaque closure-site types do not participate in represented source function callable-signature equality because they are not parameter/result-admissible in this slice.

Under the current bounded elision rule, two otherwise equal represented parameter/result type-expression sequences deterministically derive the same result contract. Retaining the contract as an explicit semantic signature dimension establishes independent call/recursion behavior without making body implementation dataflow or one concrete generic application part of signature equality.

Callable-signature equality does not make two source function entities identical.

It also does not establish function-value payload identity or formation, closure-site/type equality or compatibility, implicit conversion, trait conformance beyond the separately owned marker requirement structure, overload relation, substitutability relation, or ABI compatibility. The bounded first-class captureless function-value type is owned separately by `function-values.md`; closure type identity is owned separately by `closures.md`.

## Exported-signature source accessibility

If a represented function binding is exported, every concrete nominal record source type exposed by one of these positions MUST be exported from the source module that defines that record type:

- a nominal record appearing directly as a parameter type;
- a nominal record appearing directly as a result type;
- the direct nominal referent `T` of a parameter type `SharedRef(T)`;
- the direct nominal referent `T` of a parameter type `ExclusiveReplaceRef(T)`;
- the direct nominal referent `T` of a result type `SharedRef(T)`; or
- any nominal record reached by recursively traversing a concrete function-value type that appears directly as a parameter/result type, including nominal records reached through an admitted safe-reference component of that finite function-value interface.

The function-value traversal follows only finite callable-interface parameter/result type syntax. It recursively enters nested concrete function-value types, follows the one direct referent edge of an admitted safe-reference component, and does not traverse fields of a nominal record merely because that record identity has been reached. Function-valued nominal record fields are independently absent in this slice.

If an exported function is generic, every marker trait identity contained in any of its type-parameter requirement sets MUST denote an exported marker trait binding under `names-modules.md`/`traits.md`.

A module-private marker trait MAY constrain a module-private generic function. It MUST NOT appear in the externally relevant marker-requirement structure of an exported generic function.

Intrinsic scalar source types have no module-binding accessibility requirement. A safe reference whose direct referent is intrinsic likewise adds no module-binding accessibility requirement.

An abstract generic type parameter has no module-binding accessibility requirement because it denotes the function-local semantic binder rather than one nominal record declaration. Its marker-requirement set has the separate trait-binding accessibility requirement above because those exact trait identities are part of the exported callable contract. A later explicit application resolves any concrete nominal-record type argument at the application site under `generics.md` and `names-modules.md`; that application does not retroactively expose the caller-selected nominal type as a named declaration in the generic function's defining-module signature.

No raw-pointer or opaque-closure accessibility traversal is needed because those parameter/result types are invalid regardless of accessibility.

A nominal record source type from another source module is already required to be exported for the function declaration to resolve its binding through qualified cross-module lookup. The rule above additionally prevents an exported function from exposing a module-private same-module nominal record directly or through one admitted safe-reference parameter/result edge.

The safe-reference result contract itself has no separate accessibility requirement. It selects an ordered ordinary parameter position already present in the callable interface; source parameter binding names and generic type-parameter lexical keys do not become exported interface value names.

Outside the finite function-value traversal above, this rule follows only the one direct safe-reference referent edge admitted by this slice. Nested references remain invalid and nominal-record fields are not recursively traversed for exported callable-interface accessibility.

This rule does not recursively redefine accessibility of fields contained by a nominal record type; record field/member accessibility remains outside this owner.

This accessibility rule concerns source name/type/marker-contract accessibility only. It does not define ABI symbol export, linkage, calling convention, interface serialization, physical visibility, binary compatibility, lifetime publication, generic package instantiation, reference representation, marker implementation publication, or replacement capability realization.

The represented concrete function form exercises this rule when modified by `export`; the unmodified form remains module-private under `concrete-syntax.md` and `names-modules.md`.

## Explicitly absent callable dimensions

This revision does not define callable-signature/interface dimensions for:

- generic value/lifetime parameters, parameter packs, capability-bearing constraints, supertrait/associated-item requirements, or where-clauses beyond the first marker-requirement sets integrated through `generics.md`/`traits.md`;
- variadic parameter lists;
- default arguments;
- parameter names in signature identity;
- named-argument calls;
- receiver or `self` distinctions;
- closure capture slots as callable parameters;
- a separate ownership/borrow/reference pass mode beyond the parameter's represented source value type/expression;
- explicit source lifetime names, outlives clauses, or an explicit result-origin/result-contract selector spelling;
- unsafe callable/caller-obligation contracts or an unsafe call qualifier;
- effect, purity, async/task, const, target, placement, numeric-contract, calling-convention, ABI, FFI, or fault qualifiers;
- raw-pointer parameter/result transfer or a raw-pointer escape/effect contract; or
- opaque closure values as ordinary parameter/result types.

The bounded safe-reference result contract changes exact result authority behavior but adds no separate lifetime or borrowed-pass dimension. `ExclusiveReplaceRef(T)` remains an ordinary owned parameter type whose permission class is part of concrete type identity; it does not create a borrowed-call pass mode or hidden effect dimension.

Lexical unsafe admission inside a function or closure body under `raw-pointers-unsafe.md` does not add a callable-signature/interface dimension. All represented source functions, including generic functions, and represented closures remain safe callables and must discharge represented unsafe raw-operation preconditions internally.

The absence of the other dimensions does not imply that represented bodies are pure, non-faulting, synchronous, target-independent, or ABI-neutral. Those dimensions remain undefined until their canonical owners are accepted.

## Execution boundary

`function-execution.md` owns the represented source-function execution relation built on source function entities/signatures after direct or bounded-indirect target selection and the shared call validation/argument/result/fault/cleanup relations consumed by bounded closure calls after `closures.md` performs closure-specific target/snapshot handling. `generics.md` owns the type-argument/substitution and marker-obligation facts consumed when the selected source function is generic, including generic-application validation and the non-observable activation substitution context. This callable owner therefore does not duplicate:

- represented source function or closure body attachment;
- straight-line body execution order;
- source-function activation identity or closure activation identity;
- source-call execution after target classification;
- generic application presence/arity/type-argument/substitution/marker-obligation validation or generic activation substitution context;
- argument evaluation order or argument/result ownership transfer;
- closure-specific callee snapshot/held-environment semantics;
- safe-reference argument carrier production/transfer and caller suspension consequences;
- replacement-capable call-entry full-authority/full-availability checks;
- replacement-capable external-referent state and normal restoration;
- safe-reference result-carrier preservation/derived-child transfer consequences;
- lexical-scope or activation cleanup;
- return, direct/bounded-indirect recursion, or closure-call completion;
- source-call divergence; or
- defined-fault propagation through source or closure activations.

[Source marker traits](traits.md) owns marker trait identity, explicit implementation propositions, compilation-global coherence, and exact marker-obligation satisfaction; no marker evidence becomes an execution value or dispatch relation here.

[Source safe references](references.md) owns the reference-specific target/authority/carrier/lifetime, external-referent, call-entry/restoration, and advertised safe-reference result-contract relation consumed by that execution.

[Source raw pointers and unsafe admission](raw-pointers-unsafe.md) owns activation-local raw-pointer target/origin/unsafe semantics. Because raw-pointer parameter/result types are invalid here and generic type arguments cannot be raw pointers, the represented source-call relation transports no raw-pointer value or pointer-origin provenance across activation boundaries through ordinary/generic parameters or a closure interface.

[Source closures and explicit by-value capture](closures.md) owns closure-site identity, opaque closure types, capture environments, dedicated closure bindings, bounded closure target selection, held-environment snapshot/transfer, and closure-to-Core refinement. Reusing the concrete callable-interface relation here does not turn a closure into a source function entity.

[Core faults](../core/faults.md) remains the authority for the currently represented Core fault classification and facts. `function-execution.md` consumes that fault identity to define propagation through represented source and closure activations; broader panic forms, catch boundaries, payloads, and non-source propagation relations remain incomplete until their canonical owners are accepted.

Function-value payload/formation and the bounded indirect function-value target branch remain owned by `function-values.md`; the bounded closure branch remains owned by `closures.md`. Overload dispatch, methods, external/FFI execution, intrinsic execution, async/task invocation, and other future callable forms remain unrepresented and are not implied by either relation.

## Implementation boundary

This revision does not add or require a particular parser, lossless-syntax representation, HIR, Core MIR production representation, runtime representation, or backend representation.

`concrete-syntax.md` defines one bounded concrete function/closure/generic-parameter/marker-requirement/parameter/result/body/call/generic-application/return subset including safe-reference and raw-pointer type spellings. General expression syntax, parser recovery, broader callable forms, explicit result-origin/result-contract selectors, multiple candidate result origins, lifetime syntax, capability-bearing generic constraints/inference, unsafe callable syntax, and raw-pointer call-transfer syntax remain outside this owner and cannot be inferred from represented callable semantics.

A faithful frontend representation of a source function MUST retain the ordered generic binder structure, each aligned slot's exact marker-requirement set, and parametric type-expression uses required by `generics.md`, plus the safe-reference result-contract variant and advertised ordinary parameter slot when one exists. A faithful representation of a closure site MUST retain its explicit parameter/result interface and the same derived contract over explicit slots while keeping closure identity/captures separately under `closures.md`. Neither may reconstruct these facts from body implementation dataflow, capture storage, lexical spelling accidents, concrete specialization accidents, lower Core local numbering, or runtime reference state.

A frontend MUST retain exact safe-reference permission in concrete parameter type identity. It MUST reject replacement-capable result declarations, raw-pointer parameter/result declarations, and opaque closure parameter/result declarations under this callable admission relation even when other grammar can represent related source forms.

## Further boundaries

Beyond the concrete subset owned by `concrete-syntax.md`, the first function-only explicit generic relation owned by `generics.md`, the marker-only trait/coherence relation owned by `traits.md`, and the bounded closure relation owned by `closures.md`, this revision does not define broader closures/captures, plain-Exclusive source reference forms, multiple or explicit source result-origin choices, projected/subregion reference results, arbitrary descendant result contracts, reference/pass-mode signature dimensions beyond represented safe-reference value types and the bounded result contract, lifetime names/parameters/outlives clauses, generic records, generic reference/raw-pointer constructors, type inference/default generic arguments, capability-bearing trait bounds, where clauses, trait methods/associated items/supertraits, generic/blanket/negative/specialized implementations, trait objects/dynamic dispatch/runtime witnesses, overload sets, effect-system completion, async/tasks, unsafe callable/call contracts, raw-pointer transfer, reference-containing aggregate results, static/global reference origins, ABI/calling conventions/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR lowering, or backend behavior.

The activation-local raw-pointer and lexical unsafe-admission relation from `raw-pointers-unsafe.md` is deliberately not a callable dimension in this revision.
