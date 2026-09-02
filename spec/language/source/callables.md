# Source Callables

Status: **provisional normative; incomplete**

This document owns the represented source function entity identity, callable-signature structure and equality, contextual parameter/result type admission, the bounded safe-reference result contract, and exported-signature source accessibility.

It consumes module binding identity and accessibility from [Source names and modules](names-modules.md), represented source value type identity and equality from [Source type foundation](types.md), safe-reference contextual/type/result-contract accessibility facts from [Source safe references](references.md), and the first-slice activation-local raw-pointer callable exclusion plus absence of an unsafe callable dimension from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md). It does not redefine those owners.

Represented source function body attachment, dynamic activations, direct calls, owned argument/result transfer including safe-reference carrier/result/external-referent consequences, recursion, cleanup, return, divergence, and defined-fault propagation through direct calls are owned by [Source function execution](function-execution.md). Function-local parameter binding identity, scope, mutability, availability, and ordinary owned use are owned by [Source function-local bindings](local-bindings.md). The represented concrete function-definition, parameter, result, safe-reference type, and raw-pointer type spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define indirect calls or function values, effects, generics, ABI, source lifetime names, a separate reference pass mode, an unsafe callable contract, or an implementation representation.

## Source function entities

A **function declaration** is a module-level source declaration that introduces exactly one module binding under `names-modules.md`.

That binding denotes one **source function entity**. Function entity identity is the identity of that declaration/binding.

Distinct function declarations denote distinct source function entities even when their callable signatures are structurally equal.

The binding's module-private or exported accessibility is determined only by `names-modules.md`; this document does not redefine accessibility.

Different callable signatures do not make duplicate module binding keys legal. This revision defines no function overloading or overload set.

The represented `fn` definition in `concrete-syntax.md` establishes one function declaration/entity, one callable signature as mapped below, and one represented source body attached under `function-execution.md`. Without the concrete `export` modifier that function binding is module-private; with the modifier it is exported under `names-modules.md`. Concrete spelling does not redefine function identity or callable-signature structure.

A future concrete declaration-only or alternate definition form, if accepted, must map to these semantic entities without creating a competing identity relation.

## Callable signatures

Every represented source function entity has exactly one **callable signature** containing:

1. one finite ordered sequence of parameter slots;
2. one result specification; and
3. one **safe-reference result contract**.

The safe-reference result contract is exactly one of:

- **None**;
- **SharedIdentity(origin)**, designating one parameter-slot index; or
- **SharedDirectChild(origin)**, designating one parameter-slot index.

Each parameter slot contains exactly one **parameter-admissible represented source value type**. The parameter sequence may be empty.

A source type is parameter-admissible in this revision exactly when it is either:

- one represented intrinsic scalar or nominal record source type admitted by `types.md`;
- one represented `SharedRef(T)` satisfying the Shared-reference restrictions from `references.md`; or
- one represented `ExclusiveReplaceRef(T)` satisfying the replacement-reference restrictions from `references.md`.

A represented `RawPtr(T)` is not parameter-admissible. The raw-pointer type may be syntactically present in a parameter position through the general `Type` grammar, but such a function declaration is source-invalid before body execution or lowering. This preserves the activation-local raw-pointer contract and the existing Core prohibition on raw-pointer-containing parameter transfer.

Safe-reference parameters remain ordinary source parameter slots. Their exact safe-reference type is the slot's source value type; no second borrowed/reference/pass-mode dimension is added to the callable signature.

Parameter slots have no lexical identifier key or source binding name under this signature contract. For a concrete function definition under `concrete-syntax.md`, concrete parameter order maps directly to parameter-slot order; `local-bindings.md` owns the corresponding parameter binding keys, identities, scope, mutability, and availability. Those body-local facts do not become callable-signature dimensions.

The ordered parameter sequence is semantic signature structure. Positional direct-call validation, argument evaluation order, owned-value transfer, reference-carrier transfer, call-entry authority, and replacement-capable external-referent consequences are owned by `function-execution.md` and `references.md`; physical register, stack, ABI, storage-layout, or other calling mechanisms remain outside this owner.

The result specification is exactly one of:

- **no result value**; or
- **one result value** of exactly one **result-admissible represented source value type**.

A source type is result-admissible exactly when it is either:

- one represented intrinsic scalar or nominal record source type already result-admissible; or
- one represented `SharedRef(T)` satisfying the bounded safe-reference result-contract relation below.

`ExclusiveReplaceRef(T)` is not result-admissible. A concrete `-> &mut T` spelling is therefore source-invalid. This slice defines no replacement-capable result escape or restoration contract.

`RawPtr(T)` is likewise not result-admissible. A concrete `-> raw T` spelling is source-invalid.

For a result `SharedRef(T)`, `T` MUST satisfy the Shared-referent-admission relation from `references.md`. The callable contract is established deterministically from the parameter sequence before body validation as follows.

Let `S` be the ordered set of parameter-slot indices whose exact resolved source type is `SharedRef(T)`.

- If `S` contains exactly one slot `i`, the callable contract is **SharedIdentity(i)**.
- If `S` contains two or more slots, the declaration is source-invalid because the identity-preserving origin is ambiguous.
- Only when `S` is empty, let `R` be the ordered set of parameter-slot indices whose exact resolved source type is `ExclusiveReplaceRef(T)`.
- If `R` contains exactly one slot `i`, the callable contract is **SharedDirectChild(i)**.
- If `R` contains two or more slots, the declaration is source-invalid because the direct-child parent origin is ambiguous.
- If both `S` and `R` are empty, the declaration is source-invalid because no represented safe-reference result contract can be established.

Consequently:

- every currently represented identity-valid `SharedRef(T)` signature retains the exact same origin slot and identity-preserving contract;
- one exact `SharedRef(T)` parameter continues to establish identity even when one or more `ExclusiveReplaceRef(T)` parameters with the same referent also exist;
- a `SharedRef(T)` result with no exact Shared candidate may now use one unique `ExclusiveReplaceRef(T)` parameter as its direct-child parent origin;
- a source `ExclusiveReplaceRef(T)` parameter never establishes `SharedIdentity` merely because its referent is `T`;
- source exposes no plain Core `ExclusiveRef(T)`, so the represented source direct-child parent class is exactly `ExclusiveReplaceRef(T)`; and
- an ordinary, replacement-capable-result-invalid, raw-result-invalid, or no-result callable has contract **None**.

A mixed signature containing one exact `SharedRef(T)` candidate and one or more `ExclusiveReplaceRef(T)` candidates therefore remains identity-contract-bearing. This slice defines no syntax or alternate elision that selects a replacement-capable candidate instead while an exact Shared candidate exists.

The contract descriptor selects an ordered parameter slot. It is not a source parameter name, body-local binding identity, lifetime name, implementation local identifier, dynamic activation identity, storage identity, physical address, or lower Core identifier.

This **bounded result-contract elision** is the represented concrete source way to establish safe-reference result behavior. It is not body-derived origin inference. The advertised contract is established from callable structure before body validation and therefore remains available to independent direct-call validation, nested calls, direct recursion, and mutual recursion without inspecting or expanding a callee body.

For **SharedIdentity(i)**, normal result validity preserves the exact incoming Shared authority/target identity selected by slot `i`.

For **SharedDirectChild(i)**, normal result validity requires one complete-referent Shared authority whose direct parent is the exact incoming replacement-capable authority selected by slot `i` and whose target is that parent's exact target. The detailed target, authority, carrier, restoration, and result validity laws are owned by `references.md`; Return and caller-transfer ordering are owned by `function-execution.md` and `references.md`.

Under `concrete-syntax.md`, omission of a result clause maps to `no result value`, while an explicit result type maps to one result value only when that type satisfies this result-admission rule. Existing `-> &T` syntax requires no lifetime or origin-selector grammar for either represented contract. `&*r` already spells the explicit complete-referent Shared child producer used by a direct-child body. Syntactically represented `-> &mut T` and raw-pointer result types remain rejected by semantic admission.

`no result value` is callable-signature structure. It does not introduce an intrinsic Unit, Void, or equivalent source value type.

A future Unit-like source value type, if accepted, would be an ordinary result-bearing source type unless its canonical owner explicitly defines a different relation.

## Callable-signature equality

Two represented callable signatures are structurally equal exactly when all of the following hold:

- they have the same number of parameter slots;
- each pair of corresponding parameter source types is equal under `types.md`;
- either both specify no result value, or both specify one result value whose source types are equal under `types.md`; and
- their safe-reference result contracts are equal, meaning both are `None`, both are `SharedIdentity` with the same origin slot, or both are `SharedDirectChild` with the same origin slot.

A parameter whose type is `SharedRef(T)` or `ExclusiveReplaceRef(T)` participates in signature equality through that exact source type identity. A Shared-reference result additionally participates through its result-contract variant and advertised origin slot. No hidden lifetime/pass-mode dimension is compared.

Raw-pointer types do not participate in represented callable-signature equality because they are not parameter/result-admissible in this slice.

Under the current bounded elision rule, two otherwise equal represented parameter/result type sequences deterministically derive the same result contract. Retaining the contract as an explicit semantic signature dimension establishes independent call/recursion behavior without making body implementation dataflow part of signature equality.

Callable-signature equality does not make two source function entities identical.

It also does not establish a first-class function type, function-pointer type, closure type, implicit conversion, trait conformance, overload relation, substitutability relation, or ABI compatibility.

## Exported-signature source accessibility

If a represented function binding is exported, every nominal record source type exposed by one of these positions MUST be exported from the source module that defines that record type:

- a nominal record appearing directly as a parameter type;
- a nominal record appearing directly as a result type;
- the direct nominal referent `T` of a parameter type `SharedRef(T)`;
- the direct nominal referent `T` of a parameter type `ExclusiveReplaceRef(T)`; or
- the direct nominal referent `T` of a result type `SharedRef(T)`.

Intrinsic scalar source types have no module-binding accessibility requirement. A safe reference whose direct referent is intrinsic likewise adds no module-binding accessibility requirement.

No raw-pointer accessibility traversal is needed because raw-pointer parameter/result types are invalid regardless of pointee accessibility.

A nominal record source type from another source module is already required to be exported for the function declaration to resolve its binding through qualified cross-module lookup. The rule above additionally prevents an exported function from exposing a module-private same-module nominal record directly or through one admitted safe-reference parameter/result edge.

The safe-reference result contract itself has no separate accessibility requirement. It selects an ordered parameter position already present in the callable interface; source parameter binding names remain body-local and do not become exported interface names.

This rule follows only the one direct safe-reference referent edge admitted by this slice. Nested references are invalid and record fields cannot contain references, so no broader recursive reference-interface traversal is defined.

This rule does not recursively redefine accessibility of fields contained by a nominal record type; record field/member accessibility remains outside this owner.

This accessibility rule concerns source name/type accessibility only. It does not define ABI symbol export, linkage, calling convention, interface serialization, physical visibility, binary compatibility, lifetime publication, reference representation, or replacement capability realization.

The represented concrete function form exercises this rule when modified by `export`; the unmodified form remains module-private under `concrete-syntax.md` and `names-modules.md`.

## Explicitly absent callable dimensions

This revision does not define callable-signature dimensions for:

- generic type, value, or lifetime parameters, packs, constraints, or where-clauses;
- variadic parameter lists;
- default arguments;
- parameter names in signature identity;
- named-argument calls;
- receiver or `self` distinctions;
- a separate ownership/borrow/reference pass mode beyond the parameter's represented source value type;
- explicit source lifetime names, outlives clauses, or an explicit result-origin/result-contract selector spelling;
- unsafe callable/caller-obligation contracts or an unsafe call qualifier;
- effect, purity, async/task, const, target, placement, numeric-contract, calling-convention, ABI, FFI, or fault qualifiers;
- raw-pointer parameter/result transfer or a raw-pointer escape/effect contract; or
- first-class function or function-pointer values.

The bounded safe-reference result contract changes exact result authority behavior but adds no separate lifetime or borrowed-pass dimension. `ExclusiveReplaceRef(T)` remains an ordinary owned parameter type whose permission class is part of type identity; it does not create a borrowed-call pass mode or hidden effect dimension.

Lexical unsafe admission inside a function body under `raw-pointers-unsafe.md` does not add a callable-signature dimension. All represented functions remain safe callables and must discharge represented unsafe raw-operation preconditions internally.

The absence of the other dimensions does not imply that represented function bodies are pure, non-faulting, synchronous, non-generic, target-independent, or ABI-neutral. Those dimensions remain undefined until their canonical owners are accepted.

## Execution boundary

`function-execution.md` is the sole source owner for the represented direct-function execution relation built on these function entities and signatures. This callable owner therefore does not duplicate:

- represented source body attachment;
- straight-line body execution order;
- dynamic activation identity or state;
- direct-call target validity;
- argument evaluation order or argument/result ownership transfer;
- safe-reference argument carrier production/transfer and caller suspension consequences;
- replacement-capable call-entry full-authority/full-availability checks;
- replacement-capable external-referent state and normal restoration;
- safe-reference result-carrier preservation/derived-child transfer consequences;
- lexical-scope or activation cleanup;
- direct return or recursion;
- direct-call divergence; or
- defined-fault propagation through source activations.

[Source safe references](references.md) owns the reference-specific target/authority/carrier/lifetime, external-referent, call-entry/restoration, and advertised safe-reference result-contract relation consumed by that execution.

[Source raw pointers and unsafe admission](raw-pointers-unsafe.md) owns activation-local raw-pointer target/origin/unsafe semantics. Because raw-pointer parameter/result types are invalid here, the represented direct-call relation transports no raw-pointer value or pointer-origin provenance across activation boundaries.

[Core faults](../core/faults.md) remains the authority for the currently represented Core fault classification and facts. `function-execution.md` consumes that fault identity to define propagation through represented source activations; broader panic forms, catch boundaries, payloads, and non-source propagation relations remain incomplete until their canonical owners are accepted.

Indirect calls, function values, closures, overload dispatch, methods, external/FFI execution, intrinsic execution, async/task invocation, and other future callable forms are not implied by the direct-call relation.

## Implementation boundary

This revision does not add or require a parser, lossless-syntax representation, HIR, Core MIR production representation, runtime representation, or backend representation.

`concrete-syntax.md` defines one bounded concrete function/parameter/result/body/call/return subset including safe-reference and raw-pointer type spellings. General expression syntax, parser recovery, broader callable forms, explicit result-origin/result-contract selectors, multiple candidate result origins, lifetime syntax, unsafe callable syntax, and raw-pointer call-transfer syntax remain outside this owner and cannot be inferred from represented callable semantics.

A faithful frontend representation MUST retain the safe-reference result-contract variant and advertised parameter slot when one exists. It MUST NOT reconstruct that fact from body implementation dataflow, parameter names, lower Core local numbering, or runtime reference state.

A frontend MUST retain exact safe-reference permission in parameter type identity. It MUST reject replacement-capable result declarations and raw-pointer parameter/result declarations under this callable admission relation even when the grammar can represent those type spellings.

## Further boundaries

Beyond the concrete subset owned by `concrete-syntax.md`, this revision does not define closures/captures, plain-Exclusive source reference forms, multiple or explicit source result-origin choices, projected/subregion reference results, arbitrary descendant result contracts, reference/pass-mode signature dimensions beyond represented safe-reference value types and the bounded result contract, lifetime names/parameters/outlives clauses, generics, traits/coherence, methods, overload sets, effect-system completion, async/tasks, unsafe callable/call contracts, raw-pointer transfer, reference-containing aggregate results, static/global reference origins, ABI/calling conventions/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR lowering, or backend behavior.

The activation-local raw-pointer and lexical unsafe-admission relation from `raw-pointers-unsafe.md` is deliberately not a callable dimension in this revision.