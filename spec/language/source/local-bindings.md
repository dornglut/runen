# Source Function-Local Bindings

Status: **provisional normative; incomplete**

This document owns the represented source semantics for function-local binding identity, abstract lexical scopes and local lookup, binding assignment mutability, binding availability, and ordinary whole-binding owned-value use.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module lookup from [Source names and modules](names-modules.md), source value types and owned-value duplicability from [Source type foundation](types.md), and callable parameter-slot types from [Source callables](callables.md). It does not redefine those owners.

This document does not define concrete body/local syntax, expression evaluation, calls, returns, references, patterns, traits, ABI, or an implementation representation.

## Function-local binding identity

When another accepted source rule establishes that a represented source function entity has a body, that body has exactly one **parameter binding** corresponding to each callable-signature parameter slot.

Each parameter binding has:

- exactly the source value type of its corresponding signature parameter slot;
- one lexical identifier key governed by `lexical.md`;
- one stable source-semantic binding identity; and
- one assignment-mutability classification defined below.

Parameter lexical keys MUST be unique within one function body.

Parameter lexical keys, parameter binding identities, and assignment-mutability classifications are body-local facts. They are not part of source function entity identity, callable-signature structure, or callable-signature equality.

Parameter bindings and ordinary function-local bindings occupy one **function-local value-binding domain**.

Every represented parameter/local binding identity is independent of original identifier spelling, token or source offset, physical storage address, compiler collection index, HIR or MIR identifier choice, and runtime storage identity.

## Represented ordinary local declarations

A represented ordinary local declaration:

- belongs to exactly one lexical scope;
- introduces exactly one lexical identifier key and one stable local binding identity;
- has exactly one represented source value type;
- has exactly one initializer; and
- classifies the binding as immutable or mutable for assignment purposes.

Uninitialized ordinary local declarations are not represented by this revision.

The initializer is resolved and typed in the lexical environment that exists before the new binding is introduced. The new binding enters scope only after the declaration's initialization boundary. Therefore the new binding is not available for self-reference from its own initializer.

This revision does not define type inference, initializer expression evaluation, concrete local-declaration syntax, or concrete mutability spelling/defaulting.

## Abstract lexical scopes

A represented function body has one root lexical scope. Later body constructs may establish nested child lexical scopes. The resulting lexical scopes form a finite rooted tree.

The semantic scope tree does not prescribe braces, indentation, parser nodes, lossless-syntax nodes, source-range representation, or recovery behavior.

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

A function-local binding key MAY equal a module-level declaration key. For an ordinary unqualified local value lookup, active function-local bindings are consulted first. Only when no active local binding resolves the key does lookup fall through to the accepted same-module lookup relation in `names-modules.md`.

Source-unit module aliases remain a distinct qualified-lookup mechanism owned by `names-modules.md`. They are not searched by ordinary unqualified local value lookup.

This revision does not define qualified path syntax, fields, members, labels, pattern bindings, generic parameters, lifetime names, methods, associated items, or another future name domain.

## Binding assignment mutability

Every represented parameter/local binding is classified as exactly one of:

- **immutable**; or
- **mutable**.

Assignment mutability is a binding property independent of source type identity, source owned-value duplicability, callable-signature identity or equality, and future source alias/borrow authority.

Consuming an owned value from an immutable binding is permitted when the applicable owned-use rule permits the consumption. Immutability restricts later assignment or reinitialization; it does not require the binding to retain ownership forever.

This revision defines the mutability classification only. It does not define assignment syntax, replacement expression evaluation, or an operation that mutates an available binding.

## Source binding availability

At a source program point where a represented parameter/local binding is in scope, source validation tracks whether that binding is:

- **available** — the binding currently owns one source value of its declared source type; or
- **unavailable** — the binding currently owns no source value because its previous owned value was consumed.

Availability is a source-validation fact. It is not Core `Live`, `Dead`, or Never-initialized state; not a storage extent; not a runtime moved-value flag; and not a requirement to materialize a physical local slot.

A parameter binding is available at represented function-body entry. A later call/activation owner must define how argument transfer establishes that value; this document does not define the transfer.

A represented ordinary local binding becomes available after its initializer establishes its initial source value.

A source operation that requires an owned value from a binding is valid only when that binding is **definitely available** at the operation's source program point.

A later control-flow owner MUST preserve this requirement when defining branches, loops, joins, or other multiple-path control flow. It MUST NOT replace a statically required availability check with a defined runtime use-after-consumption fault merely because physical execution could detect the state dynamically.

## Ordinary whole-binding owned-value use

This revision defines **ordinary owned-value use** only for the complete value owned by one parameter/local binding.

When ordinary owned-value use selects an available binding whose source type is duplicable under `types.md`, the use applies that accepted duplicability capability: it produces another owned source value preserving the source semantic value and leaves the binding available.

When ordinary owned-value use selects an available binding whose source type is non-duplicable, the use transfers/consumes the complete owned value and leaves the binding unavailable.

Ordinary owned-value use of an unavailable or not-definitely-available binding is language-invalid. It is not a defined runtime `Fault`.

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

Applicable Core termination cleanup remains the execution-semantic foundation when a later source-to-Core lowering and function-execution relation are accepted. This document does not duplicate Core destruction domains, cleanup order, or stored-value lifetime rules.

## Function, call, and fault boundary

This revision defines body-local binding facts only. It does not define:

- function body execution;
- call expressions or argument evaluation;
- parameter ownership transfer at call boundaries;
- result production or return transfer;
- activation identity or recursion;
- caller/callee cleanup sequencing; or
- fault/panic propagation across activations.

Core's accepted cleanup rules for one activation on `Return` and defined `Fault` remain unchanged. A later executable-function/call owner must consume those rules rather than recreate them.

## Implementation boundary

This revision does not add or require a parser, lossless-syntax representation, typed HIR, Core MIR production lowering, runtime representation, or backend representation.

The represented local semantics remove a source-validation ambiguity, but concrete body/expression/call semantics and concrete grammar are still absent. An implementation surface for those constructs would therefore still force unowned syntax or execution decisions.

## Further boundaries

This revision does not define concrete function/parameter/local/body syntax, keywords, punctuation, block grammar, literals, precedence, parser recovery, type inference, general expression typing, calls, returns, activation execution, recursion, fault propagation, partial field moves, member access, destructuring, patterns, references, borrow syntax, lifetime inference, closures/captures, generics, traits/coherence, methods, overload sets, explicit clone/copy/move operators, custom destructors, must-consume/drop abilities, const/static semantics, ABI/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR production code, or backend behavior.
