# Source Callables

Status: **provisional normative; incomplete**

This document owns the represented source function entity identity, callable-signature structure and equality, contextual parameter/result type admission, the first-slice Shared-reference result-origin parameter-slot contract, and exported-signature source accessibility.

It consumes module binding identity and accessibility from [Source names and modules](names-modules.md), represented source value type identity and equality from [Source type foundation](types.md), and first-slice Shared-reference contextual/type/result-origin accessibility facts from [Source Shared references](references.md). It does not redefine those owners.

Represented source function body attachment, dynamic activations, direct calls, owned argument/result transfer including Shared-reference carrier/result consequences, recursion, cleanup, return, divergence, and defined-fault propagation through direct calls are owned by [Source function execution](function-execution.md). Function-local parameter binding identity, scope, mutability, availability, and ordinary owned use are owned by [Source function-local bindings](local-bindings.md). The represented concrete function-definition, parameter, result, and Shared-reference type spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define indirect calls or function values, effects, generics, ABI, source lifetime names, a separate reference pass mode, or an implementation representation.

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
3. one optional **Shared-reference result-origin parameter slot**.

Each parameter slot contains exactly one **parameter-admissible represented source value type**. The parameter sequence may be empty.

A source type is parameter-admissible in this revision exactly when it is either:

- one represented intrinsic scalar or nominal record source type that was already admissible before the Shared-reference slice; or
- one represented `SharedRef(T)` satisfying the first-slice Shared-reference type/referent restrictions from `references.md` and `types.md`.

A Shared-reference parameter remains an ordinary source parameter slot. Its reference type is the slot's source value type; no second borrowed/reference/pass-mode dimension is added to the callable signature.

Parameter slots have no lexical identifier key or source binding name under this signature contract. For a concrete function definition under `concrete-syntax.md`, concrete parameter order maps directly to parameter-slot order; `local-bindings.md` owns the corresponding parameter binding keys, identities, scope, mutability, and availability. Those body-local facts do not become callable-signature dimensions.

The ordered parameter sequence is semantic signature structure. Positional direct-call validation, argument evaluation order, owned-value transfer, Shared-reference carrier transfer, and Shared-reference result-carrier consequences are owned by `function-execution.md`; physical register, stack, ABI, storage-layout, or other calling mechanisms remain outside this owner.

The result specification is exactly one of:

- **no result value**; or
- **one result value** of exactly one **result-admissible represented source value type**.

A source type is result-admissible in this revision exactly when it is either:

- one represented intrinsic scalar or nominal record source type that was already result-admissible; or
- one represented `SharedRef(T)` satisfying the bounded Shared-reference result-origin contract below.

For a result `SharedRef(T)`, `T` MUST satisfy the existing first-slice Shared-referent-admission relation from `references.md`. Let `M` be the ordered set of parameter-slot indices whose exact resolved source type is the same source type `SharedRef(T)`. The function declaration is source-valid only when `M` contains exactly one slot. That sole slot is the callable signature's Shared-reference result-origin parameter slot.

Consequently:

- a Shared-reference result with zero exact matching Shared-reference parameter slots is source-invalid because no result origin exists;
- a Shared-reference result with two or more exact matching Shared-reference parameter slots is source-invalid because the first-slice origin is ambiguous; and
- an ordinary or no-result callable has no Shared-reference result-origin slot.

The origin descriptor selects the ordered parameter slot itself. It is not a source parameter name, body-local binding identity, lifetime name, implementation local identifier, dynamic activation identity, storage identity, physical address, or lower Core identifier.

This **unique-origin elision** is the first represented concrete source way to establish the semantic origin slot. It is not body-derived origin inference. The advertised origin is established from callable structure before body validation and therefore remains available to independent direct-call validation, nested calls, direct recursion, and mutual recursion without inspecting or expanding a callee body.

The source-validity law for a normal Shared-reference result value, including exact dynamic target/authority identity and permitted identity-preserving body transport, is owned by `references.md`. Return and caller-transfer ordering are owned by `function-execution.md`.

Under `concrete-syntax.md`, omission of a result clause maps to `no result value`, while an explicit result type maps to one result value only when that type satisfies this result-admission rule. Existing `-> &T` syntax therefore requires no lifetime or origin-selector grammar to exercise this bounded contract.

`no result value` is callable-signature structure. It does not introduce an intrinsic Unit, Void, or equivalent source value type.

A future Unit-like source value type, if accepted, would be an ordinary result-bearing source type unless its canonical owner explicitly defines a different relation.

## Callable-signature equality

Two represented callable signatures are structurally equal exactly when all of the following hold:

- they have the same number of parameter slots;
- each pair of corresponding parameter source types is equal under `types.md`;
- either both specify no result value, or both specify one result value whose source types are equal under `types.md`; and
- either both have no Shared-reference result-origin slot, or both designate the same parameter-slot index as their Shared-reference result origin.

A parameter or result whose type is `SharedRef(T)` therefore participates in signature equality through that exact source type identity, and a Shared-reference result additionally participates through its advertised origin slot. No hidden lifetime/pass-mode dimension is compared.

Under the current unique-origin elision rule, two otherwise equal represented parameter/result type sequences deterministically derive the same optional origin slot. Retaining the origin as an explicit semantic signature dimension nevertheless establishes the durable callable contract required for later explicit multiple-origin/source-lifetime designs without making body implementation dataflow part of signature equality.

Callable-signature equality does not make two source function entities identical.

It also does not establish a first-class function type, function-pointer type, closure type, implicit conversion, trait conformance, overload relation, substitutability relation, or ABI compatibility.

## Exported-signature source accessibility

If a represented function binding is exported, every nominal record source type exposed by one of these positions MUST be exported from the source module that defines that record type:

- a nominal record appearing directly as a parameter type;
- a nominal record appearing directly as a result type;
- the direct nominal referent `T` of a parameter type `SharedRef(T)`; or
- the direct nominal referent `T` of a result type `SharedRef(T)`.

Intrinsic scalar source types have no module-binding accessibility requirement. A Shared reference whose direct referent is intrinsic likewise adds no module-binding accessibility requirement.

A nominal record source type from another source module is already required to be exported for the function declaration to resolve its binding through qualified cross-module lookup. The rule above additionally prevents an exported function from exposing a module-private same-module nominal record either directly or through one first-slice Shared-reference parameter or result.

The Shared-reference result-origin parameter slot itself has no separate accessibility requirement. It selects an ordered parameter position already present in the callable interface; source parameter binding names remain body-local and do not become exported interface names.

This rule follows only the one direct Shared-reference referent edge admitted by this slice. Nested Shared-reference referents are invalid and record fields cannot contain Shared references, so no broader recursive reference-interface traversal is defined.

This rule does not recursively redefine accessibility of fields contained by a nominal record type; record field/member accessibility remains outside this owner.

This accessibility rule concerns source name/type accessibility only. It does not define ABI symbol export, linkage, calling convention, interface serialization, physical visibility, binary compatibility, lifetime publication, or reference representation.

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
- explicit source lifetime names, outlives clauses, or an explicit result-origin selector spelling;
- effect, purity, async/task, unsafe, const, target, placement, numeric-contract, calling-convention, ABI, FFI, or fault qualifiers;
- first-class function or function-pointer values.

The bounded Shared-reference result-origin parameter slot defined above is a callable contract dimension because it changes the identity-preserving source result relation. It does not create a lifetime parameter, pass mode, or another callable category.

Their absence from this signature does not imply that represented function bodies are pure, non-faulting, safe, synchronous, non-generic, target-independent, or ABI-neutral. Those dimensions remain undefined until their canonical owners are accepted.

## Execution boundary

`function-execution.md` is the sole source owner for the represented direct-function execution relation built on these function entities and signatures. This callable owner therefore does not duplicate:

- represented source body attachment;
- straight-line body execution order;
- dynamic activation identity or state;
- direct-call target validity;
- argument evaluation order or argument/result ownership transfer;
- Shared-reference argument carrier production/transfer and caller suspension consequences;
- Shared-reference result-carrier preservation/transfer consequences;
- lexical-scope or activation cleanup;
- direct return or recursion;
- direct-call divergence; or
- defined-fault propagation through source activations.

[Source Shared references](references.md) owns the reference-specific target/authority/carrier/lifetime and advertised result-origin identity relation consumed by that execution.

[Core faults](../core/faults.md) remains the authority for the currently represented Core fault classification and facts. `function-execution.md` consumes that fault identity to define propagation through represented source activations; broader panic forms, catch boundaries, payloads, and non-source propagation relations remain incomplete until their canonical owners are accepted.

Indirect calls, function values, closures, overload dispatch, methods, external/FFI execution, intrinsic execution, async/task invocation, and other future callable forms are not implied by the direct-call relation.

## Implementation boundary

This revision does not add or require a parser, lossless-syntax representation, HIR, Core MIR production representation, runtime representation, or backend representation.

`concrete-syntax.md` defines one bounded concrete function/parameter/result/body/call/return subset including the first Shared-reference parameter/result type spelling. General expression syntax, parser recovery, broader callable forms, explicit result-origin selectors, multiple candidate result origins, and lifetime syntax remain outside this owner and cannot be inferred from the represented callable semantics.

A faithful frontend representation MUST retain the advertised Shared-reference result-origin parameter slot as source callable contract structure when one exists. It MUST NOT reconstruct that fact from body implementation dataflow, parameter names, lower Core local numbering, or runtime reference state.

## Further boundaries

Beyond the concrete subset owned by `concrete-syntax.md`, this revision does not define closures/captures, mutable/exclusive reference forms, multiple or explicit source result-origin choices, derived/subregion reference results, reference/pass-mode signature dimensions beyond the represented `SharedRef(T)` value type and bounded result-origin slot, lifetime names/parameters/outlives clauses, generics, traits/coherence, methods, overload sets, effect-system completion, async/tasks, source `unsafe`, raw-pointer transfer, reference-containing aggregate results, static/global reference origins, ABI/calling conventions/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR lowering, or backend behavior.