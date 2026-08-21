# Source Concrete Syntax

Status: **provisional normative; incomplete**

This document owns the represented concrete source spellings, token forms, grammar, and mapping from those forms to the accepted abstract source-language relations.

It consumes source text, whitespace, identifier-form tokens, identifier-token extent, and lexical identifier keys from [Source lexical foundation](lexical.md); module bindings and lookup from [Source names and modules](names-modules.md); source types and record declarations from [Source type foundation](types.md); boolean and integer literal semantics from [Source literal semantics](literals.md); function entities and callable signatures from [Source callables](callables.md); parameter/local binding semantics, assignment mutability and availability, and function-local lookup from [Source function-local bindings](local-bindings.md); and direct-call, initialization, assignment/replacement, return, cleanup, divergence, fault, and straight-line body/block execution semantics from [Source function execution](function-execution.md). It does not redefine those owners.

The grammar in this document is normative independently of any parser, syntax-tree, HIR, source-range, diagnostic, or backend representation.

## Lexical integration

Lexical processing begins only after the valid-UTF-8 and optional initial byte-order-mark handling defined by `lexical.md`.

Pattern whitespace from `lexical.md` and ordinary comments defined below are **trivia**. Trivia is semantically inert and MAY occur before the first grammar token, between grammar tokens wherever doing so does not split one token, and after the final grammar token. A represented source unit MAY contain only trivia. Trivia separates otherwise adjacent lexical material.

The original spelling or extent of trivia MAY be preserved by source tooling. Such preservation does not make trivia program state or semantic identity.

Identifier-form token extent is determined only by `lexical.md`. Reserved-key classification under this document occurs after the complete maximal identifier-form token and its lexical identifier key have been determined. A longer identifier-form token is never split merely because an initial substring would be a reserved key.

Outside trivia, every source scalar participating in this represented grammar MUST belong to one identifier-form token under `lexical.md`, one decimal-magnitude token defined below, or one represented punctuation token below. The `//`, `/*`, and `*/` sequences participate only in the ordinary-comment rules below. Other non-trivia material is malformed source under this concrete subset.

## Reserved identifier keys

The represented concrete subset reserves exactly these lexical identifier keys:

- `fn`;
- `record`;
- `let`;
- `mut`;
- `return`;
- `import`;
- `export`;
- `true`;
- `false`;
- `Bool`;
- `I8`, `I16`, `I32`, `I64`;
- `U8`, `U16`, `U32`, `U64`;
- `F16`, `F32`, `F64`.

A **user identifier** is an identifier-form token under `lexical.md` whose lexical identifier key is not one of the reserved keys above.

A reserved key is not legal where the grammar requires a user identifier. This revision reserves no other identifier key and defines no escaping mechanism for a reserved key.

Reserved-key classification uses the lexical identifier key, not original source spelling. It does not change identifier formation, Unicode normalization, or identifier-key equality. In particular, longer identifier-form tokens such as `mutable`, `trueish`, and `falsehood` are each one complete identifier token and are not split because they begin with a reserved key.

## Punctuation tokens

The represented punctuation tokens are exactly:

```text
( ) { } : :: , -> - = ;
```

`->` and `::` are each one punctuation token. Where more than one represented punctuation token could begin at one source position, the longest represented token is selected; consequently `::` is never tokenized as two `:` tokens and `->` is never tokenized as `-` followed by unrepresented `>` material.

The standalone `-` punctuation token participates only in the represented negative decimal integer literal production below. It does not by itself define unary negation, subtraction, or another operator. This revision defines no standalone `>` token and no other punctuation or operator token.

## Decimal magnitude tokens

A **decimal magnitude token** is one non-empty maximal contiguous sequence of ASCII decimal digits `0` through `9`.

When token processing begins at an ASCII decimal digit outside trivia or a comment, the token consumes every immediately following ASCII decimal digit and stops before the first other source scalar or the end of the source unit.

Only ASCII decimal digits participate in this token form. Leading zeroes are preserved as concrete spelling and have no radix significance. This token form has no suffix, digit separator, binary/octal/hexadecimal prefix, sign, exponent, or decimal point.

The token establishes only concrete decimal spelling. Its mathematical integer meaning, required-type materialization, and representability rules are owned by `literals.md`.

## Ordinary comments

A **line comment** begins with the two-scalar sequence `//` outside another comment and extends up to, but does not include, the next logical line boundary defined by `lexical.md`, or through the end of the source unit when no later logical line boundary exists.

A **block comment** begins with `/*` outside a line comment and ends at its matching `*/`. Block comments nest: each `/*` encountered inside a block comment increases the nesting depth, and each `*/` decreases it. The block comment ends when that depth returns to zero.

An unterminated block comment is malformed source.

Comment contents do not form identifiers, reserved keys, decimal magnitude tokens, punctuation tokens, or grammar items. Comments have no Runen program semantics.

This revision defines no documentation-comment category or documentation semantics. Spellings such as `///`, `//!`, or `/**` are ordinary comments when they satisfy the rules above.

## Grammar notation

The productions below use quoted text for reserved keys or punctuation, `?` for an optional element, `*` for zero or more repetitions, and `|` for alternatives. `UserIdentifier` denotes one user identifier as defined above. `DecimalMagnitude` denotes one decimal magnitude token as defined above.

Trivia MAY occur around and between the tokens shown by these productions. Line boundaries have no statement-termination role. Semicolons are required exactly where a grammar production includes `;`; a represented `BlockStatement` terminates at its closing `}` and has no trailing semicolon.

## Source units and items

A represented source unit has this grammar:

```text
SourceUnit        = SourceUnitElement*
SourceUnitElement = ImportDeclaration | Item
Item              = ExportModifier? (RecordDefinition | FunctionDefinition)
ExportModifier    = "export"
```

A well-formed source unit under this concrete subset is fully consumed by `SourceUnit` plus permitted trivia. No unmatched non-trivia material may remain before, between, or after represented elements.

Import declarations and module-level items MAY be interspersed. Their textual order does not change the order-independent module binding, source-unit alias, or qualified lookup relations owned by `names-modules.md`.

A represented source unit MAY contain imports and no module-level declarations.

A record or function definition without `ExportModifier` establishes one **module-private** module binding. The same definition with `ExportModifier` establishes one **exported** module binding. Those accessibility classes and their lookup consequences are owned by `names-modules.md`; `export` has no ABI, linkage, FFI, runtime, or realization meaning.

`export` modifies only a represented record or function item. `export import` is not a represented form.

This subset has no declaration-without-body, re-export, package, constant, static, or other module-item syntax.

## Module import declarations

```text
ImportDeclaration = "import" UserIdentifier ";"
```

The concrete identifier supplies the lexical identifier key of one source-unit-local module alias under `names-modules.md`.

The source spelling does not identify, name, discover, or derive the target source module. For each represented import declaration, the source compilation context supplies exactly one opaque target source-module identity associated with that alias key for that source unit. The concrete declaration maps to the module-import relation consisting of that alias key and the supplied target identity.

Duplicate aliases, alias conflicts with declarations in the source unit's own module, and self-import are governed by `names-modules.md`. Distinct aliases in one source unit may target the same module, and the same alias key in different source units may target different modules when the compilation context supplies those relations.

An external or build-system mapping for an alias key that has no corresponding concrete import declaration does not create a source alias and has no source lookup effect.

This form defines no source-visible module path, package coordinate, dependency locator, filename, filesystem relation, or source-visible canonical module name.

## Record definitions

```text
RecordDefinition = "record" UserIdentifier "{" RecordFields? "}"
RecordFields     = RecordField ("," RecordField)* ","?
RecordField      = UserIdentifier ":" Type
```

A represented record definition maps to exactly one nominal record declaration under `types.md` using the record name's lexical identifier key and the field sequence in concrete source order. Its module accessibility is determined by the enclosing optional `ExportModifier` as described above.

The field sequence MAY be empty. A trailing comma is permitted.

The concrete record form makes no positive owned-value duplicability selection. Its duplicability classification therefore follows the no-selection case defined by `types.md`.

Record construction, member access, field expressions, destructuring, and duplicability-selection syntax are not represented.

## Type forms

```text
Type                  = IntrinsicType | UserIdentifier | QualifiedModuleMember
QualifiedModuleMember = UserIdentifier "::" UserIdentifier

IntrinsicType = "Bool"
              | "I8"  | "I16" | "I32" | "I64"
              | "U8"  | "U16" | "U32" | "U64"
              | "F16" | "F32" | "F64"
```

Each intrinsic spelling maps one-to-one to the source type identity with the same specification label in `types.md`.

A `UserIdentifier` used as a type form undergoes same-module lookup under `names-modules.md`. The resolved binding MUST denote a nominal record source type. Resolution does not skip a binding of another category merely because the type context requires a type.

A `QualifiedModuleMember` used as a type form maps its first identifier to the source-unit module-alias key and its second identifier to the target-member key consumed by qualified cross-module lookup under `names-modules.md`. The resolved target binding MUST be exported and MUST denote a nominal record source type. Lookup does not bypass an inaccessible or wrong-category binding.

This subset has no nested module path, type inference, type alias, generic application, pointer/reference type, tuple, array, vector, or other type form.

## Function definitions

```text
FunctionDefinition = "fn" UserIdentifier "(" Parameters? ")" ResultClause? Body
Parameters         = Parameter ("," Parameter)* ","?
Parameter          = UserIdentifier ":" Type
ResultClause       = "->" Type
```

A represented function definition maps to exactly one source function declaration/entity under `callables.md`, one callable signature, one body attachment under `function-execution.md`, and the corresponding parameter bindings under `local-bindings.md`. Its module accessibility is determined by the enclosing optional `ExportModifier` as described above.

Parameter source order maps directly to callable-signature parameter-slot order. Each concrete parameter identifier establishes the parameter binding corresponding to that slot. Every concrete parameter binding in this subset is immutable for assignment purposes.

When `ResultClause` is present, the callable signature has one result value of that source type. When it is absent, the callable signature has no result value. Absence of a result clause does not introduce Unit, Void, or another source value.

The concrete function form attaches the following body to the same function entity introduced by the item. This revision defines no declaration-only, generic, unsafe, async, effect, placement, target, ABI, FFI, linkage, receiver, method, overload, or other function form.

## Function bodies

The represented body grammar delimits the function root lexical scope and admits recursively nested child lexical scopes through `BlockStatement`:

```text
Body           = "{" BodyStatement* ReturnStatement? "}"
BodyStatement  = LocalDeclaration
               | AssignmentStatement
               | CallStatement
               | BlockStatement
BlockStatement = "{" BodyStatement* "}"
```

A represented return statement, when present, is terminal at the function-root body level. Source containing another root body statement after that represented return does not match this body grammar.

A represented `BlockStatement` is statement-only and produces no source value. Its closing `}` is the complete statement terminator; no trailing semicolon is present. The enclosed sequence may be empty, and block statements may nest recursively because `BlockStatement` is itself a `BodyStatement`.

Because `ReturnStatement` is not a `BodyStatement`, this block form does not admit a return inside a nested block. It does not create a block expression, tail value, Unit/Void value, label, branch, loop, break, continue, or catch form.

Each `BlockStatement` maps to exactly one child lexical scope under `local-bindings.md`. Execution order, normal child-scope cleanup, fault propagation, and divergence consequences are owned by `function-execution.md`.

## Ordinary local declarations

```text
LocalDeclaration = "let" MutableModifier? UserIdentifier ":" Type "=" Value ";"
MutableModifier  = "mut"
```

The concrete form maps to one ordinary local declaration under `local-bindings.md`. The explicit type and initializer are mandatory in both forms.

Without `MutableModifier`, the declaration establishes an immutable binding. With `MutableModifier`, it establishes a mutable binding under the assignment-mutability classification owned by `local-bindings.md`. `mut` does not create a second declaration category, a reference/memory value, or a distinct storage identity.

Initializer lookup, owned-value production, transfer, availability, and the point at which the new local enters scope are determined by `local-bindings.md` and `function-execution.md`.

This subset has no uninitialized local, inferred local type, pattern binding, destructuring local, or mutable-parameter spelling.

## Whole-binding assignment statements

```text
AssignmentStatement = UserIdentifier "=" Value ";"
```

The target identifier is resolved using the unqualified function-body lookup precedence from `local-bindings.md`. The concrete form maps that selected target and RHS `Value` to the whole-binding assignment relation owned there. A source-valid assignment therefore requires the selected entity, assignment mutability, availability transition, and target/RHS source types to satisfy `local-bindings.md`; concrete syntax does not redefine those requirements.

Assignment is a statement and produces no source value. It does not introduce Unit/Void or participate in `Value` grammar.

RHS evaluation, source-first old-value replacement cleanup, value transfer, successful target availability, straight-line sequencing, and fault/divergence consequences are owned by `function-execution.md`.

This form targets only the complete selected binding. There is no field/member assignment, destructuring assignment, compound assignment, qualified assignment target, pointer/reference assignment, or general place/lvalue grammar in this subset.

## Direct calls

```text
DirectCall       = DirectCallTarget "(" Arguments? ")"
DirectCallTarget = UserIdentifier | QualifiedModuleMember
Arguments        = Value ("," Value)* ","?
```

An unqualified `UserIdentifier` call target maps to the direct-call relation owned by `function-execution.md` after its target identifier is resolved using the function-local lookup precedence from `local-bindings.md` and the same-module fallback from `names-modules.md`.

A qualified `alias::member` call target resolves only through the source-unit module-alias and qualified cross-module lookup relation in `names-modules.md`. Function-local bindings do not participate in that syntactically qualified lookup.

In either form, the resolved entity MUST be one source function entity with a represented source body. Lookup does not bypass a selected wrong-category or inaccessible binding merely because the call context requires a function.

Argument source order is the direct-call argument order consumed by `function-execution.md`. A trailing comma is permitted.

This subset has no indirect call, function-value call, method call, named argument, default argument, variadic argument, nested module path, or arbitrary member-call form.

## Call statements

```text
CallStatement = DirectCall ";"
```

A direct call used as a body statement is language-valid only when its resolved callable signature specifies no result value. A result-bearing direct call cannot be used as a statement under this grammar because this subset defines no arbitrary produced-value discard relation.

A valid no-result call statement produces no source value to discard.

## Value forms

```text
Value                 = Literal | IdentifierUse | DirectCall
Literal               = BooleanLiteral | DecimalIntegerLiteral
BooleanLiteral        = "true" | "false"
DecimalIntegerLiteral = "-"? DecimalMagnitude
IdentifierUse         = UserIdentifier
```

The represented literal forms map to `literals.md`. `true` and `false` denote the boolean literal forms owned there. A `DecimalIntegerLiteral` supplies its concrete sign and decimal magnitude to the exact mathematical-integer and required-type materialization relation owned there. This grammar does not assign an integer default type, abstract literal type, conversion, or arithmetic semantics.

The optional `-` in `DecimalIntegerLiteral` is part of this literal grammar only. It does not establish a unary-negation expression or subtraction operator. Because it and `DecimalMagnitude` are distinct grammar tokens, ordinary trivia may occur between them under the general trivia rule above without changing the denoted signed decimal literal form.

An `IdentifierUse` maps to ordinary whole-binding owned-value use under `local-bindings.md`. Its identifier is resolved using the function-local lookup precedence owned there. In this subset, the selected entity MUST be an available parameter or ordinary local binding; another selected entity category does not become a value merely because the context requires one.

A `DirectCall` may be used as a `Value` only when its callable signature specifies one result value. The successful call result is the owned value produced by `function-execution.md`.

A qualified module member without a direct-call argument list is not an `IdentifierUse` value under this subset. Module aliases and module-level declarations do not become source values.

This subset has no floating, string, byte, character, aggregate, pointer, or other additional literal form; grouping expression; general unary or binary operator; conversion; record construction; member access; assignment expression; block expression; closure; or other value form.

## Returns and normal completion

```text
ReturnStatement = "return" Value? ";"
```

For a result-bearing function, the body MUST end with `return Value;`. The returned value's type and ownership transfer are governed by `function-execution.md` and MUST satisfy the callable result type.

For a no-result function, the body MAY end with `return;` or omit the return statement and complete normally at `}`.

`return;` is invalid in a result-bearing function. `return Value;` is invalid in a no-result function.

This subset defines no tail-expression return and no earlier/nonterminal return position.

## Unqualified lookup and category validation

For the represented unqualified function-body identifier forms, lookup first applies the function-local precedence defined by `local-bindings.md`. Only when no active parameter/local binding resolves the lexical identifier key does lookup fall through to same-module lookup under `names-modules.md`.

After lookup selects an entity, the consuming syntactic context validates its category. The lookup MUST NOT skip the selected entity to find another binding of a context-preferred category.

Consequently, when a parameter or local binding has the same lexical key as a module-level function, an unqualified direct-call spelling with that key resolves to the function-local binding and is invalid as a direct call rather than silently bypassing the local binding. For an assignment target, a selected parameter/local binding is validated for assignment mutability; when no local binding exists and same-module lookup selects a module declaration, that selected entity is invalid as an assignment target rather than being bypassed.

Imported modules are not searched by this unqualified lookup relation.

This rule does not introduce overload resolution or separate type/value module namespaces.

## Qualified module lookup and category validation

A concrete `alias::member` form is explicitly qualified. Its first identifier is interpreted only as a source-unit module alias under `names-modules.md`; it does not perform function-local or same-module declaration lookup. Its second identifier is resolved only in the aliased target module's declaration namespace under the exported-binding requirement owned by `names-modules.md`.

After qualified lookup selects the target binding, the consuming type or direct-call context validates the entity category. The lookup MUST NOT skip a private or wrong-category target to search for another entity.

A parameter or local binding MAY have the same lexical key as a module alias because the two participate in distinct lookup domains. Such a local continues to control an unqualified spelling but does not block the syntactically qualified `alias::member` form.

The two-part qualification syntax does not create general member access, nested module paths, associated-item lookup, methods, or re-export behavior.

## Deliberate boundaries

This revision does not define:

- floating, string, byte, character, or other literal syntax beyond the represented boolean and signed decimal integer forms, nor any literal suffix, digit separator, or alternate-radix form;
- arithmetic, comparison, logical, compound-assignment, general unary-negation, subtraction, or other operator forms;
- grouping or general expression grammar;
- assignment expressions, assignment-as-value, or general place/lvalue syntax beyond the represented whole-binding statement;
- uninitialized locals, type inference, or mutable parameters;
- branches, loops, patterns, or other multiple-path/control-transfer forms;
- source-visible module identities, dependency locators, package paths, nested module paths, selective imports, glob imports, re-exports, implicit preludes, or transitive import lookup;
- record construction, member access, field assignment, or destructuring;
- positive record duplicability-selection syntax;
- references, borrow syntax, source interior mutability, raw-pointer assignment, or lifetime syntax;
- indirect calls, function values, or closures;
- generics, traits, or coherence;
- const/static forms or a general constant-expression category;
- panic payload or catch forms;
- ABI, layout, FFI, or linkage forms;
- Exec or Model source forms;
- package or filesystem discovery;
- malformed-source recovery, syntax-tree structure, source-range representation, or diagnostic wording;
- source-to-Core lowering or backend behavior.

Those concerns require their own accepted semantic owners and concrete consumers before this grammar is extended.