# Core External Callable Imports

Status: **provisional normative; incomplete**

This document owns the represented Core semantics for declaration-only external callable requirements, stable in-program external callable identity, scalar-only external callable interfaces, environment-provider admission, one external-call control-transfer form, provider normal return/divergence, and the corresponding normal-continuation validation summary.

It consumes Core type/value identity and semantic scalar values from [Core value and storage semantics](value-storage.md); canonical callable-interface structure, result-destination admission, and ordinary ordered call-argument production from [Core functions and calls](functions.md); environment admission from [Language lifecycle](../lifecycle.md); host/environment observation authority from [Program behavior](../behavior.md); and the physical representation boundary from [Core layout and ABI](layout-abi.md). It does not redefine those owners. [Core control flow](control-flow.md) consumes this document's external-call terminator as one delegated control-transfer category and owns CFG composition of its normal continuation.

This relation is representation-neutral. It does not define a binary ABI, calling convention, symbol table, physical function address, dynamic-library mechanism, stable data layout, or backend calling sequence.

## External callable declaration identity

A represented Core program MAY contain one finite sequence of **external callable declarations** in addition to its ordinary Core functions and execution-persistent storage declarations.

Each external declaration has one stable declaration identity within that Core program. An implementation may encode the identity as an index such as `ExternalCallableId`, but no numeric encoding is semantically required.

External callable identity is distinct from every `FunctionId`. An external declaration is not a represented Core function entity, has no Core body, creates no Core function activation, has no parameter locals, and is not a Core callable value target.

Two external declarations remain distinct even when their callable interfaces are equal. The identity is not a physical symbol name, linker identity, code pointer, provider address, dynamic-library handle, ABI tag, source name, source module binding, or reflection value.

## External callable interface

Every external declaration carries exactly one canonical callable interface in the structural sense owned by `functions.md`:

1. one finite ordered sequence of parameter `TypeId`s;
2. either no result or exactly one result `TypeId`; and
3. safe-reference result contract `None`.

An **external scalar type** is exactly one existing Core scalar type whose scalar kind corresponds to `Bool`, a represented fixed-width signed/unsigned integer, or `F16`, `F32`, or `F64`.

Every external parameter type MUST exist in the program-wide type domain and MUST be an external scalar type. An external result, when present, MUST exist and MUST be an external scalar type.

Raw-pointer scalar types, safe-reference scalar types, callable scalar types, structural aggregates, and every other type outside the exact admitted scalar-kind list are not external parameter/result types in this revision.

The exact semantic scalar value domain is the one already owned by Core numeric/value semantics. External interface admission adds no physical encoding, size, alignment, bit pattern, byte order, or calling convention.

The program-wide Core type domain is shared by ordinary functions, persistent declarations, and external callable declarations. Equal `TypeId` use denotes the same Core semantic type; external admission does not create ABI-shadow types.

## Canonical validation

Core program validation MUST establish all of the following independently of any particular runtime provider:

- every external declaration identity resolves within the finite declaration sequence;
- every external declaration interface satisfies the scalar-only rules above;
- every external-call target identity resolves;
- the call's argument count exactly equals the selected external interface parameter count;
- every argument operand has exactly the corresponding parameter type;
- no-result/result-destination structure exactly matches the interface;
- a result destination, when required, has exactly the result type and satisfies the reusable result-destination admission relation from `functions.md` before argument operand effects;
- the normal continuation identifies an existing block in the caller body; and
- no external call transfers reference, raw-pointer, callable, or aggregate state.

Core program validity does not depend on provider availability. Provider availability belongs to environment admission after language validation.

For control-flow validation, this owner supplies the external call's **normal-successor state consequence**: the caller state after successful ordered argument evaluation plus normal result initialization when applicable. `control-flow.md` owns propagation of that state along the represented normal continuation and validation of the resulting CFG-reachable state. Provider divergence is an execution outcome; it does not erase the represented normal edge from static CFG validation.

## External provider requirement

Every external declaration in a valid represented Core program establishes one hard **external provider requirement** for environment admission.

An admissible environment MUST associate each external declaration identity with exactly one provider contract whose semantic callable interface is exactly equal to the declaration's interface.

Environment admission rejects the program when a required provider is missing, when more than one provider is ambiguously associated with the same requirement, when the provider interface differs, or when the environment cannot preserve one required semantic scalar value domain. A legal adapter or emulation MAY satisfy the requirement when it preserves the exact semantic contract.

Provider association is semantic environment configuration. It is not Core function lookup, source module lookup, dynamic target selection, physical symbol resolution, dynamic-library loading, or linker behavior under this revision.

The declaration sequence is the requirement set: admission does not perform Core CFG reachability or dead-code analysis to decide whether an external declaration is required. An unused external declaration remains an explicit program environment requirement.

## Provider contract

For one admitted external declaration with interface `I`, its provider contract accepts exactly one ordered semantic value for every parameter of `I`.

For each invocation, the provider contract permits exactly one of these first-slice outcome categories:

- normal return with no value when `I` has no result;
- normal return with exactly one semantic Core value matching `I`'s exact result type when `I` has a result; or
- divergence.

The contract MAY additionally define explicit host/environment observations for an invocation. Only observations explicitly supplied by the applicable admitted contract participate in program behavior under `behavior.md`.

The provider contract does not originate a represented Core `Fault`, Core undefined behavior, exception, panic, unwind, callback, re-entry, reference/raw-pointer effect, or hidden mutation of Core storage in this revision.

A provider implementation that produces an outcome or value outside its admitted contract is non-conforming to that provider/environment contract. Such behavior is not retroactively classified as Core undefined behavior and is not an admitted execution of the validated Core program.

## External-call control transfer

A represented **external call** belongs to one active ordinary Core function activation and contains exactly:

- one external callable declaration identity;
- one ordered argument operand for every selected external parameter;
- either no result destination or exactly one direct result destination place according to the selected interface; and
- one normal continuation block in the same caller body.

External-call target selection is static. The call evaluates no callable value, code pointer, symbol value, or dynamic provider handle.

The selected external interface governs result-destination admission and ordinary argument production using the reusable call-site relations from `functions.md`. In particular:

1. result-destination admission is established before argument operand state effects;
2. argument count and exact types are checked against the interface;
3. argument operands evaluate strictly left to right;
4. each successful argument value is held as one owned transient call value until every argument succeeds; and
5. existing operand semantics determine each argument's state effects.

Because every external argument is an ordinary non-pointer scalar, no safe-reference call-entry authority/liveness check, raw-pointer provenance transfer, callable target transfer, aggregate state transfer, or parameter-local initialization is part of external invocation.

If an existing operand semantics selects undefined behavior while producing an argument, no defined external invocation follows. A future operand family that introduces another abnormal evaluation outcome must define its interaction with held transient values before external calls may consume it.

## Invocation, caller suspension, and normal return

After every argument operand has produced its exact scalar value successfully, invoke the provider associated at environment admission exactly once with those ordered semantic values.

No Core function activation is created. No external declaration acquires Core locals, explicit loans, a function body, parameter storage, result-origin state, or activation cleanup.

While the provider is executing, the caller has no Core execution step. The provider cannot observe or mutate caller Core storage through this relation because it receives only semantic scalar values and no storage-relative capability.

On normal provider return:

- a no-result interface yields no Core value;
- a result-bearing interface yields exactly one semantic value of the exact declared result type;
- the result-bearing value initializes the already-admitted vacant destination without replacement destruction, using the same normal call-result initialization consequence consumed from `functions.md`; and
- execution enters the external call's normal continuation block with the caller state produced by argument evaluation plus that result initialization.

Held transient argument values end at the external-call boundary after they have been supplied to the provider. Since the admitted domain contains only ordinary scalar values and no reference carriers or observable destructors, their end introduces no additional first-slice authority or cleanup behavior.

## Divergence

If the admitted provider diverges, the external call diverges.

The normal continuation is not selected, the result destination is not initialized, and no caller cleanup is performed merely because the provider never returns. The caller activation and execution-persistent storage retain their continuing extents for the diverging execution under their existing owners.

Provider divergence does not become a `Fault`, timeout, cancellation, or implementation-defined result merely because a realization or harness has waited for some duration.

An external call may lie on a control-flow or recursive path that otherwise contains ordinary Core calls. Normal external-call continuation edges may participate in represented cycles; provider divergence is independently a non-returning execution outcome.

## Defined-fault boundary

The first external provider relation does not originate a represented Core defined fault.

A provider cannot select `Fault`, inject a fault reason, unwind into Core, skip the external-call continuation through an exception edge, or convert a host failure into a Core fault under this revision.

An argument operand that already has an accepted abnormal source/Core outcome remains governed by its own owner before provider invocation. This document does not redefine that outcome merely because the operand occurs at an external call site.

## Function-value and indirect-call boundary

An external declaration identity is not a `FunctionId` and is not a callable scalar value payload under `callable-values.md`.

No represented function value may name an external declaration. An indirect call therefore cannot select an external provider in this revision. There is no callback or foreign re-entry relation.

External calls are a distinct static control-transfer category rather than a hidden ordinary function body or an indirect call through a provider address.

## Environment observations and determinism

External provider behavior composes with `behavior.md`'s admitted external-observation boundary.

A provider contract may permit observations or outcomes that depend on external environment state. The language does not infer semantic determinism from the fact that argument values are equal across two executions. General determinism remains conditional on the same explicit inputs and admitted external observations under [Correctness relations](../correctness.md).

The physical invocation mechanism, marshalling buffers, stack/register choices, provider address, loader/linker state, and adapter implementation are not observations unless a later normative contract exposes them.

## Representation, ABI, and linkage boundary

The external-callable relation transfers semantic values only.

It establishes no requirement that:

- signed integers use two's-complement storage;
- `Bool` has any particular bit pattern;
- floating values use a particular physical IEEE encoding;
- parameter/result values occupy particular registers, stack slots, bytes, or alignment classes;
- any external declaration has a physical symbol, stable code address, linker name, section, relocation, visibility, or version;
- source or Core external declaration identity is preserved as a binary symbol identity; or
- one named platform ABI is used.

A realization MAY use an adapter, thunk, host callback table, direct static binding, or a later accepted physical ABI when it preserves this semantic provider relation. Such choices do not become language semantics through implementation use.

## Deliberate boundaries

This revision does not define:

- inbound external invocation of Runen functions;
- executable entry-point semantics;
- stable scalar or aggregate memory layout;
- record/aggregate external transfer;
- raw-pointer or safe-reference FFI;
- callback/function-pointer FFI;
- external function values or indirect external calls;
- generic or trait ABI;
- closure/environment ABI;
- static data import/export;
- variadic calls;
- dynamic-library loading/unloading;
- binary symbol naming, linkage, visibility, versioning, or collision rules;
- exception/panic/unwind ABI;
- package/build/linker policy; or
- a backend representation.

Those require separately accepted consumers and owners.
