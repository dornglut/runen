# Source Generics

Status: **provisional normative; incomplete**

This document owns the first represented source generic relation: function type-parameter identity and scope, abstract parametric type expressions, bounded type-argument admission, exact explicit substitution, generic-body capability facts, generic-call substitution composition, generic activation substitution context, validation independence from concrete use sites, and the finite refinement boundary to existing concrete Core functions.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module/function/nominal-type lookup and accessibility from [Source names and modules](names-modules.md), represented concrete source type identity/equality and owned-value duplicability from [Source type foundation](types.md), callable entity/signature structure from [Source callables](callables.md), concrete captureless function-value types and generic-function-value exclusions from [Source function values and indirect calls](function-values.md), structural ownership from [Source structural ownership](structural-ownership.md), function-local binding lifecycle and whole-binding use from [Source function-local bindings](local-bindings.md), direct-call execution from [Source function execution](function-execution.md), represented concrete generic declaration/application spelling from [Source concrete syntax](concrete-syntax.md), and first-slice nominal marker trait identity plus exact marker-obligation satisfaction from [Source marker traits](traits.md). Existing concrete Core function semantics are owned by [Core functions and direct calls](../core/functions.md).

Trait declaration identity, explicit marker implementation propositions, compilation-global coherence, and the concrete/abstract marker-satisfaction relation remain owned only by `traits.md`. This document owns only how existing generic slots carry marker requirements/evidence and when generic application validates those requirements.

This document does not redefine concrete source type identity, concrete duplicability, module binding identity, value-binding lookup, structural ownership mathematics, direct-call execution ordering, concrete syntax, marker trait identity/coherence, or Core generic semantics. The coupled source owners integrate this relation only at their existing responsibility boundaries.

## Generic function relation

A represented **generic function** is an existing source function entity whose callable structure contains one non-empty finite ordered sequence of **type-parameter slots**.

Only type parameters are represented. This revision defines no value, const, lifetime, pack, effect, placement, or other generic parameter kind.

Type-parameter slot identity is the pair:

- the declaring source function entity; and
- the zero-based position of the slot in that function's ordered type-parameter sequence.

Each slot has exactly one lexical user-identifier key. Keys MUST be unique within one generic function's type-parameter sequence.

Each type-parameter slot additionally carries one finite **marker-requirement set**, which MAY be empty. Every member of that set is one exact marker trait entity identity owned by `traits.md`.

Marker-requirement source order is not semantic. Two source requirement occurrences that resolve to the same exact marker trait identity on one slot are invalid duplicates; they do not create multiplicity in the semantic set.

The marker-requirement set does not change type-parameter slot identity. The semantic slot remains exactly the declaring function entity plus its ordered slot position.

The lexical key is lookup spelling, not semantic slot identity. Renaming a type parameter while preserving its slot position and every corresponding use does not change that slot's semantic identity or the generic callable structure.

Generic arity does not create a function overload set. The existing source function entity and its module binding remain the function identity, and existing duplicate module-binding-key rules continue to apply independently of generic arity, parameter spelling, or marker requirements.

A function with no type-parameter sequence is **non-generic** under this relation.

## Type-parameter scope and lookup

The complete ordered type-parameter sequence is established immediately after the generic declaration list owned concretely by `concrete-syntax.md` has been accepted for the function declaration.

Every established type-parameter slot is in scope in:

- every following ordinary parameter type in that function's callable signature;
- that function's result type, when present; and
- represented type annotations in that function's body.

A type parameter is not in scope before its own complete generic declaration list has been established. This revision has no nested generic declaration and therefore introduces no nested generic-parameter shadowing relation.

In one bare unqualified **type position** within that scope, lookup first compares the lexical key against the declaring function's type-parameter keys. A matching type parameter is selected before same-module nominal-type lookup.

That selection is final for the applicable bare type-position lookup. The consuming type form then validates whether an abstract type parameter is admitted in that position. If the selected parameter is not admitted there—for example because the identifier occurs as a Shared-reference, replacement-reference, or raw-pointer referent/pointee—the source form is invalid. Lookup MUST NOT bypass the selected type parameter and fall through to a same-named module declaration merely because the consuming type constructor requires a concrete nominal type.

Qualified `alias::member` type lookup remains the existing module relation and MUST NOT denote a function type parameter.

Marker trait references attached to a type-parameter declaration use the module declaration/qualified lookup domain consumed by `traits.md`; they do not participate in this function-local type-parameter lookup domain. Therefore a bound-side name in `fn f[T: T] (...)` may denote a same-module marker trait binding `T` while admitted type-position uses of the generic slot inside the function denote the function-local type parameter. Marker lookup never consults or falls back through this generic slot domain.

The type-parameter lookup domain participates only in type positions. It does not participate in function-local value-binding lookup, module value lookup, direct-call target lookup, record/pattern-head lookup, marker trait lookup, field lookup, or another name domain. A parameter/local value binding MAY therefore have the same lexical key as an in-scope type parameter without ambiguity: the receiving syntactic/semantic position selects the applicable name domain.

A same-module declaration MAY likewise have the same key as a function type parameter. The function-local type parameter shadows that declaration only in the admitted bare type positions above; it creates no general module-binding shadowing rule. Category/admission rejection after that selection does not revive the shadowed module declaration.

## Abstract parametric type expressions

Within a generic function declaration/body, one use of an in-scope type parameter as an admitted bare complete type denotes one **abstract parametric type expression** identified by that exact semantic type-parameter slot.

During generic declaration/body validation, abstract type equality is exact:

- one abstract type parameter is equal to itself exactly when both uses designate the same semantic slot identity;
- two distinct type-parameter slots are unequal even when a later application may substitute the same concrete type for both; and
- an abstract type parameter is unequal to every concrete intrinsic, nominal-record, safe-reference, raw-pointer, and function-value source type.

This abstract equality relation is a generic-validation relation consumed alongside the concrete source type equality owned by `types.md`. It does not redefine equality between concrete source types.

Concrete substitution MAY map two distinct abstract slots to one equal concrete source type. That later concrete equality MUST NOT retroactively validate an operation rejected under abstract equality and MUST NOT change an operation mode already selected during generic-body validation.

### First-slice type-position boundary

An abstract type parameter is admitted only as one bare complete declared type in these positions:

- a function parameter type;
- that function's result type; and
- an ordinary local declared type in that function body.

An abstract type parameter is not admitted in this revision:

- as the referent of `SharedRef`, `ExclusiveReplaceRef`, `&`, or `&mut`;
- as the pointee of `RawPtr` / `raw`;
- as a nominal record field type;
- nested in another aggregate or type constructor, including a captureless function-value type;
- as a generic record/type-alias/opaque-type argument because those declaration forms are not represented; or
- as an ABI, layout, representation, linkage, or target dimension.

Existing concrete non-parametric source types MAY appear in the same generic callable signature/body under their existing owners. This includes a concrete function-value type from `function-values.md`, provided that function-value type itself contains no abstract type parameter under that owner's first-slice boundary.

## Type arguments

A first-slice generic type-argument expression is exactly one of:

- a represented intrinsic scalar source type;
- a represented nominal record source type legally selected at the application site; or
- when the application occurs inside another generic function body, one in-scope abstract type parameter of that enclosing function.

Safe-reference, raw-pointer, and function-value types are not generic type arguments in this revision. No nested generic type application exists because this revision defines no generic type constructor.

A concrete nominal-record type argument is resolved at the **application site** using the existing same-module or qualified cross-module type lookup/accessibility relation. The generic callee does not repeat source-name lookup for that concrete record in the callee's defining module.

Consequently an application may instantiate an accessible exported generic function with a nominal record that is private to the caller's own module, provided the caller can legally resolve that record and all other generic/call requirements hold. The generic callable receives the record's semantic type identity through substitution; this fact does not create callee-side lexical access to the record declaration and establishes no package, separate-compilation, linkage, or ABI rule.

An abstract type argument inside an enclosing generic function is selected only by that enclosing function's type-parameter lookup relation. It remains abstract until an enclosing activation substitution maps it to a concrete admitted type.

## Exact explicit substitution

Every application of a generic function supplies exactly one ordered type argument for every target type-parameter slot.

Let a target generic function have ordered type-parameter slots `P0 ... Pn-1` and an application have ordered admitted type arguments `A0 ... An-1`. The application's **substitution** is the exact position-wise mapping `Pi -> Ai` for every slot.

Generic arity MUST match exactly. Empty type-parameter and type-argument lists are not represented by the first concrete syntax slice.

Substitution performs no:

- type inference from value arguments or result context;
- omitted/default type argument selection;
- subtyping;
- coercion, conversion, or numeric promotion;
- variance;
- overload selection;
- trait/implementation search;
- body-derived choice; or
- target/backend-dependent selection.

Applying a substitution to one parametric callable/local type expression replaces each occurrence of a target type-parameter slot with its mapped argument and leaves existing concrete type expressions unchanged. A concrete function-value type is therefore left unchanged by first-slice substitution because abstract type parameters are not admitted inside that type constructor.

When an application occurs inside another generic function, a mapped argument may itself be one enclosing abstract type parameter. The resulting instantiated type expression therefore MAY remain abstract. Substitution composition preserves the enclosing semantic slot identity until a later enclosing application/activation supplies the final concrete type.

Two distinct target slots remain distinct during target body validation even when one application maps both to the same argument. Substitution is application data, not a rewrite of the validated generic declaration's abstract type-equality relation.

Exact substitution and marker-obligation validation are distinct relations. Constructing a substitution performs no trait/implementation search. After an exact substitution exists, the generic application relation below separately requires each mapped type argument to satisfy the target slot's declared marker-requirement set through `traits.md` before ordinary value-argument validation may commit effects.

## Generic-body capability environment

Concrete source owned-value duplicability remains owned only by `types.md`. This generic relation does not create a second duplicability classification or infer operational capability from later concrete specialization.

An abstract type parameter has **no positive duplicability evidence or other concrete operational capability merely from arbitrary first-slice marker requirements**. Marker membership is an application-validity predicate under `traits.md`; it does not establish record shape, intrinsic/scalar category, safe-reference/raw-pointer category, duplicability, operator support, field access, construction, or another represented operation.

For a parameter or ordinary local whose declared type is one abstract type parameter:

- the binding owns exactly one opaque complete structural value root under `structural-ownership.md`;
- the abstract root exposes no represented non-empty structural path;
- ordinary whole-binding owned-value use selects consuming transfer/Move;
- successful whole-binding transfer consumes that complete root;
- ordinary parameter transfer, local initialization, whole-binding assignment where otherwise admitted, return, and cleanup MAY transport/destroy the complete opaque owned value through their existing relations;
- non-consuming owned duplication is not available merely from the abstract type or marker requirements;
- field selection, record construction/destructuring, scalar operators/comparisons, safe-reference/raw-pointer formation or access, and every other operation requiring a concrete type category, concrete structural shape, or other unproven capability are invalid when their only supporting type fact would be the abstract parameter and its arbitrary marker requirements.

The rule above is a validation-capability rule for the generic body. It does **not** classify any later substituted concrete type as non-duplicable.

If a concrete application substitutes a duplicable intrinsic or nominal-record type for an abstract parameter, the already validated generic-body Move MUST remain a Move. Concrete specialization MUST NOT reconstruct that operation as Copy merely because the substituted type has positive duplicability.

Likewise, later substitution MUST NOT make previously invalid abstract field/operator/reference behavior valid by revealing a convenient concrete category or shape. A generic body is accepted or rejected from its declared abstract contract, not from an opportunistic use site.

Likewise, an implementation proposition satisfying one of the parameter's marker requirements at a concrete use site MUST NOT retroactively grant the generic body an operation not established by its accepted abstract capability environment. A marker trait's lexical spelling has no privileged operational meaning.

### Caller production versus callee use

Generic-body capability conservatism does not change ordinary producer behavior in the caller.

At an application site, each ordinary value argument is produced under the caller's exact current source type and the instantiated required parameter type. Therefore an ordinary caller binding of a concrete duplicable type may produce a non-consuming duplicate according to its existing owner, while an ordinary caller binding of a concrete non-duplicable type transfers/consumes its value.

After that produced value transfers into a generic activation parameter whose declared type is abstract in the generic body, use of that parameter follows the operation mode selected under generic-body validation. For example, a caller may Copy an `I64` value to form an argument while the generic body later Moves the activation-local parameter value to its result.

Caller-side production mode and generic-body use mode are distinct semantic steps and MUST NOT be conflated by validation or lowering.

## Generic callable validation

A generic function body is validated once against:

- its semantic type-parameter identities;
- abstract type equality;
- its concrete non-parametric type facts;
- its declared marker-requirement sets as non-operational obligation evidence for generic-to-generic applications; and
- the capability environment defined above.

Where an unchanged receiving relation in `function-execution.md` requires a producer/result type to be exactly equal to a parameter, local, or result source type, generic-body validation supplies the applicable **type-expression equality**: concrete-to-concrete comparison uses `types.md`, while any comparison involving an admitted abstract type parameter uses the exact abstract equality defined here. This composition changes neither producer/evaluation ordering nor the dynamic direct-call relation. Once an enclosing activation chain is concrete, the activation substitution supplies the corresponding concrete runtime/source value types while preserving every operation mode selected during abstract generic-body validation.

Generic-body validity MUST NOT depend on inspecting a concrete use site, enumerating the applications that happen to occur, or revalidating the body with capabilities discovered only after substitution.

When a generic body passes one of its own abstract type-parameter slots as an explicit type argument to another generic function, every target marker requirement must be satisfied from the enclosing slot's own declared exact marker-requirement set under `traits.md`. Generic-body validation MUST NOT inspect concrete callers or concrete implementation propositions for types that might later instantiate the enclosing slot.

The callable/signature owner integrates the ordered binder structure, marker-requirement sets, and alpha-invariant signature comparison; this document owns the identity/substitution/evidence facts those rules consume.

A generic function may contain concrete safe-reference parameter/result positions that do not mention an abstract type parameter. Their existing type/admission/result-contract rules apply unchanged. Because an abstract parameter cannot instantiate to a safe-reference type and cannot occur inside a safe-reference type in this slice, generic substitution cannot create a new hidden safe-reference result-origin candidate or alter an existing reference permission class.

## Explicit generic direct application

The existing direct-call target is resolved first under its ordinary lookup relation. Generic type arguments and marker requirements do not create or select an overload set.

For one resolved direct-call target, before any ordinary value-argument producer may commit effects, validation MUST:

1. determine whether the selected function entity is generic under this relation;
2. require an explicit type-argument list exactly when the target is generic;
3. reject a type-argument list when the target is non-generic;
4. require exact generic arity;
5. resolve/admit every type argument at the application site;
6. construct the exact substitution;
7. require every mapped type argument to satisfy every marker requirement declared by its target type-parameter slot under `traits.md`;
8. instantiate the target's parametric parameter/result type expressions under that substitution; and
9. provide those exact instantiated types to the existing ordinary direct-call argument/result validation relations.

Marker-obligation failure is part of this pre-argument generic-application validation. It MUST NOT commit ordinary value-argument ownership, safe-reference authority/carrier, raw-pointer, or other producer state effects.

There is no inference fallback. Missing type arguments for a generic target remain invalid even when value arguments or a receiving result context would uniquely suggest a concrete type.

A generic direct application remains the same direct-call producer/call-statement semantic category owned by `function-execution.md`. `function-values.md` independently excludes generic function entities and explicit specializations as function values; a bounded indirect call through a function-value local therefore never consumes this generic-application relation. This document adds no generic function value, method/associated lookup, or second generic call mechanism.

If a generic application appears inside another generic body and its substitution contains enclosing abstract type parameters, the instantiated target signature may remain abstract. Ordinary call validation then uses the exact composed abstract type expressions and equality/capability facts already available in the enclosing generic validation environment. Any marker requirements are discharged from the enclosing abstract argument's declared exact requirement set, not from concrete implementation search for hypothetical future substitutions.

## Generic activations and composition

Every successful dynamic direct call of a generic function carries one exact substitution tuple as non-observable source-semantic activation context.

For each abstract type occurrence in that activation, the substitution context determines the corresponding concrete runtime/source value type once the outer application chain is concrete. This context does not change generic-body operation modes selected during abstract validation.

Activation substitution identity is not source-observable data. It does not create a runtime type object, reflection value, hidden source parameter, module binding, ABI argument, dictionary, trait witness, physical code address, or calling-convention dimension.

A successfully discharged marker obligation likewise contributes no source-observable activation witness, hidden value, dictionary, dispatch identity, or runtime trait state.

Direct and mutual recursion remain valid under the existing call execution relation. Every generic recursive call MUST provide an exact admitted type-argument sequence and satisfy the target marker requirements under the same application rule. A generic body MAY pass one of its own in-scope type parameters as an explicit type argument to itself or another generic function; substitution composition preserves the referenced outer slot until a concrete outer activation determines it, while marker evidence propagates only from that outer slot's declared exact requirement set.

No rule requires a recursive generic application to use the same substitution as its caller. Validation is governed by the explicit application and ordinary call rules.

## Refinement to existing concrete Core

This source generic relation introduces no generic Core callable or generic Core type constructor.

For one supplied finite source compilation under this revision:

- the represented intrinsic type set is finite;
- the set of represented nominal record declarations is finite; and
- generic type arguments add no new type constructor and are restricted to those plain concrete type identities once an outer application is concrete.

Therefore the concrete substitution domain for every finite generic arity is finite.

A faithful source-to-Core refinement MAY realize the represented generic source program as a finite family of existing **concrete** Core function entities and direct calls obtained after exact substitution, provided it preserves all accepted source semantics, including the generic-body-selected ownership operation modes and the source call/recursion graph behavior.

One valid semantic existence construction may materialize a concrete Core function for every admitted concrete substitution tuple of every generic source function. This construction is not a requirement to generate unreachable or unused machine code.

A faithful implementation MAY instead materialize only a sufficient subset of concrete instances or use another non-observable realization strategy, provided the resulting behavior refines the same source relation and every emitted Core program uses only already represented concrete Core types/functions/calls.

First-slice marker requirements do not enlarge a specialization key or add a Core parameter/type. Once source validation has discharged them, the same concrete substitution tuple and already selected generic-body operations are sufficient for refinement.

The number, identity, sharing, caching, timing, or physical representation of specialized lower functions is not source-observable. **Monomorphization is not source semantics.** This revision introduces no dictionary-passing contract, runtime type descriptor, dynamic trait dispatch, ABI specialization rule, generic linkage identity, or Core semantic extension.

## Concrete-syntax boundary

The represented square-bracket declaration/application spelling, inline marker-requirement spelling, and list grammar are owned only by `concrete-syntax.md`.

This document consumes the resulting ordered type-parameter/type-argument sequences and resolved per-slot marker-requirement sets. It does not independently assign punctuation meaning, tokenization, trivia behavior, comma policy, marker separators, parser recovery, or array/index syntax.

## Explicitly absent generic dimensions

This revision does not define:

- trait methods, associated items/types/constants, supertraits, trait implication, operational/capability-bearing trait bounds, where clauses, runtime witnesses, or trait-derived duplicability beyond the marker-only relation owned by `traits.md`;
- a distinguished duplicability bound or other generic capability-bound syntax;
- generic records, type aliases, opaque types, enums, unions, or another generic nominal declaration;
- generic safe-reference/raw-pointer constructors or safe-reference/raw-pointer generic arguments;
- lifetime, value, const, pack, effect, placement, target, or numeric-contract generic parameters;
- type inference, omitted/default type arguments, named type arguments, partial application, or higher-kinded parameters;
- subtyping, variance, coercion, conversion, or promotion through generic substitution;
- methods, associated items/types, extension lookup, overload sets, dynamic dispatch, trait objects, generic/blanket implementations, specialization, or negative implementations;
- generic function values, indirect generic calls, closures/captures, or generic closure environments;
- reflection/reification of type parameters, marker evidence, or activation substitutions;
- generic ABI/layout/FFI/linkage/package/separate-compilation policy; or
- a generic Core semantic layer.

Those are open specification items, not implementation-defined behavior and not permissions to infer semantics from another language or realization.

## Implementation boundary

A frontend representation of this slice MUST retain semantic type-parameter slot identity independently of lexical spelling, exact resolved marker-requirement identities for every slot, exact abstract type occurrences, explicit ordered type-argument applications, and composed substitutions where needed for independent validation/lowering.

A frontend or lowerer MUST NOT recover type-parameter identity from coincident lexical names across functions, recover marker requirements from lexical spelling instead of resolved trait identity, reconstruct generic-body Copy/structure/operator capability from a marker requirement or concrete specialization, infer omitted type arguments, treat physical specialization identity as source function identity, or manufacture a generic/trait Core semantic operation.

Parser/HIR/lowering representation is otherwise non-normative and remains outside this specification owner.
