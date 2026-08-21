# Source Function-Local Bindings

Status: **provisional normative; incomplete**

This document owns the represented source semantics for function-local binding identity, abstract lexical scopes and function-local lookup precedence, binding assignment mutability, binding availability, and ordinary whole-binding owned-value use.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module lookup from [Source names and modules](names-modules.md), source value types and owned-value duplicability from [Source type foundation](types.md), and callable parameter-slot types from [Source callables](callables.md). It does not redefine those owners.

Represented source body attachment, dynamic activations, direct calls, owned argument/result transfer, local-initializer execution interaction, lexical-scope and activation cleanup, direct return, recursion, divergence, defined-fault propagation, and straight-line body execution are owned by [Source function execution](function-execution.md). The represented concrete parameter/local/value/call spellings are owned by [Source concrete syntax](concrete-syntax.md).

This document does not define general expression evaluation, references, patterns, traits, ABI, or an implementation representation.

## Function-local binding identity

When a represented source function entity has a body under `function-execution.md`, that body has exactly one **parameter binding** corresponding to each callable-signature parameter slot.

Each parameter binding has:

- exactly the source value type of its corresponding signature parameter slot;
- one lexical identifier key governed by `lexical.md`;
- one stable source-semantic binding identity; and
- one assignment-mutability classification defined below.

Parameter lexical keys MUST be unique within one function body.

Parameter lexical keys, parameter binding identities, and assignment-mutability classifications are body-local facts. They are not part of source function entity identity, callable-signature structure, or callable-signature equality.

Parameter bindings and ordinary function-local bindings occupy one **function-local value-binding domain**.

Every represented parameter/local binding identity is independent of original identifier spelling, token or source offset, physical storage address, compiler collection index, HIR or MIR identifier choice, and runtime storage identity.

For the function form represented by `concrete-syntax.md`, concrete parameter source order maps to callable parameter-slot order and each concrete parameter identifier supplies the lexical key for the corresponding parameter binding. Every such concrete parameter binding is immutable for assignment purposes.

## Represented ordinary local declarations

A represented ordinary local declaration:

- belongs to exactly one lexical scope;
- introduces exactly one lexical identifier key and one stable local binding identity;
- has exactly one represented source value type;
- has exactly one initializer; and
- classifies the binding as immutable or mutable for assignment purposes.

Uninitialized ordinary local declarations are not represented by this revision.

The initializer is resolved and typed in the lexical environment that exists before the new binding is introduced. The new binding enters scope only after the declaration's initialization boundary. Therefore the new binding is not available for self-reference from its own initializer.

The concrete `let` form in `concrete-syntax.md` supplies an explicit source type and initializer and establishes an immutable ordinary local declaration under these rules.

This revision does not define type inference or a mutable/uninitialized local form. `function-execution.md` owns represented initializer value production, transfer, and abnormal-completion interaction for the currently accepted owned value producers.

## Abstract lexical scopes

A represented function body has one root lexical scope. Later body constructs may establish nested child lexical scopes. The resulting lexical scopes form a finite rooted tree.

The root braces in the concrete body form owned by `concrete-syntax.md` delimit that already-defined root lexical scope. The current concrete subset establishes no nested lexical-scope form.

The semantic scope tree does not prescribe parser nodes, lossless-syntax nodes, source-range representation, or recovery behavior.

A parameter binding belongs to the function root scope and is in scope throughout the represented function body, including descendant lexical scopes.

An ordinary local binding is in scope from immediately after its declaration/initialization boundary through the end of its containing lexical scope, including descendant lexical scopes.

## Function-local shadowing and key reuse

**Overlapping function-local shadowing is forbidden.**

A parameter/local binding declaration MUST NOT introduce a lexical identifier key equal to the key of another parameter/local binding whose lexical scope contains the new declaration point.

Consequently:

- a local cannot shadow a parameter;
- a nested local cannot shadow an enclosing local;
- two sequential locals in the same continuing lexical scope cannot reuse one key; and
- disjoint sibling lexical scopes MAY independently introduce the same key because their binding scopes do not overlap.

This prohibition applies only inside the function-local value-binding domain.

A function-local binding key MAY equal a module-level declaration key.

## Function-local lookup precedence

Within a represented function body, an **unqualified function-body identifier lookup** that participates in the function-local value-binding domain consults active parameter/local bindings first.

If exactly one active parameter/local binding has the requested lexical identifier key, lookup resolves to that binding. The consuming source form then determines whether that selected binding is a valid entity category for the use.

Only when no active parameter/local binding resolves the key does lookup fall through to the accepted same-module lookup relation in `names-modules.md`.

Lookup MUST NOT skip an active function-local binding merely because the consuming context would prefer a module-level entity of another category.

The concrete whole-binding value uses and **unqualified** direct-call target identifiers in `concrete-syntax.md` consume this precedence. Consequently, if a local binding has the same key as a module-level function, an unqualified direct-call spelling selects the local binding and is invalid as a direct call rather than bypassing that binding.

Source-unit module aliases remain a distinct qualified-lookup mechanism owned by `names-modules.md`. They are not searched by unqualified function-body identifier lookup. The concrete `alias::member` direct-call target in `concrete-syntax.md` is explicitly qualified and resolves through that module-alias domain rather than this function-local lookup. Therefore an active local binding whose key equals the alias key does not block that syntactically qualified lookup.

Beyond the represented two-part module-alias qualification owned by `concrete-syntax.md` and `names-modules.md`, this revision does not define arbitrary member access, nested module paths, fields, labels, pattern bindings, generic parameters, lifetime names, methods, associated items, or another future name domain.

## Binding assignment mutability

Every represented parameter/local binding is classified as exactly one of:

- **immutable**; or
- **mutable**.

Assignment mutability is a binding property independent of source type identity, source owned-value duplicability, callable-signature identity or equality, and future source alias/borrow authority.

Consuming an owned value from an immutable binding is permitted when the applicable owned-use rule permits the consumption. Immutability restricts later assignment or reinitialization; it does not require the binding to retain ownership forever.

The concrete parameter and `let` forms in `concrete-syntax.md` establish immutable bindings only. The abstract mutable classification remains represented here for later accepted assignment/reinitialization consumers; no concrete mutable-binding spelling exists in the current subset.

This revision defines the mutability classification only. It does not define replacement expression evaluation or an operation that mutates an available binding.

## Source binding availability

At a source program point where a represented parameter/local binding is in scope, source validation tracks whether that binding is:

- **available** — the binding currently owns one source value of its declared source type; or
- **unavailable** — the binding currently owns no source value because its previous owned value was consumed.

Availability is a source-validation fact. It is not Core `Live`, `Dead`, or Never-initialized state; not a storage extent; not a runtime moved-value flag; and not a requirement to materialize a physical local slot.

A parameter binding is available at represented function-body entry. `function-execution.md` owns how successful direct-call argument transfer establishes each parameter value before body entry.

A represented ordinary local binding becomes available after its initializer establishes its initial source value. `function-execution.md` owns the currently represented initializer evaluation/transfer relation; future expression owners may add additional value producers without redefining availability.

A source operation that requires an owned value from a binding is valid only when that binding is **definitely available** at the operation's source program point.

A later control-flow owner MUST preserve this requirement when defining branches, loops, joins, or other multiple-path control flow. It MUST NOT replace a statically required availability check with a defined runtime use-after-consumption fault merely because physical execution could detect the state dynamically.

## Ordinary whole-binding owned-value use

This revision defines **ordinary owned-value use** only for the complete value owned by one parameter/local binding.

When ordinary owned-value use selects an available binding whose source type is duplicable under `types.md`, the use applies that accepted duplicability capability: it produces another owned source value preserving the source semantic value and leaves the binding available.

When ordinary owned-value use selects an available binding whose source type is non-duplicable, the use transfers/consumes the complete owned value and leaves the binding unavailable.

Ordinary owned-value use of an unavailable or not-definitely-available binding is language-invalid. It is not a defined runtime `Fault`.

The concrete `IdentifierUse` value form in `concrete-syntax.md` maps to this ordinary whole-binding use after lookup selects a parameter/local binding.

This implicit duplicate-or-consume relation applies only to ordinary owned-value contexts. A later borrow/reference, explicit consume/move, explicit clone/copy-construction, pattern/destructuring, field/member, or other context may define distinct behavior without redefining this ordinary owned-use relation.

Partial field moves and member-level availability are not represented by this revision.

## Reinitialization boundary

An immutable binding that becomes unavailable through consumption cannot become available again under the represented binding model.

A mutable unavailable binding is eligible for later legal reinitialization by an accepted source assignment/reinitialization operation. Such an operation must establish a new value before making the binding available again.

This revision does not define that operation, source-first replacement ordering, replacement of an already available value, assignment expression evaluation, or concrete assignment grammar.

A mutable binding that is already available is not implicitly replaced merely because it is mutable.

## Scope termination and discard boundary

Owned-value duplicability and source discard/must-consume policy are distinct concerns.

A binding that remains available when its lexical scope or function activation terminates is not language-invalid **solely because its source type is non-duplicable**.

This revision introduces no source `drop` ability, must-consume type class, custom destructor, or unused-value prohibition. A later source capability may define independently justified restrictions for its represented types without redefining owned-value duplicability.

`function-execution.md` owns which still-available source bindings are selected for normal-return or defined-fault cleanup and their source ordering. Applicable [Core value and storage semantics](../core/value-storage.md) remains authoritative for structural destruction domains, stored-value lifetime endings, and Core storage cleanup. This document does not duplicate either owner's cleanup relation.

## Function, call, and fault boundary

This document defines body-local binding facts and function-local lookup only. It does not redefine the direct execution relation owned by `function-execution.md`, including:

- function body execution;
- direct-call argument evaluation and parameter ownership transfer;
- result production and return transfer;
- dynamic activation identity or recursion;
- lexical-scope, caller, or callee cleanup sequencing; or
- defined-fault propagation across source activations.

Indirect calls, function values, closures, references/pass modes, broader panic/catch forms, and other future execution relations remain outside this owner.

## Implementation boundary

This revision does not add or require a parser, lossless-syntax representation, typed HIR, Core MIR production lowering, runtime representation, or backend representation.

`concrete-syntax.md` provides one bounded concrete parameter/local/value/call subset. That syntax does not alter binding identity, scope, lookup, mutability, availability, or owned-use authority defined here.

## Further boundaries

Beyond the concrete subset owned by `concrete-syntax.md`, this revision does not define type inference, assignment/replacement operations, nested-block forms, literals, precedence, parser recovery, general expression typing, partial field moves, member access, destructuring, patterns, references, borrow syntax, lifetime inference, closures/captures, generics, traits/coherence, methods, overload sets, explicit clone/copy/move operators, custom destructors, must-consume/drop abilities, const/static semantics, ABI/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR production code, or backend behavior.