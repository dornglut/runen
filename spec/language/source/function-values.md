# Source Function Values and Indirect Calls

Status: **provisional normative; incomplete**

This document owns the represented source-language relation for first-class captureless function-value types, function values carrying source function-entity identity, function-value formation from existing non-generic function entities, function-value duplicability and ordinary transport, bounded indirect-call target classification through function-local bindings, and source-to-Core refinement of function-value types, values, and indirect calls.

It consumes source function-entity identity, callable-signature structure, concrete parameter/result admission, bounded safe-reference result-contract derivation, and exported callable-interface accessibility from [Source callables](callables.md); represented source value-type integration and equality from [Source type foundation](types.md); same-module and qualified declaration lookup from [Source names and modules](names-modules.md); function-local lookup, binding identity, ordinary binding use, local lifecycle, and assignment from [Source function-local bindings](local-bindings.md); dynamic call validation/execution, activation creation, argument/result transfer, recursion, divergence, cleanup, and defined-fault propagation from [Source function execution](function-execution.md); safe-reference call-entry, result provenance, and bounded result-contract consequences from [Source safe references](references.md); generic type-parameter/application boundaries from [Source generics](generics.md); and represented function-type/value/call spelling from [Source concrete syntax](concrete-syntax.md). It does not redefine those owners.

The lower refinement target is the representation-neutral callable-value and indirect-call relation in [Core callable values and indirect calls](../core/callable-values.md), together with the shared function activation/call relation in [Core functions](../core/functions.md) and ordinary Core value/storage rules in [Core value and storage semantics](../core/value-storage.md).

This revision is deliberately captureless and non-generic at function-value formation. It does not define closure literals, capture sets or environments, generic or polymorphic function values, specialization-as-value, callable traits, methods, dynamic dispatch, physical function addresses, ABI/linkage, or an implementation representation.

## Function-value types

A represented **source function-value type** is one structural source value type with exactly these semantic components:

1. one finite ordered sequence of parameter source types;
2. either no result value or exactly one result source type; and
3. one exact bounded safe-reference result contract supplied by `callables.md` from that parameter/result structure.

The represented concrete spelling is owned by `concrete-syntax.md` and has the shape `fn(T0, T1, ...) -> R`, with the result clause omitted for a no-result function-value type. Parameter identifiers are not part of this type because function-local parameter binding identity belongs to a particular function declaration/activation rather than to callable value type identity.

A function-value type is not a source function entity, declaration, body, activation, closure environment, trait, nominal record, raw pointer, safe reference, ABI function pointer, physical code address, or module binding. It describes the exact callable interface required of one represented captureless function value.

### Component admission

Each parameter/result component is itself one represented `Type` from `concrete-syntax.md`, subject to the callable parameter/result admission relation owned by `callables.md`.

Consequently, in this revision:

- represented intrinsic scalar and nominal record types remain admissible where `callables.md` admits them;
- another represented **concrete** function-value type is admissible recursively as a parameter or result component;
- represented `SharedRef(T)` and `ExclusiveReplaceRef(T)` parameter forms remain admissible exactly under the existing source callable rules;
- represented `SharedRef(T)` results remain admissible only when `callables.md` derives one valid bounded safe-reference result contract from the ordered parameter sequence;
- represented `ExclusiveReplaceRef(T)` results remain invalid;
- represented `RawPtr(T)` parameters and results remain invalid; and
- an abstract generic type parameter is not admitted anywhere inside a function-value type in this first slice.

Recursive function-value type syntax is finite source syntax. Function-value type edges are semantic callable-signature edges, not direct nominal-record contained-value edges. A nested function-value type therefore does not imply record structural containment, recursive nominal layout, a closure environment, an ABI layout, or code-address storage.

No source type alias or named function-type declaration is introduced. Source function-value type identity is structural as defined below.

### Safe-reference result contract

A function-value type whose result is not `SharedRef(T)` has contract **None**, subject to the existing result-admission rules in `callables.md`.

A function-value type whose result is `SharedRef(T)` consumes exactly the same deterministic bounded result-contract derivation as a source function declaration with the same ordered parameter types and result type. The result contract is therefore either the exact applicable `SharedIdentity(origin)` or `SharedDirectChild(origin)` selected by `callables.md`, or the function-value type is source-invalid because no unique represented contract exists.

The contract is semantic function-value type structure even though current source syntax does not spell it explicitly. A call through a function value therefore has the same static safe-reference result fact available before the dynamic target function entity is selected.

This rule does not infer a contract from a function body, function value, runtime target set, call-site dataflow, lifetime name, or implementation representation.

## Function-value type equality

Two represented source function-value types are equal exactly when all of the following hold:

- they contain the same number of parameter types;
- each corresponding parameter type is equal under the represented source type-equality relation;
- either both have no result value or both have one result whose types are equal under the represented source type-equality relation; and
- their bounded safe-reference result contracts are equal, including the same contract variant and the same origin parameter slot where applicable.

This equality is structural. No declaration identity is part of function-value type identity.

Because abstract generic parameters are excluded from function-value type components in this revision, function-value type equality does not add a second alpha-equivalence rule. Generic function callable-signature equality remains separately owned by `callables.md`/`generics.md`.

Two distinct source function entities whose non-generic callable signatures produce equal function-value type structure remain distinct function entities and distinct function values. Equal function-value type does not merge declarations, create an overload set, or make the function entities themselves equal.

Function-value type equality is not ABI compatibility, representation compatibility, pointer compatibility, trait conformance, implicit conversion, subtyping, variance, overload compatibility, or closure compatibility.

## Contextual type admission

A represented function-value type is admitted directly in this first slice as:

- one ordinary source function parameter type;
- one ordinary source function result type; or
- one ordinary local declared type.

A function-value type is **not** admitted as a nominal record field type in this revision. This restriction keeps record structural shape, record construction, record destructuring, exported-field accessibility, and field structural ownership unchanged. It does not claim that future aggregate storage of function values is fundamentally invalid.

A function-value type is also not admitted as the referent of `SharedRef` or `ExclusiveReplaceRef`, or as the pointee of `RawPtr`, because `concrete-syntax.md` does not add `FunctionType` to `ReferenceReferentType`. This revision therefore introduces no safe-reference-to-function-value, raw-pointer-to-function-value, function-address, pointer conversion, or callable provenance relation.

A concrete function-value type may occur directly in the parameter/result/local type surface of a generic source function. That fact does not make the generic function entity itself a function value and does not admit abstract generic parameters inside the function-value type.

Function-value types remain outside the generic type-argument domain owned by `generics.md`.

## Source function values

A represented **source function value** contains exactly one source function-entity identity established by `callables.md`.

A function value is valid at one exact source function-value type only when:

1. the target source function entity exists in the program/module graph under the applicable lookup relation;
2. the target source function is non-generic, meaning its callable signature has no generic type-parameter slots; and
3. the target function's exact parameter types, result structure, and bounded safe-reference result contract produce a function-value type equal to the required function-value type.

Distinct function entities remain distinct function values even when both satisfy the same function-value type.

The function value carries no source-observable body pointer, activation identity, code address, symbol name, linker identity, calling convention, module handle, storage address, vtable/table index, serialization identity, or backend representation.

The value itself has no capture environment. A later closure feature may require a distinct value/type relation or an explicit adaptation to function-value types; this revision does not preselect that design.

## Function-value duplicability and ordinary transport

Every represented captureless function value is source-duplicable.

Duplicating a function value produces another owned value carrying the same source function-entity identity. It creates no function activation, storage alias, safe-reference authority, raw-pointer provenance, capture environment, external resource, or cleanup obligation.

Function values otherwise use the existing ordinary owned-value and local-binding relations:

- ordinary whole-binding use of a function-value parameter/local duplicates its value and preserves the source binding's stored value because the type is duplicable;
- initialization transfers the produced function value into the receiving ordinary local;
- a mutable ordinary function-value local may be reassigned under the existing whole-binding assignment relation;
- function values may cross ordinary function parameter and result boundaries when the exact function-value types match;
- lexical/activation cleanup ends the stored-value lifetime using ordinary cleanup and has no function-specific dynamic effect; and
- no function-value operation implicitly calls, resolves, loads, links, or realizes the target function merely because the value is copied, stored, transferred, returned, or cleaned.

The existence of distinct semantic function values does not introduce source equality, inequality, ordering, hashing, pattern tests, integer conversion, address comparison, stringification, reflection, or any other observation operation over function identity.

## Function-value formation

Function-value formation is one owned-value producer selected only in a receiving position that supplies one exact required source function-value type.

There is no inferred/default function-value type and no source operation that forms an untyped function value first and chooses its type later.

### Unqualified formation

For one bare `UserIdentifier` in an ordinary value position with required function-value type `F`, apply the existing function-body unqualified lookup precedence:

1. perform the function-local parameter/local lookup owned by `local-bindings.md`;
2. if an active parameter/local binding resolves the key, that selection is final and the occurrence is ordinary binding use rather than function-value formation;
3. only when no active parameter/local resolves the key, perform same-module declaration lookup under `names-modules.md`;
4. the selected module binding MUST denote one non-generic source function entity; and
5. that function entity's exact callable structure MUST produce source function-value type equal to `F`.

If same-module lookup instead selects a constant, nominal record, marker trait, generic function, or another wrong category, the occurrence is source-invalid for function-value formation and lookup MUST NOT search another declaration or imported module merely because `F` requires a function value.

An active local binding with the same key as a module function continues to shadow that function. If the active local has function-value type, ordinary binding use may itself produce a function value. If it has another type, exact required-type validation fails; source lookup does not fall through to the module function.

### Qualified formation

For one represented `QualifiedModuleMember` in an ordinary value position with required function-value type `F`, use the existing source-unit module-alias and qualified exported-member lookup from `names-modules.md`.

The selected exported target binding MUST denote one non-generic source function entity whose exact callable structure produces function-value type equal to `F`.

An unresolved alias/member, inaccessible target, selected constant/record/trait, selected generic function, or exact function-value type mismatch is source-invalid. Qualified lookup does not fall back to a local binding, same-module declaration, another import, or another category.

### Formation execution

After successful source validation, function-value formation is finite, effect-free, non-faulting, and non-diverging. It yields exactly one owned function value carrying the selected source function entity identity.

Formation does not execute or validate a new copy of the function body, create an activation, evaluate ordinary arguments, perform safe-reference call-entry checks, realize an ABI symbol, observe a code address, or allocate an environment.

Function bodies remain independently source-valid under their existing owners.

## Generic-function boundary

A source function entity whose callable signature contains one or more generic type-parameter slots is not a function-value formation target in this revision.

Neither bare `f` nor qualified `module::f` forms a value for such a generic function, even when the surrounding required function-value type might resemble one concrete specialization.

The source form `f[T]` is not a function-value producer. An explicit generic type-argument list remains part of direct-call syntax/semantics only. This revision defines no:

- specialization-as-value;
- inferred or omitted function-value specialization;
- polymorphic or higher-ranked function value;
- generic closure value;
- callable generic parameter kind;
- callable trait abstraction; or
- target selection from a family of specializations.

A generic function may nevertheless use a separately represented **concrete** function-value type in one of its ordinary parameter/result/local positions. A generic direct call may pass a valid non-generic function value to such a concrete parameter after the existing generic call relation has established the complete instantiated direct-call signature.

An abstract type-parameter use inside a function-value type is source-invalid in this first slice. If a `UserIdentifier` inside a function-value type resolves to an in-scope generic type parameter, that selected binder is final and lookup does not fall through to a same-named nominal record. `generics.md` owns this generic lookup/admission boundary.

Function-value types are not added to the represented generic type-argument domain. This restriction is independent of their ordinary concrete use inside generic function bodies/signatures.

## Bounded call-target classification

The represented concrete call token shape is neutral with respect to source direct versus indirect invocation. `concrete-syntax.md` owns the syntax; this document owns only the new function-value target classification.

For one unqualified call target `UserIdentifier`:

1. apply the existing function-local lookup precedence;
2. if an active parameter/local binding is selected and its exact source type is a function-value type, the call is one **bounded indirect source call** through that binding;
3. if an active parameter/local binding is selected and its exact type is not a function-value type, the call is source-invalid and no module fallback occurs;
4. only when no local binding is selected does same-module declaration lookup occur; and
5. a same-module source function selected there retains the existing **direct source call** relation.

A represented qualified call target `alias::member` cannot select a function-local binding and therefore retains the existing qualified direct-call relation. This revision does not define an indirect qualified call target.

A generic type-argument list on a call is consumed only by the existing generic direct-call relation. If unqualified lookup selects a function-value local/parameter, presence of any generic type-argument list makes that call source-invalid before ordinary argument producer consequences may commit.

This target classification preserves wrong-category finality. In particular, a non-callable local shadowing a same-named module function continues to make `name(...)` invalid, while a function-valued local with that key now makes the same token shape a valid indirect-call candidate.

## Bounded indirect calls

A represented **bounded indirect source call** invokes the exact non-generic source function entity carried by one function-value parameter/local binding.

The first slice admits only an unqualified function-local binding as an indirect callee. It does not admit:

- a grouped callee such as `(f)(x)`;
- immediate invocation of a call result such as `make_callable()(x)`;
- record-field invocation such as `record.handler(x)`;
- a qualified module member as an indirect callee;
- an operator, conditional, construction, reference, raw-pointer, pattern, or arbitrary produced value as a callee; or
- a general postfix/value-call grammar.

A separately produced function value may be stored in an ordinary local and then invoked through that local. This bounded route is sufficient to establish first-class callable transport without defining universal callable expressions.

### Static interface

Once target classification selects a function-value local/parameter, that binding's exact function-value type supplies the complete static callable interface for the call:

- exact argument count;
- exact ordered parameter types;
- result absence or exact result type; and
- exact bounded safe-reference result contract.

No dynamic target-set analysis is required to establish those facts. Every valid function value stored at that source type already names one non-generic function entity whose callable structure exactly matches the type.

Static validation of the indirect call establishes the local target/type and applicable result-receiving facts before any ordinary argument producer state may commit. Detailed producer transaction, argument order, final safe-reference call-entry validation, activation behavior, and result/fault/cleanup relations are owned by `function-execution.md` and `references.md`.

### Dynamic target identity

Dynamic execution of a valid indirect call uses the exact function entity identity carried by the evaluated function value. A source-valid program has no separate dynamic `unknown function`, signature mismatch, missing result contract, overload failure, or vtable-lookup outcome for this relation.

Selecting the target entity does not expose that identity as source data beyond the already-held opaque function value and does not reveal a physical address or symbol.

### Common call semantics

After target selection, indirect invocation creates the same source function activation and uses the same parameter/result transfer, recursion, divergence, safe-reference, defined-fault, and cleanup semantics as an otherwise equivalent direct call to that exact function entity.

The source call relation therefore has two target-selection forms but one activation/execution model. This document does not define a second function body semantics or activation kind for indirect calls.

## Call producers and bounded consuming positions

A successful result-bearing indirect call is an owned-value producer under `function-execution.md` in the same sense as a successful result-bearing direct call.

Where an existing source receiving relation deliberately admits a **result-bearing call producer** as a bounded producer category rather than an arbitrary `Value`, that category includes both represented direct and bounded indirect calls after this revision. In particular, the call-backed producer receiver in `field-access.md` and the call-backed producer scrutinee in `patterns.md` consume this neutral result-bearing call category.

This does not widen those operations to arbitrary postfix/value receivers. Their existing non-call producer restrictions remain unchanged.

A no-result direct or indirect call remains usable only in the existing no-result call statement position and cannot supply a result-bearing receiving position.

## Source-to-Core refinement

Every source-valid function-value type refines to one Core callable scalar type from `core/callable-values.md` whose interface contains the recursively refined ordered parameter types, recursively refined optional result type, and the same bounded safe-reference result contract.

Within one lowered Core program, all occurrences of one equal source function-value type MUST refine consistently to the same selected Core callable `TypeId`. This is a source-to-Core mapping/canonicalization obligation required to preserve one source type identity across Core uses. It does not redefine Core callable `TypeId` identity, make independently declared Core callable types structurally equal, or require global/cross-program Core interning.

Finite nested source function-value types refine recursively to Core callable `TypeId` signature edges. Those edges retain the accepted Core classification as semantic/non-structural.

A valid source function value carrying non-generic source function entity `F` refines to the Core function value naming the one Core function entity that refines `F` in the non-generic source-to-Core mapping. The Core value relation is the dedicated `FunctionValue(FunctionId)` relation; no physical function address or ordinary scalar constant payload is introduced.

A bounded source indirect call refines to one Core `IndirectCall` with:

- the Core callable `TypeId` selected for the source callee binding's function-value type;
- a callee operand refining the source callable-local use;
- the recursively lowered ordinary arguments;
- the applicable optional result destination; and
- the normal continuation selected by the existing lowering relation.

The resulting Core indirect call MUST preserve the source order required by `function-execution.md`: destination/result admission before callee/argument state effects where applicable, callee value evaluation before ordinary arguments, ordinary arguments left-to-right, final safe-reference call-entry checks after argument effects, and target selection before common activation execution.

Existing source direct calls continue to refine to Core direct calls and need not be re-expressed through function values.

The refinement creates no code-address observation, pointer conversion, ABI/calling convention, stable callable layout, symbol/linker identity, FFI compatibility, dynamic-library handle, vtable/table representation, or serialization format.

## Export and accessibility relationship

Function-value formation through a qualified module member consumes the existing requirement that the target function binding be exported.

Separately, when an exported source function's own parameter/result interface contains a function-value type, the nested nominal source types exposed by that function-value type must satisfy the exported-signature accessibility relation owned by `callables.md`. This document supplies the function-value type structure to that traversal; it does not create a second accessibility rule.

Because function-value type syntax is finite and function-valued nominal record fields are excluded, this nested interface traversal is finite source type-expression traversal rather than nominal record-field graph traversal.

Accessibility has no ABI/linkage implication and does not expose function value identity, code addresses, or implementation symbols.

## Explicitly absent function-value dimensions

This revision does not define:

- function-valued nominal record fields;
- safe references or raw pointers whose referent/pointee is a function-value type;
- generic or polymorphic function values;
- explicit generic specialization as a value;
- abstract generic parameters nested inside function-value types;
- function-value generic type arguments;
- inferred/default function-value types;
- implicit conversion between distinct function-value types;
- callable equality, inequality, ordering, hashing, pattern tests, reflection, or serialization;
- closure literals, capture lists, capture inference, capture modes, closure environments, capture cleanup, or closure escape;
- methods, associated functions as values, bound receivers, trait-callable objects, dynamic dispatch, vtables, witness tables, or overload sets;
- arbitrary expression/postfix invocation or invocation of immediate produced/grouped/field values;
- async/coroutine/generator callable values, variadics, default arguments, effect signatures, or unsafe-callable types;
- code-address observation, integer/pointer casts, symbol identity, calling convention, stable layout, ABI, linkage, FFI, plugin, or dynamic-library semantics; or
- any required compiler/HIR/backend representation.

In particular, the duplicability and structural type relation defined here applies only to captureless source function values. It does not decide whether a future capturing closure is duplicable, has the same type category, can coerce to a function-value type, or uses the same runtime representation.
