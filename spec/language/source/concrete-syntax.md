# Source Concrete Syntax

Status: **provisional normative; incomplete**

This document owns the represented concrete source spellings, token forms, grammar, and mapping from those forms to the accepted abstract source-language relations.

It consumes source text, whitespace, identifier-form tokens, identifier-token extent, and lexical identifier keys from [Source lexical foundation](lexical.md); module bindings and lookup from [Source names and modules](names-modules.md); source types and record declarations from [Source type foundation](types.md); boolean and integer literal semantics from [Source literal semantics](literals.md); function entities and callable signatures from [Source callables](callables.md); structural paths and ownership availability from [Source structural ownership](structural-ownership.md); parameter/local binding semantics, assignment mutability, and function-local lookup from [Source function-local bindings](local-bindings.md); binding-rooted field-path selection, direct field accessibility, and final-field value production from [Source field-value access](field-access.md); recursive exhaustive record-pattern semantics, including direct binding-root and producer-backed scrutinees, from [Source patterns](patterns.md); direct-call, initialization, assignment/replacement, record-construction evaluation and assembly, producer-backed pattern scrutinee evaluation and transient cleanup, return, cleanup, divergence, fault, and body/block execution semantics from [Source function execution](function-execution.md); and represented statement-level conditional selection and definite normal ownership joins from [Source control flow](control-flow.md). It does not redefine those owners.

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
- `if`;
- `else`;
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

Reserved-key classification uses the lexical identifier key, not original source spelling. It does not change identifier formation, Unicode normalization, or identifier-key equality. In particular, longer identifier-form tokens such as `mutable`, `trueish`, `falsehood`, and `ifonly` are each one complete identifier token and are not split because they begin with a reserved key.

## Punctuation tokens

The represented punctuation tokens are exactly:

```text
( ) { } : :: , -> - = ; .
```

`->` and `::` are each one punctuation token. Where more than one represented punctuation token could begin at one source position, the longest represented token is selected; consequently `::` is never tokenized as two `:` tokens and `->` is never tokenized as `-` followed by unrepresented `>` material.

The standalone `-` punctuation token participates only in the represented negative decimal integer literal production below. It does not by itself define unary negation, subtraction, or another operator. The `.` punctuation token participates only in the represented `FieldValueUse` production below. It does not define floating-point literal spelling, general member access, a method call, field assignment, or another operator. This revision defines no standalone `>` token and no other punctuation or operator token.

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

Trivia MAY occur around and between the tokens shown by these productions. Line boundaries have no statement-termination role. Semicolons are required exactly where a grammar production includes `;`; a represented `BlockStatement` or `IfStatement` terminates at its final closing `}` and has no trailing semicolon.

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

An external or build-system mapping for an alias key that has no corresponding concrete import declaration does not create a source alias and has no source lookup effect under this document.

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

The represented record definition does not itself construct, access, or destructure a value. Record construction, binding-rooted field-value access, and represented record destructuring are represented separately below. Field assignment, partial-field reinitialization, methods, and duplicability-selection syntax are not represented.

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

The represented body grammar delimits the function root lexical scope and admits recursively nested child lexical scopes through `BlockStatement` and represented conditional arms:

```text
Body           = "{" BodyStatement* ReturnStatement? "}"
BodyStatement  = LocalDeclaration
               | RecordDestructuringDeclaration
               | AssignmentStatement
               | CallStatement
               | BlockStatement
               | IfStatement
BlockStatement = "{" BodyStatement* "}"
```

A represented return statement, when present, is terminal at the function-root body level. Source containing another root body statement after that represented return does not match this body grammar.

A represented `BlockStatement` is statement-only and produces no source value. Its closing `}` is the complete statement terminator; no trailing semicolon is present. The enclosed sequence may be empty, and block statements may nest recursively because `BlockStatement` is itself a `BodyStatement`.

Because `ReturnStatement` is not a `BodyStatement`, this block form does not admit a return inside a nested block. A block statement itself does not create a block expression, tail value, Unit/Void value, label, loop, break, continue, or catch form. Conditional selection is introduced only by `IfStatement` below.

Each `BlockStatement` maps to exactly one child lexical scope under `local-bindings.md`. Execution order, normal child-scope cleanup, fault propagation, and divergence consequences are owned by `function-execution.md`. When a block is a conditional arm, `control-flow.md` owns its relationship to conditional selection and the definite normal join.

## Conditional statements

The represented statement-level conditional has this grammar:

```text
IfStatement =
    "if" ConditionalValue BlockStatement ("else" BlockStatement)?

ConditionalValue =
    BooleanLiteral
  | DecimalIntegerLiteral
  | IdentifierUse
  | DirectCall
  | FieldValueUse
```

`ConditionalValue` deliberately reuses every currently represented `Value` producer except `RecordConstruction`.

This exclusion is part of normative concrete grammar. The existing record-construction form begins with `UserIdentifier "{"`; admitting unrestricted `Value` immediately after `if` would therefore make a spelling such as `if flag { ... }` collide with the record-construction token shape. Under the grammar above, the bare `flag` is one `IdentifierUse` conditional value and the following `{ ... }` begins the then `BlockStatement`. No semantic lookup, inferred type, or parser-only context rule is needed to choose that structure.

A decimal integer literal remains syntactically represented as a `ConditionalValue`. Exact condition typing is owned by `control-flow.md`; a decimal integer therefore remains a syntax-valid conditional spelling but is source-invalid when it cannot produce the exact intrinsic `Bool` type. Concrete grammar does not encode that type error.

A `DirectCall` conditional value retains both its represented unqualified and `alias::member(...)` target forms. A `FieldValueUse` retains its binding-rooted selector grammar. All lookup and producer rules remain owned by their existing semantic owners.

`RecordConstruction` is not a represented conditional-value spelling in this revision. This restriction does not remove record construction from ordinary `Value` positions or producer-backed record-pattern scrutinees.

The then arm is always one explicit `BlockStatement`. `else` is optional; when present it is followed by exactly one explicit `BlockStatement`. Each explicit arm therefore maps to one ordinary child lexical scope. The omitted-else false outcome and definite enclosing ownership join are owned by `control-flow.md`; omission does not synthesize a concrete block or lexical scope.

This revision defines no direct `else if` production. A nested conditional may instead occur as a `BodyStatement` inside an explicit else block, for example the abstract shape `else { if ... { ... } }`.

An `IfStatement` produces no source value and has no trailing semicolon. It does not add a conditional expression, block value, Unit/Void value, pattern condition, guard, truthiness relation, comparison, or logical operator.

`ReturnStatement` remains absent from `BodyStatement`; conditional arms therefore do not introduce nested or early return under this grammar.

Runtime condition selection, condition producer ordering, arm validation, normal arm cleanup composition, fault/divergence behavior, and exact structural-ownership-state equality at the normal successor are owned by `control-flow.md`.

## Ordinary local declarations

```text
LocalDeclaration = "let" MutableModifier? UserIdentifier ":" Type "=" Value ";"
MutableModifier  = "mut"
```

The concrete form maps to one ordinary local declaration under `local-bindings.md`. The explicit type and initializer are mandatory in both forms.

Without `MutableModifier`, the declaration establishes an immutable binding. With `MutableModifier`, the declaration establishes a mutable binding under the assignment-mutability classification owned by `local-bindings.md`. `mut` does not create a second declaration category, a reference/memory value, or a distinct storage identity.

Initializer lookup, owned-value production, transfer, the resulting initial structural ownership state, and the point at which the new local enters scope are determined by `local-bindings.md`, `structural-ownership.md`, and `function-execution.md`.

This ordinary-local form has no uninitialized local, inferred local type, pattern binding, destructuring local, or mutable-parameter spelling.

## Recursive exhaustive record destructuring

```text
RecordDestructuringDeclaration =
    "let" RecordPattern "=" RecordPatternScrutinee ";"
RecordPattern =
    UserIdentifier "{" RecordPatternFields? "}"
RecordPatternFields =
    RecordPatternField ("," RecordPatternField)* ","?
RecordPatternField =
    UserIdentifier ":" RecordPatternTarget
RecordPatternTarget =
    UserIdentifier | RecordPattern
RecordPatternScrutinee =
    DirectRecordPatternRoot | ProducerBackedRecordPatternScrutinee
DirectRecordPatternRoot =
    UserIdentifier
ProducerBackedRecordPatternScrutinee =
    DirectCall | RecordConstruction | FieldValueUse
```

Every `RecordPattern` begins with one explicit nominal record-pattern head. Each `RecordPatternField` maps its first identifier to one selected declared field key. Its target is either one binding leaf identifier or another explicit nested `RecordPattern`.

The field target is classified syntactically without semantic lookup: a bare `UserIdentifier` target is a binding leaf, while `UserIdentifier "{"` begins a nested record pattern. The grammar therefore remains lossless and unambiguous without type information.

Every record-pattern node MAY have an empty field sequence and MAY use a trailing comma. Pattern field presentation order is retained exactly. `patterns.md` defines the resulting recursive structure, exhaustive validation, depth-first binding-leaf source order, exact type/accessibility requirements, and ownership behavior.

A nested record-pattern target introduces no binding merely for naming its record head. Only bare binding-leaf targets introduce function-local bindings under `local-bindings.md`.

A `DirectRecordPatternRoot` is exactly one bare unqualified `UserIdentifier`. In this declaration position it maps to the accepted direct binding-root pattern relation, not to ordinary `IdentifierUse` value production. The grammar does not insert an implicit whole-record value use or scrutinee transient.

A `ProducerBackedRecordPatternScrutinee` remains deliberately narrower than `Value`. It admits exactly one syntactically non-bare already-represented producer: a result-bearing `DirectCall`, a `RecordConstruction`, or a binding-rooted `FieldValueUse`. The top record-pattern head supplies the exact required nominal record type under `patterns.md` and `function-execution.md`.

The producer-backed alternatives reuse their existing concrete forms unchanged. A direct call may be unqualified or use the represented `alias::member(...)` target. Record construction remains same-module and named-field. Field-value use remains binding-rooted.

The top scrutinee alternatives are distinguishable from the bare direct root by their complete token shapes: a direct root ends after its `UserIdentifier`; a direct call continues with call syntax; a record construction continues with `{`; and a field-value use continues with one or more `.` selectors. Scrutinee category does not depend on semantic lookup or inferred type.

This recursive pattern form introduces no new reserved key or punctuation. It remains distinguished from `LocalDeclaration` after `let`: optional `mut` followed by `UserIdentifier ":"` continues the ordinary-local form, while `UserIdentifier "{"` begins a record pattern.

Boolean and decimal integer literals are not producer-backed record-pattern scrutinees. A bare identifier is not admitted through the producer-backed alternative even though `IdentifierUse` is an ordinary `Value` producer elsewhere. No parenthesized/grouped value, general expression, qualified bare module member, or other `Value` form is admitted as a record-pattern scrutinee.

Complete recursive pattern validation, binding-leaf ordering, producer evaluation ordering, transient structural ownership/cleanup, grouped binding establishment, and fault/divergence behavior are owned by `patterns.md`, `structural-ownership.md`, and `function-execution.md`.

This revision defines no `let mut Record { ... }`, shorthand field pattern, wildcard/ignore, rest/omission, tuple/array/enum pattern, literal/alternative/guard pattern, qualified record-pattern head, refutable pattern, destructuring assignment, reference-binding mode, or mutable pattern-binding modifier.

## Whole-binding assignment statements

```text
AssignmentStatement = UserIdentifier "=" Value ";"
```

The target identifier is resolved using the unqualified function-body lookup precedence from `local-bindings.md`. The concrete form maps that selected target and RHS `Value` to the whole-binding assignment relation owned there. A source-valid assignment therefore requires the selected entity, assignment mutability, structural-ownership transition, and target/RHS source types to satisfy `local-bindings.md`, `structural-ownership.md`, and `types.md`; concrete syntax does not redefine those requirements.

Assignment is a statement and produces no source value. It does not introduce Unit/Void or participate in `Value` grammar.

RHS evaluation, source-first old-value replacement cleanup, value transfer, successful target structural ownership reset, straight-line sequencing, and fault/divergence consequences are owned by `function-execution.md`.

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

## Record construction

The represented same-module named-field record constructor has this grammar:

```text
RecordConstruction = UserIdentifier "{" RecordInitializers? "}"
RecordInitializers = RecordInitializer ("," RecordInitializer)* ","?
RecordInitializer  = UserIdentifier ":" Value
```

The constructor target `UserIdentifier` maps directly to same-module lookup under `names-modules.md` for the source module containing the construction. Function-local parameter and ordinary-local bindings do not participate in constructor-target lookup, even when an active local has the same lexical identifier key. The selected same-module binding MUST denote one nominal record declaration under `types.md`; lookup does not bypass a selected wrong-category module binding merely because the constructor context requires a record.

This constructor-specific lookup does not create a general type/value namespace rule. Imported modules are not searched, and `alias::Record { ... }` is not a represented constructor form. Cross-module construction and the field-construction accessibility contract it would require remain outside this revision.

Each `RecordInitializer` key selects the unique declared field of the resolved record whose lexical field key is equal under the accepted identifier-key relation. Every declared field MUST be selected exactly once. A duplicate initializer key, an initializer key that denotes no declared field, or omission of any declared field makes the construction source-invalid.

Initializers MAY appear in any source order and MAY have a trailing comma. The initializer sequence remains the source evaluation sequence consumed by `function-execution.md`; declaration field order does not reorder initializer evaluation. For a record declaration with no fields, `Empty {}` is valid when `Empty` resolves to that record. For a record with one or more fields, an empty initializer list is invalid because required fields are missing.

The selected declaration field's source type is the required source type for its initializer `Value` producer. The initializer MUST produce exactly that source type under `types.md`; this form introduces no conversion, coercion, defaulting, widening, narrowing, or inference. The construction itself produces exactly the resolved nominal record type under `function-execution.md`, and any containing `Value` consumer or represented producer-backed record-pattern receiving position continues to require exact source type equality with its own required type.

The resulting record value has exactly the declaration-defined field/value shape from `types.md`, independent of initializer source order. Evaluation, transient ownership, defined-fault cleanup, divergence, and final ownership transfer into the selected fields are owned by `function-execution.md`.

Because each initializer contains a `Value`, record construction composes recursively with another record construction as well as the other represented value producers.

This form defines no inferred or anonymous constructor target, positional field list, field-init shorthand, default field value, update/spread/base syntax, field-value access itself, field assignment, partial-field reinitialization, constructor/method body, or positive duplicability selection.

## Binding-rooted field-value access

The represented field-value form has this grammar:

```text
FieldValueUse = UserIdentifier FieldSelector+
FieldSelector = "." UserIdentifier
```

The root `UserIdentifier` maps to the unqualified function-body lookup precedence owned by `local-bindings.md`. The sequence of `FieldSelector` entries supplies the lexical field keys consumed by `field-access.md` in source order.

At least one selector is required, so a bare `UserIdentifier` remains `IdentifierUse` in an ordinary `Value` position and remains the distinct direct binding-root scrutinee in a `RecordDestructuringDeclaration`.

`FieldValueUse` is binding-rooted. This grammar does not admit a record construction, direct call, parenthesized value, qualified module member, or another arbitrary value as its receiver.

The exact field-path selection, same-module direct field accessibility, final-path structural-availability requirement, final-field duplicate-or-consume ownership consequence, and resulting source type are owned by `field-access.md` and `structural-ownership.md`. This grammar does not duplicate those relations.

The same concrete path form represents both cases: a source-valid duplicable final field is duplicated without consumption, while a source-valid non-duplicable final field is transferred/consumed. No second move/extract token is introduced by this revision.

The `.` token in this production has no decimal-literal, method, assignment, reference, place/lvalue, or general-member meaning.

## Value forms

```text
Value                 = Literal | IdentifierUse | DirectCall | RecordConstruction | FieldValueUse
Literal               = BooleanLiteral | DecimalIntegerLiteral
BooleanLiteral        = "true" | "false"
DecimalIntegerLiteral = "-"? DecimalMagnitude
IdentifierUse         = UserIdentifier
```

The represented literal forms map to `literals.md`. `true` and `false` denote the boolean literal forms owned there. A `DecimalIntegerLiteral` supplies its concrete sign and decimal magnitude to the exact mathematical-integer and required-type materialization relation owned there. This grammar does not assign an integer default type, abstract literal type, conversion, or arithmetic semantics.

The optional `-` in `DecimalIntegerLiteral` is part of this literal grammar only. It does not establish a unary-negation expression or subtraction operator. Because it and `DecimalMagnitude` are distinct grammar tokens, ordinary trivia may occur between them under the general trivia rule above without changing the denoted signed decimal literal form.

An `IdentifierUse` maps to ordinary whole-binding owned-value use under `local-bindings.md`. Its identifier is resolved using the function-local lookup precedence owned there. In this subset, the selected entity MUST be a parameter or ordinary local binding whose complete structural root is fully available under `structural-ownership.md`; another selected entity category does not become a value merely because the context requires one.

A `DirectCall` may be used as a `Value` only when its callable signature specifies one result value. The successful call result is the owned value produced by `function-execution.md`. The same result-bearing concrete call form may also appear in the dedicated producer-backed record-pattern scrutinee position, where the top pattern head supplies the exact required record type.

A `RecordConstruction` maps to the same-module record-construction relation above and produces one owned record value under `function-execution.md`. Record construction is not a literal and does not add a general expression hierarchy. The same concrete construction form may also appear in the dedicated producer-backed record-pattern scrutinee position.

A `FieldValueUse` maps to the binding-rooted field-value relation above and produces one owned value under `field-access.md` when source-valid. It does not create a general member or place expression hierarchy. The same concrete field-value form may also appear in the dedicated producer-backed record-pattern scrutinee position when its exact result type is the nominal record selected by the top pattern head.

A qualified module member without a direct-call argument list is not an `IdentifierUse` value and is not a record-pattern scrutinee under this subset. Module aliases and module-level declarations do not become source values.

This subset has no floating, string, byte, character, aggregate, pointer, or other additional literal form; grouping expression; general unary or binary operator; conversion; arbitrary-receiver member access; assignment expression; block expression; closure; or other value form beyond the represented producers above.

## Returns and normal completion

```text
ReturnStatement = "return" Value? ";"
```

For a result-bearing function, the body MUST end with `return Value;`. The returned value's type and ownership transfer are governed by `function-execution.md` and MUST satisfy the callable result type.

For a no-result function, the body MAY end with `return;` or omit the return statement and complete normally at `}`.

`return;` is invalid in a result-bearing function. `return Value;` is invalid in a no-result function.

This subset defines no tail-expression return and no earlier/nonterminal return position.

## Unqualified lookup and category validation

Except for a `RecordConstruction` target and every `RecordPattern` head, whose same-module record-declaration lookup is defined by their respective owners, represented unqualified function-body identifier forms first apply the function-local precedence defined by `local-bindings.md`. Only when no active parameter/local binding resolves the lexical key does lookup fall through to same-module lookup under `names-modules.md`.

After lookup selects an entity, the consuming syntactic context validates its category. Lookup MUST NOT skip the selected entity to find another binding of a context-preferred category.

Consequently, when a parameter/local binding has the same key as a module-level function, an unqualified direct-call spelling resolves to the local binding and is invalid as a direct call rather than bypassing it. For assignment, a selected parameter/local binding is validated for assignment mutability; when no local exists and same-module lookup selects a module declaration, that entity is invalid as an assignment target rather than bypassed. A `FieldValueUse` root and direct binding-root record-pattern scrutinee follow the same precedence and require a parameter/local binding under their owners.

A producer-backed pattern scrutinee applies the lookup relation of its concrete producer before the pattern consumes the produced value: unqualified direct call uses ordinary function-body lookup, qualified direct call uses module-alias lookup, record construction uses same-module record lookup, and field-value use uses ordinary function-body lookup. Pattern-introduced bindings are not yet in scope during any of those lookups.

Record-construction targets and every recursive record-pattern head are explicit same-module declaration lookups. Active parameter/locals of equal key do not participate in those head/target lookups, and the selected module binding must be a record declaration.

Imported modules are not searched by ordinary unqualified lookup or by represented constructor/pattern-head relations.

This rule does not introduce overload resolution or general separate type/value module namespaces.

## Qualified module lookup and category validation

A concrete `alias::member` form is explicitly qualified. Its first identifier is interpreted only as a source-unit module alias under `names-modules.md`; it does not perform function-local or same-module declaration lookup. Its second identifier is resolved only in the aliased target module's declaration namespace under the exported-binding requirement owned by `names-modules.md`.

After qualified lookup selects the target binding, the consuming type or direct-call context validates the entity category. Lookup MUST NOT skip a private or wrong-category target to search for another entity.

A parameter/local binding MAY have the same lexical key as a module alias because the two participate in distinct lookup domains. Such a local controls ordinary unqualified spelling but does not block syntactically qualified `alias::member`.

The two-part qualification syntax does not create arbitrary member access, nested module paths, associated-item lookup, methods, re-export behavior, qualified record construction, or qualified record-pattern heads. A qualified direct call may appear as a producer-backed record-pattern scrutinee because that position reuses `DirectCall`; this does not make any record-pattern head qualified. Binding-rooted field-value access uses the distinct `.` form and semantics owned by `field-access.md`.

## Deliberate boundaries

This revision does not define:

- floating, string, byte, character, or other literal syntax beyond represented boolean and signed decimal integer forms, nor literal suffixes, separators, alternate radices, or decimal-point forms;
- arithmetic, comparison, logical, compound-assignment, general unary-negation, subtraction, or other operator forms;
- grouping or general expression grammar;
- assignment expressions, assignment-as-value, field assignment, partial-field reinitialization, destructuring assignment, or general place/lvalue syntax beyond represented whole-binding assignment;
- uninitialized locals, type inference, mutable parameters, or mutable record-pattern binding modifiers;
- conditional expressions, direct `else if`, early/nested return, loops, refutable/literal/alternative/guard patterns, `match`, wildcard/rest/shorthand patterns, catch, labels, break, continue, or other control-transfer forms beyond represented statement-level `if`;
- record-pattern scrutinees beyond the represented bare direct binding root and dedicated `DirectCall`, `RecordConstruction`, and binding-rooted `FieldValueUse` producer-backed forms; in particular no literal, bare `IdentifierUse`-as-value, grouping, operator expression, conversion, or arbitrary general expression is admitted there;
- source-visible module identities, dependency locators, package paths, nested module paths, selective imports, glob imports, re-exports, implicit preludes, or transitive import lookup;
- qualified/cross-module, inferred/anonymous, positional, shorthand, defaulted, update/spread/base, constructor-body, or method-based record construction;
- arbitrary-receiver member access, cross-module field access, field visibility modifiers, methods, or associated-item lookup;
- qualified/cross-module record-pattern heads;
- positive record duplicability-selection syntax;
- references, borrow syntax or pattern binding modes, source interior mutability, raw-pointer assignment, or lifetime syntax;
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
