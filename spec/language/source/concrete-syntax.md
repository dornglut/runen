# Source Concrete Syntax

Status: **provisional normative; incomplete**

This document owns the represented concrete source spellings, token forms, grammar, and mapping from those forms to the accepted abstract source-language relations.

It consumes source text, whitespace, identifier-form tokens, and lexical identifier keys from [Source lexical foundation](lexical.md); module bindings and lookup from [Source names and modules](names-modules.md); source types and record declarations from [Source type foundation](types.md); function entities and callable signatures from [Source callables](callables.md); parameter/local binding semantics and function-local lookup from [Source function-local bindings](local-bindings.md); and direct-call, initialization, return, cleanup, divergence, fault, and straight-line execution semantics from [Source function execution](function-execution.md). It does not redefine those owners.

The grammar in this document is normative independently of any parser, syntax-tree, HIR, source-range, diagnostic, or backend representation.

## Lexical integration

Lexical processing begins only after the valid-UTF-8 and optional initial byte-order-mark handling defined by `lexical.md`.

Pattern whitespace from `lexical.md` and ordinary comments defined below are **trivia**. Trivia is semantically inert and MAY occur between grammar tokens wherever doing so does not split one token. Trivia separates otherwise adjacent lexical material.

The original spelling or extent of trivia MAY be preserved by source tooling. Such preservation does not make trivia program state or semantic identity.

## Reserved identifier keys

The represented concrete subset reserves exactly these lexical identifier keys:

- `fn`;
- `record`;
- `let`;
- `return`;
- `Bool`;
- `I8`, `I16`, `I32`, `I64`;
- `U8`, `U16`, `U32`, `U64`;
- `F16`, `F32`, `F64`.

A **user identifier** is an identifier-form token under `lexical.md` whose lexical identifier key is not one of the reserved keys above.

A reserved key is not legal where the grammar requires a user identifier. This revision reserves no other identifier key and defines no escaping mechanism for a reserved key.

Reserved-key classification uses the lexical identifier key, not original source spelling. It does not change identifier formation, Unicode normalization, or identifier-key equality.

## Punctuation tokens

The represented punctuation tokens are exactly:

```text
( ) { } : , -> = ;
```

`->` is one punctuation token. This revision defines no standalone `-` or `>` token and no other punctuation or operator token.

## Ordinary comments

A **line comment** begins with the two-scalar sequence `//` outside another comment and extends up to, but does not include, the next logical line boundary defined by `lexical.md`, or through the end of the source unit when no later logical line boundary exists.

A **block comment** begins with `/*` outside a line comment and ends at its matching `*/`. Block comments nest: each `/*` encountered inside a block comment increases the nesting depth, and each `*/` decreases it. The block comment ends when that depth returns to zero.

An unterminated block comment is malformed source.

Comment contents do not form identifiers, reserved keys, punctuation tokens, or grammar items. Comments have no Runen program semantics.

This revision defines no documentation-comment category or documentation semantics. Spellings such as `///`, `//!`, or `/**` are ordinary comments when they satisfy the rules above.

## Grammar notation

The productions below use quoted text for reserved keys or punctuation, `?` for an optional element, `*` for zero or more repetitions, and `|` for alternatives. `UserIdentifier` denotes one user identifier as defined above.

Trivia MAY occur between the tokens shown by these productions. Line boundaries have no statement-termination role; represented statements use mandatory semicolons.

## Source units and items

A represented source unit has this grammar:

```text
SourceUnit = Item*
Item       = RecordDefinition | FunctionDefinition
```

The textual order of module-level items does not change the order-independent module binding and lookup relations owned by `names-modules.md`.

Every represented record or function definition establishes one **module-private** module binding in the source module to which the source compilation context assigns that source unit.

This subset has no declaration-without-body, import, export, re-export, package, alias, constant, static, or other module-item syntax.

## Record definitions

```text
RecordDefinition = "record" UserIdentifier "{" RecordFields? "}"
RecordFields     = RecordField ("," RecordField)* ","?
RecordField      = UserIdentifier ":" Type
```

A represented record definition maps to exactly one nominal record declaration under `types.md` using the record name's lexical identifier key and the field sequence in concrete source order.

The field sequence MAY be empty. A trailing comma is permitted.

The concrete record form makes no positive owned-value duplicability selection. Its duplicability classification therefore follows the no-selection case defined by `types.md`.

Record construction, member access, field expressions, destructuring, and duplicability-selection syntax are not represented.

## Type forms

```text
Type = IntrinsicType | UserIdentifier

IntrinsicType = "Bool"
              | "I8"  | "I16" | "I32" | "I64"
              | "U8"  | "U16" | "U32" | "U64"
              | "F16" | "F32" | "F64"
```

Each intrinsic spelling maps one-to-one to the source type identity with the same specification label in `types.md`.

A `UserIdentifier` used as a type form undergoes same-module lookup under `names-modules.md`. The resolved binding MUST denote a nominal record source type. Resolution does not skip a binding of another category merely because the type context requires a type.

This subset has no qualified type path, type inference, type alias, generic application, pointer/reference type, tuple, array, vector, or other type form.

## Function definitions

```text
FunctionDefinition = "fn" UserIdentifier "(" Parameters? ")" ResultClause? Body
Parameters         = Parameter ("," Parameter)* ","?
Parameter          = UserIdentifier ":" Type
ResultClause       = "->" Type
```

A represented function definition maps to exactly one source function declaration/entity under `callables.md`, one callable signature, one body attachment under `function-execution.md`, and the corresponding parameter bindings under `local-bindings.md`.

Parameter source order maps directly to callable-signature parameter-slot order. Each concrete parameter identifier establishes the parameter binding corresponding to that slot. Every concrete parameter binding in this subset is immutable for assignment purposes.

When `ResultClause` is present, the callable signature has one result value of that source type. When it is absent, the callable signature has no result value. Absence of a result clause does not introduce Unit, Void, or another source value.

The concrete function form attaches the following body to the same function entity introduced by the item. This revision defines no declaration-only, generic, unsafe, async, effect, placement, target, ABI, FFI, linkage, receiver, method, overload, or other function form.

## Function bodies

The represented body grammar has exactly the function root lexical scope established by `local-bindings.md`:

```text
Body          = "{" BodyStatement* ReturnStatement? "}"
BodyStatement = LocalDeclaration | CallStatement
```

A represented return statement, when present, is terminal in this grammar. Source containing another body statement after a represented return does not match this body grammar.

Nested block statements are not represented.

Execution order and abnormal completion of the represented straight-line body are owned by `function-execution.md`.

## Ordinary local declarations

```text
LocalDeclaration = "let" UserIdentifier ":" Type "=" Value ";"
```

The concrete form maps to one ordinary local declaration under `local-bindings.md`. The explicit type and initializer are mandatory. Every concrete local binding in this subset is immutable for assignment purposes.

Initializer lookup, owned-value production, transfer, availability, and the point at which the new local enters scope are determined by `local-bindings.md` and `function-execution.md`.

This subset has no mutable-local spelling, uninitialized local, inferred local type, pattern binding, destructuring local, or assignment/reinitialization form.

## Direct calls

```text
DirectCall = UserIdentifier "(" Arguments? ")"
Arguments  = Value ("," Value)* ","?
```

A direct call maps to the direct-call relation owned by `function-execution.md` after its target identifier is resolved using the function-local lookup precedence from `local-bindings.md` and the same-module fallback from `names-modules.md`.

The resolved entity MUST be one source function entity with a represented source body. Lookup does not bypass a nearer function-local binding merely because that binding is not callable.

Argument source order is the direct-call argument order consumed by `function-execution.md`. A trailing comma is permitted.

This subset has no qualified call, indirect call, function-value call, method call, named argument, default argument, or variadic argument form.

## Call statements

```text
CallStatement = DirectCall ";"
```

A direct call used as a body statement is language-valid only when its resolved callable signature specifies no result value. A result-bearing direct call cannot be used as a statement under this grammar because this subset defines no arbitrary produced-value discard relation.

A valid no-result call statement produces no source value to discard.

## Value forms

```text
Value = IdentifierUse | DirectCall
IdentifierUse = UserIdentifier
```

An `IdentifierUse` maps to ordinary whole-binding owned-value use under `local-bindings.md`. Its identifier is resolved using the function-local lookup precedence owned there. In this subset, the selected entity MUST be an available parameter or ordinary local binding; another selected entity category does not become a value merely because the context requires one.

A `DirectCall` may be used as a `Value` only when its callable signature specifies one result value. The successful call result is the owned value produced by `function-execution.md`.

This subset has no literal, grouping expression, unary or binary operator, conversion, record construction, member access, assignment expression, block expression, closure, or other value form.

## Returns and normal completion

```text
ReturnStatement = "return" Value? ";"
```

For a result-bearing function, the body MUST end with `return Value;`. The returned value's type and ownership transfer are governed by `function-execution.md` and MUST satisfy the callable result type.

For a no-result function, the body MAY end with `return;` or omit the return statement and complete normally at `}`.

`return;` is invalid in a result-bearing function. `return Value;` is invalid in a no-result function.

This subset defines no tail-expression return and no earlier/nonterminal return position.

## Unqualified lookup and category validation

For the represented function-body identifier forms, lookup first applies the function-local precedence defined by `local-bindings.md`. Only when no active parameter/local binding resolves the lexical identifier key does lookup fall through to same-module lookup under `names-modules.md`.

After lookup selects an entity, the consuming syntactic context validates its category. The lookup MUST NOT skip the selected entity to find another binding of a context-preferred category.

Consequently, when a parameter or local binding has the same lexical key as a module-level function, a direct-call spelling with that key resolves to the function-local binding and is invalid as a direct call rather than silently bypassing the local binding.

This rule does not introduce overload resolution, separate type/value module namespaces, or qualified lookup syntax.

## Deliberate boundaries

This revision does not define:

- numeric, boolean, string, byte, or character literal syntax or literal typing;
- arithmetic, comparison, or other operator forms;
- grouping or general expression grammar;
- assignment/replacement operations or mutable-binding syntax;
- nested blocks, branches, loops, patterns, or general control flow;
- import, export, re-export, or qualified module-path syntax;
- record construction, member access, or destructuring;
- positive record duplicability-selection syntax;
- references, borrow syntax, or lifetime syntax;
- indirect calls, function values, or closures;
- generics, traits, or coherence;
- const/static forms;
- panic payload or catch forms;
- ABI, layout, FFI, or linkage forms;
- Exec or Model source forms;
- package or filesystem discovery;
- malformed-source recovery, syntax-tree structure, source-range representation, or diagnostic wording;
- source-to-Core lowering or backend behavior.

Those concerns require their own accepted semantic owners and concrete consumers before this grammar is extended.