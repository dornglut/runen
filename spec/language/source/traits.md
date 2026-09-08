# Source Marker Traits

Status: **provisional normative; incomplete**

This document owns the first represented source trait relation: nominal method-free marker trait entity identity, explicit marker implementation propositions, the represented compilation-global coherence rule, exact marker-obligation satisfaction for concrete and enclosing abstract generic arguments, and the absence of source-observable runtime witness/dispatch semantics for this marker-only slice.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), source module binding identity, accessibility, same-module lookup, and qualified cross-module lookup from [Source names and modules](names-modules.md), represented concrete intrinsic and nominal-record type identity from [Source type foundation](types.md), generic type-parameter slot identity, marker-requirement attachment, explicit type arguments, and exact substitution from [Source generics](generics.md), exported callable-interface accessibility consequences from [Source callables](callables.md), and represented trait/implementation/requirement spelling from [Source concrete syntax](concrete-syntax.md). Existing concrete Core type/function/call semantics remain owned by their Core specification owners.

This document does not redefine module namespaces or lookup, concrete source type identity/equality/duplicability, generic parameter identity or substitution, callable signature equality, direct-call execution ordering, concrete syntax, or Core function semantics. Those owners consume this document only where they need trait identity, implementation/coherence, or marker-obligation facts.

## Marker trait entities

A represented **marker trait declaration** introduces exactly one source **marker trait entity** through one ordinary module binding under `names-modules.md`.

Marker trait entity identity is the identity of that declaration/module binding. The lexical declaration key is lookup spelling and is not an independent structural identity.

Two distinct marker trait declarations therefore denote distinct trait entities even when they have identical presentation or no additional structure.

The marker trait binding has exactly the module-private or exported accessibility established by `names-modules.md`. Existing duplicate module-binding-key rules apply across every declaration category participating in that namespace. A record, function, and marker trait declaration do not become legal same-key overloads merely because their entity categories differ.

A first-slice marker trait has no represented members. It contains no method, associated function, associated type, associated constant, default body, receiver, supertrait relation, inherited requirement, implementation body, or executable operation.

The represented concrete declaration spelling is owned only by `concrete-syntax.md`. This document assigns no punctuation, contextual-key, trivia, or parser-recovery meaning.

## Marker trait references

A source relation that requires one marker trait identity uses the ordinary **module declaration lookup domain**, never the function-local generic type-parameter lookup domain.

A bare marker trait reference resolves through same-module declaration lookup from `names-modules.md`. A qualified marker trait reference resolves through the existing source-unit module-alias and qualified cross-module lookup relation. After that lookup succeeds, the consuming marker-trait relation requires the selected binding to denote one marker trait entity.

This rule is intentional even inside a generic binder. For example, if one module contains `trait T;` and a function declares a generic slot also spelled `T`, the marker reference in `fn f[T: T] (...)` denotes the module binding `T`, while admitted bare type-position uses of the slot inside that function denote the generic type parameter under `generics.md`. Marker lookup does not consult, shadow through, or fall back through the generic type-parameter domain.

A resolved record or function binding does not become a marker trait because the consuming position expected one. Lookup MUST NOT skip an existing wrong-category binding in order to search for another same-spelled declaration.

Qualified access to a trait in another source module therefore requires that trait binding to be exported under the existing accessibility relation. This document does not add a trait-specific accessibility class or import mechanism.

## Implementation targets

A first-slice **implementation target type** is exactly one represented concrete source type identity in either of these categories:

- one represented intrinsic scalar source type; or
- one represented nominal record source type.

Safe-reference source types, raw-pointer source types, abstract generic type-parameter expressions, generic type constructors, function types, and every other unrepresented/future type form are not implementation targets in this revision.

An intrinsic target requires no module binding lookup. A nominal record target is selected through the existing same-module or qualified nominal-type lookup/accessibility relation at the implementation declaration site.

A nominal binding selected for an implementation target MUST denote one record type. A selected function or marker trait binding does not become a record merely because the implementation target position requires a concrete type.

## Explicit implementation propositions

A represented **marker implementation declaration** establishes exactly one proposition:

`implements(Q, A)`

where:

- `Q` is one resolved marker trait entity identity; and
- `A` is one admitted implementation target concrete source type identity.

The semantic identity of this proposition is the exact ordered pair `(Q, A)`.

A marker implementation declaration introduces no module binding, lexical implementation name, member scope, visibility class, callable, runtime object, or source value. The source module/unit containing the declaration is not an additional proposition-identity dimension.

Implementation establishment is explicit only. No proposition is inferred from:

- a type's field structure;
- concrete duplicability;
- intrinsic versus nominal category;
- lexical spelling of the trait or type;
- lower Core representation;
- applications that happen to require the trait; or
- another implementation proposition.

Consequently, in a coherent supplied source compilation, the absence of an explicit proposition for exact pair `(Q, A)` means `implements(Q, A)` is false for this represented relation. This is absence of marker conformance, not negative-implementation syntax or an independently named negative proposition.

Cross-module implementation declarations use only ordinary name accessibility. In particular, one source unit MAY explicitly establish a proposition for an exported foreign marker trait and an exported foreign nominal record when both bindings are legally resolvable there. This revision imposes no additional module-ownership or orphan restriction.

The permission above does not define package ownership, dependency policy, interface serialization, downstream extension compatibility, linkage, or separate-compilation coherence.

## Compilation-global coherence and evidence availability

Coherence is evaluated over the complete supplied source compilation already participating in source validation.

For every exact pair `(Q, A)` consisting of one marker trait identity and one admitted implementation target type identity, the supplied source compilation MUST contain **at most one** marker implementation declaration establishing `implements(Q, A)`.

Two implementation declarations conflict exactly when they establish the same exact pair.

Because this revision defines only concrete implementation targets and no generic, blanket, structural, negative, prioritized, or specialized implementations, there is no broader overlap relation to solve. In particular there is no best-match selection, declaration priority, textual-order preference, module-order preference, specialization order, or fallback implementation.

If two or more declarations establish the same pair, the supplied source compilation is invalid. Source-unit presentation order, module presentation order, declaration order, filesystem order, or implementation processing order MUST NOT select one conflicting declaration.

One marker trait MAY have distinct implementation propositions for multiple concrete types. One concrete type MAY have implementation propositions for multiple distinct marker traits. Those propositions do not conflict because their exact pairs differ.

A coherent implementation proposition is compilation-global evidence after its declaration has legally resolved its trait and target. It is not a source-unit-local import, module member, re-export, or lookup name. A generic application that needs `implements(Q, A)` does not import or name an implementation declaration; it asks whether the coherent proposition exists in the complete supplied source compilation.

The textual position of an implementation declaration relative to a generic application has no semantic effect. Once the complete supplied source compilation is fixed, declaration/application source order cannot make the same proposition visible or invisible.

This deliberately makes marker conformance non-modular at this represented compilation boundary. Adding another supplied source unit can make a previously unsatisfied concrete marker obligation satisfied by adding the unique required proposition, or can make the entire supplied compilation incoherent by adding a duplicate proposition. This is source semantics of the represented whole-compilation relation, not an implementation accident.

This compilation-global rule does not promise that independently validated source compilations or serialized interfaces may later be combined without revalidating coherence and obligations. Package/separate-compilation compatibility is not represented here.

## Generic marker obligations

`generics.md` may attach one exact marker-trait requirement to an existing generic type-parameter slot and may supply one admitted type argument for that slot at a generic application. This document owns whether the supplied argument **satisfies** that marker requirement.

Let `Q` be one required marker trait identity and `A` the admitted type argument for the corresponding generic slot.

### Concrete type argument

When `A` is one represented concrete intrinsic scalar or nominal record source type, `A` satisfies requirement `Q` exactly when the coherent supplied source compilation contains the proposition:

`implements(Q, A)`.

No structural fact, type spelling, concrete duplicability, other trait implementation, or callee-body behavior can satisfy the requirement in place of that exact proposition.

### Enclosing abstract type argument

When `A` is one in-scope abstract type-parameter slot of an enclosing generic function, `A` satisfies requirement `Q` exactly when the enclosing slot's own declared marker-requirement set contains the exact marker trait identity `Q`.

Concrete implementation propositions for types that some later call might supply to the enclosing slot MUST NOT be used as evidence for the abstract slot. Generic-body validity and generic-to-generic obligation propagation depend only on the enclosing declaration's explicit marker requirements.

### No implication relation

Marker requirement satisfaction has no implication rule beyond exact identity membership described above.

This revision defines no:

- supertrait relation;
- trait inheritance;
- structural conformance;
- transitive marker implication;
- inferred or default requirement;
- body-derived obligation;
- associated-item requirement;
- overload-based evidence;
- negative reasoning; or
- use-site evidence mining.

Thus satisfying marker `A` never implies marker `B` merely because of naming, declaration order, common implementations, or another language's trait hierarchy.

A generic function declaration does not require that every possible or currently represented concrete type satisfy its declared marker requirements. The declaration may be valid without any current concrete implementation proposition for a requirement, provided its generic body is otherwise valid under the abstract rules. Concrete implementation evidence is required when a concrete generic application must discharge the applicable obligation.

## Marker capability boundary

Marker membership is a source-validity predicate only. An arbitrary marker trait establishes no owned-value, structural, scalar, reference, raw-pointer, construction, comparison, arithmetic, destruction, conversion, layout, ABI, target, or another operational capability.

In particular, `types.md` remains the sole owner of represented concrete owned-value duplicability. Establishing or omitting `implements(Q, A)` MUST NOT change whether concrete type `A` is duplicable.

Likewise, an abstract generic type parameter carrying one or more marker requirements gains no positive duplicability evidence, field shape, scalar category, safe-reference/raw-pointer category, operator support, construction ability, or other operational fact merely from those marker requirements.

The lexical spelling of a marker trait has no privileged capability meaning. A marker named `Copy`, `Clone`, `Duplicable`, `Add`, `Eq`, `Send`, or any other key remains an ordinary marker trait unless a future separately accepted canonical owner establishes a distinguished capability relation.

Later concrete substitution or implementation evidence MUST NOT retroactively change an ownership/operation mode already selected during generic-body validation or validate an operation rejected there for lack of concrete category/capability.

## Runtime and Core refinement boundary

A first-slice marker trait entity, marker implementation proposition, and marker-obligation proof have no source-observable runtime representation.

This revision introduces no:

- runtime witness value;
- witness/dictionary parameter;
- vtable;
- trait object;
- dynamic-dispatch target;
- hidden source function parameter;
- runtime type descriptor;
- reflection identity;
- ABI argument;
- linkage symbol; or
- generic/trait Core semantic form.

After source validation has established every required marker obligation, marker evidence MAY be discarded for execution/lowering. The existing generic refinement to ordinary concrete Core functions/types/calls remains applicable because marker membership selects no runtime operation and changes no generic substitution type identity.

A compiler MAY retain non-observable metadata for validation, diagnostics, caching, or incremental compilation. Such metadata is implementation data and does not create Runen program state or semantic witness identity.

## Concrete-syntax boundary

The represented marker trait declaration, implementation declaration, and marker-requirement spellings are owned only by `concrete-syntax.md`.

This document consumes resolved trait identities and implementation target identities. It does not independently assign meaning to contextual identifier keys, `:`, `+`, `;`, `::`, `export`, trivia, parser recovery, or generic square brackets.

## Explicitly absent trait dimensions

This revision does not define:

- trait methods, associated functions, associated types, associated constants, defaults, receivers, or implementation bodies;
- supertraits, trait inheritance, implication, aliases, or trait algebra;
- trait-derived duplicability or another operational capability;
- distinguished built-in capability traits;
- generic/blanket implementations or implementation type patterns;
- negative implementations, specialization, priorities, overlapping implementation selection, or fallback implementations;
- automatic, derived, structural, or implicit implementations;
- safe-reference/raw-pointer/abstract/generic implementation targets;
- trait objects, dynamic dispatch, vtables, runtime witness values, reflection, or runtime type IDs;
- methods, extension lookup, associated-item lookup, overload sets, or argument-dependent lookup;
- package/module orphan rules, dependency ownership, interface serialization, separate-compilation compatibility, ABI, FFI, linkage, or physical symbol policy;
- where-clauses, requirement inference, supertrait closure, or omitted generic evidence; or
- any Core generic/trait semantic extension.

These are open specification items, not implementation-defined behavior and not permissions to infer semantics from another language or realization.
