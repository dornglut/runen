# Source External Callable Imports

Status: **provisional normative; incomplete**

This document owns the represented source-language semantics for declaration-only external callable requirements, their external execution origin, the first scalar-only external callable interface restriction, direct external-call execution after ordinary source call-site evaluation succeeds, environment-provider admission consequences, and source-to-Core refinement of this bounded relation.

It consumes source function-entity identity and callable-interface structure from [Source callable signatures](callables.md); module binding identity, accessibility, local-before-module participation, and qualified lookup from [Source names and modules](names-modules.md); represented intrinsic scalar type identities from [Source type foundation](types.md); ordinary source call-site producer transaction, argument evaluation, result receiving, and caller-state consequences from [Source function execution](function-execution.md); function-value formation and indirect-target boundaries from [Source function values and indirect calls](function-values.md); generic-body and closure-body module lookup from [Source generics](generics.md) and [Source closures and explicit by-value capture](closures.md); environment admission from [Language lifecycle](../lifecycle.md); host/environment observation authority from [Program behavior](../behavior.md); the lower external-call relation from [Core external callable imports](../core/external-calls.md); and concrete spelling from [Source concrete syntax](concrete-syntax.md). It does not redefine those owners.

This relation is a semantic environment-call interface. It does not define a binary ABI, calling convention, stable memory representation, linker symbol, physical function address, dynamic-library contract, entry point, or backend mechanism.

## External callable declaration and identity

A represented **external callable declaration** establishes one ordinary source function entity under `callables.md` and one ordinary module binding under `names-modules.md`.

The function entity has one **external execution origin**. It has no represented Runen source body and MUST NOT simultaneously have a represented Runen body. Its source function-entity identity is the declaration identity already owned by `callables.md`; this document does not create a second source callable identity merely to mark the external origin.

For source validation and environment admission, that same externally originated function entity identifies one **external callable requirement**. The requirement identity is therefore stable within the source compilation and distinct from every other external declaration even when two declarations have equal callable interfaces. It is not a physical symbol name, code address, dynamic-library handle, ABI tag, linker identity, serialized package identity, or reflection value.

The declaration contributes to the same single module declaration namespace as an ordinary Runen-body function. Existing duplicate-key rules therefore prohibit a same-key record, Runen-body function, external function, marker trait, constant, static, or other binding in that module.

The declaration has the ordinary module-private or exported source accessibility established by `names-modules.md`. Exported accessibility permits Runen cross-module lookup only. It does not establish binary symbol export, FFI publication, physical visibility, or linkage.

## External callable interface

An external callable declaration is source-valid only when all of these hold:

- it has no generic type-parameter slots;
- every parameter slot has exactly one represented intrinsic scalar type from `types.md`;
- it has either no result or exactly one represented intrinsic scalar result type;
- its safe-reference result contract under `callables.md` is `None`; and
- it has no source body.

The admitted intrinsic scalar types are exactly `Bool`, `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, `U64`, `F16`, `F32`, and `F64`.

Nominal records, safe references, replacement-capable references, raw pointers, captureless function-value types, opaque closure-site types, abstract generic parameters, and every other source type are not external parameter or result types in this revision.

The interface consists only of semantic source value types. Fixed integer width does not establish two's-complement storage, byte order, alignment, or ABI encoding. Binary floating semantic format does not establish physical IEEE bit representation or calling convention. `Bool` has no physical bit representation under this relation.

The declaration has positional semantic parameter slots only. The represented concrete declaration form therefore uses parameter **types** rather than introducing body-local parameter binding names: there is no Runen body in which such bindings could exist.

## Source validation and environment admission boundary

Source language validation establishes the external function entity, its exact scalar-only callable interface, all direct-call source validity, and the external callable requirement identity. Source validation does not resolve or execute an external provider.

A source-valid program containing an external callable requirement is executable in one environment only when environment admission satisfies the provider requirement owned by `core/external-calls.md` after source-to-Core refinement. A missing provider, ambiguous provider, provider-interface mismatch, or inability to preserve one required semantic scalar type makes that program inadmissible for that environment unless an otherwise legal adapter or emulation preserves the exact semantic contract.

Provider admission is not source module lookup and does not alter source name resolution. It also is not a physical symbol lookup, dynamic-library load, package dependency resolution, or backend-specific link step under this revision.

Environment admission happens before realization and execution under `lifecycle.md`. Consequently a missing or mismatched provider is not a runtime source fault, catchable result, undefined behavior, or fallback search for another source declaration.

## Direct target classification

External callables participate only in the existing **direct source call** target category.

For an unqualified call, existing local-first target classification runs unchanged. An active local callable/non-callable selection remains final under its existing owner. Only when module lookup is reached may the selected module binding denote an external callable declaration. When it does, the call is one direct external call governed by this document rather than one Runen-body call governed by the activation relation in `function-execution.md`.

For a represented qualified `alias::member(...)` call, existing qualified lookup may likewise select one exported external callable declaration. Source accessibility is the only meaning of that export fact.

A selected wrong-category binding remains final. External-call classification does not search another module, provider, symbol, declaration, overload, or fallback simply because the selected entity cannot satisfy the required call form.

A generic type-argument list is invalid on an external-call target because external declarations are non-generic.

## Function-value and closure boundary

An externally originated function entity is not a source function-value formation target in this revision. It cannot be placed in a represented `fn(...)` value, selected by a bounded indirect call, captured as a function value, returned as a function value, or used as a callback target through the existing first-class callable relation.

`function-values.md` therefore continues to form function values only from eligible non-generic **Runen-body** function entities.

A generic Runen function body MAY directly call a concrete external callable through ordinary module lookup. That use does not make the external declaration generic and introduces no value/const/lifetime generic parameter.

A closure body MAY likewise directly call an external callable through its retained declaration-site module/source-unit lookup context. The external function is not a closure capture target and occupies no closure environment slot.

No external provider may call back into Runen under this first relation. Callback/re-entry semantics require a separately accepted extension.

## Source external-call transaction

A represented external direct call uses the existing source `Call` token shape and the ordinary call-site validation transaction from `function-execution.md` up to, but not including, Runen callee activation creation.

Before any argument producer consequence commits, source validation establishes:

- the exact selected external function entity;
- its exact scalar-only callable interface;
- argument arity and each exact required argument type;
- whether the call has no result or exactly one required scalar result; and
- every ordinary receiving-position/result fact required by the enclosing source form.

Arguments then validate/evaluate in the same strict left-to-right order as ordinary source-call arguments. Each argument MUST produce exactly the corresponding intrinsic scalar type. The complete call remains one ordinary speculative source producer-validation transaction where `function-execution.md` already requires that behavior.

If an argument producer fails source validation, no rejected transaction consequence commits. During execution, if argument evaluation selects an already represented defined fault or diverges, the external provider is not invoked and the existing fault/divergence relation remains controlling.

Because the external interface contains no safe reference, raw pointer, function value, closure value, or aggregate, successful argument production yields only owned scalar transient values and requires no external-referent, safe-reference call-entry, raw-provenance, or closure-environment transfer.

## External invocation and normal continuation

After every argument value has been produced successfully, the admitted provider is invoked exactly once with the ordered exact semantic scalar values.

The provider relation has exactly these first-slice execution outcomes:

- **normal return** with no value when the external interface has no result;
- **normal return** with exactly one semantic scalar value of the declared result type when the interface has a result; or
- **divergence**.

A normal result is received by the existing source call receiving relation and ordinary caller execution continues. The provider creates no source function activation, source parameter/local bindings, source lexical scope, safe-reference authority, raw-pointer origin, static storage, or Runen cleanup domain.

Provider divergence makes the source call diverge. No normal continuation is selected and no source activation cleanup is performed merely because the external interaction does not return.

The first external provider relation does not originate a represented Runen defined fault. It also defines no foreign exception, panic, trap, unwind, status-code convention, cancellation, recoverable failure value, or hidden fallback result. Those require separately accepted contracts.

A provider or realization behavior outside the admitted provider contract is not a new permission for valid Runen source to exhibit undefined behavior. Such behavior fails the provider/environment/implementation contract and is not an admitted execution of the source program.

## Host and environment observations

An admitted provider contract MAY expose explicit host/environment observations under `behavior.md`. Only observations explicitly included by that admitted contract are observable through this relation.

The physical mechanism used to invoke the provider, temporary marshalling storage, host stack/register state, provider address, symbol table, loader state, adapter implementation, and other realization details remain unobservable unless a later normative contract explicitly exposes them.

This relation does not require an external provider to be deterministic independently of its admitted external observations. The general correctness and determinism relations remain those of [Correctness relations](../correctness.md) and `behavior.md`.

## Safety and authority

Calling a valid scalar-only external callable is not an unsafe-admission-required source operation merely because the provider is external.

No source safe-reference authority, raw-pointer provenance, storage identity, replacement capability, or hidden caller proof obligation crosses this boundary. Exact scalar interface agreement is established before execution by source validation and environment admission.

Execution capability does not imply security authority. A future or applicable Security authority contract MAY independently restrict which external callable requirements are permitted. The presence of a technically capable environment provider does not override [Language authority](../authority.md).

## Source-to-Core refinement

Faithful lowering maps every distinct source external callable declaration exactly once to one distinct Core external callable declaration under `core/external-calls.md`.

The mapping preserves:

- declaration distinction even for equal interfaces;
- exact ordered intrinsic scalar parameter types;
- exact no-result/scalar-result structure; and
- external execution origin.

The lowering erases source module identity, lexical declaration name, parameter presentation, and source accessibility after selection. No source `export` fact becomes Core linkage or symbol metadata.

Every direct source call selected as external lowers to one Core external-call control-transfer form naming the corresponding Core external callable identity. Ordinary Runen-body direct calls continue to lower to Core direct function calls, and bounded indirect calls continue to lower through `core/callable-values.md`.

Lowering MUST preserve the source call-site order: result destination admission before argument state effects where applicable, ordinary arguments left-to-right, provider invocation only after every argument succeeds, and normal result initialization before the normal continuation.

## Deliberate boundaries

This revision does not define:

- a named physical ABI or calling convention;
- stable scalar or record memory representation;
- endianness, padding, alignment, physical field layout, or register/stack classification;
- source raw-pointer or safe-reference FFI;
- record/aggregate external parameters or results;
- function-value, closure, callback, or foreign re-entry semantics;
- generic external declarations, trait ABI, or polymorphic external calls;
- external Runen exports or inbound foreign invocation;
- static data import/export;
- physical symbol names, symbol visibility/versioning, or linker identity;
- dynamic-library loading/unloading;
- executable entry-point syntax or selection;
- package/build/linker policy;
- variadics;
- panic/exception/unwind ABI; or
- a backend representation.

Those remain separate open items whose first concrete consumers must establish their own normative contracts.
