# Source Concrete Syntax

Status: **provisional normative; incomplete**

This document owns the represented concrete source spellings, token forms, grammar, and mapping from those forms to the accepted abstract source-language relations.

It consumes source text, whitespace, identifier-form tokens, identifier-token extent, and lexical identifier keys from [Source lexical foundation](lexical.md); module bindings and lookup from [Source names and modules](names-modules.md); source types and record declarations from [Source type foundation](types.md); boolean and integer literal semantics from [Source literal semantics](literals.md); function entities and callable signatures from [Source callables](callables.md); structural paths and ownership availability from [Source structural ownership](structural-ownership.md); parameter/local binding semantics, assignment mutability, and function-local lookup from [Source function-local bindings](local-bindings.md); bounded binding-root/producer-receiver field-path selection, direct field accessibility, receiver-transient ownership, and final-field value production from [Source field-value access](field-access.md); recursive exhaustive record-pattern semantics, including qualified/unqualified heads, direct binding-root scrutinees, and producer-backed scrutinees, from [Source patterns](patterns.md); direct-call, initialization, assignment/replacement, record-construction evaluation and assembly, field-receiver evaluation/cleanup, producer-backed pattern scrutinee evaluation and transient cleanup, return, payload-free explicit-fault execution, normal-continuation presence, cleanup, divergence, defined-fault propagation, and body/block execution semantics from [Source function execution](function-execution.md); and represented statement-level conditional selection, bounded `while` selection/backedge admission, definite normal ownership, and normal-continuation composition from [Source control flow](control-flow.md). It does not redefine those owners.

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
- `copy`;
- `let`;
- `mut`;
- `return`;
- `fault`;
- `if`;
- `else`;
- `while`;
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

Reserved-key classification uses the lexical identifier key, not original source spelling. It does not change identifier formation, Unicode normalization, or identifier-key equality. In particular, longer identifier-form tokens such as `mutable`, `trueish`, `falsehood`, `ifonly`, `whiled`, and `faulty` are each one complete identifier token and are not split because they begin with a reserved key.

## Punctuation tokens

The represented punctuation tokens are exactly:

```text
( ) { } : :: , -> - = ; .
```

`->` and `::` are each one punctuation token. Where more than one represented punctuation token could begin at one source position, the longest represented token is selected; consequently `::` is never tokenized as two `:` tokens and `->` is never tokenized as `-` followed by unrepresented `>` material.

The standalone `-` punctuation token participates only in the represented negative decimal integer literal production below. It does not by itself define unary negation, subtraction, or another operator. The `.` punctuation token participates only in the represented `FieldValueUse` production below. It does not define floating-point literal spelling, a general member/postfix system, a method call, field assignment, or another operator. This revision defines no standalone `>` token and no other punctuation or operator token.

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

Trivia MAY occur around and between the tokens shown by these productions. Line boundaries have no statement-termination role. Semicolons are required exactly where a grammar production includes `;`; a represented `BlockStatement`, `IfStatement`, or `WhileStatement` terminates at its final closing `}` and has no trailing semicolon.

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

At source-unit item position, `export` modifies only a represented record or function item. The same reserved key has the separate bounded record-field position defined below. `export import` is not a represented form, and this reuse does not establish a general declaration-modifier system.

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
RecordDefinition              = "record" RecordDuplicabilitySelection? UserIdentifier "{" RecordFields? "}"
RecordDuplicabilitySelection  = "copy"
RecordFields                  = RecordField ("," RecordField)* ","?
RecordField                   = ExportModifier? UserIdentifier ":" Type
```

A represented record definition maps to exactly one nominal record declaration under `types.md` using the record name's lexical identifier key and the field sequence in concrete source order. Its module accessibility is determined by the enclosing optional `ExportModifier` as described above.

The optional `RecordDuplicabilitySelection` is record-specific. When present, it maps exactly to the positive nominal-record duplicability selection owned by `types.md`; when absent, the declaration makes no positive selection. The concrete `copy` key has no meaning as a general declaration modifier, trait/protocol/derive/attribute form, value-copy/clone operation, representation directive, ABI promise, or bitwise-copy instruction. Because `copy` belongs to the global reserved-key set above, this revision introduces no contextual-keyword mechanism.

Existing item-level `export` remains orthogonal to the record-specific selection and retains its existing position before `record`. Consequently `export record copy Name { ... }` is represented, while `copy record Name`, `export copy record Name`, and `record Name copy` are not represented forms.

Each `RecordField` maps its identifier and `Type` to the field identity/type/order relation in `types.md`. Without the field-position `ExportModifier`, the field has **module-private** direct accessibility; with it, the field has **exported** direct accessibility under `field-access.md`.

The record item's export class and each field's direct accessibility are independent. Exporting a record does not export any field, and exporting one field does not export the containing record or any sibling field. `field-access.md` owns the resulting cross-module direct-access rule and the source-accessibility requirement for the direct declared type of an exported field in an exported record.

A field-position `export` does not introduce a module declaration binding, ABI/linkage visibility, layout contract, synthetic getter/setter, or general modifier mechanism. The field remains one ordinary record field for nominal identity and structural order.

The field sequence MAY be empty. A trailing comma is permitted.

Presence or absence of `RecordDuplicabilitySelection` supplies only the positive/no-selection fact consumed by `types.md`. Eligibility, including recursive nominal-field eligibility and the zero-field case, is owned by that type-semantic relation and is not inferred from parser shape or field accessibility.

The represented record definition does not itself construct, access, destructure, duplicate, or clone a value. Record construction, represented field-value access, and represented record destructuring are represented separately below. Field assignment, partial-field reinitialization, methods, explicit copy/clone operations, and other duplicability-selection spellings are not represented.

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

The represented body grammar delimits the function root lexical scope and admits recursively nested child lexical scopes through `BlockStatement`, represented conditional arms, and represented `while` bodies:

```text
Body           = "{" BodyStatement* ReturnStatement? "}"
BodyStatement  = LocalDeclaration
               | RecordDestructuringDeclaration
               | AssignmentStatement
               | CallStatement
               | FaultStatement
               | BlockStatement
               | IfStatement
               | WhileStatement
BlockStatement = "{" BodyStatement* ReturnStatement? "}"
```

`ReturnStatement` is not a `BodyStatement`. It appears only as the optional terminal element of the immediately containing root `Body` or nested `BlockStatement`. Consequently, concrete source cannot place another `BodyStatement` or second `ReturnStatement` after that return in the same lexical block.

`FaultStatement` is deliberately a `BodyStatement`. Concrete grammar may therefore represent another `BodyStatement` or the optional terminal `ReturnStatement` after `fault;`; `function-execution.md` rejects any such later sibling semantically because `fault;` has no normal continuation. This deliberate asymmetry reuses the ordinary statement-sequencing rule instead of adding a second terminal-statement grammar category.

A represented `BlockStatement` is statement-only and produces no source value. Its closing `}` is the complete statement terminator; no trailing semicolon is present. Its `BodyStatement` sequence may be empty, its optional terminal return may be absent, and block statements may nest recursively because `BlockStatement` is itself a `BodyStatement`.

A terminal return inside a nested block terminates the current source function activation under `function-execution.md`; it does not merely break out of that block. A `fault;` reached inside a nested block likewise terminates the current activation abnormally through the defined-fault relation rather than merely exiting that block. The block form still does not create a block expression, tail value, Unit/Void value, label, break, continue, or catch form. Conditional and bounded-loop selection are introduced only by `IfStatement` and `WhileStatement` below.

Each `BlockStatement` maps to exactly one child lexical scope under `local-bindings.md`. Execution order, normal-continuation presence, normal child-scope cleanup, return cleanup, explicit-fault/defined-fault cleanup and propagation, and divergence consequences are owned by `function-execution.md`. When a block is a conditional arm or represented `while` body, `control-flow.md` owns its relationship to the applicable selection, successor, and backedge rules.

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

`ConditionalValue` deliberately reuses every currently represented `Value` producer except a **standalone** `RecordConstruction`.

This exclusion is part of normative concrete grammar. The unqualified record-construction form begins with `UserIdentifier "{"`; admitting unrestricted `Value` immediately after `if` would therefore make a spelling such as `if flag { ... }` collide with the record-construction token shape. Under the grammar above, the bare `flag` is one `IdentifierUse` conditional value and the following `{ ... }` begins the then `BlockStatement`. No semantic lookup, inferred type, or parser-only context rule is needed to choose that structure. The exclusion applies equally to a standalone qualified construction; qualification does not create a second conditional producer category.

A decimal integer literal remains syntactically represented as a `ConditionalValue`. Exact condition typing is owned by `control-flow.md`; a decimal integer therefore remains a syntax-valid conditional spelling but is source-invalid when it cannot produce the exact intrinsic `Bool` type. Concrete grammar does not encode that type error.

A `DirectCall` conditional value retains both its represented unqualified and `alias::member(...)` target forms. A `FieldValueUse` may use either its binding-root or bounded producer-backed receiver grammar. All lookup, receiver-transient, and producer rules remain owned by their existing semantic owners.

A standalone `RecordConstruction` is not a represented conditional-value spelling in this revision. A `ProducerFieldValueUse` whose receiver is a `RecordConstruction` is instead one distinct `FieldValueUse` and includes at least one mandatory `.` selector after the constructor's closing `}`. Consequently, forms such as `if Record { ready: true }.ready { ... }` and `if dep::Record { ready: true }.ready { ... }` do not make `if flag { ... }` ambiguous: the construction receiver is complete before the mandatory selector and the later then-arm block begins only after the complete `ConditionalValue`.

The then arm is always one explicit `BlockStatement`. `else` is optional; when present it is followed by exactly one explicit `BlockStatement`. Each explicit arm therefore maps to one ordinary child lexical scope and may contain ordinary `BodyStatement` entries, including `fault;`, followed by its own optional terminal `ReturnStatement` when the preceding body-statement sequence still has a normal continuation. The omitted-else false outcome and definite normal-successor composition are owned by `control-flow.md`; omission does not synthesize a concrete block or lexical scope.

This revision defines no direct `else if` production. A nested conditional may instead occur as a `BodyStatement` inside an explicit else block, for example the abstract shape `else { if ... { ... } }`.

An `IfStatement` produces no source value and has no trailing semicolon. It does not add a conditional expression, block value, Unit/Void value, pattern condition, guard, truthiness relation, comparison, or logical operator.

Because `ReturnStatement` is an optional terminal element of `BlockStatement`, a conditional arm may return from the current function. Return remains absent from `BodyStatement`, so this grammar still does not admit an arbitrary nonterminal return followed by more statements in that same arm block. Because `FaultStatement` is a `BodyStatement`, the grammar may represent a following sibling after `fault;`; the no-normal-continuation rule rejects that sibling semantically.

Runtime condition selection, condition producer ordering, arm validation, normal-continuation composition, normal arm cleanup, return behavior, explicit-fault behavior, other fault/divergence behavior, and exact structural-ownership-state equality whenever two normal outcomes meet are owned by `control-flow.md` and `function-execution.md`.

## While statements

The represented bounded statement-level loop has exactly this grammar:

```text
WhileStatement = "while" ConditionalValue BlockStatement
```

`WhileStatement` reuses the exact `ConditionalValue` nonterminal above. It therefore admits the same literal, identifier-use, direct-call, and bounded field-value producer shapes and preserves the same standalone-`RecordConstruction` exclusion. The grammar does not introduce a separate loop-condition expression category, truthiness rule, pattern condition, or semantic lookahead rule.

The loop body is exactly one ordinary `BlockStatement` and therefore one child lexical scope under `local-bindings.md`. `WhileStatement` is itself one `BodyStatement`, so loops may nest recursively and may appear inside represented conditional arms or other blocks. The closing body `}` terminates the complete `WhileStatement`; no trailing semicolon is present.

A `WhileStatement` produces no source value. It has no `else` arm, result value, label, `break`, `continue`, iteration binding, pattern, iterator protocol, unconditional-loop spelling, or do/while form. `while true` is syntactically represented but remains subject to the conservative static false-outcome rule owned by `control-flow.md`.

Exact Bool condition admission, condition ownership effects, the pre-condition environment `H`, post-condition environment `C`, body validation from `C`, exact normal-backedge structural ownership equality with `H`, the represented false normal successor `C`, no-normal-body behavior, dynamic repeated condition/body execution, and source-to-Core cyclic refinement are owned by `control-flow.md` and `function-execution.md`.

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
    RecordPatternHead "{" RecordPatternFields? "}"
RecordPatternHead =
    UserIdentifier | QualifiedModuleMember
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

Every `RecordPattern` begins with one explicit nominal record-pattern head. The head is either one unqualified `UserIdentifier` or the existing two-part `QualifiedModuleMember`. Each `RecordPatternField` maps its first identifier to one selected declared field key. Its target is either one bare binding leaf identifier or another explicit nested `RecordPattern`.

The field target is classified syntactically without semantic lookup: a bare `UserIdentifier` target is a binding leaf; `UserIdentifier "{"` begins an unqualified nested record pattern; and `UserIdentifier "::" UserIdentifier "{"` begins a qualified nested record pattern. No field type inference or lookup is needed to select among those concrete shapes.

Every record-pattern node MAY have an empty field sequence and MAY use a trailing comma. Pattern field presentation order is retained exactly. `patterns.md` defines the resulting recursive structure, exhaustive validation, depth-first binding-leaf source order, exact type/accessibility requirements, and ownership behavior.

A nested record-pattern target introduces no binding merely for naming its record head. Only bare binding-leaf targets introduce function-local bindings under `local-bindings.md`. Qualification is therefore never a binding-leaf spelling.

An unqualified pattern head maps to the same-module record-declaration lookup owned by `patterns.md` and `names-modules.md`. A qualified `alias::Record` pattern head maps to the existing source-unit module-alias and exported qualified cross-module lookup relation in `names-modules.md`; the selected entity must be a nominal record. Function-local bindings do not participate in either pattern-head lookup.

A `DirectRecordPatternRoot` is exactly one bare unqualified `UserIdentifier`. In this declaration position it maps to the accepted direct binding-root pattern relation, not to ordinary `IdentifierUse` value production. The grammar does not insert an implicit whole-record value use or scrutinee transient. Pattern-head qualification does not alter the direct-root scrutinee grammar.

A `ProducerBackedRecordPatternScrutinee` remains deliberately narrower than `Value`. It admits exactly one syntactically non-bare already-represented producer: a result-bearing `DirectCall`, a `RecordConstruction`, or a `FieldValueUse`. The top record-pattern head supplies the exact required nominal record type under `patterns.md` and `function-execution.md`.

The producer-backed alternatives reuse their existing concrete forms. A direct call may be unqualified or use the represented `alias::member(...)` target. A record construction may use its represented unqualified same-module target or qualified `alias::Record` target and remains named-field/exhaustive. Field-value use may be binding-rooted or use the bounded direct-call/record-construction producer receiver defined below.

A qualified construction may directly satisfy a qualified top record pattern when both qualified forms resolve to the same nominal record and their independent target/field-accessibility rules are source-valid. This is exact nominal producer typing, not a second pattern-scrutinee or construction category. Different record declarations remain unequal even when their fields are structurally equal.

When a producer-backed `FieldValueUse` is the scrutinee, its complete field-value operation ends before the resulting owned record enters the pattern-specific receiving relation. The field-receiver transient and the pattern scrutinee transient are therefore distinct sequential semantic objects under `field-access.md`, `patterns.md`, and `function-execution.md`.

The top scrutinee alternatives remain classifiable from their complete token shapes without semantic lookup. A direct root ends after its `UserIdentifier`; a direct call has call syntax; a record construction has constructor syntax; and a field-value use contains one or more `.` selectors after either its binding root or its complete bounded producer receiver. Scrutinee category does not depend on inferred type.

This recursive pattern form introduces no new reserved key or punctuation. It remains distinguished from `LocalDeclaration` after `let` by concrete token shape: optional `mut` followed by `UserIdentifier ":"` continues the ordinary-local form; `UserIdentifier "{"` begins an unqualified record pattern; and `UserIdentifier "::" UserIdentifier "{"` begins a qualified record pattern. This classification uses bounded token lookahead only and does not consult module lookup or source types.

Boolean and decimal integer literals are not producer-backed record-pattern scrutinees. A bare identifier is not admitted through the producer-backed alternative even though `IdentifierUse` is an ordinary `Value` producer elsewhere. No parenthesized/grouped value, operator expression, conversion, arbitrary postfix/member expression, qualified bare module member, or other general `Value` form is admitted as a record-pattern scrutinee.

Complete recursive pattern validation, binding-leaf ordering, producer evaluation ordering, transient structural ownership/cleanup, grouped binding establishment, and fault/divergence behavior are owned by `patterns.md`, `field-access.md`, `structural-ownership.md`, and `function-execution.md`.

Pattern-head qualification is discharged by source validation. A faithful typed representation may retain the resolved top nominal record identity and complete leaf paths/types/ownership facts without retaining qualified versus unqualified head spelling or separate nested-head qualification facts.

This revision defines no `let mut Record { ... }`, shorthand field pattern, wildcard/ignore, rest/omission, tuple/array/enum pattern, literal/alternative/guard pattern, qualified binding leaf, qualified field name, nested module path beyond the represented alias/member pair, refutable pattern, destructuring assignment, reference-binding mode, or mutable pattern-binding modifier.

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

## Explicit fault statements

```text
FaultStatement = "fault" ";"
```

This exact payload-free form maps to the explicit-fault execution relation owned by `function-execution.md` and selects its distinguished source-semantic defined-fault reason `ExplicitFault`.

`FaultStatement` is a statement only. It is not a `Value`, `ConditionalValue`, owned-value producer, expression, direct call, return value, declaration modifier, or general effect form. It contains no payload, message, numeric code, source value/type, site identity, exception object, or catch/matching surface.

The only represented spelling in this revision is exactly `fault;` modulo ordinary trivia. There is no `fault(...)`, `fault Value;`, `throw`, `panic`, alternate fault key, or escaping/contextual-keyword variant.

Because `fault` is globally reserved after complete maximal identifier formation, it cannot be used where `UserIdentifier` is required. Longer identifier-form tokens such as `faulty` remain ordinary user identifiers when they otherwise satisfy `lexical.md`.

`function-execution.md` owns the statement's no-normal-continuation classification, active-scope/parameter cleanup, same-fault propagation, and source-to-Core refinement. This grammar introduces no Core operation or implementation fault representation.

## Record construction

The represented exhaustive named-field record constructor has this grammar:

```text
RecordConstruction =
    RecordConstructionTarget "{" RecordInitializers? "}"
RecordConstructionTarget =
    UserIdentifier | QualifiedModuleMember
RecordInitializers = RecordInitializer ("," RecordInitializer)* ","?
RecordInitializer  = UserIdentifier ":" Value
```

An unqualified constructor target `UserIdentifier` maps directly to same-module lookup under `names-modules.md` for the source module containing the construction. Function-local parameter and ordinary-local bindings do not participate in this constructor-target lookup, even when an active local has the same lexical identifier key. The selected same-module binding MUST denote one nominal record declaration under `types.md`; lookup does not bypass a selected wrong-category module binding merely because the constructor context requires a record.

A qualified constructor target `alias::member` resolves only through the source-unit module-alias and qualified cross-module lookup relation in `names-modules.md`. Function-local bindings do not participate in that syntactically qualified lookup. The selected binding therefore already satisfies the exported-binding requirement of qualified lookup and MUST denote one nominal record declaration; an exported function or another wrong-category entity is invalid and is not bypassed.

The complete qualified member is classified syntactically before target-category validation. `alias::member { ... }` is a `RecordConstruction`, while `alias::member(...)` is a `DirectCall`; the following `{` versus `(` selects the represented form without semantic lookup. This reuse of `QualifiedModuleMember` does not create nested module paths, a constructor namespace, an overload relation, a general expression/member grammar, or a method/associated-item system.

Each `RecordInitializer` key first selects the unique declared field of the resolved record whose lexical field key is equal under the accepted identifier-key relation. An initializer key that denotes no declared field is invalid as an unknown field. A known selected field MUST then satisfy the direct record-field accessibility relation owned by `field-access.md` relative to the source module containing the construction. Consequently same-module construction may initialize module-private or exported fields, while a qualified construction of a foreign record may explicitly initialize only exported fields of that exported record. Field identity is resolved before accessibility; a known inaccessible field is not diagnosed as unknown.

Every declared field MUST still be selected exactly once. A duplicate initializer key or omission of any declared field makes the construction source-invalid. Therefore an exported foreign record containing any module-private field has no valid qualified construction under this exhaustive form: naming the private field is inaccessible, while omitting it leaves the construction incomplete. This is a consequence of the existing exhaustive shape plus direct field accessibility, not a separate constructibility or public-constructor capability.

Initializers MAY appear in any source order and MAY have a trailing comma. The initializer sequence remains the source evaluation sequence consumed by `function-execution.md`; declaration field order does not reorder initializer evaluation. For a record declaration with no fields, `Empty {}` or a source-valid qualified `alias::Empty {}` is valid when the target resolves to that record. For a record with one or more fields, an empty initializer list is invalid because required fields are missing.

The selected declaration field's source type is the required source type for its initializer `Value` producer. The initializer MUST produce exactly that source type under `types.md`; this form introduces no conversion, coercion, defaulting, widening, narrowing, or inference. The construction itself produces exactly the resolved nominal record type under `function-execution.md`, and any containing `Value` consumer or represented producer-backed record-pattern receiving position continues to require exact source type equality with its own required type.

Complete static construction validation precedes initializer ownership consequences. Before any initializer `Value` may commit an ownership transition, validation MUST establish the target lookup/accessibility/category, exact nominal result type, every initializer field identity, direct field accessibility for every known initializer field, duplicate status, exhaustive field coverage, and exact surrounding required-type equality when the receiving position supplies one. A rejection of any such structural fact commits no speculative initializer ownership.

The resulting record value has exactly the declaration-defined field/value shape from `types.md`, independent of initializer source order. Evaluation, transient ownership, defined-fault cleanup, divergence, and final ownership transfer into the selected fields are owned by `function-execution.md` and are unchanged by target qualification. Qualification introduces no runtime module-loading, ABI, linkage, layout, or physical-symbol effect.

Because each initializer contains a `Value`, record construction composes recursively with another record construction as well as the other represented value producers, including a bounded producer-backed field-value use.

A complete source-valid record construction may itself be the receiver of one or more field selectors under `FieldValueUse` below. The mandatory selector distinguishes that composite field-value producer from the bare construction producer. The construction target may be either represented target form; a qualified construction does not create a new receiver category.

After source validation, target qualification has no independent semantic identity needed by lower execution. A faithful typed representation may retain the resolved nominal record identity, resolved initializer field identities/types, produced values, and source location without retaining whether the source target was qualified. No Core/module visibility metadata or runtime access check follows from this syntax.

This form defines no inferred or anonymous constructor target, positional field list, field-init shorthand, default field value, update/spread/base syntax, field assignment, partial-field reinitialization, constructor/method body, public-constructor flag, or positive duplicability selection.

## Field-value access

The represented field-value forms have this grammar:

```text
FieldValueUse        = BindingFieldValueUse | ProducerFieldValueUse
BindingFieldValueUse = UserIdentifier FieldSelector+
ProducerFieldValueUse = FieldReceiverProducer FieldSelector+
FieldReceiverProducer = DirectCall | RecordConstruction
FieldSelector         = "." UserIdentifier
```

A `BindingFieldValueUse` root maps to the unqualified function-body lookup precedence owned by `local-bindings.md`. A `ProducerFieldValueUse` receiver is exactly one complete existing `DirectCall` or `RecordConstruction`. In both cases, the sequence of `FieldSelector` entries supplies the lexical field keys consumed by `field-access.md` in source order.

At least one selector is required after either receiver category. Consequently:

- a bare `UserIdentifier` remains `IdentifierUse` in an ordinary `Value` position and remains the distinct direct binding-root scrutinee in a `RecordDestructuringDeclaration`;
- a bare `DirectCall` remains the existing direct-call producer; and
- a bare `RecordConstruction` remains the existing construction producer.

A producer-backed field receiver does not admit an arbitrary `Value`. Boolean/decimal literals, a bare `IdentifierUse` as an expression receiver, parenthesized/grouped values, general expressions, methods, references, places, or another universal postfix receiver category are not represented. A qualified call is available only because `QualifiedModuleMember` is already one `DirectCallTarget`; a qualified construction is available only because that same bounded two-part form is now one `RecordConstructionTarget`. A qualified bare module member does not become a field receiver by itself.

A selector chain such as `make().outer.inner` is one `ProducerFieldValueUse` with one complete receiver producer and one static selector sequence. It does not recursively reinterpret each intermediate field result as a new arbitrary expression receiver.

Because direct-call arguments and construction initializers contain `Value`, bounded producer-backed field-value uses may compose recursively inside those already represented positions. This recursion does not create grouping or operator precedence.

Exact receiver result-type selection, direct record-field accessibility at every selector step, selector-path resolution, binding-root final-path availability, producer-receiver transient ownership, final-field duplicate-or-consume consequence, remaining-frontier selection, and resulting source type are owned by `field-access.md` and `structural-ownership.md`. A qualified direct-call receiver may therefore yield a foreign exported record whose exported field is selected under that owner without making the field selector itself a qualified module lookup. A qualified record-construction receiver similarly resolves and validates its complete construction before the field-value operation consumes its produced record. This grammar does not duplicate those relations.

The same `.` selector spelling represents both duplicate and consume outcomes. No second move/extract token is introduced.

The `.` token in this production has no decimal-literal, method, assignment, reference, place/lvalue, general postfix/member, or other operator meaning.

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

A `DirectCall` may be used as a `Value` only when its callable signature specifies one result value. The successful call result is the owned value produced by `function-execution.md`. The same result-bearing concrete call form may also appear as the receiver of a `ProducerFieldValueUse` or in the dedicated producer-backed record-pattern scrutinee position.

A `RecordConstruction` maps to the exhaustive record-construction relation above and produces one owned nominal record value under `function-execution.md`. Its target may be unqualified or qualified as specified above; target qualification is discharged during source validation and does not add a new value category. Record construction is not a literal and does not add a general expression hierarchy. The same concrete construction form may also appear as the receiver of a `ProducerFieldValueUse` or in the dedicated producer-backed record-pattern scrutinee position.

A `FieldValueUse` maps to the bounded binding-root/producer-backed relation above and produces one owned value under `field-access.md` when source-valid. It does not create a general member, postfix, place, or expression hierarchy. The same concrete field-value form may also appear in the dedicated producer-backed record-pattern scrutinee position when its exact result type is the nominal record selected by the top pattern head, and in `ConditionalValue` when its exact result type is `Bool` under `control-flow.md` for either represented `if` or `while` selection.

A qualified module member without a direct-call argument list, record-construction body, or record-pattern body is not an `IdentifierUse` value and is not a record-pattern scrutinee under this subset. Module aliases and module-level declarations do not become source values.

The represented `FaultStatement` is not admitted by `Value` or `ConditionalValue` and does not create a produced-value category.

This subset has no floating, string, byte, character, aggregate, pointer, or other additional literal form; grouping expression; general unary or binary operator; conversion; arbitrary-receiver member/postfix access beyond the bounded field receiver categories above; assignment expression; block expression; closure; or other value form beyond the represented producers above.

## Returns and normal completion

```text
ReturnStatement = "return" Value? ";"
```

A `ReturnStatement` may be the optional terminal element of the root `Body` or of any nested `BlockStatement`. It always returns from the current source function activation; there is no block-local return meaning.

In a result-bearing function, every represented path that reaches a `ReturnStatement` must use `return Value;`, and the returned value's type and ownership transfer are governed by `function-execution.md` and MUST satisfy the callable result type.

In a no-result function, every represented `ReturnStatement` must be `return;`. The root body may also omit a terminal return and complete normally at `}`.

`return;` is invalid in a result-bearing function. `return Value;` is invalid in a no-result function.

A result-bearing function is not required syntactically to end with one root `return Value;`. Instead, `function-execution.md` requires that no represented path reach the root closing boundary normally without a valid result-bearing return. A represented path may instead terminate abnormally through `fault;` and then needs no result value. A conditional whose two explicit arms both have no normal continuation may therefore eliminate the root normal continuation when those arms return, explicitly fault, or use a represented mixture of the two. A represented `while`, including `while true`, always retains its statically represented false normal continuation under `control-flow.md` and therefore cannot by itself discharge the missing-result obligation.

This subset defines no tail-expression return, no return as a `BodyStatement`, and no arbitrary nonterminal return followed by another statement in the same lexical block.

## Unqualified lookup and category validation

Except for an **unqualified** `RecordConstruction` target and an **unqualified** `RecordPatternHead`, whose same-module record-declaration lookup is defined by their respective owners, represented unqualified function-body identifier forms first apply the function-local precedence defined by `local-bindings.md`. Only when no active parameter/local binding resolves the lexical key does lookup fall through to same-module lookup under `names-modules.md`.

After lookup selects an entity, the consuming syntactic context validates its category. Lookup MUST NOT skip the selected entity to find another binding of a context-preferred category.

Consequently, when a parameter/local binding has the same key as a module-level function, an unqualified direct-call spelling resolves to the local binding and is invalid as a direct call rather than bypassing it. For assignment, a selected parameter/local binding is validated for assignment mutability; when no local exists and same-module lookup selects a module declaration, that entity is invalid as an assignment target rather than bypassed. A `BindingFieldValueUse` root and direct binding-root record-pattern scrutinee follow the same precedence and require a parameter/local binding under their owners.

A `ProducerFieldValueUse` applies the lookup relation of its complete receiver producer: unqualified direct call uses ordinary function-body lookup, qualified direct call uses module-alias lookup, unqualified record construction uses same-module record lookup, and qualified record construction uses module-alias lookup. The later field selectors do not cause a second root/name lookup; they consume nominal field identities and per-field accessibility under `field-access.md`.

A producer-backed pattern scrutinee likewise applies the lookup relation of its concrete producer before the pattern consumes the produced value. When that producer is a `ProducerFieldValueUse`, its receiver lookup and complete field-value production occur before the resulting owned value enters the pattern transient relation. Pattern-introduced bindings are not yet in scope during any of those lookups.

Every unqualified record-construction target and unqualified recursive record-pattern head is an explicit same-module declaration lookup. Active parameter/locals of equal key do not participate in those head/target lookups, and the selected module binding must be a record declaration. Qualified record-construction targets and qualified record-pattern heads instead use only the represented source-unit module-alias lookup relation described below.

Imported modules are not searched by ordinary unqualified lookup. They participate in construction and pattern-head lookup only through explicit qualified `alias::Record` forms.

This rule does not introduce overload resolution or general separate type/value module namespaces.

## Qualified module lookup and category validation

A concrete `alias::member` form is explicitly qualified. Its first identifier is interpreted only as a source-unit module alias under `names-modules.md`; it does not perform function-local or same-module declaration lookup. Its second identifier is resolved only in the aliased target module's declaration namespace under the exported-binding requirement owned by `names-modules.md`.

After qualified lookup selects the target binding, the consuming type, direct-call, record-construction, or record-pattern-head context validates the entity category. Lookup MUST NOT skip a private or wrong-category target to search for another entity.

A parameter/local binding MAY have the same lexical key as a module alias because the two participate in distinct lookup domains. Such a local controls ordinary unqualified spelling but does not block syntactically qualified `alias::member`.

The two-part qualification syntax is reused only in the explicitly represented type, direct-call target, record-construction target, and record-pattern-head positions. It does not create arbitrary member access, nested module paths, associated-item lookup, methods, re-export behavior, qualified binding leaves, or qualified field names. A qualified direct call may appear as a producer-backed record-pattern scrutinee or as the receiver of a `ProducerFieldValueUse`; a qualified record construction may likewise appear wherever the existing `RecordConstruction` producer category is admitted. Any resulting record value may then undergo ordinary `.` field selection through `field-access.md`. A qualified record-pattern head uses the same lookup relation only to select its nominal record and does not turn its field selectors or binding leaves into qualified module members.

## Deliberate boundaries

This revision does not define:

- floating, string, byte, character, or other literal syntax beyond represented boolean and signed decimal integer forms, nor literal suffixes, separators, alternate radices, or decimal-point forms;
- arithmetic, comparison, logical, compound-assignment, general unary-negation, subtraction, or other operator forms;
- grouping or general expression grammar;
- assignment expressions, assignment-as-value, field assignment, partial-field reinitialization, destructuring assignment, or general place/lvalue syntax beyond represented whole-binding assignment;
- uninitialized locals, type inference, mutable parameters, or mutable record-pattern binding modifiers;
- conditional expressions, direct `else if`, unrestricted nonterminal-within-block return or arbitrary unreachable tails, additional loop forms (`loop`, `for`, do/while), loop `else`, labels, `break`, `continue`, loop values, refutable/literal/alternative/guard patterns, `match`, wildcard/rest/shorthand patterns, catch/recovery, or other control-transfer forms beyond represented statement-level `if`, bounded statement-level `while`, terminal return, and payload-free explicit `fault;`;
- record-pattern scrutinees beyond the represented bare direct binding root and dedicated `DirectCall`, `RecordConstruction`, and bounded `FieldValueUse` producer-backed forms; in particular no literal, bare `IdentifierUse`-as-value, grouping, operator expression, conversion, arbitrary postfix/member expression, or other general expression is admitted there;
- source-visible module identities, dependency locators, package paths, nested module paths beyond the represented alias/member pair, selective imports, glob imports, re-exports, implicit preludes, or transitive import lookup;
- inferred/anonymous, positional, shorthand, defaulted, update/spread/base, constructor-body, method-based, or partial record construction, nor a constructor namespace or separate public-constructor capability;
- arbitrary-receiver member/postfix access beyond the explicit binding-root/direct-call/record-construction field-value forms; field accessibility beyond the represented module-private/exported direct relation; package/friend/protected accessibility; methods; properties; or associated-item lookup;
- qualified binding leaves or qualified field names inside record patterns;
- explicit copy/clone value operations, custom copy constructors, or duplicability-selection syntax beyond the record-specific `copy` selection;
- references, borrow syntax or pattern binding modes, source interior mutability, raw-pointer assignment, or lifetime syntax;
- indirect calls, function values, or closures;
- generics, traits, or coherence;
- const/static forms or a general constant-expression category;
- fault payload/message/code/site/value/type forms, `fault(...)`, `fault Value;`, panic/throw syntax, catch/recovery, backtrace syntax, or another fault spelling beyond the represented payload-free `fault;`;
- ABI, layout, FFI, or linkage forms;
- Exec or Model source forms;
- package or filesystem discovery;
- malformed-source recovery, syntax-tree structure, source-range representation, or diagnostic wording;
- source-to-Core lowering or backend behavior.

Those concerns require their own accepted semantic owners and concrete consumers before this grammar is extended.
