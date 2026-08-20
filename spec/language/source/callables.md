# Source Callables

Status: **provisional normative; incomplete**

This document owns the represented source function entity identity, callable-signature structure and equality, and exported-signature source accessibility.

It consumes module binding identity and accessibility from [Source names and modules](names-modules.md) and represented source value type identity and equality from [Source type foundation](types.md). It does not redefine those owners.

Represented source function body attachment, dynamic activations, direct calls, owned argument/result transfer, recursion, cleanup, return, divergence, and defined-fault propagation through direct calls are owned by [Source function execution](function-execution.md). Function-local parameter binding identity, scope, mutability, availability, and ordinary owned use are owned by [Source function-local bindings](local-bindings.md).

This document does not define concrete function syntax, indirect calls or function values, effects, generics, ABI, or an implementation representation.

## Source function entities

A **function declaration** is a module-level source declaration that introduces exactly one module binding under `names-modules.md`.

That binding denotes one **source function entity**. Function entity identity is the identity of that declaration/binding.

Distinct function declarations denote distinct source function entities even when their callable signatures are structurally equal.

The binding's module-private or exported accessibility is determined only by `names-modules.md`; this document does not redefine accessibility.

Different callable signatures do not make duplicate module binding keys legal. This revision defines no function overloading or overload set.

This revision does not define concrete function-declaration or function-definition syntax. `function-execution.md` owns whether a represented source function entity has a represented source body and the execution relation attached to that body; those facts do not redefine function entity identity or callable-signature structure.

## Callable signatures

Every represented source function entity has exactly one **callable signature** containing:

1. one finite ordered sequence of parameter slots; and
2. one result specification.

Each parameter slot contains exactly one represented source value type from `types.md`. The parameter sequence may be empty.

Parameter slots have no lexical identifier key or source binding name under this signature contract. When a represented function body exists under `function-execution.md`, `local-bindings.md` owns the corresponding parameter binding keys, identities, scope, mutability, and availability. Those body-local facts do not become callable-signature dimensions.

The ordered parameter sequence is semantic signature structure. Positional direct-call validation, argument evaluation order, and owned-value transfer are owned by `function-execution.md`; physical register, stack, ABI, storage-layout, or other calling mechanisms remain outside this owner.

The result specification is exactly one of:

- **no result value**; or
- **one result value** of exactly one represented source value type from `types.md`.

`no result value` is callable-signature structure. It does not introduce an intrinsic Unit, Void, or equivalent source value type.

A future Unit-like source value type, if accepted, would be an ordinary result-bearing source type unless its canonical owner explicitly defines a different relation.

## Callable-signature equality

Two represented callable signatures are structurally equal exactly when all of the following hold:

- they have the same number of parameter slots;
- each pair of corresponding parameter source types is equal under `types.md`; and
- either both specify no result value, or both specify one result value whose source types are equal under `types.md`.

Callable-signature equality does not make two source function entities identical.

It also does not establish a first-class function type, function-pointer type, closure type, implicit conversion, trait conformance, overload relation, substitutability relation, or ABI compatibility.

## Exported-signature source accessibility

If a represented function binding is exported, every nominal record source type that appears directly as a parameter type or result type in its callable signature MUST be exported from the source module that defines that record type.

Intrinsic scalar source types have no module-binding accessibility requirement.

A nominal record source type from another source module is already required to be exported for the function declaration to resolve its binding through qualified cross-module lookup. The rule above additionally prevents an exported function from directly exposing a module-private nominal record type defined in its own module.

This rule checks only source types directly present in the callable signature. It does not recursively redefine accessibility of fields contained by a nominal record type; record field/member accessibility remains outside this owner.

This accessibility rule concerns source name/type accessibility only. It does not define ABI symbol export, linkage, calling convention, interface serialization, physical visibility, or binary compatibility.

## Explicitly absent callable dimensions

This revision does not define callable-signature dimensions for:

- generic type, value, or lifetime parameters, packs, constraints, or where-clauses;
- variadic parameter lists;
- default arguments;
- parameter names in signature identity;
- named-argument calls;
- receiver or `self` distinctions;
- effect, purity, async/task, unsafe, const, target, placement, numeric-contract, calling-convention, ABI, FFI, or fault qualifiers;
- first-class function or function-pointer values.

Their absence from this signature does not imply that represented function bodies are pure, non-faulting, safe, synchronous, non-generic, target-independent, or ABI-neutral. Those dimensions remain undefined until their canonical owners are accepted.

## Execution boundary

`function-execution.md` is the sole source owner for the represented direct-function execution relation built on these function entities and signatures. This callable owner therefore does not duplicate:

- represented source body attachment;
- dynamic activation identity or state;
- direct-call target validity;
- argument evaluation order or argument/result ownership transfer;
- lexical-scope or activation cleanup;
- direct return or recursion;
- direct-call divergence; or
- defined-fault propagation through source activations.

[Core faults](../core/faults.md) remains the authority for the currently represented Core fault classification and facts. `function-execution.md` consumes that fault identity to define propagation through represented source activations; broader panic forms, catch boundaries, payloads, and non-source propagation relations remain incomplete until their canonical owners are accepted.

Indirect calls, function values, closures, overload dispatch, methods, external/FFI execution, intrinsic execution, async/task invocation, and other future callable forms are not implied by the direct-call relation.

## Implementation boundary

This revision does not add or require a parser, lossless-syntax representation, HIR, Core MIR production representation, runtime representation, or backend representation.

The accepted function-execution relation does not by itself define concrete function, parameter, body, call, or return grammar, general expression syntax, literal syntax, or parser recovery. Those concerns require their own accepted owner before a frontend implementation may rely on them.

## Further boundaries

This revision does not define concrete function/parameter/result syntax, keywords, punctuation, comments, literals, patterns, closures/captures, reference/borrow/pass-mode signature dimensions, source references/lifetimes, generics, traits/coherence, methods, overload sets, effect-system completion, async/tasks, ABI/calling conventions/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR lowering, or backend behavior.
