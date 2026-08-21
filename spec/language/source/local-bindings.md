# Source Function-Local Bindings

Status: **provisional normative; incomplete**

This document owns the represented source semantics for function-local binding identity, abstract lexical scopes and function-local lookup precedence, binding assignment mutability, structural binding availability, ordinary whole-binding owned-value use, and whole-binding assignment legality and post-assignment availability.

It consumes lexical identifier keys from [Source lexical foundation](lexical.md), module lookup from [Source names and modules](names-modules.md), source value types, nominal record field identity and structure, and owned-value duplicability from [Source type foundation](types.md), and callable parameter-slot types from [Source callables](callables.md). It does not redefine those owners.

Represented binding-rooted field-path selection, direct field accessibility, and final-field duplicate-or-consume value production are owned by [Source field-value access](field-access.md), which consumes the root lookup and structural availability relations defined here. Represented source body attachment, dynamic activations, direct calls, owned argument/result transfer, local-initializer and assignment execution interaction, assignment replacement ordering, lexical-scope and activation cleanup, direct return, recursion, divergence, defined-fault propagation, and straight-line body execution are owned by [Source function execution](function-execution.md). The represented concrete parameter/local/value/call/field-value/assignment/block spellings are owned by [Source concrete syntax](concrete-syntax.md).

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

The concrete `let` forms in `concrete-syntax.md` supply an explicit source type and initializer. `let name: Type = Value;` establishes an immutable ordinary local declaration, while `let mut name: Type = Value;` establishes a mutable ordinary local declaration under these rules.

This revision does not define type inference or an uninitialized local form. `function-execution.md` owns represented initializer value production, transfer, and abnormal-completion interaction for the currently accepted owned value producers.

## Abstract lexical scopes

A represented function body has one root lexical scope. A represented nested block establishes one child lexical scope of its containing lexical scope. The resulting lexical scopes form a finite rooted tree.

The root braces in the concrete body form owned by `concrete-syntax.md` delimit the root lexical scope. Each concrete `BlockStatement` establishes exactly one child lexical scope containing its enclosed `BodyStatement` sequence and ending at that block's closing boundary. Recursively nested block statements therefore establish descendant lexical scopes.

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

The concrete whole-binding value uses, binding-rooted `FieldValueUse` roots, whole-binding assignment targets, and **unqualified** direct-call target identifiers in `concrete-syntax.md` consume this precedence. Consequently, if a local binding has the same key as a module-level function, an unqualified direct-call spelling selects the local binding and is invalid as a direct call rather than bypassing that binding. Conversely, an assignment target whose key has no active local binding may select a same-module declaration, but that selected module entity is invalid as an assignment target rather than being bypassed to find another binding. A field-value root likewise does not bypass the selected entity merely to obtain a record-valued binding.

Source-unit module aliases remain a distinct qualified-lookup mechanism owned by `names-modules.md`. They are not searched by unqualified function-body identifier lookup. The concrete `alias::member` direct-call target in `concrete-syntax.md` is explicitly qualified and resolves through that module-alias domain rather than this function-local lookup. Therefore an active local binding whose key equals the alias key does not block that syntactically qualified lookup.

Beyond the represented two-part module-alias qualification owned by `concrete-syntax.md` and `names-modules.md` and the operation-specific field selector owned by `field-access.md`, this revision does not define arbitrary member lookup, nested module paths, labels, pattern bindings, generic parameters, lifetime names, methods, associated items, or another future name domain.

## Binding assignment mutability

Every represented parameter/local binding is classified as exactly one of:

- **immutable**; or
- **mutable**.

Assignment mutability is a binding property independent of source type identity, source owned-value duplicability, callable-signature identity or equality, and future source alias/borrow authority.

Consuming an owned value from an immutable binding, including a represented field subvalue when `field-access.md` permits that consumption, is permitted. Immutability restricts later assignment or reinitialization; it does not require the binding to retain ownership of every subvalue forever.

The concrete parameter form in `concrete-syntax.md` establishes immutable parameter bindings. The concrete ordinary-local forms establish immutable bindings without `mut` and mutable bindings with `mut`. No concrete parameter-mutability form is represented by this revision.

Assignment to an immutable binding is language-invalid regardless of whether its complete value is fully available, partially available, or unavailable. Assignment to a mutable binding is permitted by this mutability rule subject to the target, type, structural-availability transition, and execution requirements below and in `function-execution.md`.

Binding mutability does not by itself replace an owned value or reinitialize an unavailable binding; replacement remains an explicit source assignment operation.

## Structural source binding availability

At every source program point where a represented parameter/local binding is in scope, source validation tracks ownership availability over the binding's **structural source paths**.

A structural source path is a finite sequence of resolved nominal-record field identities beginning at that binding's declared source type. The **empty path** denotes the complete binding value. A non-empty path is structurally valid only when each prefix selects a field from the nominal record type reached by the preceding prefix under `types.md`.

Structural paths use source-semantic field identity. They are not identifier spellings, parser nodes, compiler field indices, Core projections, byte offsets, physical addresses, storage identities, or backend layout.

For each in-scope binding, source validation maintains one finite **consumed-path set**. That set MUST be prefix-free: no member is an ancestor or descendant of another member.

For a structurally valid path `p` of one binding:

- `p` is **fully available** exactly when no consumed path is equal to `p`, is an ancestor of `p`, or is a descendant of `p`;
- `p` is **partially available** exactly when no consumed path is equal to or an ancestor of `p`, and at least one consumed path is a strict descendant of `p`;
- `p` is **unavailable** exactly when some consumed path is equal to or an ancestor of `p`.

These classifications are mutually exclusive for every structurally valid path.

The complete binding value is therefore:

- fully available exactly when the consumed-path set is empty;
- unavailable exactly when the consumed-path set contains the empty path; and
- partially available otherwise.

A source operation that consumes the owned value at path `p` is source-valid only when `p` is fully available immediately before that operation. Successful consumption adds `p` to the consumed-path set. Because a fully available path has no comparable consumed path, this transition preserves the prefix-free invariant.

A source operation that duplicates the owned value at path `p` is source-valid only when `p` is fully available and leaves the consumed-path set unchanged.

Structural availability is a source-validation fact. It is not Core `Live`, `Dead`, Never-initialized, or a Core destruction domain; not a storage extent; not a runtime moved-value flag; and not a requirement to materialize a physical local slot or per-field runtime state.

The relation includes zero-field and recursively zero-leaf record subvalues. Such a source subvalue can be owned, consumed, unavailable, or selected for source cleanup even when a lower structural storage model has no scalar leaf whose state changes. Source validity MUST NOT be reconstructed from the presence or absence of lower scalar liveness.

A parameter binding has an empty consumed-path set at represented function-body entry. `function-execution.md` owns how successful direct-call argument transfer establishes each parameter value before body entry.

A represented ordinary local binding receives an empty consumed-path set after its initializer successfully establishes its initial source value. `function-execution.md` owns the represented initializer evaluation and transfer relation.

A successful represented whole-binding assignment to a mutable binding resets the target consumed-path set to empty after replacement transfer completes, regardless of whether the target was fully available, partially available, or unavailable when assignment began. `function-execution.md` owns when this reset occurs relative to RHS evaluation, remaining-old-value cleanup, and transfer.

For the represented straight-line `BlockStatement`, entering or normally exiting a child lexical scope does not itself change the structural availability of an ancestor binding. Valid consumption or assignment transitions affecting ancestor bindings inside the child remain in force at the following parent-scope program point after normal child exit. A child binding instead ceases to participate in lookup when its child lexical scope ends.

A later control-flow owner MUST preserve definite structural availability when defining branches, loops, joins, or other multiple-path control flow. It MUST NOT replace a statically required ownership check with a defined runtime use-after-consumption fault merely because physical execution could detect state dynamically.

## Ordinary whole-binding owned-value use

This revision defines **ordinary whole-binding owned-value use** for the complete value owned by one parameter/local binding. It is the empty-path case of the structural availability relation above.

When ordinary whole-binding use selects a binding whose complete value is fully available and whose source type is duplicable under `types.md`, the use applies that accepted duplicability capability: it produces another owned source value preserving the source semantic value and leaves the consumed-path set unchanged.

When ordinary whole-binding use selects a binding whose complete value is fully available and whose source type is non-duplicable, the use transfers/consumes the complete owned value and adds the empty path to the consumed-path set. The complete binding value is then unavailable.

Ordinary whole-binding owned use of a partially available or unavailable complete value is language-invalid. It is not a defined runtime `Fault`.

The concrete `IdentifierUse` value form in `concrete-syntax.md` maps to this ordinary whole-binding use after lookup selects a parameter/local binding.

This implicit duplicate-or-consume relation applies only to the complete binding value. Binding-rooted field-value use is independently owned by `field-access.md`: it selects a non-empty structural path and may either duplicate or consume exactly the final selected subvalue according to that operation's accepted rules.

A later borrow/reference, explicit clone/copy-construction, pattern/destructuring, or other context may define distinct behavior without redefining this ordinary whole-binding relation.

## Whole-binding assignment and reinitialization

A represented whole-binding assignment target MUST resolve through the function-local lookup relation above and MUST denote one represented parameter/local binding. The selected binding MUST be mutable. The assignment RHS MUST produce exactly one owned source value whose source type is exactly equal under `types.md` to the target binding's declared source type.

A mutable target may be fully available, partially available, or unavailable when assignment begins. Successful assignment always replaces or reinitializes the complete binding value:

- a fully available target has one complete old value before RHS evaluation;
- a partially available target owns only the structurally remaining subvalues after prior consumption;
- an unavailable target has no old target-owned subvalue; and
- after successful replacement transfer, the target consumed-path set is empty and the complete binding value is fully available.

The target binding remains in scope during RHS evaluation. Every RHS owned use is governed by its applicable operation and the target's then-current structural availability. In particular, an ordinary non-duplicable whole-binding self-use may make the target unavailable before replacement transfer, while a represented field-value use may leave the target partially available by consuming only one final field path.

This section owns assignment legality and the resulting source structural-availability fact only. [Source function execution](function-execution.md) owns RHS evaluation, source-first replacement ordering, cleanup of the target's still-owned old subvalues, transfer, straight-line statement sequencing, and fault/divergence interaction.

This revision defines no assignment through a field/member, partial-field reinitialization, borrow/reference, pointer, interior-mutability mechanism, destructuring target, pattern, or other place form.

## Remaining ownership frontier

For cleanup selection, each binding and structurally valid path has a deterministic **remaining ownership frontier** derived solely from the binding's declared source type, resolved source field identities, and current consumed-path set.

The frontier for path `p` is defined recursively:

1. if `p` is unavailable, the frontier is empty;
2. if `p` is fully available, the frontier contains exactly `p`;
3. if `p` is partially available, the source type reached by `p` MUST be a nominal record type; visit that record's fields in reverse declaration order and concatenate the remaining ownership frontiers of the corresponding child paths in that order.

The frontier of the empty path is the binding's remaining ownership frontier.

Every frontier member is a maximal fully available source subvalue under the selected root. Frontier members are pairwise structurally disjoint. No consumed subvalue is a frontier member, and every still-owned structural subvalue lies within exactly one frontier member.

A zero-field record path that is fully available contributes that path as one frontier member even though it contains no scalar leaf. Source ownership termination and lower physical destruction are separate concerns.

`function-execution.md` owns when a remaining ownership frontier is selected for assignment replacement, lexical-scope termination, normal return, or defined-fault cleanup and the source order in which frontier members are cleaned.

## Scope termination and discard boundary

Owned-value duplicability and source discard/must-consume policy are distinct concerns.

A binding that still owns one or more source subvalues when its lexical scope or function activation terminates is not language-invalid **solely because any such subvalue's source type is non-duplicable**.

This revision introduces no source `drop` ability, must-consume type class, custom destructor, or unused-value prohibition. A later source capability may define independently justified restrictions for its represented types without redefining owned-value duplicability or structural availability.

`function-execution.md` owns which remaining ownership frontiers are selected for normal-return, assignment replacement, lexical-scope, or defined-fault cleanup and their source ordering. Applicable [Core value and storage semantics](../core/value-storage.md) remains authoritative for Core structural destruction domains, stored-value lifetime endings, and Core storage cleanup. This document does not duplicate either owner's cleanup relation.

## Function, call, assignment, and fault boundary

This document defines body-local binding facts, function-local lookup, assignment target legality, structural availability consequences, and the remaining ownership frontier. It does not redefine the execution relation owned by `function-execution.md`, including:

- function body execution;
- direct-call argument evaluation and parameter ownership transfer;
- assignment RHS evaluation, replacement cleanup, and transfer ordering;
- result production and return transfer;
- dynamic activation identity or recursion;
- lexical-scope, caller, or callee cleanup sequencing; or
- defined-fault propagation across source activations.

Indirect calls, function values, closures, references/pass modes, broader panic/catch forms, and other future execution relations remain outside this owner.

## Implementation boundary

This revision does not add or require a parser, lossless-syntax representation, typed HIR, Core MIR production lowering, runtime representation, or backend representation.

`concrete-syntax.md` provides one bounded concrete parameter/local/value/call/field-value/assignment/block subset. Its field-value form consumes the root lookup and structural availability relation above through `field-access.md`; its block form maps to the abstract child-scope relation above. Concrete syntax does not redefine binding identity, lookup, mutability, structural availability, remaining ownership, or owned-use authority.

A faithful implementation MAY represent source structural paths by resolved indices or other internal identities after source field resolution, but those representations are not source semantic identity. Lower Core path state or scalar liveness MUST NOT be imported as the source consumed-path set or used to decide source validity.

## Further boundaries

Beyond the concrete subset owned by `concrete-syntax.md`, this revision does not define type inference, assignment expressions or assignment-as-value, uninitialized locals, literals, precedence, parser recovery, general expression typing, field assignment or partial-field reinitialization, arbitrary member/method lookup, destructuring, patterns, references, borrow syntax, lifetime inference, closures/captures, generics, traits/coherence, methods, overload sets, explicit clone/copy operators, custom destructors, must-consume/drop abilities, const/static semantics, ABI/FFI/linkage, package/filesystem mapping, parser/HIR/Core MIR production code, or backend behavior.
