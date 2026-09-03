# Source Concrete Syntax

Status: **provisional normative; incomplete**

This document owns the represented concrete source spellings, token forms, grammar, and mapping from those forms to the accepted abstract source-language relations.

It consumes source text, whitespace, identifier-form tokens, identifier-token extent, and lexical identifier keys from [Source lexical foundation](lexical.md); module bindings and lookup from [Source names and modules](names-modules.md); source types and record declarations from [Source type foundation](types.md); boolean, integer, and decimal floating literal semantics from [Source literal semantics](literals.md); Boolean logical-negation, Boolean short-circuit conjunction, plain fixed-width integer-negation/bitwise-complement/multiplication/addition/subtraction/exclusive-or/bitwise-OR, same-format binary floating multiplication, division, addition, and subtraction including their selected numeric-contract facts, and Boolean equality/inequality operand/result typing and semantic value transformation from [Source operator semantics](operators.md); function entities and callable signatures from [Source callables](callables.md); binding and replacement-capable external-referent structural paths, ownership availability, and bounded non-empty subpath installation from [Source structural ownership](structural-ownership.md); parameter/local binding semantics, assignment mutability, function-local lookup, bounded binding-root field assignment, and raw-pointer local integration from [Source function-local bindings](local-bindings.md); bounded Shared and replacement-capable exclusive reference types, complete-root and Shared binding-field root formation, complete-referent dereference, explicit complete/Shared field-relative reborrow, reference-relative replacement, authority/carrier lifetime, external-referent structural state, call-entry/restoration, and bounded safe-reference result-contract semantics from [Source safe references](references.md); activation-local raw-pointer type/value/origin semantics, raw address formation, unsafe raw ownership move/replacement, and lexical unsafe admission from [Source raw pointers and unsafe admission](raw-pointers-unsafe.md); bounded binding-root/producer-receiver field-path selection, bounded binding-root field-assignment target selection, direct field accessibility, receiver-transient ownership, and final-field value production from [Source field-value access](field-access.md); recursive irrefutable record-pattern semantics with bounded node-local rest/omission, including qualified/unqualified heads, direct binding-root scrutinees, and producer-backed scrutinees, from [Source patterns](patterns.md); direct-call, represented operator operand validation/evaluation, bounded contextual grouping transparency, operation-local numeric-contract-selected-value applicability/execution transparency, initialization, ordinary whole-binding and bounded binding-root field assignment/replacement, complete-referent replacement, raw-operation and unsafe-block execution, record-construction evaluation and assembly, field-receiver evaluation/cleanup, producer-backed pattern scrutinee evaluation and transient cleanup, return, payload-free explicit-fault execution, loop-transfer cleanup, normal-continuation presence, cleanup, divergence, defined-fault propagation, and body/block execution semantics from [Source function execution](function-execution.md); and represented statement-level conditional selection, bounded `while` selection/backedge admission, bounded `break`/`continue` target/state admission, definite normal binding/external-referent structural ownership plus raw-pointer origin, and normal-continuation composition from [Source control flow](control-flow.md). It does not redefine those owners.

The grammar in this document is normative independently of any parser, syntax-tree, HIR, source-range, diagnostic, or backend representation.

## Lexical integration

Lexical processing begins only after the valid-UTF-8 and optional initial byte-order-mark handling defined by `lexical.md`.

Pattern whitespace from `lexical.md` and ordinary comments defined below are **trivia**. Trivia is semantically inert and MAY occur before the first grammar token, between grammar tokens wherever doing so does not split one token, and after the final grammar token. A represented source unit MAY contain only trivia. Trivia separates otherwise adjacent lexical material.

The original spelling or extent of trivia MAY be preserved by source tooling. Such preservation does not make trivia program state or semantic identity.

Identifier-form token extent is determined only by `lexical.md`. Reserved-key classification under this document occurs after the complete maximal identifier-form token and its lexical identifier key have been determined. A longer identifier-form token is never split merely because an initial substring would be a reserved key.

Outside trivia, every source scalar participating in this represented grammar MUST belong to one identifier-form token under `lexical.md`, one decimal magnitude or decimal floating magnitude token defined below, or one represented punctuation token below. The `//`, `/*`, and `*/` sequences participate only in the ordinary-comment rules below. Other non-trivia material is malformed source under this concrete subset.

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
- `break`;
- `continue`;
- `unsafe`;
- `raw`;
- `import`;
- `export`;
- `true`;
- `false`;
- `Bool`;
- `I8`, `I16`, `I32`, `I64`;
- `U8`, `U16`, `U32`, `U64`;
- `F16`, `F32`, `F64`.

A **user identifier** is an identifier-form token under `lexical.md` whose lexical identifier key is not one of the reserved keys above.

A reserved key is not legal where the grammar requires a user identifier. This revision reserves no other identifier key and defines no escaping mechanism for a reserved key. In particular, `fast`, `move`, and `assign` are **not** reserved: an identifier-form token whose lexical identifier key is `fast`, `move`, or `assign` remains a user identifier except in the exact contextual positions defined by `NumericContractSelectedValue`, `RawMoveValue`, and `RawAssignStatement` below.

Reserved-key classification uses the lexical identifier key, not original source spelling. It does not change identifier formation, Unicode normalization, or identifier-key equality. In particular, longer identifier-form tokens such as `mutable`, `trueish`, `falsehood`, `ifonly`, `whiled`, `breakable`, `continued`, `unsafeish`, `rawness`, and `faulty` are each one complete identifier token and are not split because they begin with a reserved key.

## Punctuation tokens

The represented punctuation tokens are exactly:

```text
( ) { } : :: , -> - + * / = == ; . .. ! != ~ ^ | & && @
```

`->`, `::`, `==`, `!=`, `..`, and `&&` are each one punctuation token. Where more than one represented punctuation token could begin at one source position, the longest represented token is selected; consequently `::` is never tokenized as two `:` tokens, `->` is never tokenized as `-` followed by unrepresented `>` material, `==` is never tokenized as two `=` tokens, adjacent `!=` is never tokenized as `!` followed by `=`, adjacent `..` is never tokenized as two standalone `.` tokens, and adjacent `&&` is one token rather than two standalone `&` tokens. The ordinary-comment recognition rules below take priority over ordinary punctuation interpretation at the same source position, so this punctuation inventory does not decompose `//`, `/*`, or a matching `*/` into standalone punctuation tokens.

The existing `(` and `)` punctuation tokens additionally delimit the bounded grouped-value and numeric-contract-selected-value productions below. This adds no new delimiter spelling and does not change their existing parameter-list, direct-call, or argument-list roles. The same delimiters do not by themselves define Unit/empty-group values, tuples, parenthesized `Type`, indirect/grouped call targets, a general expression category, or another postfix system.

The standalone `@` punctuation token participates only in `NumericContractSelectedValue` below. Before this revision `@` was not represented punctuation and source containing it outside trivia was malformed, so introducing this one bounded role reinterprets no previously source-valid spelling. `@` does not introduce a generic attribute, annotation, decorator, pragma, compiler-directive, macro, metadata, or namespace mechanism. It does not reserve arbitrary following identifier keys. Only when the next non-trivia token after `@` is an identifier-form token whose lexical identifier key is `fast`, followed by the required parenthesized complete `Value`, does the selected-value production apply. Ordinary trivia may occur between these grammar tokens under the general trivia rule. Spellings such as `@name(...)`, `@standard(...)`, and `@reproducible(...)` are not represented selector forms in this revision.

The standalone `&` punctuation token participates only in the bounded safe-reference type/root/reborrow forms and as the address-selection punctuation inside the bounded `RawAddressOfValue` form `raw &x`. The Shared root branch admits one unqualified root identifier followed by zero or more existing `FieldSelector`s, so `&x` and `&x.field...` are bounded Shared root-reference forms. The Shared reborrow branch admits `*`, one unqualified parent safe-reference identifier, and zero or more existing `FieldSelector`s, so `&*r` is the zero-selector complete-referent child and `&*r.field...` is bounded Shared field-relative child reborrow. The reserved `mut` key following `&` selects the replacement-capable forms `&mut T`, `&mut x`, or `&mut *r` only in the exact grammar positions below; `&mut` is not a distinct punctuation token and both replacement-capable root and reborrow branches remain complete-root/complete-referent-only. The raw-address role is available only after the reserved `raw` key in its exact production and remains complete-root-only. `&` does not define binary bitwise AND, a general address-of operator, reference pattern/binding modes, lifetime syntax, an arbitrary borrow/address operand, plain-Exclusive syntax, or another prefix/infix operation. The distinct longest-match `&&` token remains Boolean short-circuit conjunction and is never decomposed into two root-borrow/reborrow/address tokens.

The standalone `-` punctuation token has exactly three represented grammar roles in this revision. First, at a value/prefix start, when `-` is followed across permitted ordinary trivia by `DecimalMagnitude` or `DecimalFloatingMagnitude`, the complete applicable `DecimalIntegerLiteral` or `DecimalFloatingLiteral` production has priority: the `-` is the existing negative literal sign and retains exactly the literal meaning owned by `literals.md`. Second, at a value/prefix start where neither signed-literal production applies, `-` introduces the bounded plain fixed-width integer-negation prefix form mapped to `operators.md`. Third, after one complete left `MultiplicativeValue` in the bounded additive tier, `-` is the bounded binary subtraction form. Under the exact surrounding required-type relation owned by `operators.md` and `function-execution.md`, that binary form maps to plain fixed-width integer subtraction for an admitted exact integer type or to same-format binary floating subtraction for exact `F16`, `F32`, or `F64`. For a floating subtraction occurrence, its numeric contract is established separately by the accepted semantic selection rule: an unqualified occurrence receives `standard` by fallback, while a valid operation-local `NumericContractSelectedValue` may establish `fast` for exactly that occurrence.

These roles do not define decrement, compound assignment, floating unary negation, unary plus, or another operator. `-1`, `- 1`, and `-1.0` therefore remain existing signed literals; `-(1)` and `-value` are integer-negation prefix forms; `-(1.0)` remains integer-negation prefix syntax and does not become floating unary negation; `--1` is an outer integer-negation prefix whose recursively parsed operand is the existing signed integer literal `-1`, not decrement or a new signed-literal form. `a - -1` is represented as binary subtraction whose right operand is the existing negative integer literal. Because `--` is not a punctuation token, adjacent `a--1` has the same represented token sequence and grammar meaning as `a - -1`; it is not decrement. `-=` is standalone `-` followed by standalone `=` and is not a represented operator form. Longest-match `->` remains one arrow token and is never reinterpreted as subtraction or negation followed by `>`.

The signed-literal priority is normative rather than an implementation lookahead convenience. A parser, typed frontend, optimizer, or lowerer MUST NOT rewrite a complete represented signed literal into an integer-negation operator before source validation. The distinction is source-observable for unsigned required types: for example, `-1` remains a negative literal that cannot materialize as `U8`, while `-(1)` may be a valid `U8` integer-negation operation whose result is governed by `operators.md`. The same priority preserves `-1.0` and trivia-equivalent signed decimal floating spellings as literals rather than a floating-negation operation.

The standalone `+` punctuation token participates only in the bounded additive productions below. Under the exact surrounding required-type relation owned by `operators.md` and `function-execution.md`, the same concrete `+` form maps to plain fixed-width integer addition for an admitted exact integer type or to same-format binary floating addition for exact `F16`, `F32`, or `F64`. For a floating occurrence, its numeric contract is established separately by the accepted semantic selection rule: an unqualified occurrence receives `standard` by fallback, while a valid operation-local `NumericContractSelectedValue` may establish `fast` for exactly that occurrence. Syntax itself performs no operand-derived inference, overload search, conversion, promotion, defaulting, generic arithmetic dispatch, or ambient contract selection. Standalone `+` is not unary plus, part of a numeric literal, compound assignment, increment, or another operator. Consequently `+1` is not a represented leading-plus literal or unary-plus value, `++` tokenizes as two standalone `+` tokens, and `+=` tokenizes as standalone `+` followed by standalone `=`; none is a represented value/operator form under this revision.

The standalone `*` punctuation token has exactly three bounded grammar roles. At a value/prefix start, `*` followed by one bare `UserIdentifier` forms `SafeDereferenceValue` below; exact source type determines whether that bounded complete-referent use is Shared duplicate behavior or replacement-capable duplicate/Move behavior under `references.md`, not parser dispatch. At statement start, `*` followed by one bare `UserIdentifier`, `=`, one complete `Value`, and `;` forms `ReferenceReplaceStatement` below; semantic admission requires an exact replacement-capable safe-reference operand. After one complete left `PrefixValue` in the bounded multiplicative tier, `*` is the existing multiplication operator; under the exact surrounding required-type relation owned by `operators.md` and `function-execution.md`, that binary form maps to plain fixed-width integer multiplication for an admitted exact integer type or to same-format binary floating multiplication for exact `F16`, `F32`, or `F64`. Grammar position and complete token shape distinguish these roles. Thus `*r * x` contains one safe-reference dereference as its left prefix operand followed by binary multiplication, while `*r = value;` is the distinct statement and `a * r` contains only binary multiplication. The bounded dereference/replacement forms do not admit `**r`, `*(r)`, a field/path operand, an arbitrary `Value`, raw-pointer dereference, wildcard/pattern syntax, exponentiation, or compound assignment. `*=` tokenizes as standalone `*` followed by standalone `=` and is not represented because the bounded replacement form requires an intervening `UserIdentifier`. The ordinary block-comment delimiters `/*` and matching `*/` are recognized under the comment rules below and are never decomposed into standalone `*` or `/` punctuation.

The standalone `/` punctuation token participates only in the bounded multiplicative division production below. Under the exact surrounding required-type relation owned by `operators.md` and `function-execution.md`, concrete `/` maps only to same-format binary floating division when the exact surrounding required type is `F16`, `F32`, or `F64`; every other required type, including every represented fixed-width integer type, is rejected before operand-side ownership may commit. A floating division occurrence receives `standard` by fallback unless a valid operation-local `NumericContractSelectedValue` establishes `fast` for exactly that occurrence. Syntax performs no operand-derived inference, integer division selection, mixed integer/floating arithmetic, promotion, conversion, coercion, defaulting, overload search, generic numeric dispatch, reciprocal transformation, or ambient contract selection. Standalone `/` is not remainder/modulo, path syntax, comment syntax after ordinary comment recognition has already selected a delimiter, or another operator. `/=` tokenizes as standalone `/` followed by standalone `=` and is not a represented compound-assignment form. The ordinary comment delimiters `//`, `/*`, and matching `*/` retain priority under the comment rules below and are not decomposed into division/multiplication punctuation.

The standalone `.` punctuation token participates only as the existing `FieldSelector` in four bounded contexts: represented `FieldValueUse`, the bounded `FieldAssignmentTarget`, the Shared root target after `&` in `&x.field...`, and the Shared reborrow target after `&*r` in `&*r.field...`. The decimal point inside one `DecimalFloatingMagnitude` is consumed as interior material of that single decimal token and is therefore not a `.` punctuation token. Adjacent `..` is the distinct longest-match punctuation token described next and is never two standalone field selectors. Reusing `FieldSelector` across these four bounded contexts does not make a field-value use a place, make a safe-reference value a field-value receiver, create a general postfix/member system, or create assignability outside the exact `FieldAssignmentTarget` grammar.

The `..` punctuation token participates only as `RecordPatternRest` in the bounded recursive record-pattern grammar below. It is node-local rest/omission syntax, not a field selector, range, spread/update, constructor-rest form, arbitrary wildcard, value, type, operator, module path, or general punctuation facility. Before this revision adjacent `..` tokenized as two standalone `.` punctuation tokens, but no represented grammar admitted two adjacent field selectors or another source-valid `..` spelling, so longest-match recognition of `..` reinterprets no previously source-valid source. Decimal floating tokenization remains unchanged: when token processing begins at a digit, the decimal-floating selection rule below consumes a qualifying digit-run `.` digit-run as one numeric token before ordinary punctuation tokenization can see that decimal point.

The standalone `=` punctuation token retains only its represented declaration/assignment roles, including the separators in bounded `ReferenceReplaceStatement` and `RawAssignStatement`. `==` instead participates only in the bounded Boolean equality productions below and maps those forms to the equality semantic relation in `operators.md`.

The standalone `!` punctuation token participates only in the represented Boolean logical-negation prefix productions below. `!=` instead participates only in the bounded Boolean inequality productions and maps those forms to the inequality semantic relation in `operators.md`. None of these punctuation tokens reserves an identifier key.

The standalone `~` punctuation token participates only in the represented plain fixed-width integer-bitwise-complement prefix productions below and maps those forms to the complement semantic relation in `operators.md`. It is not a reserved identifier key, binary bitwise operator, shift, pointer/reference form, destructor marker, pattern marker, type form, or another prefix/postfix operation. Because `~` was not a represented punctuation token before this revision, adding it reinterprets no previously source-valid spelling. `~~value` is two standalone `~` tokens and is represented as nested right-recursive complement; `~=` tokenizes as standalone `~` followed by standalone `=` and is not a represented operator form.

The standalone `^` punctuation token participates only in the bounded plain fixed-width integer-exclusive-or productions below and maps those forms to the exclusive-or semantic relation in `operators.md`. It is not exponentiation, binary AND/OR, a shift, pointer/reference syntax, pattern syntax, compound assignment, or another operator. Because `^` was not a represented punctuation token before this revision, adding it reinterprets no previously source-valid spelling. `^=` tokenizes as standalone `^` followed by standalone `=` and is not a represented operator form. `^^` tokenizes as two standalone `^` tokens and is not one distinct punctuation or operator token.

The standalone `|` punctuation token participates only in the bounded plain fixed-width integer-bitwise-OR productions below and maps those forms to the bitwise-OR relation in `operators.md`. It does not define Boolean disjunction, a closure or function-value delimiter, a pattern alternative, reference/borrow syntax, pipeline syntax, type syntax, compound assignment, or another operation. Because `|` was not a represented punctuation token before this revision, adding it reinterprets no previously source-valid spelling. `|=` tokenizes as standalone `|` followed by standalone `=` and is not a represented operator form. `||` tokenizes as two standalone `|` tokens and is not one punctuation token, Boolean-disjunction operator, or other represented operator form. `|||` likewise tokenizes as repeated standalone `|` tokens and introduces no distinct punctuation or operator token. Assigning this bounded role to standalone `|` neither claims nor denies any future closure/function-value or pattern-alternative spelling; any such future syntax requires its own accepted semantic and concrete-syntax authority.

The `&&` punctuation token participates only in the bounded Boolean short-circuit-conjunction productions below and maps those forms to the conjunction semantic relation in `operators.md`. Adjacent `&&` is selected by longest match before standalone `&`; consequently `&&=` tokenizes as `&&` followed by standalone `=`, and `&&&` tokenizes as `&&` followed by standalone `&`. Neither sequence is a represented operator/value form. Trivia-separated `& &` is two standalone `&` tokens and is not a represented safe-reference/raw-address form. `&mut` and `&*` use one standalone `&` followed respectively by reserved key `mut` or standalone `*`, while `&&` remains one distinct longest-match token. Adding bounded safe-reference and raw-address roles to standalone `&` does not define binary bitwise AND, closure syntax, compound assignment, or another operator, and it does not change the existing `&&` token or conjunction grammar.

Before `!=` was represented, adjacent `!=` was malformed as `!` followed by `=`; making the adjacent spelling one longest-match token therefore changes no previously source-valid spelling. Trivia separates tokens: `! =` remains a standalone `!` followed by standalone `=` and is malformed where no represented Boolean-not operand begins with `=`. Likewise `===` is `==` followed by `=` and `!==` is `!=` followed by `=` under longest-token selection; neither sequence is a represented operator form.

This revision defines no standalone `>` token and no other punctuation or operator token beyond the inventory above.

## Decimal numeric tokens

A **decimal magnitude token** is one non-empty maximal contiguous sequence of ASCII decimal digits `0` through `9`, except when the digit-start selection rule below extends that initial digit run into one decimal floating magnitude token.

A **decimal floating magnitude token** is one contiguous token with exactly this lexical shape:

```text
ASCII_DECIMAL_DIGIT+ "." ASCII_DECIMAL_DIGIT+
```

Its decimal point is token-internal. Trivia or comments cannot occur among the token's digits or on either side of that internal decimal point because doing so would split the token.

When token processing begins at an ASCII decimal digit outside trivia or a comment:

1. consume the maximal initial contiguous ASCII decimal digit run;
2. if that run is immediately followed by `.` and at least one ASCII decimal digit, consume the `.` and the maximal immediately following ASCII decimal digit run and emit one decimal floating magnitude token; otherwise
3. emit the initial run as one decimal magnitude token, leaving any later `.` to ordinary punctuation tokenization.

Only ASCII decimal digits participate in either decimal token form. Leading zeroes are preserved as concrete spelling and have no radix significance. A decimal floating magnitude also preserves trailing zeroes in its fractional digit run as concrete spelling.

Neither decimal token form contains a sign, suffix, digit separator, binary/octal/hexadecimal prefix, or exponent. A decimal magnitude contains no decimal point. A decimal floating magnitude contains exactly the one required internal decimal point and requires at least one digit on each side; `.5` and `1.` are therefore not decimal floating magnitude tokens.

The token forms establish only concrete decimal spelling. Their exact mathematical integer or decimal-rational meaning, required-type materialization, representability, and floating formation rules are owned by `literals.md`.

## Ordinary comments

A **line comment** begins with the two-scalar sequence `//` outside another comment and extends up to, but does not include, the next logical line boundary defined by `lexical.md`, or through the end of the source unit when no later logical line boundary exists.

A **block comment** begins with `/*` outside a line comment and ends at its matching `*/`. Block comments nest: each `/*` encountered inside a block comment increases the nesting depth, and each `*/` decreases it. The block comment ends when that depth returns to zero.

Comment recognition is prior to ordinary punctuation interpretation at the same source position. Therefore `//` begins the existing line comment, `/*` begins the existing block comment, and a matching `*/` closes the current block-comment nesting level; none of those delimiter sequences is decomposed into standalone `/` or `*` punctuation merely because both standalone tokens are now represented outside comments.

An unterminated block comment is malformed source.

Comment contents do not form identifiers, reserved keys, decimal magnitude tokens, decimal floating magnitude tokens, punctuation tokens, or grammar items. Comments have no Runen program semantics.

This revision defines no documentation-comment category or documentation semantics. Spellings such as `///`, `//!`, or `/**` are ordinary comments when they satisfy the rules above.

## Grammar notation

The productions below use quoted text for reserved keys or punctuation, `?` for an optional element, `*` for zero or more repetitions, and `|` for alternatives. `UserIdentifier` denotes one user identifier as defined above. `FastContextualKey`, `RawMoveContextualKey`, and `RawAssignContextualKey` each denote one identifier-form token whose lexical identifier key is exactly `fast`, `move`, and `assign`, respectively. Each is interpreted specially only in its exact grammar position below and remains a `UserIdentifier` elsewhere. `DecimalMagnitude` denotes one decimal magnitude token as defined above. `DecimalFloatingMagnitude` denotes one decimal floating magnitude token as defined above.

Trivia MAY occur around and between the tokens shown by these productions. Line boundaries have no statement-termination role. Semicolons are required exactly where a grammar production includes `;`; a represented `BlockStatement`, `UnsafeBlockStatement`, `IfStatement`, or `WhileStatement` terminates at its final closing `}` and has no trailing semicolon.

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

The optional `RecordDuplicabilitySelection` is record-specific. When present, it maps exactly to the positive nominal-record duplicability selection owned by `types.md`; when absent, the declaration makes no positive selection. The concrete `copy` key has no meaning as a general declaration modifier, trait/protocol/derive/attribute form, value-copy/clone operation, representation directive, ABI promise, or bitwise-copy instruction. Because `copy` belongs to the global reserved-key set above, the contextual-key mechanism remains limited to the explicitly bounded `fast`, `move`, and `assign` positions defined in this document.

Existing item-level `export` remains orthogonal to the record-specific selection and retains its existing position before `record`. Consequently `export record copy Name { ... }` is represented, while `copy record Name`, `export copy record Name`, and `record Name copy` are not represented forms.

Each `RecordField` maps its identifier and `Type` to the field identity/type/order relation in `types.md`. Without the field-position `ExportModifier`, the field has **module-private** direct accessibility; with it, the field has **exported** direct accessibility under `field-access.md`.

The record item's export class and each field's direct accessibility are independent. Exporting a record does not export any field, and exporting one field does not export the containing record or any sibling field. `field-access.md` owns the resulting cross-module direct-access rule and the source-accessibility requirement for the direct declared type of an exported field in an exported record.

A field-position `export` does not introduce a module declaration binding, ABI/linkage visibility, layout contract, synthetic getter/setter, or general modifier mechanism. The field remains one ordinary record field for nominal identity and structural order.

The field sequence MAY be empty. A trailing comma is permitted.

Presence or absence of `RecordDuplicabilitySelection` supplies only the positive/no-selection fact consumed by `types.md`. Eligibility, including recursive nominal-field eligibility and the zero-field case, is owned by that type-semantic relation and is not inferred from parser shape or field accessibility.

The represented record definition does not itself construct, access, destructure, duplicate, clone, or assign a value. Record construction, represented field-value access, represented record destructuring, and bounded binding-root field assignment are represented separately below. Methods, explicit copy/clone operations, and other duplicability-selection spellings are not represented.

A `SharedReferenceType`, `ReplacementReferenceType`, or `RawPointerType` is syntactically a `Type`, so the record-field grammar can represent spellings such as `field: &T`, `field: &mut T`, or `field: raw T`; `types.md`, `references.md`, and `raw-pointers-unsafe.md` reject safe-reference and raw-pointer record fields in this slice. Concrete representability therefore does not widen nominal record value shape or direct-containment semantics.

## Type forms

```text
Type                     = RawPointerType | ReplacementReferenceType | SharedReferenceType | ReferenceReferentType
RawPointerType           = "raw" ReferenceReferentType
ReplacementReferenceType = "&" "mut" ReferenceReferentType
SharedReferenceType      = "&" ReferenceReferentType
ReferenceReferentType    = IntrinsicType | UserIdentifier | QualifiedModuleMember
QualifiedModuleMember    = UserIdentifier "::" UserIdentifier

IntrinsicType = "Bool"
              | "I8"  | "I16" | "I32" | "I64"
              | "U8"  | "U16" | "U32" | "U64"
              | "F16" | "F32" | "F64"
```

Each intrinsic spelling maps one-to-one to the source type identity with the same specification label in `types.md`.

A `UserIdentifier` used as a `ReferenceReferentType` undergoes same-module lookup under `names-modules.md`. The resolved binding MUST denote a nominal record source type. Resolution does not skip a binding of another category merely because the type context requires a type.

A `QualifiedModuleMember` used as a `ReferenceReferentType` maps its first identifier to the source-unit module-alias key and its second identifier to the target-member key consumed by qualified cross-module lookup under `names-modules.md`. The resolved target binding MUST be exported and MUST denote a nominal record source type. Lookup does not bypass an inaccessible or wrong-category binding.

A `SharedReferenceType` maps to `SharedRef(T)` under `references.md`; a `ReplacementReferenceType` maps to `ExclusiveReplaceRef(T)` there; and a `RawPointerType` maps to `RawPtr(T)` under `raw-pointers-unsafe.md`. In each case the one `ReferenceReferentType` supplies `T`. All three constructors are syntactically nonrecursive: no safe-reference or raw-pointer type is a `ReferenceReferentType`, so spellings such as `&&T`, `&mut &T`, `&mut &mut T`, `raw raw T`, `raw &T`, `raw &mut T`, `&raw T`, or `&mut raw T` are not represented. Referent/pointee and contextual admission remain semantic requirements owned by `references.md`, `raw-pointers-unsafe.md`, and `types.md`.

This subset has no nested module path, type inference, type alias, generic application, plain-Exclusive reference type/spelling, lifetime spelling, tuple, array, vector, or other type form beyond the bounded raw-pointer, Shared-reference, and replacement-capable reference constructors above.

## Function definitions

```text
FunctionDefinition = "fn" UserIdentifier "(" Parameters? ")" ResultClause? Body
Parameters         = Parameter ("," Parameter)* ","?
Parameter          = UserIdentifier ":" Type
ResultClause       = "->" Type
```

A represented function definition maps to exactly one source function declaration/entity under `callables.md`, one callable signature, one body attachment under `function-execution.md`, and the corresponding parameter bindings under `local-bindings.md`. Its module accessibility is determined by the enclosing optional `ExportModifier` as described above.

Parameter source order maps directly to callable-signature parameter-slot order. Each concrete parameter identifier establishes the parameter binding corresponding to that slot. Every concrete parameter binding in this subset is immutable for assignment purposes. A source-valid `SharedReferenceType` or `ReplacementReferenceType` parameter remains one ordinary owned value parameter slot; no reference/pass-mode dimension, implicit reborrow, or alternate parameter grammar is introduced. Although the general `Type` production can syntactically place a `RawPointerType` in a parameter, `callables.md` rejects every raw-pointer parameter in this activation-local slice.

When `ResultClause` is present, the callable signature has one result value of that source type. When it is absent, the callable signature has no result value. A `SharedReferenceType` result uses the existing `-> &T` grammar and is source-valid only when `callables.md` derives one bounded safe-reference result contract from the ordered parameters under its deterministic parameter-structure rule. That contract is either identity-preserving or direct-child-bearing as defined there; concrete syntax does not select between them. A syntactically represented `-> &mut T` result is source-invalid because replacement-capable results are not admitted, and `-> raw T` is likewise source-invalid. The concrete form adds no lifetime annotation, result-contract selector, or explicit origin-selector syntax, and body implementation dataflow does not choose the contract or origin slot. Absence of a result clause does not introduce Unit, Void, or another source value.

The concrete function form attaches the following body to the same function entity introduced by the item. This revision defines no declaration-only, generic, unsafe-function, async, effect, placement, target, ABI, FFI, linkage, receiver, method, overload, or other function form. Lexical `unsafe` is a body-statement form below and does not qualify a function declaration.

## Function bodies

The represented body grammar delimits the function root lexical scope and admits recursively nested child lexical scopes through `BlockStatement`, `UnsafeBlockStatement`, represented conditional arms, and represented `while` bodies:

```text
Body           = "{" BodyStatement* ReturnStatement? "}"
BodyStatement  = LocalDeclaration
               | RecordDestructuringDeclaration
               | AssignmentStatement
               | ReferenceReplaceStatement
               | RawAssignStatement
               | CallStatement
               | FaultStatement
               | BreakStatement
               | ContinueStatement
               | BlockStatement
               | UnsafeBlockStatement
               | IfStatement
               | WhileStatement
BlockStatement = "{" BodyStatement* ReturnStatement? "}"
UnsafeBlockStatement = "unsafe" BlockStatement
```

`ReturnStatement` is not a `BodyStatement`. It appears only as the optional terminal element of the immediately containing root `Body` or nested `BlockStatement`. Because an `UnsafeBlockStatement` contains one ordinary `BlockStatement`, that contained block may itself have its optional terminal return; such a return terminates the current function activation. Consequently, concrete source cannot place another `BodyStatement` or second `ReturnStatement` after that return in the same contained lexical block.

`FaultStatement`, `BreakStatement`, and `ContinueStatement` are deliberately `BodyStatement` forms. Concrete grammar may therefore represent another `BodyStatement` or the optional terminal `ReturnStatement` after one of them; `function-execution.md` rejects any such later sibling semantically because the preceding statement has no local normal continuation. This deliberate asymmetry reuses the ordinary statement-sequencing rule instead of adding a generalized terminal-statement grammar category. `AssignmentStatement`, `ReferenceReplaceStatement`, `RawAssignStatement`, and a normally completing `UnsafeBlockStatement` retain local fallthrough under `function-execution.md`.

A represented `BlockStatement` is statement-only and produces no source value. Its closing `}` is the complete statement terminator; no trailing semicolon is present. Its `BodyStatement` sequence may be empty, its optional terminal return may be absent, and block statements may nest recursively because `BlockStatement` is itself a `BodyStatement`. `UnsafeBlockStatement` likewise has no trailing semicolon: its contained `BlockStatement` closing `}` ends the complete unsafe statement.

A terminal return inside a nested ordinary or unsafe block terminates the current source function activation under `function-execution.md`; it does not merely exit that block. A `fault;` reached inside either likewise terminates the current activation abnormally through the defined-fault relation. A source-valid `break;` or `continue;` reached inside a represented loop exits the active child lexical scopes required by its nearest enclosing `while` target under `function-execution.md` and `control-flow.md`, including any intervening unsafe block scope. The block forms themselves still do not create a block expression, tail value, Unit/Void value, label, or catch form. Conditional and bounded-loop selection are introduced only by `IfStatement` and `WhileStatement` below, loop transfer only by the explicit statements defined below, and unsafe admission only by `UnsafeBlockStatement`.

Each `BlockStatement` maps to exactly one child lexical scope under `local-bindings.md`. `UnsafeBlockStatement` adds the lexical unsafe-admission fact from `raw-pointers-unsafe.md` to that same contained child scope; it does not create a second hidden scope. Execution order, normal-continuation presence, normal child-scope cleanup, loop-transfer cleanup, return cleanup, explicit-fault/defined-fault cleanup and propagation, whole-binding and bounded field-assignment ordering, complete-referent replacement ordering, raw-operation ordering, safe-reference carrier/authority consequences, external-referent restoration, and divergence consequences are owned by `function-execution.md` and `references.md`. When a block is a conditional arm or represented `while` body, `control-flow.md` owns its relationship to the applicable selection, successor, backedge, and loop-transfer target/state rules.

## Conditional statements

The represented statement-level conditional has this grammar:

```text
IfStatement =
    "if" ConditionalValue BlockStatement ("else" BlockStatement)?

ConditionalValue = ConditionalLogicalAndValue
ConditionalLogicalAndValue =
    ConditionalEqualityValue ConditionalLogicalAndSuffix?
ConditionalLogicalAndSuffix = "&&" ConditionalEqualityValue
ConditionalEqualityValue =
    ConditionalOrValue (EqualityOperator ConditionalOrValue)?
ConditionalOrValue =
    ConditionalXorValue ConditionalOrSuffix?
ConditionalOrSuffix = "|" ConditionalXorValue
ConditionalXorValue =
    ConditionalAdditiveValue ConditionalXorSuffix?
ConditionalXorSuffix = "^" ConditionalAdditiveValue
ConditionalAdditiveValue =
    ConditionalMultiplicativeValue ConditionalAdditiveSuffix?
ConditionalAdditiveSuffix = AdditiveOperator ConditionalMultiplicativeValue
ConditionalMultiplicativeValue =
    ConditionalPrefixValue ConditionalMultiplicativeSuffix?
ConditionalMultiplicativeSuffix = MultiplicativeOperator ConditionalPrefixValue
MultiplicativeOperator = "*" | "/"
ConditionalPrefixValue =
    ConditionalBooleanNotValue
  | ConditionalIntegerNegValue
  | ConditionalIntegerComplementValue
  | ConditionalValueAtom
ConditionalBooleanNotValue = "!" ConditionalPrefixValue
ConditionalIntegerNegValue = "-" ConditionalPrefixValue
ConditionalIntegerComplementValue = "~" ConditionalPrefixValue
ConditionalValueAtom =
    BooleanLiteral
  | DecimalIntegerLiteral
  | DecimalFloatingLiteral
  | IdentifierUse
  | DirectCall
  | FieldValueUse
  | ConditionalGroupedValue
ConditionalGroupedValue = "(" ConditionalValue ")"
```

At every `ConditionalPrefixValue` decision point, the signed-literal priority from the punctuation/value rules applies before `ConditionalIntegerNegValue`: `-` followed across permitted trivia by `DecimalMagnitude` or `DecimalFloatingMagnitude` begins the existing signed literal atom; only another value-start `-` may begin `ConditionalIntegerNegValue`. The standalone `~` form has no literal role and therefore selects `ConditionalIntegerComplementValue` directly at a conditional prefix start.

`ConditionalValue` deliberately has its own logical-conjunction, equality, bitwise-OR, exclusive-or, additive, multiplicative, and prefix tiers rather than reusing unrestricted ordinary `LogicalAndValue`, `EqualityValue`, `OrValue`, `XorValue`, `AdditiveValue`, `MultiplicativeValue`, or `PrefixValue`. This preserves the accepted exclusion of a **standalone** `RecordConstruction` at every conditional conjunction operand, equality operand, bitwise-OR operand, exclusive-or operand, additive operand, multiplicative operand, and recursive prefix depth. It also deliberately does not add `NumericContractSelectedValue`, any direct safe-reference root/reborrow/dereference producer, `RawAddressOfValue`, or `RawMoveValue` as a conditional atom/prefix. Those bounded producers remain available in ordinary `Value` receiving positions; this narrower conditional grammar avoids adding a second condition-specific safe-reference/raw-pointer surface. The omission does not prohibit such an ordinary `Value` inside an otherwise admitted conditional atom's own receiving position, such as a direct-call argument, when the corresponding semantic type admission succeeds.

`ConditionalGroupedValue` is likewise context-preserving: its one inner value is another complete `ConditionalValue`, not unrestricted ordinary `Value`. Parentheses therefore add explicit tree nesting without resetting conditional grammar. The existing standalone-construction and direct-selector/safe-reference/raw exclusions remain true through every grouping depth and through any nested `&&`, `==`, `!=`, `|`, `^`, `!`, prefix `-`, `~`, `*`, `/`, `+`, or binary `-` reached inside a group. Delimiter-based widening of conditional syntax is not part of this grouping relation.

The exclusion is part of normative concrete grammar. The unqualified record-construction form begins with `UserIdentifier "{"`; admitting unrestricted ordinary `Value`, `LogicalAndValue`, `EqualityValue`, `OrValue`, `XorValue`, `AdditiveValue`, `MultiplicativeValue`, or `PrefixValue` immediately after `if` would make a spelling such as `if flag { ... }` collide with the record-construction token shape. A leading `!`, prefix `-`, or prefix `~` does not delimit its operand and therefore does not remove that ambiguity: `if !flag { ... }`, `if -flag { ... }`, and `if ~flag { ... }` must still parse `flag` as one conditional `IdentifierUse` and the following `{ ... }` as the then `BlockStatement`, not as a standalone construction operand. The multiplicative, additive, exclusive-or, bitwise-OR, equality, and conjunction tiers likewise do not reset either side to unrestricted ordinary syntax. `ConditionalBooleanNotValue`, `ConditionalIntegerNegValue`, and `ConditionalIntegerComplementValue` consequently recurse only through `ConditionalPrefixValue`, `ConditionalMultiplicativeValue` contains only conditional prefix operands, `ConditionalAdditiveValue` contains only conditional multiplicative operands, `ConditionalXorValue` contains only conditional additive operands, `ConditionalOrValue` contains only conditional exclusive-or operands, `ConditionalEqualityValue` contains only conditional bitwise-OR operands, and `ConditionalLogicalAndValue` contains only conditional equality operands.

The optional exclusive-or suffix requires a second `ConditionalAdditiveValue`, not an unrestricted ordinary value. The optional bitwise-OR suffix requires a second `ConditionalXorValue`, not an unrestricted ordinary value. The optional equality suffix requires a second `ConditionalOrValue`, not an unrestricted ordinary value. The optional conjunction suffix requires a second `ConditionalEqualityValue`, not an unrestricted ordinary value. Consequently the standalone-construction and direct raw/safe-reference/selector exclusions remain true on both sides of `^`, `|`, `&&`, `==`, or `!=`, including beneath any number or mixture of Boolean-not, integer-negation, or integer-complement prefixes, inside either operand of bounded multiplication/division or addition/subtraction, and inside any number of conditional groups. Forms such as `if flag == other { ... }`, `while !flag != !other { ... }`, `if ready && enabled { ... }`, and syntactically represented numeric-operator forms complete the conditional value before the following block opener.

`ConditionalBooleanNotValue` maps to the same Boolean logical-negation semantic relation in `operators.md` as the ordinary-value `BooleanNotValue` defined below. `ConditionalIntegerNegValue` maps to the same plain fixed-width integer-negation semantic relation as ordinary `IntegerNegValue`; `ConditionalIntegerComplementValue` maps to the same plain fixed-width integer-bitwise-complement semantic relation as ordinary `IntegerComplementValue`. Their concrete presence does not weaken the exact-`Bool` condition requirement. A `ConditionalMultiplicativeValue` containing `*` uses the same exact surrounding required-type selection boundary as the ordinary multiplicative tier: exact fixed-width integer `T` selects plain integer multiplication and exact `F16`/`F32`/`F64` selects same-format floating multiplication. A `ConditionalMultiplicativeValue` containing `/` maps only to same-format binary floating division when the exact surrounding required type is `F16`, `F32`, or `F64`; an integer or any other required type rejects the division form before operand ownership may commit. A `ConditionalAdditiveValue` containing `+` maps by the same exact surrounding required-type rule as the ordinary additive tier: exact fixed-width integer `T` selects plain integer addition and exact `F16`/`F32`/`F64` selects same-format floating addition. A `ConditionalAdditiveValue` containing binary `-` uses that same exact-type selection boundary: exact fixed-width integer `T` selects plain integer subtraction and exact `F16`/`F32`/`F64` selects same-format floating subtraction. Without an explicit selector, any floating multiplication, division, addition, or subtraction occurrence receives `standard` from the accepted fallback. A `ConditionalXorValue` containing `^` maps to the same plain fixed-width integer-exclusive-or relation as the ordinary exclusive-or tier. A `ConditionalOrValue` containing `|` maps to the same plain fixed-width integer-bitwise-OR relation as the ordinary bitwise-OR tier. `ConditionalEqualityValue` with `==` or `!=` maps to the same Boolean equality/inequality relations as the ordinary equality tier. `ConditionalLogicalAndValue` containing `&&` maps to the same Boolean short-circuit-conjunction relation as the ordinary conjunction tier. These concrete placements do not define condition-specific operators or type-driven parsing.

Repeated and mixed prefix forms such as `!!flag`, `! ! flag`, `--1`, `~~value`, `~-value`, `-~value`, `~!flag`, `-!flag`, or `!-value` are recursively represented according to the signed-literal priority and associate from the right by grammar nesting; syntactic representation does not make a type-invalid prefix composition source-valid. The multiplicative, additive, exclusive-or, bitwise-OR, equality, and conjunction tiers are not part of that prefix recursion. A group may explicitly contain a complete conditional conjunction tier, so `if !(a == b) { ... }`, `if (a == b) == c { ... }`, `if a == (b != c) { ... }`, `if (a ^ b) ^ c { ... }`, `if a ^ (b ^ c) { ... }`, `if (a | b) | c { ... }`, `if a | (b | c) { ... }`, `if (a && b) && c { ... }`, and `if a && (b && c) { ... }` remain represented according to the same operator semantics and exact-Bool condition requirement.

Exactly zero or one ungrouped multiplicative operator is represented at each conditional multiplicative level. Ungrouped `a * b * c`, `a * b / c`, `a / b * c`, `a / b / c`, and longer repeated or mixed multiplicative chains are therefore not represented. Existing context-preserving grouping can express `(a * b) * c`, `a * (b / c)`, `(a / b) * c`, or `a / (b / c)` without introducing multiplicative associativity or division reassociation.

Exactly zero or one ungrouped additive operator is represented at each conditional additive level. Ungrouped `a + b + c`, `a + b - c`, `a - b + c`, and `a - b - c` are therefore not represented. Existing context-preserving grouping can express explicit nested syntax such as `(a + b) - c`, `(a - b) + c`, `a - (b - c)`, or `a + (b - c)` without introducing additive associativity. The multiplicative tier is structurally tighter: `a + b * c` and `a + b / c` have the complete multiplicative operation as the right additive operand, while `a * b + c` and `a / b + c` have the complete multiplicative operation as the left additive operand. Grouping may explicitly override that nesting, as in `(a + b) * c`, `a * (b + c)`, `(a + b) / c`, or `a / (b + c)`.

Exactly zero or one ungrouped exclusive-or operator is represented at each conditional exclusive-or level. Ungrouped `a ^ b ^ c` and longer exclusive-or chains are therefore not represented. Existing context-preserving grouping can express `(a ^ b) ^ c` or `a ^ (b ^ c)` without introducing exclusive-or associativity. Additive is structurally tighter than exclusive-or, while bitwise OR is structurally looser: `a + b ^ c` has `a + b` as its left exclusive-or operand, and `a ^ b | c` has the complete exclusive-or as its left bitwise-OR operand.

Exactly zero or one ungrouped bitwise-OR operator is represented at each conditional bitwise-OR level. Ungrouped `a | b | c` and longer bitwise-OR chains are therefore not represented. Existing context-preserving grouping can express `(a | b) | c` or `a | (b | c)` without introducing bitwise-OR associativity. Exclusive-or is structurally tighter than bitwise OR, while equality is structurally looser: `a ^ b | c` has `a ^ b` as its left bitwise-OR operand, and `a | b == c` has the complete bitwise OR as its left equality operand.

Exactly zero or one ungrouped conjunction operator is represented at each conditional conjunction level. Ungrouped `a && b && c` and longer conjunction chains are therefore not represented. Existing context-preserving grouping can express `(a && b) && c` or `a && (b && c)` without introducing conjunction associativity. Equality is structurally tighter than conjunction: `a == b && c` has the complete equality as its left conjunction operand, while `a && b == c` has the complete equality as its right conjunction operand.

Decimal integer and decimal floating literals remain syntactically represented as conditional atoms. Exact condition and operator typing are owned by `control-flow.md`, `operators.md`, and `function-execution.md`. A bare numeric literal remains syntax-valid as a `ConditionalValue` but is source-invalid because it cannot produce the exact intrinsic `Bool` required by control flow. The same is true of a plain integer-negation, integer-complement, integer-multiplication, floating-multiplication, floating-division, integer-addition, floating-addition, integer-subtraction, floating-subtraction, integer-exclusive-or, or integer-bitwise-OR value used directly as a condition: the condition supplies required type `Bool`, while `operators.md` admits those numeric operations only under their exact integer or floating required-type domains, so the operation is rejected before its operand or operands may commit ownership. Likewise, a numeric operation appearing as an operand of exact-Bool equality or conjunction is syntactically represented but source-invalid because those Boolean operators supply required type `Bool` to their complete operand. A Boolean-negated numeric atom, such as `!-1`, or numeric equality such as `1 == 1`, may likewise be syntactically represented while failing the exact Bool operand rule owned by the operator semantics.

A `DirectCall` conditional atom retains both its represented unqualified and `alias::member(...)` target forms. A `FieldValueUse` may use either its binding-root or bounded producer-backed receiver grammar. All lookup, receiver-transient, operator-operand, grouping-transparency, and producer rules remain owned by their existing semantic owners. Because direct-call arguments are ordinary `Value`s, safe-reference roots/reborrows/dereferences, raw ownership move producing an ordinary parameter `T`, or operation-local selected floating multiplication, division, addition, or subtraction may occur inside such an argument when its parameter supplies the applicable exact required type. A raw-pointer value itself cannot cross the call boundary because `RawPtr(T)` parameters are invalid. Such a nested ordinary value does not make any excluded direct form a conditional atom.

A standalone `RecordConstruction` is not a represented conditional-value atom, including beneath any number or mixture of Boolean-not, integer-negation, and integer-complement prefixes, within either operand of a conditional multiplicative, additive, exclusive-or, or bitwise-OR operation, inside any number of conditional groups, on either side of a conditional equality operator, or on either side of a conditional conjunction operator. A `ProducerFieldValueUse` whose receiver is a `RecordConstruction` is instead one distinct admitted `FieldValueUse` and includes at least one mandatory `.` selector after the constructor's closing `}`. Consequently, forms such as `if Record { ready: true }.ready { ... }`, `if !Record { ready: true }.ready { ... }`, `if (Record { ready: true }.ready) { ... }`, `if flag == Record { ready: true }.ready { ... }`, and `if flag && Record { ready: true }.ready { ... }` remain unambiguous: the complete construction-backed field-value atom contains its mandatory selector, while the later then-arm block begins only after the complete `ConditionalValue`.

In contrast, `if (Record { ready: true }) { ... }`, `if ((Record { ready: true })) { ... }`, `if !(Record { ready: true }) { ... }`, `if -(Record { ready: true }) { ... }`, and `if ~(Record { ready: true }) { ... }` are not represented by this conditional grammar because the group recursively requires `ConditionalValue`; grouping does not opt into unrestricted ordinary construction syntax. A standalone construction likewise cannot be introduced merely as an operand of `^`, `|`, or `&&`. Direct safe-reference forms and direct `raw &x` / `raw move p` are excluded because none of those productions occurs in the conditional grammar.

The then arm is always one explicit `BlockStatement`. `else` is optional; when present it is followed by exactly one explicit `BlockStatement`. Each explicit arm therefore maps to one ordinary child lexical scope and may contain ordinary `BodyStatement` entries, including bounded field assignment, complete-referent replacement, unsafe blocks, raw replacement, `fault;` and, when nested in a represented `while`, `break;` or `continue;`, followed by its own optional terminal `ReturnStatement` only when the preceding body-statement sequence still has a local normal continuation. The omitted-else false outcome and definite normal-successor binding/external-referent structural state plus raw-pointer-origin composition are owned by `control-flow.md`; omission does not synthesize a concrete block or lexical scope.

This revision defines no direct `else if` production. A nested conditional may instead occur as a `BodyStatement` inside an explicit else block, for example the abstract shape `else { if ... { ... } }`.

An `IfStatement` produces no source value and has no trailing semicolon. It does not add a conditional expression, block value, Unit/Void value, pattern condition, guard, truthiness relation, ordering/numeric comparison, or short-circuit logical operator beyond the represented conjunction producer.

Because `ReturnStatement` is an optional terminal element of `BlockStatement`, a conditional arm may return from the current function. Return remains absent from `BodyStatement`, so this grammar still does not admit an arbitrary nonterminal return followed by more statements in that same arm block. Because `FaultStatement`, `BreakStatement`, and `ContinueStatement` are `BodyStatement` forms, the grammar may represent a following sibling after them; the no-local-normal-continuation rule rejects that sibling semantically.

Runtime condition selection, condition producer ordering, represented operator operand execution, grouped-value transparency, arm validation, local normal-continuation composition, normal arm cleanup, loop-transfer behavior, return behavior, explicit-fault behavior, safe-reference authority/carrier state, replacement-capable external-referent structural state, raw-pointer-origin state, other fault/divergence behavior, and exact normal-state equality whenever two normal outcomes meet are owned by `control-flow.md`, `function-execution.md`, `references.md`, `raw-pointers-unsafe.md`, and `operators.md` under their respective boundaries.

## While statements

The represented bounded statement-level loop has exactly this grammar:

```text
WhileStatement = "while" ConditionalValue BlockStatement
```

`WhileStatement` reuses the exact `ConditionalValue` nonterminal above. It therefore admits the same bounded Boolean short-circuit-conjunction, Boolean equality/inequality, bitwise-OR, exclusive-or, multiplicative, and additive tiers, recursively prefixed Boolean logical-negation, integer-negation, and integer-complement forms, contextual grouped values, and the same literal, identifier-use, direct-call, and bounded field-value atoms while preserving the same standalone-`RecordConstruction`, direct-`NumericContractSelectedValue`, direct-safe-reference, direct-`RawAddressOfValue`, and direct-`RawMoveValue` exclusions at every conditional conjunction operand, equality operand, bitwise-OR operand, exclusive-or operand, additive operand, multiplicative operand, prefix depth, and grouping depth. The grammar does not introduce a separate loop-condition expression category, truthiness rule, pattern condition, or semantic lookahead rule.

The loop body is exactly one ordinary `BlockStatement` and therefore one child lexical scope under `local-bindings.md`. `WhileStatement` is itself one `BodyStatement`, so loops may nest recursively and may appear inside represented conditional arms, unsafe blocks, or other blocks. The closing body `}` terminates the complete `WhileStatement`; no trailing semicolon is present.

A `WhileStatement` produces no source value. It has no `else` arm, result value, label, iteration binding, pattern, iterator protocol, unconditional-loop spelling, or do/while form. Its body may contain the bounded unlabeled `break;` and `continue;` statements defined below. `while true` is syntactically represented but remains subject to the conservative static false-outcome rule owned by `control-flow.md`.

Exact Bool condition admission, condition producer state effects, the pre-condition environment `H`, post-condition environment `C`, body validation from `C`, exact normal-backedge binding/external-referent structural state plus raw-pointer-origin equality with `H`, explicit break/continue target-state admission, the represented false normal successor `C`, no-local-normal-body behavior, dynamic repeated condition/body execution, and source-to-Core cyclic refinement are owned by `control-flow.md` and `function-execution.md`. Represented operator semantics within a condition remain owned by `operators.md` and add no second loop rule.

## Loop transfer statements

The represented bounded loop-transfer statements have exactly these forms:

```text
BreakStatement    = "break" ";"
ContinueStatement = "continue" ";"
```

Both are statement-only and produce no source value. They introduce no Unit/Void value, owned-value producer, operand, result, or expression category.

The concrete grammar admits either form wherever `BodyStatement` is admitted. Source validity requires the statement to be lexically nested in the body of at least one represented `while`; `control-flow.md` selects the nearest enclosing represented `while` as the transfer target and rejects an occurrence with no such target. This is semantic target validation, not context-sensitive keyword or grammar classification.

`break;` exits to that loop's represented post-loop continuation subject to the exact source target-state rule in `control-flow.md`; `continue;` transfers to that loop's condition point subject to its exact loop-head-state rule. Those target states include binding structural ownership, replacement-capable external-referent structural ownership, and raw-pointer origins where applicable. `function-execution.md` owns cleanup of every active child lexical scope exited by either transfer before control changes.

An inner represented `while` is the nearest target for transfers lexically inside its body. Ordinary blocks, unsafe blocks, and conditional arms do not establish transfer targets.

Because both forms are `BodyStatement`s, concrete grammar may represent another statement or terminal return later in the same immediate block. Such later syntax is source-invalid as unreachable because each transfer has no local normal continuation in that sequence.

This revision defines no label declaration/use, labeled transfer, transfer value, loop result, `break Value;`, `continue Value;`, alternate transfer key, or transfer to an outer loop while a nearer represented loop encloses the statement.

## Ordinary local declarations

```text
LocalDeclaration = "let" MutableModifier? UserIdentifier ":" Type "=" Value ";"
MutableModifier  = "mut"
```

The concrete form maps to one ordinary local declaration under `local-bindings.md`. The explicit type and initializer are mandatory in both forms.

Without `MutableModifier`, the declaration establishes an immutable binding. With it, the declaration establishes a mutable binding under the assignment-mutability classification owned by `local-bindings.md`. `mut` does not create a second declaration category, a reference/memory value, or a distinct storage identity. `local-bindings.md` and `references.md` reject a mutable ordinary local whose declared type is any safe-reference type; both Shared and replacement-capable reference locals are represented only as immutable ordinary locals. A `RawPointerType` local may be immutable or mutable under `local-bindings.md`; `mut` controls ordinary replacement/retargeting of the stored pointer value and grants no raw pointee-mutation authority.

Initializer lookup, owned-value production, carrier/authority transfer where applicable, resulting initial structural ownership state, exact raw-pointer origin/lexical target-validity requirement where applicable, and the point at which the new local enters scope are determined by `local-bindings.md`, `structural-ownership.md`, `references.md`, `raw-pointers-unsafe.md`, and `function-execution.md` under their respective boundaries.

This ordinary-local form has no uninitialized local, inferred local type, pattern binding, destructuring local, or mutable-parameter spelling.

## Recursive record destructuring with bounded rest

```text
RecordDestructuringDeclaration =
    "let" RecordPattern "=" RecordPatternScrutinee ";"
RecordPattern =
    RecordPatternHead "{" RecordPatternEntries? "}"
RecordPatternHead =
    UserIdentifier | QualifiedModuleMember
RecordPatternEntries =
    RecordPatternRest ","?
  | RecordPatternField ("," RecordPatternField)* ("," RecordPatternRest)? ","?
RecordPatternRest =
    ".."
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

Every `RecordPattern` begins with one explicit nominal record-pattern head. The head is either one unqualified `UserIdentifier` or the existing two-part `QualifiedModuleMember`. Each `RecordPatternField` maps its first identifier to one explicitly selected declared field key. Its target is either one bare binding leaf identifier or another explicit nested `RecordPattern`.

The field target is classified syntactically without semantic lookup: a bare `UserIdentifier` target is a binding leaf; `UserIdentifier "{"` begins an unqualified nested record pattern; and `UserIdentifier "::" UserIdentifier "{"` begins a qualified nested record pattern. No field type inference or lookup is needed to select among those concrete shapes.

A record-pattern node MAY have no entries, MAY contain only `RecordPatternRest`, or MAY contain one or more explicit `RecordPatternField` entries optionally followed by one `RecordPatternRest`. The grammar admits an optional trailing comma in every represented non-empty case. `RecordPatternRest` is therefore always the final non-trivia semantic entry of its node. Forms such as `R { .., a: x }`, `R { a: x, .., b: y }`, `R { .., .. }`, or more than one rest marker in one node are not represented. Rest may recur independently inside a nested `RecordPattern` because the complete production is recursive.

Pattern field presentation order is retained exactly for explicit fields. `RecordPatternRest` contributes no field target, binding leaf, binding key, or source-order item. `patterns.md` defines the resulting recursive field/rest structure, no-rest exhaustiveness, rest-authorized omission, depth-first explicit binding-leaf source order, exact type/accessibility requirements, and ownership behavior.

A nested record-pattern target introduces no binding merely for naming its record head. Only bare binding-leaf targets introduce function-local bindings under `local-bindings.md`. Qualification and rest are therefore never binding-leaf spellings.

An unqualified pattern head maps to the same-module record-declaration lookup owned by `patterns.md` and `names-modules.md`. A qualified `alias::Record` pattern head maps to the existing source-unit module-alias and exported qualified cross-module lookup relation in `names-modules.md`; the selected entity must be a nominal record. Function-local bindings do not participate in either pattern-head lookup.

A `DirectRecordPatternRoot` is exactly one bare unqualified `UserIdentifier`. In this declaration position it maps to the accepted direct binding-root pattern relation, not to ordinary `IdentifierUse` value production. The grammar does not insert an implicit whole-record value use or scrutinee transient. Pattern-head qualification and rest do not alter the direct-root scrutinee grammar.

A `ProducerBackedRecordPatternScrutinee` remains deliberately narrower than `Value`. It admits exactly one syntactically non-bare already-represented producer: a result-bearing `DirectCall`, a `RecordConstruction`, or a `FieldValueUse`. The top record-pattern head supplies the exact required nominal record type under `patterns.md` and `function-execution.md`.

The producer-backed alternatives reuse their existing concrete forms. A direct call may be unqualified or use the represented `alias::member(...)` target. A record construction may use its represented unqualified same-module target or qualified `alias::Record` target and remains named-field/exhaustive. Field-value use may be binding-rooted or use the bounded direct-call/record-construction producer receiver defined below. Pattern rest does not authorize constructor field omission, update, or spread.

A qualified construction may directly satisfy a qualified top record pattern when both qualified forms resolve to the same nominal record and their independent target/field-accessibility rules are source-valid. This is exact nominal producer typing, not a second pattern-scrutinee or construction category. Different record declarations remain unequal even when their fields are structurally equal.

When a producer-backed `FieldValueUse` is the scrutinee, its complete field-value operation ends before the resulting owned record enters the pattern-specific receiving relation. The field-receiver transient and the pattern scrutinee transient are therefore distinct sequential semantic objects under `field-access.md`, `patterns.md`, and `function-execution.md`.

The top scrutinee alternatives remain classifiable from their complete token shapes without semantic lookup. A direct root ends after its `UserIdentifier`; a direct call has call syntax; a record construction has constructor syntax; and a field-value use contains one or more `.` selectors after either its binding root or its complete bounded producer receiver. Scrutinee category does not depend on inferred type.

This recursive pattern form introduces no contextual safe-reference or raw-pointer production. It remains distinguished from `LocalDeclaration` after `let` by concrete token shape: optional `mut` followed by `UserIdentifier ":"` continues the ordinary-local form; `UserIdentifier "{"` begins an unqualified record pattern; and `UserIdentifier "::" UserIdentifier "{"` begins a qualified record pattern. This classification uses bounded token lookahead only and does not consult module lookup or source types. Because `raw` and `unsafe` are reserved keys, neither can be a pattern head or leaf identifier. The represented `!`, prefix or binary `-`, `~`, `*`, `/`, `+`, `^`, `|`, `&`, `==`, `!=`, `&&`, `@`, standalone `.`, and longest-match `..` punctuation do not otherwise participate in record-pattern-head classification.

Boolean, decimal integer, and decimal floating literals and represented operator/safe-reference/raw-pointer values are not producer-backed record-pattern scrutinees. A bare identifier is not admitted through the producer-backed alternative even though `IdentifierUse` is an ordinary `Value` producer elsewhere. No parenthesized/grouped value, numeric-contract-selected value, safe root formation, complete-referent dereference, explicit reborrow, raw address formation, raw ownership move, operator value, conversion, arbitrary postfix/member expression, qualified bare module member, or other general `Value` form is admitted as a record-pattern scrutinee.

Complete recursive pattern validation, explicit binding-leaf ordering, rest-authorized omission, producer evaluation ordering, transient structural ownership/cleanup, grouped binding establishment, and fault/divergence behavior are owned by `patterns.md`, `field-access.md`, `structural-ownership.md`, and `function-execution.md`.

Pattern-head qualification and concrete rest spelling are discharged by source validation. A faithful typed representation may retain the resolved top nominal record identity and complete explicit leaf paths/types/ownership facts plus the source-selected producer cleanup frontier without retaining qualified versus unqualified head spelling, separate nested-head qualification facts, the concrete rest marker, or omitted field identities.

This revision defines no `let mut Record { ... }`, shorthand field pattern, wildcard/ignore binding, tuple/array/enum pattern, literal/alternative/guard pattern, qualified binding leaf, qualified field name, nested module path beyond the represented alias/member pair, refutable pattern, destructuring assignment, reference-binding mode, raw-pointer-binding pattern mode, mutable pattern-binding modifier, range pattern, constructor spread/update/rest, or general spread syntax.

## Assignment statements

```text
AssignmentStatement             = WholeBindingAssignmentStatement | FieldAssignmentStatement
WholeBindingAssignmentStatement = UserIdentifier "=" Value ";"
FieldAssignmentStatement        = FieldAssignmentTarget "=" Value ";"
FieldAssignmentTarget           = UserIdentifier FieldSelector+
```

`WholeBindingAssignmentStatement` is exactly the previously represented zero-selector assignment form. Its target identifier is resolved using the unqualified function-body lookup precedence from `local-bindings.md`, and the concrete form maps that selected target and RHS `Value` to the existing whole-binding assignment relation owned there. Its grammar and semantics are unchanged by adding the non-empty field alternative.

`FieldAssignmentStatement` is the bounded non-empty alternative selected by its mandatory one-or-more `FieldSelector`s. The first identifier is one bare unqualified function-local root resolved under `local-bindings.md`; `field-access.md` resolves every selector from the root binding's declared exact type using the existing nominal field identity and direct-accessibility relation. The complete selector sequence yields one exact non-empty structural path `p` and final exact source type `T = type(p)`. The statement supplies `T` unchanged as the required type of its RHS `Value`. Target selection produces no field value and performs no ownership transition.

Both alternatives require the selected root binding to satisfy the assignment-mutability rule from `local-bindings.md`. Whole-binding assignment continues to consume its existing complete-root structural replacement relation. Bounded field assignment additionally consumes the exact-path Exclusive safe-authority compatibility and post-RHS non-empty subpath-installation relation owned by `references.md`, `structural-ownership.md`, `local-bindings.md`, and `function-execution.md`. No field-level mutability property is introduced.

Assignment is a statement and produces no source value. It does not introduce Unit/Void or participate in `Value` grammar.

RHS evaluation, source-first replacement ordering, fault/divergence consequences, and successful transfer are owned by `function-execution.md`. For the whole-binding alternative, its existing old-value cleanup, complete-root reset, and raw-pointer-origin installation where applicable remain unchanged. For the bounded field alternative, `function-execution.md` owns post-RHS target-state/Exclusive compatibility admission, then-current `frontier(p)` replacement cleanup, exact-`T` installation, and the successful consumed-path transition from `structural-ownership.md`.

A mutable raw-pointer local may therefore still be ordinarily assigned/retargeted only by the whole-binding alternative when the incoming exact `RawPtr(T)` value is valid. Safe-reference locals are immutable and cannot be rebound through either assignment alternative. The bounded field target is exactly a bare unqualified binding root followed by one-or-more static field selectors; it does not admit a qualified root, producer receiver, call, construction, grouped value, dereference/reference-relative target, raw pointer target, arbitrary postfix chain, or general place/lvalue grammar. This revision defines no compound assignment, assignment expression, assignment-as-value, or destructuring assignment.

## Complete-referent replacement statements

The represented bounded replacement through a replacement-capable safe reference has exactly this grammar:

```text
ReferenceReplaceStatement = "*" UserIdentifier "=" Value ";"
```

The one `UserIdentifier` after `*` is a safe-reference operand binding. It resolves through ordinary unqualified function-local lookup. Source validity requires its exact type to be `ExclusiveReplaceRef(T)` under `references.md`; a Shared-reference or raw-pointer binding does not become a replacement target merely because the concrete spelling uses `*`.

The complete RHS is one ordinary `Value` whose required type is exact referent type `T`. `references.md` and `function-execution.md` own source-first evaluation, the requirement that the destination carrier/authority remain live and retain full replacement-capable exclusive authority after RHS success, then-current referent remaining-frontier cleanup, exact-`T` installation, and complete structural reset. If the RHS faults or diverges, no outer referent replacement occurs.

`ReferenceReplaceStatement` is statement-only and requires its trailing semicolon. It does not consume the destination reference carrier, define a value result, expose a stale snapshotted destination when the RHS disables that carrier, admit Shared or plain-Exclusive replacement, admit field/subregion replacement, or establish a general dereference-place/lvalue grammar.

## Raw replacement statements

The represented unsafe raw-pointee replacement statement has exactly this grammar:

```text
RawAssignStatement     = "raw" RawAssignContextualKey UserIdentifier "=" Value ";"
RawAssignContextualKey = identifier-form token whose lexical identifier key is exactly "assign"
```

`RawAssignContextualKey` is contextual rather than reserved. Outside this exact position, the same maximal identifier-form token `assign` remains an ordinary `UserIdentifier` when otherwise admitted. The globally reserved `raw` key distinguishes this statement from ordinary whole-binding assignment and from any user binding named `raw`.

The one `UserIdentifier` after the contextual key is the raw-pointer operand binding, not the pointee binding name and not a general assignment place. It resolves through the ordinary unqualified function-local lookup relation and must denote an active binding of exact type `RawPtr(T)` under `raw-pointers-unsafe.md`. The complete RHS is one ordinary `Value` whose required type is the exact pointee `T`.

Concrete syntax admits this statement wherever `BodyStatement` is admitted, including outside an unsafe block. Source validity, however, requires an active lexical unsafe-admission region, exact pointer origin/target validity, source-first RHS evaluation, canonical Exclusive safe-authority compatibility at the post-source commit point, remaining-target-frontier cleanup, and complete-root replacement under `raw-pointers-unsafe.md`, `references.md`, and `function-execution.md`. The pointee target's ordinary binding `mut` classification is not a raw-replacement grammar or validity requirement.

`RawAssignStatement` is statement-only, produces no value, and requires its trailing semicolon. It does not define `raw assign` as a general compound/indirect assignment operator, field/path raw assignment, pointer retargeting, or a general place/lvalue category. The distinct `*r = Value;` spelling is owned by `ReferenceReplaceStatement` and is semantically invalid for a `RawPtr(T)` operand.

## Unsafe blocks

The represented lexical unsafe-admission statement has exactly this grammar:

```text
UnsafeBlockStatement = "unsafe" BlockStatement
```

`unsafe` is globally reserved. The following `BlockStatement` is the same ordinary child lexical block used elsewhere; the `unsafe` wrapper adds the admission fact owned by `raw-pointers-unsafe.md` and does not create another nested scope, block expression, value, callable qualifier, or effect annotation.

The complete statement ends at the contained block's closing `}` and has no trailing semicolon. It is admitted wherever `BodyStatement` is admitted and therefore may nest recursively. Nested unsafe blocks are syntactically represented but are semantically idempotent for unsafe admission under `raw-pointers-unsafe.md`.

An unsafe block does not make invalid raw-pointer or safe-authority preconditions valid, does not transfer obligations to callers, and does not make the enclosing function an unsafe callable. `function-execution.md` owns ordinary child-scope execution/cleanup, return/fault/loop-transfer behavior, and local-normal-continuation presence for the contained block.

## Direct calls

```text
DirectCall       = DirectCallTarget "(" Arguments? ")"
DirectCallTarget = UserIdentifier | QualifiedModuleMember
Arguments        = Value ("," Value)* ","?
```

An unqualified `UserIdentifier` call target maps to the direct-call relation owned by `function-execution.md` after its target identifier is resolved using the function-local lookup precedence from `local-bindings.md` and the same-module fallback from `names-modules.md`.

A qualified `alias::member` call target resolves only through the source-unit module-alias and qualified cross-module lookup relation in `names-modules.md`. Function-local bindings do not participate in that syntactically qualified lookup.

In either form, the resolved entity MUST be one source function entity with a represented source body. Lookup does not bypass a selected wrong-category or inaccessible binding merely because the call context requires a function.

Argument source order is the direct-call argument order consumed by `function-execution.md`. A trailing comma is permitted. Because arguments are ordinary `Value`s, an existing safe-reference binding, Shared root `&x` or `&x.field...`, complete-root replacement-capable `&mut x`, explicit Shared child `&*r` or `&*r.field...`, complete-referent replacement-capable child `&mut *r`, bounded `*r`, or unsafe `raw move p` may appear when the corresponding parameter type accepts the exact produced value. Safe references remain ordinary owned arguments: replacement-capable carrier use moves, callers retaining a parent use an explicit reborrow, and there is no implicit call-site reborrow. After left-to-right argument production, `references.md` and `function-execution.md` require each held safe-reference argument to have a fully available complete target and retain the complete capability promised by its exact type before callee entry. For a Shared field-root or field-relative child argument, that complete target is its exact selected structural region rather than an ancestor binding/reference region. `RawPtr(T)` itself is not parameter-admissible under `callables.md`, so `raw &x` or a raw-pointer `IdentifierUse` cannot make a raw-pointer call-transfer interface source-valid.

A result-bearing direct call uses the same `DirectCall` grammar regardless of whether its callable result is ordinary or the bounded contract-bearing `SharedReferenceType`. The callable signature, not syntax at the call site, determines the result type and advertised safe-reference result-contract variant and origin slot. `references.md` and `function-execution.md` own the exact identity or direct-parent/target relation, caller-side result provenance, normal restoration, and cleanup consequences. Replacement-capable and raw-pointer result types are rejected by `callables.md`; no call-site lifetime argument, result-contract selector, result-origin selector, implicit reborrow marker, raw-pointer escape selector, or alternate result syntax is introduced.

This subset has no indirect call, function-value call, method call, named argument, default argument, variadic argument, nested module path, or arbitrary member-call form. In particular, neither a grouped value nor a numeric-contract-selected value is a call target: `(f)(x)` and `@fast(f)(x)` are not represented merely because wrappers can occur in a receiving `Value` position.

## Call statements

```text
CallStatement = DirectCall ";"
```

A direct call used as a body statement is language-valid only when its resolved callable signature specifies no result value. A result-bearing direct call cannot be used as a statement under this grammar because this subset defines no arbitrary produced-value discard relation.

A valid no-result call statement produces no source value to discard. A `GroupedValue`, `NumericContractSelectedValue`, safe-reference producer, `RawAddressOfValue`, or `RawMoveValue` does not become a `CallStatement` or another body-statement starter merely because it can be an ordinary value producer.

## Explicit fault statements

```text
FaultStatement = "fault" ";"
```

This exact payload-free form maps to the explicit-fault execution relation owned by `function-execution.md` and selects its distinguished source-semantic defined-fault reason `ExplicitFault`.

`FaultStatement` is a statement only. It is not a `Value`, `ConditionalValue`, owned-value producer, expression, direct call, return value, declaration modifier, or general effect form. It contains no payload, message, numeric code, source value/type, site identity, exception object, or catch/matching surface.

The only represented spelling in this revision is exactly `fault;` modulo ordinary trivia. There is no `fault(...)`, `fault Value;`, `throw`, `panic`, alternate fault key, or escaping/contextual-keyword variant.

Because `fault` is globally reserved after complete maximal identifier formation, it cannot be used where `UserIdentifier` is required. Longer identifier-form tokens such as `faulty` remain ordinary user identifiers when they otherwise satisfy `lexical.md`.

`function-execution.md` owns the statement's no-local-normal-continuation classification, active-scope/parameter cleanup, same-fault propagation, and source-to-Core refinement. This grammar introduces no Core operation or implementation fault representation.

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

Complete static construction validation precedes initializer producer-state consequences. Before any initializer `Value` may commit a producer-state transition, validation MUST establish the target lookup/accessibility/category, exact nominal result type, every initializer field identity, direct field accessibility for every known initializer field, duplicate status, exhaustive field coverage, and exact surrounding required-type equality when the receiving position supplies one. A rejection of any such structural fact commits no speculative initializer producer state.

The resulting record value has exactly the declaration-defined field/value shape from `types.md`, independent of initializer source order. Evaluation, transient ownership, safe-reference authority/carrier consequences within nested producers, defined-fault cleanup, divergence, and final ownership transfer into the selected fields are owned by `function-execution.md` and the producer owners and are unchanged by target qualification. Qualification introduces no runtime module-loading, ABI, linkage, layout, or physical-symbol effect.

Because each initializer contains a `Value`, record construction composes recursively with another record construction as well as the other represented value producers and wrappers, including represented operators, bounded safe-reference producers, bounded raw-pointer producers, bounded contextual grouping, operation-local numeric-contract selection, and bounded producer-backed field-value use. Record fields themselves remain unable to have safe-reference or raw-pointer type under `types.md`, `references.md`, and `raw-pointers-unsafe.md`, so those producers cannot successfully initialize such a record field in this slice.

A complete source-valid record construction may itself be the receiver of one or more field selectors under `FieldValueUse` below. The mandatory selector distinguishes that composite field-value producer from the bare construction producer. The construction target may be either represented target form; a qualified construction does not create a new receiver category.

After source validation, target qualification has no independent semantic identity needed by lower execution. A faithful typed representation may retain the resolved nominal record identity, resolved initializer field identities/types, produced values, and source location without retaining whether the source target was qualified. No Core/module visibility metadata or runtime access check follows from this syntax.

This form defines no inferred or anonymous constructor target, positional field list, field-init shorthand, default field value, update/spread/base syntax, field assignment, partial-field reinitialization, constructor/method body, public-constructor flag, or positive duplicability selection.

## Field-value access

The represented field-value forms have this grammar:

```text
FieldValueUse         = BindingFieldValueUse | ProducerFieldValueUse
BindingFieldValueUse  = UserIdentifier FieldSelector+
ProducerFieldValueUse = FieldReceiverProducer FieldSelector+
FieldReceiverProducer = DirectCall | RecordConstruction
FieldSelector         = "." UserIdentifier
```

A `BindingFieldValueUse` root maps to the unqualified function-body lookup precedence owned by `local-bindings.md`. A `ProducerFieldValueUse` receiver is exactly one complete existing `DirectCall` or `RecordConstruction`. In both cases, the sequence of `FieldSelector` entries supplies the lexical field keys consumed by `field-access.md` in source order.

At least one selector is required after either receiver category. Consequently:

- a bare `UserIdentifier` remains `IdentifierUse` in an ordinary `Value` position and remains the distinct direct binding-root scrutinee in a `RecordDestructuringDeclaration`;
- a bare `DirectCall` remains the existing direct-call producer; and
- a bare `RecordConstruction` remains the existing construction producer.

A producer-backed field receiver does not admit an arbitrary `Value`. Boolean, decimal integer, and decimal floating literals, represented operator values, a bare `IdentifierUse` as an expression receiver, parenthesized/grouped values, numeric-contract-selected values, safe-reference values/dereferences/reborrows, raw address/move values, general expressions, methods, places, or another universal postfix receiver category are not represented. A qualified call is available only because `QualifiedModuleMember` is already one `DirectCallTarget`; a qualified construction is available only because that same bounded two-part form is now one `RecordConstructionTarget`. A qualified bare module member does not become a field receiver by itself.

A selector chain such as `make().outer.inner` is one `ProducerFieldValueUse` with one complete receiver producer and one static selector sequence. It does not recursively reinterpret each intermediate field result as a new arbitrary expression receiver.

Because direct-call arguments and construction initializers contain `Value`, bounded producer-backed field-value uses may compose recursively inside those already represented positions, including inside a `GroupedValue` or `NumericContractSelectedValue`. This recursion does not make either wrapper, a safe-reference producer, or a raw-pointer producer a field receiver and does not create a general precedence or postfix hierarchy.

Exact receiver result-type selection, direct record-field accessibility at every selector step, selector-path resolution, binding-root final-path availability, canonical direct safe-authority compatibility, producer-receiver transient ownership, final-field duplicate-or-consume consequence, remaining-frontier selection, and resulting source type are owned by `field-access.md`, `references.md`, and `structural-ownership.md`. A qualified direct-call receiver may therefore yield a foreign exported record whose exported field is selected under that owner without making the field selector itself a qualified module lookup. A qualified record-construction receiver similarly resolves and validates its complete construction before the field-value operation consumes its produced record. This grammar does not duplicate those relations.

The same `.` selector spelling represents both duplicate and consume outcomes for field-value production. The `FieldSelector` nonterminal is also reused after the root identifier in the bounded `FieldAssignmentTarget`, after the root identifier in the bounded Shared root-reference target, and after the parent safe-reference identifier in bounded Shared field-relative reborrow. Those reuses select an assignment target or reference target under their corresponding owners rather than producing field values under this section.

The `.` punctuation token in these bounded selector roles has no decimal-literal, method, direct reference-relative value-access, raw-pointer-relative, general postfix/member, or other operator meaning. Its assignment-target role exists only through `FieldAssignmentTarget` and does not make `FieldValueUse` or another completed value generally assignable. The decimal point inside a `DecimalFloatingMagnitude` is instead part of that one decimal token and never reaches this punctuation production. The distinct longest-match `..` punctuation token never enters `FieldSelector` and has only its bounded record-pattern role above.

## Value forms

The ordinary represented value grammar has one recursive prefix tier above the bounded multiplicative tier, with a bounded additive tier, a bounded exclusive-or tier, a bounded bitwise-OR tier, the non-associative equality tier, one bounded logical-conjunction tier looser than equality, and bounded atom wrappers for contextual grouping and operation-local numeric-contract selection. Shared complete-root/binding-field root formation, replacement-capable complete-root formation, Shared complete/field-relative reborrow, and replacement-capable complete-referent reborrow are bounded atom producers; complete-referent dereference is one bounded nonrecursive prefix producer. Raw address formation and raw ownership move are bounded atom producers introduced by reserved key `raw`.

```text
Value                = LogicalAndValue
LogicalAndValue      = EqualityValue LogicalAndSuffix?
LogicalAndSuffix     = "&&" EqualityValue
EqualityValue        = OrValue EqualitySuffix?
EqualitySuffix       = EqualityOperator OrValue
EqualityOperator     = "==" | "!="
OrValue              = XorValue OrSuffix?
OrSuffix             = "|" XorValue
XorValue             = AdditiveValue XorSuffix?
XorSuffix            = "^" AdditiveValue
AdditiveValue        = MultiplicativeValue AdditiveSuffix?
AdditiveSuffix       = AdditiveOperator MultiplicativeValue
AdditiveOperator     = "+" | "-"
MultiplicativeValue  = PrefixValue MultiplicativeSuffix?
MultiplicativeSuffix = MultiplicativeOperator PrefixValue
PrefixValue          = SafeDereferenceValue | BooleanNotValue | IntegerNegValue | IntegerComplementValue | ValueAtom
SafeDereferenceValue = "*" UserIdentifier
BooleanNotValue      = "!" PrefixValue
IntegerNegValue      = "-" PrefixValue
IntegerComplementValue = "~" PrefixValue
ValueAtom            = Literal
                     | IdentifierUse
                     | DirectCall
                     | RecordConstruction
                     | FieldValueUse
                     | SafeReferenceValue
                     | RawAddressOfValue
                     | RawMoveValue
                     | GroupedValue
                     | NumericContractSelectedValue
SafeReferenceValue             = "&" SafeReferenceAfterAmpersand
SafeReferenceAfterAmpersand    = "mut" ReplacementReferenceAfterMut | SharedReferenceAfterAmpersand
SharedReferenceAfterAmpersand  = SharedReferenceReborrowTarget | SharedReferenceRootTarget
SharedReferenceReborrowTarget  = "*" UserIdentifier FieldSelector*
SharedReferenceRootTarget      = UserIdentifier FieldSelector*
ReplacementReferenceAfterMut   = "*" UserIdentifier | UserIdentifier
RawAddressOfValue              = "raw" "&" UserIdentifier
RawMoveValue                   = "raw" RawMoveContextualKey UserIdentifier
RawMoveContextualKey           = identifier-form token whose lexical identifier key is exactly "move"
GroupedValue                   = "(" Value ")"
NumericContractSelectedValue   = "@" FastContextualKey "(" Value ")"
Literal                        = BooleanLiteral | DecimalIntegerLiteral | DecimalFloatingLiteral
BooleanLiteral                 = "true" | "false"
DecimalIntegerLiteral          = "-"? DecimalMagnitude
DecimalFloatingLiteral         = "-"? DecimalFloatingMagnitude
IdentifierUse                  = UserIdentifier
```

`SafeReferenceValue` is classified entirely from concrete token shape after the leading standalone `&`: `&mut *r` and `&mut x` take the reserved-`mut` branch; otherwise `*` followed by one `UserIdentifier` and zero or more `FieldSelector`s takes the Shared reborrow branch, while a `UserIdentifier` followed by zero or more `FieldSelector`s takes the Shared root branch. Thus `&*r` is the zero-selector Shared reborrow, `&*r.field...` is bounded Shared field-relative reborrow, `&x` is the zero-selector Shared root, and `&x.field...` is the bounded Shared field-root form. The grammar remains disjoint despite `mut` being reserved and unavailable as `UserIdentifier`. Neither `&mut x.field...` nor `&mut *r.field...` is admitted: each replacement branch ends after its one bare `UserIdentifier`, and no generic postfix or field-value receiver rule continues a completed safe-reference value. No semantic type lookup is used to choose the syntactic branch.

`FastContextualKey` and `RawMoveContextualKey` are not reserved keys and these productions do not define families of arbitrary `@name(...)` or `raw name p` forms. Each recognizes exactly the required maximal identifier-form token in its one contextual position. The same `fast` or `move` token is an ordinary `UserIdentifier` everywhere else that `UserIdentifier` is admitted. `RawAssignContextualKey` is defined independently by the statement grammar above and likewise does not reserve `assign` globally.

The existing `XorValue = AdditiveValue XorSuffix?` production is unchanged. `OrValue` is one transparent outer wrapper around `XorValue` when `OrSuffix` is absent. Consequently every source-valid spelling containing no standalone `|` retains its previous tokenization, accepted grammar tree below the wrapper, operator nesting, and semantic mapping; the wrapper adds no new semantic node when its optional suffix is absent. XOR is structurally tighter than bitwise OR only because the bounded wrapper consumes complete `XorValue` operands; this revision establishes no family-wide bitwise precedence policy beyond this exact current grammar.

At every `PrefixValue` decision point, a standalone `-` followed across permitted ordinary trivia by `DecimalMagnitude` or `DecimalFloatingMagnitude` MUST be consumed by the complete applicable signed `Literal` production rather than `IntegerNegValue`. Only when neither signed-literal production applies may value-start `-` introduce `IntegerNegValue`. This priority resolves the deliberate shared punctuation without reclassifying signed literals as operators. Standalone `~` has no literal role and therefore introduces `IntegerComplementValue` directly at a prefix start. Standalone `*` at a prefix start selects `SafeDereferenceValue` only when followed by one bare `UserIdentifier`; binary multiplication remains only the `MultiplicativeSuffix` after a complete left prefix value. At atom start, standalone `&` selects the one `SafeReferenceValue` production and its disjoint post-`&` token branches above, while reserved key `raw` selects only `RawAddressOfValue` or `RawMoveValue`. No type-driven parser reinterpretation is used.

The non-wrapper `ValueAtom` alternatives consist of the pre-existing literal/identifier/call/construction/field-value producers, the bounded `SafeReferenceValue` family with Shared complete-root/field-root formation, Shared complete/field-relative reborrow, and the existing replacement-capable root/complete-referent reborrow forms, plus the bounded raw-address/raw-move producers. `GroupedValue` and `NumericContractSelectedValue` are wrappers around one complete existing `Value`; neither becomes another owned-value producer family, primary-expression semantic category, postfix base, place category, member-receiver system, or call target. `SafeDereferenceValue` is a separate bounded prefix producer rather than an atom wrapper. Grouping is semantically transparent. Numeric-contract selection qualifies one existing governed root operator according to `function-execution.md` and may be erased after that selected contract is retained on the operation.

The represented `SafeReferenceValue` forms map exactly as follows under `references.md`:

- `&x` or `&x.field...` requests fresh Shared root formation. The first operand is one bare unqualified `UserIdentifier`, followed only in this Shared root branch by zero or more existing `FieldSelector`s. Zero selectors selects the complete binding root; one or more selectors supply the exact bounded structural field path resolved and accessibility-checked under `references.md` and `field-access.md`. No qualified root, grouped value, dereference result, call result, construction, producer transient, or arbitrary `Value` may occupy this target position. The surrounding required source type supplies exact `SharedRef(T)` for the final selected root/field type.
- `&mut x` requests root replacement-capable formation. Its operand is one bare unqualified `UserIdentifier`; semantic validity additionally requires a complete fully available mutable ordinary local of exact admissible referent type with canonical Exclusive direct compatibility. Parameters and immutable/non-replaceable roots are rejected semantically. No `FieldSelector` is admitted in this branch. The surrounding required source type supplies exact `ExclusiveReplaceRef(T)`.
- `&*r` or `&*r.field...` requests a fresh Shared child of the parent safe-reference binding named by `r`. Zero selectors selects the parent's exact complete referent; one or more selectors supply the exact bounded relative structural field path resolved and accessibility-checked from the parent's referent type under `references.md` and `field-access.md`. The operation leaves the parent carrier in place, creates a fresh child authority/provenance targeting the exact parent-relative region, and is valid only when the parent can delegate Shared permission at that selected target and the surrounding required source type is the exact represented `SharedRef(T)` for the final selected type.
- `&mut *r` requests a fresh replacement-capable complete-referent child. Its final operand names the parent safe-reference binding. It leaves the parent carrier in place, creates a fresh child authority/provenance, and is valid only when the parent can delegate replacement-capable permission. No `FieldSelector` is admitted in this branch.

No safe-reference form permits permission strengthening, replacement-capable field/subregion child formation, implicit reborrow, plain-Exclusive child, arbitrary dereference operand, qualified target, producer/transient target, or a general address/borrow expression. Bounded Shared field-root formation remains an independent root formation, while bounded `&*r.field...` is explicitly a fresh child/reborrow relative to an existing parent authority.

`SafeDereferenceValue` maps exactly to bounded complete-referent dereference in `references.md`. Its operand is one bare unqualified `UserIdentifier` naming an active safe-reference parameter/local binding. For exact `SharedRef(T)` it retains Shared duplicate behavior. For exact `ExclusiveReplaceRef(T)`, `references.md` selects duplicate behavior when `T` is duplicable and complete-referent ownership Move when `T` is non-duplicable. The grammar is deliberately nonrecursive: `**r` and `*(r)` are not represented, while `(*r)` is ordinary grouping around one already complete `SafeDereferenceValue`. No direct reference-relative field selector or general dereference-expression category follows from this form.

`RawAddressOfValue` maps exactly to complete-root raw address formation in `raw-pointers-unsafe.md`. Its operand is one bare unqualified `UserIdentifier` naming an active parameter or ordinary local binding of a first-slice raw-pointee-admissible type. The surrounding required source type supplies exact `RawPtr(T)`. The spelling is deliberately `raw &x`; `&x` without `raw` retains safe Shared-root meaning. Raw address formation consumes canonical Shared direct compatibility and therefore remains incompatible with an overlapping active exclusive safe authority. This grammar does not admit `raw &x.field...`.

`RawMoveValue` maps exactly to unsafe raw ownership move in `raw-pointers-unsafe.md`. Its final operand is one bare unqualified `UserIdentifier` naming an active binding of exact type `RawPtr(T)`. Concrete grammar admits the form both inside and outside unsafe blocks; semantic validity requires active unsafe admission, exact target availability, and canonical Exclusive safe-authority compatibility. The grammar is nonrecursive and defines no `raw move (p)`, `raw move p.field`, `*p` raw dereference, arbitrary pointer-expression operand, or non-consuming raw load.

Because either wrapper contains a complete `Value`, wrapper recursion is structurally represented. Their semantic target rules differ. Ordinary `GroupedValue` is transparent to selector root discovery, so `@fast((a * b))` selects the same floating-multiplication root as `@fast(a * b)`, `@fast((a / b))` selects the same floating-division root as `@fast(a / b)`, `@fast((a + b))` selects the same floating-addition root as `@fast(a + b)`, and `@fast((a - b))` selects the same floating-subtraction root as `@fast(a - b)`. A `NumericContractSelectedValue` encountered while discovering the root for another selector is not transparent, so syntactically represented same-root stacked forms equivalent to `@fast(@fast(a * b))`, `@fast((@fast(a * b)))`, `@fast(@fast(a / b))`, `@fast((@fast(a / b)))`, `@fast(@fast(a + b))`, `@fast((@fast(a + b)))`, `@fast(@fast(a - b))`, and `@fast((@fast(a - b)))` are source-invalid under `function-execution.md` rather than establishing duplicate or overriding selections for one root occurrence. That opacity does not prohibit a selector on a distinct governed root nested as an operand of another governed operation. A safe-reference root/reborrow/dereference, raw address formation, or raw ownership move discovered as the selected wrapper's root is not a governed floating operation and therefore fails numeric-contract selector applicability before its producer-state effects commit.

Selectors on distinct governed occurrences compose without inheritance. `@fast(a * b) / c` places the selected inner value as the left atom of an unqualified outer `/`; when the required types make both governed floating operations, the inner FloatMul occurrence is `fast` and the outer FloatDiv occurrence independently receives `standard`. Conversely, `@fast((a / b) * c)` selects only the outer floating multiplication while the grouped inner unqualified FloatDiv independently receives `standard`. The same locality applies across multiplicative and additive operations: `@fast(a / b) + c` selects only the inner FloatDiv while an unqualified outer FloatAdd remains `standard`, and `@fast((a * b) + c)` selects only the outer FloatAdd while the unqualified inner FloatMul remains `standard`. Distinct nested selectors may validly select both roots, so the typed structure of `@fast(@fast(a / b) + c)` may retain one `fast` FloatDiv and one independently `fast` FloatAdd when both occurrences satisfy their exact floating required-type relations. These syntax trees preserve grouping, operation identity, producer-consumer relationships, and occurrence-local contracts; they do not themselves grant reassociation, contraction, reciprocal replacement, division reassociation, or fused-divide authority. The accepted multi-operation rules in the Core floating owner remain the sole authority for any result-changing transformation and for every `standard` or `reproducible` boundary.

Because the grouped inner form is a complete `Value`, grouping is recursive: `(((a)))` is represented. A group may contain the complete logical-conjunction, equality, bitwise-OR, exclusive-or, additive, or multiplicative tier and therefore supplies explicit tree nesting without changing the unparenthesized tiers. In particular, `!(a == b)` is logical negation whose prefix operand is one grouped equality value; `-(a + b)` is integer negation whose prefix operand is one grouped addition value when the surrounding required type selects integer addition; `~(a + b)` is integer complement whose prefix operand is one grouped addition value when the surrounding required type selects integer addition; `(a == b) == c` is an outer equality whose left bitwise-OR operand contains a grouped inner equality; and `a == (b != c)` is an outer equality whose right bitwise-OR operand contains a grouped inner inequality. For additive operators, `(a + b) - c`, `(a - b) + c`, `a - (b - c)`, and `a + (b - c)` explicitly nest one complete inner additive value through grouping; every concrete `+` or binary `-` selects its plain fixed-width integer or same-format floating semantic operation only from its exact required type. For multiplicative operators, `(a * b) * c`, `a * (b / c)`, `(a / b) * c`, and `a / (b / c)` explicitly nest repeated or mixed multiplicative structure; each concrete `*` independently selects plain fixed-width integer multiplication or same-format floating multiplication only from its exact required type, while each concrete `/` maps only to same-format floating division under exact floating required type. `(a + b) * c`, `a * (b + c)`, `(a + b) / c`, and `a / (b + c)` explicitly override the ordinary multiplicative-over-additive tree. For exclusive-or, `(a ^ b) ^ c` and `a ^ (b ^ c)` explicitly nest repeated exclusive-or. For bitwise OR, `(a | b) | c` and `a | (b | c)` explicitly nest repeated bitwise OR. For conjunction, `(a && b) && c` and `a && (b && c)` explicitly nest repeated conjunction that the ungrouped conjunction tier does not admit. Because safe-reference and raw forms are ordinary bounded producers, `(&x)`, `(&x.field)`, `(&mut x)`, `(&*r)`, `(&*r.field)`, `(&mut *r)`, `(*r)`, `(raw &x)`, and `(raw move p)` are ordinary grouping around one complete producer; grouping changes neither their target syntax nor their authority/unsafe admission and does not make the grouped form a field-value receiver or reference target.

`()` is not represented because `GroupedValue` requires one `Value`. `(a, b)` is not represented because the group contains exactly one `Value` and no comma-expression/tuple production. Numeric-contract selection likewise requires exactly one complete parenthesized `Value`; it does not define empty selection, tuples, selector blocks, or an unparenthesized selection extent. Neither wrapper adds a general parenthesized expression taxonomy, precedence number/table, associativity metadata, block expression, tuple, Unit/Void value, indirect call target, field receiver, pattern scrutinee, or assignment target.

`BooleanNotValue` maps to the Boolean logical-negation semantic relation in `operators.md`. `IntegerNegValue` maps to the plain fixed-width integer-negation semantic relation there. `IntegerComplementValue` maps to the plain fixed-width integer-bitwise-complement relation there. All three use right-recursive `PrefixValue` operands, so repeated or mixed operator prefix forms are unambiguous and right-associated by grammar nesting after signed-literal priority is applied. `SafeDereferenceValue` is intentionally different: it consumes exactly one bare identifier and does not recurse through `PrefixValue`. For example, `!!flag` is nested Boolean negation, `~~value` is nested integer complement, and `--1` is outer integer negation whose operand is the complete signed literal `-1`; there is no decrement token or semantic relation. Prefix recursion does not include `MultiplicativeValue`, `AdditiveValue`, `XorValue`, `OrValue`, `EqualityValue`, or `LogicalAndValue`; consequently prefix binds more tightly than multiplication/division, addition/subtraction, exclusive-or, bitwise OR, equality, and conjunction except where grouping explicitly nests a complete `Value`.

Syntactic prefix composition does not imply type validity. `-!flag` is structurally integer negation of Boolean negation, `!-value` is structurally Boolean negation of integer negation, `~!flag` is structurally integer complement of Boolean negation, `!~value` is structurally Boolean negation of integer complement, `~(1.0)` is structurally integer complement of a grouped floating literal, and `-(1.0)` is structurally integer negation of a grouped floating literal, but each must satisfy the exact surrounding and nested required-type relations in `operators.md` and `function-execution.md`. The same applies when a safe-reference or raw producer occurs as an ordinary atom beneath a represented operator: syntactic containment does not make `SharedRef(T)`, `ExclusiveReplaceRef(T)`, or `RawPtr(T)` numerical or Boolean, and an unsafe RawMove is valid only when both its own raw preconditions and the surrounding exact type relation succeed. This revision introduces no floating unary-negation/complement fallback, truthiness, implicit conversion, physical bit-pattern reinterpretation, pointer/reference numeric interpretation, or type-driven parser reinterpretation.

`!a * b` is one multiplicative value whose left operand is `!a`; `-a / b` is one multiplicative value whose left operand is `-a`; `~a * b` is one multiplicative value whose left operand is `~a`; `*r * b` is one multiplicative value whose left operand is bounded safe-reference dereference `*r`; `!a + b`, `-a + b`, and `~a + b` are additive values whose left multiplicative operand contains the prefix form; and `!a == b`, `-a == b`, or `~a == b` remain equality values whose left bitwise-OR operand contains an exclusive-or value containing an additive value containing that prefix form. The prefix grammar never reparses those ungrouped forms as a prefix operator around a looser operator tier.

`MultiplicativeValue` maps binary `*` to either the plain fixed-width integer-multiplication semantic relation or the distinct same-format binary floating-multiplication semantic relation in `operators.md`, selected solely by the exact surrounding required type. It maps `/` only to the distinct same-format binary floating-division semantic relation, which is admitted solely when the exact surrounding required type is `F16`, `F32`, or `F64`; no represented integer required type selects division. Each floating `*` or `/` occurrence retains exactly one numeric contract established by `function-execution.md`: unqualified source uses the accepted `standard` fallback, while a valid `NumericContractSelectedValue` establishes `fast` for exactly its selected root. Exactly zero or one `MultiplicativeSuffix` is represented at each multiplicative level, so ungrouped `a * b * c`, `a * b / c`, `a / b * c`, `a / b / c`, and longer repeated or mixed multiplicative chains are not represented. Explicitly grouped inner multiplicative values may participate as atoms of an outer multiplicative operation. The surrounding receiving position supplies the exact required type consumed by the selected binary semantics: exact fixed-width integer `T` selects IntegerMul only for binary `*`; exact `F16`, `F32`, or `F64` selects FloatMul for binary `*` and FloatDiv for `/`; no other required type is admitted. The separate `SafeDereferenceValue` prefix has already been selected syntactically before this binary tier and never arises from type-driven multiplication dispatch. Safe-reference/raw forms likewise remain independently selected producers and are never inferred from `*` or `/`. Syntax alone performs no operand type inference, integer division, integer/floating mixing, promotion, conversion, coercion, defaulting, overload selection, trait or generic arithmetic dispatch, result-type inference, reciprocal replacement, division reassociation, fused divide selection, pointer/reference dereference/arithmetic beyond the bounded safe-reference forms, or ambient contract selection.

`AdditiveValue` maps `+` to either the plain fixed-width integer-addition semantic relation or the same-format binary floating-addition semantic relation in `operators.md`, selected solely by the exact surrounding required type. It maps binary `-` to either the distinct plain fixed-width integer-subtraction semantic relation or the distinct same-format binary floating-subtraction semantic relation, likewise selected solely by the exact surrounding required type. Each floating `+` or binary `-` occurrence additionally retains exactly one numeric contract established by `function-execution.md`: unqualified source uses the accepted `standard` fallback, while a valid `NumericContractSelectedValue` establishes `fast` for exactly its selected root. The operands of either additive operation are complete `MultiplicativeValue`s. Exactly zero or one `AdditiveSuffix` is represented at each additive level, so ungrouped `a + b + c`, `a + b - c`, `a - b + c`, `a - b - c`, and longer mixed/repeated additive chains are not represented. The multiplicative tier is structurally tighter, so `a + b * c` and `a + b / c` contain the complete multiplicative operation as their right additive operand, while `a * b + c` and `a / b + c` contain the complete multiplicative operation as their left additive operand. Explicit grouping can override either relation. The surrounding receiving position supplies the exact required type consumed by the selected arithmetic semantics; syntax alone performs no operand type inference, promotion, conversion, defaulting, overload selection, trait dispatch, generic arithmetic dispatch, integer/floating mixed selection, pointer/reference arithmetic, or inherited numeric mode. For `+`, exact fixed-width integer `T` selects integer addition while exact `F16`, `F32`, or `F64` selects floating addition. For binary `-`, exact fixed-width integer `T` selects integer subtraction while exact `F16`, `F32`, or `F64` selects floating subtraction. No other required type is admitted by either binary additive relation.

`XorValue` maps `^` to the plain fixed-width integer-exclusive-or semantic relation in `operators.md`. Its operands are complete `AdditiveValue`s. Exactly zero or one `XorSuffix` is represented at each exclusive-or level, so ungrouped `a ^ b ^ c` and longer exclusive-or chains are not represented. Explicit grouping can represent `(a ^ b) ^ c` or `a ^ (b ^ c)` without introducing associativity. Additive is structurally tighter than exclusive-or: `a + b ^ c` contains `a + b` as its left exclusive-or operand, while `a ^ b + c` contains `b + c` as its right exclusive-or operand. Bitwise OR is structurally looser than exclusive-or. The surrounding receiving position supplies the exact required type consumed by exclusive-or semantics; syntax alone performs no operand type inference, promotion, conversion, defaulting, overload selection, physical bit-pattern reinterpretation, pointer/reference interpretation, or generic bitwise dispatch.

`OrValue` maps `|` to the plain fixed-width integer-bitwise-OR semantic relation in `operators.md`. Its operands are complete `XorValue`s. Exactly zero or one `OrSuffix` is represented at each bitwise-OR level, so ungrouped `a | b | c` and longer bitwise-OR chains are not represented. Explicit grouping can represent `(a | b) | c` or `a | (b | c)` without introducing associativity. Exclusive-or is structurally tighter than bitwise OR: `a ^ b | c` contains `a ^ b` as its left bitwise-OR operand, while `a | b ^ c` contains `b ^ c` as its right bitwise-OR operand. Equality is structurally looser than bitwise OR. The surrounding receiving position supplies the exact required type consumed by bitwise-OR semantics; syntax alone performs no operand type inference, promotion, conversion, defaulting, overload selection, physical bit-pattern reinterpretation, pointer/reference interpretation, generic bitwise dispatch, closure interpretation, or pattern-alternative interpretation.

`EqualityValue` maps `==` and `!=` to the Boolean equality/inequality semantic relations in `operators.md`. Its operands are complete `OrValue`s. Exactly zero or one `EqualitySuffix` is represented at each equality level, so ungrouped forms such as `a == b == c`, `a != b == c`, and `a == b != c` are not represented. Explicitly grouped inner equality values may participate as atoms of an outer equality as described above. This introduces explicit syntax-tree nesting, not equality associativity or a comparison-chain relation. Bitwise OR is structurally tighter than equality: `a | b == c` contains `a | b` as its left equality operand, while `a == b | c` contains `b | c` as its right equality operand. No raw-pointer or safe-reference equality is inferred from syntactic representability: the represented equality relation accepts exact Bool operands only.

`LogicalAndValue` maps `&&` to the Boolean short-circuit-conjunction semantic relation in `operators.md`. Its operands are complete `EqualityValue`s. Exactly zero or one `LogicalAndSuffix` is represented at each conjunction level, so ungrouped `a && b && c` and longer conjunction chains are not represented. Explicit grouping can represent `(a && b) && c` or `a && (b && c)` without introducing associativity. Equality is structurally tighter: `a == b && c` contains `a == b` as its left conjunction operand, while `a && b == c` contains `b == c` as its right conjunction operand. Syntax alone does not prune, speculate, or eagerly evaluate the right producer; short-circuit validation/execution and exact successful producer-state equality are owned by `function-execution.md`.

The conjunction tier is looser than the equality tier, the equality tier is looser than the bitwise-OR tier, the bitwise-OR tier is looser than the exclusive-or tier, the exclusive-or tier is looser than the additive tier, the additive tier is looser than the multiplicative tier, and the multiplicative tier is looser than the prefix tier. This structural grammar ordering is the only precedence relation introduced; it requires no precedence number/table or generic binary-expression taxonomy. Forms such as `a + b / c ^ d | e == f && g`, `a + b * c ^ d | e == f && g`, `a + b ^ c | d`, `a ^ b | c`, `a | b == c`, `a == b && c`, and `a && b == c` therefore each have one unambiguous syntax tree. Current operator typing may still reject such a tree: exact-Bool equality and conjunction require their complete operands to produce `Bool`, while plain integer negation/complement/multiplication/addition/subtraction/exclusive-or/bitwise OR and same-format floating multiplication/division/addition/subtraction are admitted only under their exact numeric required types. Concrete representability does not create numeric, pointer, or safe-reference equality; pointer/reference arithmetic; truthiness; type-driven parser dispatch; or contract inheritance.

In ordinary `Value` positions, an operator prefix operand may be any represented `PrefixValue`, including a standalone `RecordConstruction`, a `GroupedValue`, a `NumericContractSelectedValue`, a bounded safe-reference/raw atom, or the bounded `SafeDereferenceValue`. That concrete admission does not make a record/reference/raw-pointer value Boolean, integer, or floating and does not make every selected wrapper or unsafe raw operation source-valid. `operators.md` requires the logical-negation operand, both equality/inequality operands, and both conjunction operands to be exactly `Bool`; requires the integer-negation and integer-complement operand/result and both integer-multiplication/addition/subtraction/exclusive-or/bitwise-OR operands/result to have the exact surrounding represented fixed-width integer type; and requires floating-multiplication, floating-division, floating-addition, and floating-subtraction operands/results to have exact surrounding represented `F16`, `F32`, or `F64`. `references.md` separately owns safe-reference producer typing and authority/structural consequences, and `raw-pointers-unsafe.md` owns raw address/move typing and unsafe preconditions. `function-execution.md` owns transactional producer validation, eager/short-circuit sequencing, definite state, held-left lifetime, grouping transparency, and selector applicability. Consequently syntactically represented type-invalid combinations remain rejected at those semantic boundaries.

The represented literal forms map to `literals.md`. `true` and `false` denote the boolean literal forms owned there. A `DecimalIntegerLiteral` supplies its concrete sign and decimal magnitude to the exact mathematical-integer and required-type materialization relation owned there. A `DecimalFloatingLiteral` supplies its concrete sign and one contiguous decimal floating magnitude token to the exact decimal-rational and required-type floating materialization relation owned there. This grammar does not assign an integer or floating default type, abstract literal type, conversion, or arithmetic/bitwise semantics beyond the separately owned plain fixed-width integer-negation/bitwise-complement/multiplication/addition/subtraction/exclusive-or/bitwise-OR and same-format floating-multiplication/division/addition/subtraction operator relations. Numeric-contract selection does not alter literal materialization.

The optional `-` in either represented numeric literal remains part of that literal grammar and retains exactly its accepted literal semantics. Because the sign and following magnitude are distinct grammar tokens, ordinary trivia may occur between them under the general trivia rule above. No trivia may occur inside `DecimalFloatingMagnitude` because it is one token, including around its token-internal decimal point.

At value/prefix start, signed-literal formation has priority exactly when standalone `-` is followed across permitted ordinary trivia by `DecimalMagnitude` or `DecimalFloatingMagnitude`. Thus `-1`, `- 1`, `-1.0`, and `- 1.0` remain complete signed literals, not `IntegerNegValue`s and not floating-negation operations. Otherwise `-` may introduce `IntegerNegValue`, making forms such as `-(1)` and `-value` represented; `-(1.0)` therefore remains a syntactically represented integer-negation prefix around a grouped floating literal and does not acquire floating-negation semantics. `--1` applies this rule recursively: the outer `-` is not followed by a magnitude and therefore begins integer negation, while the inner `-1` is a complete signed literal. Standalone `~` never participates in literal formation; `~-1` is therefore integer complement whose recursively parsed operand is the complete signed literal `-1`. A spelling such as `!-1` is Boolean logical negation whose atom is the already represented signed decimal integer literal and is source-invalid because that operand cannot have type `Bool`; `(-1)` is represented only as grouping around that already complete signed literal. After a complete left multiplicative value, `a - -1` and adjacent `a--1` are one binary subtraction whose right multiplicative value contains the existing negative literal; neither spelling introduces decrement semantics.

This distinction MUST remain intact through source validation. Under required `U8`, for example, `-1` is rejected by literal representability while `-(1)` may validate as integer negation and produce the modulo result selected by `operators.md`. A parser, frontend, or optimizer may not normalize either spelling into the other based on host-language unary-minus behavior. The represented floating-subtraction operation does not weaken this boundary: it exists only at the binary additive `-` position after a complete left multiplicative value. The represented floating-multiplication, floating-division, exclusive-or, bitwise-OR, conjunction, selector wrappers, bounded safe-reference forms, and bounded raw-pointer forms likewise do not alter this signed-literal boundary.

An `IdentifierUse` maps to ordinary whole-binding owned-value use under `local-bindings.md`. Its identifier is resolved using the function-local lookup precedence owned there. In this subset, the selected entity MUST be a parameter or ordinary local binding whose complete structural root is fully available under `structural-ownership.md`; another selected entity category does not become a value merely because the context requires one. When the binding has `SharedRef(T)`, ordinary use duplicates a carrier for the same authority and preserves provenance. When it has `ExclusiveReplaceRef(T)`, ordinary use moves its non-copy carrier and consumes the reference binding root under the existing ownership relation. When it has raw-pointer type, ordinary use duplicates the pointer value and preserves its exact pointer-origin provenance; it does not access the pointee.

The bounded `SafeReferenceValue` forms and `SafeDereferenceValue`, plus `RawAddressOfValue` and `RawMoveValue`, use ordinary unqualified function-local lookup and require the selected entity to satisfy the exact category/type restrictions owned by `references.md` or `raw-pointers-unsafe.md`. For a Shared field-root, only the first root identifier participates in that binding lookup; for a Shared field-relative reborrow, only the parent safe-reference identifier participates. Later `FieldSelector`s in either branch resolve nominal field identities under `references.md`/`field-access.md` rather than performing another function-local or module lookup. `ReferenceReplaceStatement` and `RawAssignStatement` use the ordinary binding precedence for their operands as well. Lookup does not bypass a selected wrong-category module binding or invent a module/static target when no applicable local binding is available.

A `DirectCall` may be used as a `Value` only when its callable signature specifies one result value. The successful call result is the owned value produced by `function-execution.md`. When that result is the bounded contract-bearing `SharedReferenceType`, the same concrete call produces the Shared-reference result summarized by the callee's advertised safe-reference result contract. `SharedIdentity` preserves the designated incoming Shared authority and caller provenance; `SharedDirectChild` exposes the already-preserved Shared direct child of the designated replacement-capable authority under the exact caller-summary ordering owned by `references.md` and `function-execution.md`. The concrete call form itself creates no alternate result syntax or call-site reborrow. `ReplacementReferenceType` and `RawPointerType` results are semantically rejected and therefore no direct call produces either value in this slice. The same result-bearing concrete call form may also appear as the receiver of a `ProducerFieldValueUse` or in the dedicated producer-backed record-pattern scrutinee position when its result type satisfies those independently owned categories. Grouping or selecting a call for an ordinary receiving position does not create an indirect/grouped call target or widen either dedicated category; a selector whose root is the call is source-invalid because direct call is not a currently governed numeric operation.

A `RecordConstruction` maps to the exhaustive record-construction relation above and produces one owned nominal record value under `function-execution.md`. Its target may be unqualified or qualified as specified above; target qualification is discharged during source validation and does not add a new value category. Record construction is not a literal and does not add a general expression hierarchy. The same concrete construction form may also appear as the receiver of a `ProducerFieldValueUse` or in the dedicated producer-backed record-pattern scrutinee position. Grouping a construction as an ordinary value does not make the grouped form a receiver or pattern scrutinee; selecting a construction as a numeric-contract root is source-invalid.

A `FieldValueUse` maps to the bounded binding-root/producer-backed relation above and produces one owned value under `field-access.md` when source-valid. It does not create a general member, postfix, place, or expression hierarchy. The same concrete field-value form may also appear in the dedicated producer-backed record-pattern scrutinee position when its exact result type is the nominal record selected by the top pattern head, and in `ConditionalValue` when its exact result type is `Bool` under `control-flow.md` for either represented `if` or `while` selection. A grouped or selected field-value remains only a wrapped ordinary value and does not create a new receiver or scrutinee category; a selector whose discovered root is the field-value operation is source-invalid in this revision.

A `GroupedValue` maps to no distinct semantic value operation. Its one inner `Value` is validated and executed through the existing producer owner; `function-execution.md` defines required-type propagation, complete producer-state handling, fault/divergence, safe-reference and raw-pointer provenance preservation, and typed/lowering erasure boundaries. Parentheses do not make a bare qualified module member into an `IdentifierUse` value and do not make a grouped value into a Shared root or reborrow target.

A `NumericContractSelectedValue` likewise maps to no new owned-value producer. `function-execution.md` passes the required type through unchanged, locates one root governed operation through ordinary grouping only, validates that the current root is a same-format floating multiplication, division, addition, or subtraction selected respectively by concrete binary `*`, `/`, `+`, or `-` together with the exact floating required type, establishes `fast` for that one occurrence, and then uses that operation's existing execution relation. A selector wrapper encountered while discovering that one root remains opaque, rejecting same-root stacked selection without prohibiting independently selected nested governed roots. Safe-reference and raw-pointer producers are not governed roots and fail selector applicability before their producer-state effects may commit. The wrapper may be erased after typed validation only when the selected contract remains retained directly on that selected floating operation occurrence. It creates no block/function/module default, caller state, runtime mode, reciprocal optimization permission, division reassociation permission, or general annotation mechanism.

A qualified module member without a direct-call argument list, record-construction body, or record-pattern body is not an `IdentifierUse` value and is not a record-pattern scrutinee under this subset. Module aliases and module-level declarations do not become source values.

The represented `ReferenceReplaceStatement`, `RawAssignStatement`, `UnsafeBlockStatement`, `FaultStatement`, `BreakStatement`, and `ContinueStatement` forms are not admitted by `Value` or `ConditionalValue` and do not create produced-value categories.

This subset has no string, byte, character, aggregate, raw-pointer, or other additional literal form; no scientific-notation, hexadecimal/binary/octal floating form, explicit infinity/NaN literal, `.5`/`1.` floating shorthand, suffix, separator, alternate numeric radix, or leading-plus numeric form; no Unit/tuple/general expression grouping beyond the bounded one-value `GroupedValue`, context-preserving `ConditionalGroupedValue`, and operation-local `NumericContractSelectedValue`; no operator beyond bounded Boolean logical negation, Boolean short-circuit conjunction, plain fixed-width integer negation/bitwise complement/multiplication/addition/subtraction/exclusive-or/bitwise-OR, same-format binary floating multiplication/division/addition/subtraction, and exact-Bool equality/inequality; no source numeric-contract spelling beyond operation-local `fast`; no general unary or binary expression hierarchy beyond the bounded safe-reference dereference/root/reborrow forms, raw address/move atoms, operator prefix/multiplicative/additive/exclusive-or/bitwise-OR/equality/logical-conjunction tiers; no binary bitwise operation beyond represented exclusive-or and bitwise OR and no shift operator; no short-circuit logical operator beyond represented conjunction; no conversion; no arbitrary-receiver member/postfix access beyond the bounded field receiver categories above and the dedicated Shared root/reborrow target field-selector chains after `&`; no assignment expression; no block expression; no closure; and no other semantic value producer beyond the represented producers above.

## Returns and normal completion

```text
ReturnStatement = "return" Value? ";"
```

A `ReturnStatement` may be the optional terminal element of the root `Body` or of any nested `BlockStatement`, including the block contained by an `UnsafeBlockStatement`. It always returns from the current source function activation; there is no block-local return meaning.

In a result-bearing function, every represented path that reaches a `ReturnStatement` must use `return Value;`, and the returned value's type and ownership transfer are governed by `function-execution.md` and MUST satisfy the callable result type. Because grouping and numeric-contract selection are part of `Value`, `return (value);`, a valid `return @fast(a * b);`, a valid `return @fast(a / b);`, a valid `return @fast(a + b);`, and a valid `return @fast(a - b);` require no return-specific wrapper rule and have exactly the contained producer semantics plus the selected contract fact. For a contract-bearing Shared-reference result, the same `return Value;` grammar serves both represented result contracts. Under `SharedIdentity`, forms such as `return p;`, `return saved;`, or `return forward(p);` are source-valid only when `references.md` proves exact designated incoming Shared authority/target identity and provenance; a fresh root, including a fresh field-root, or any fresh child including a field-relative child cannot satisfy that identity contract. Under `SharedDirectChild`, the already represented zero-selector `return &*parent;` form, or identity-preserving transport/forwarding of that resulting child, may be source-valid only when `references.md` proves the result has the exact designated replacement-capable direct parent and complete target; a fresh root including a field-root, wrong parent, grandchild, or non-empty projected field-relative child cannot satisfy that contract. `return &mut local;` and `return &mut *parent;` are syntactically represented `Value`s but are source-invalid because replacement-capable results are not result-admissible. `return raw &x;` is likewise syntactically represented but raw-pointer results are invalid. An unsafe `return raw move p;` may return ordinary pointee value `T` when `T` is the admitted callable result type and all raw-move preconditions hold; the raw pointer itself does not escape. No return-specific lifetime, result-contract selector, origin-selector, reborrow selector, or raw escape syntax is introduced.

In a no-result function, every represented `ReturnStatement` must be `return;`. The root body may also omit a terminal return and complete normally at `}`.

`return;` is invalid in a result-bearing function. `return Value;` is invalid in a no-result function.

Every explicit normal return and normal no-result fallthrough additionally consumes the replacement-capable external-referent restoration requirement owned by `references.md` and `function-execution.md`: each incoming replacement-capable external referent must be fully available after return-value effects and before activation cleanup. Defined fault and divergence do not create a normal restoration obligation.

A result-bearing function is not required syntactically to end with one root `return Value;`. Instead, `function-execution.md` requires that no represented path reach the root closing boundary normally without a valid result-bearing return. A represented path may instead terminate abnormally through `fault;` and then needs no result value. A conditional whose two explicit arms both terminate the activation by return and/or explicit fault may therefore eliminate the root normal continuation without a redundant root return. A conditional whose local fallthrough is absent only because its paths perform loop transfers is meaningful only inside an enclosing `while` and does not by itself terminate the function activation. A represented `while`, including `while true`, always retains its statically represented false normal continuation under `control-flow.md` and therefore cannot by itself discharge the missing-result obligation.

This subset defines no tail-expression return, no return as a `BodyStatement`, and no arbitrary nonterminal return followed by another statement in the same lexical block.

## Unqualified lookup and category validation

Except for an **unqualified** `RecordConstruction` target and an **unqualified** `RecordPatternHead`, whose same-module record-declaration lookup is defined by their respective owners, represented unqualified function-body identifier forms first apply the function-local precedence defined by `local-bindings.md`. Only when no active parameter/local binding resolves the lexical key does lookup fall through to same-module lookup under `names-modules.md`.

After lookup selects an entity, the consuming syntactic context validates its category. Lookup MUST NOT skip the selected entity to find another binding of a context-preferred category.

Consequently, when a parameter/local binding has the same key as a module-level function, an unqualified direct-call spelling resolves to the local binding and is invalid as a direct call rather than bypassing it. For either assignment alternative, the root identifier follows the same function-local precedence and must select a represented parameter/local binding; the selected root is then validated for assignment mutability and canonical direct safe-authority compatibility as required by its operation, with raw-pointer incoming-origin requirements applying only to whole-binding raw-pointer replacement. The bounded field alternative additionally resolves its non-empty selector path under `field-access.md` and admits no module-level or qualified assignment root. A `BindingFieldValueUse` root and direct binding-root record-pattern scrutinee follow the same precedence and require a parameter/local binding under their owners. The root/parent identifier of the bounded `SafeReferenceValue` forms, plus `SafeDereferenceValue`, `ReferenceReplaceStatement`, `RawAddressOfValue`, `RawMoveValue`, and `RawAssignStatement`, likewise requires an applicable active parameter/local binding and does not reinterpret a same-module declaration as source-addressable storage. Shared field selectors after a root or reborrow parent identifier are nominal field selections, not further function-local lookups.

A `ProducerFieldValueUse` applies the lookup relation of its complete receiver producer: unqualified direct call uses ordinary function-body lookup, qualified direct call uses module-alias lookup, unqualified record construction uses same-module record lookup, and qualified record construction uses module-alias lookup. The later field selectors do not cause a second root/name lookup; they consume nominal field identities and per-field accessibility under `field-access.md`.

A producer-backed pattern scrutinee likewise applies the lookup relation of its concrete producer before the pattern consumes the produced value. When that producer is a `ProducerFieldValueUse`, its receiver lookup and complete field-value production occur before the resulting owned value enters the pattern transient relation. Pattern-introduced bindings are not yet in scope during any of those lookups.

Every unqualified record-construction target and unqualified recursive record-pattern head is an explicit same-module declaration lookup. Active parameter/locals of equal key do not participate in those head/target lookups, and the selected module binding must be a record declaration. Qualified record-construction targets and qualified record-pattern heads instead use only the represented source-unit module-alias lookup relation described below.

Imported modules are not searched by ordinary unqualified lookup. They participate in construction and pattern-head lookup only through explicit qualified `alias::Record` forms.

This rule does not introduce overload resolution or general separate type/value module namespaces.

## Qualified module lookup and category validation

A concrete `alias::member` form is explicitly qualified. Its first identifier is interpreted only as a source-unit module alias under `names-modules.md`; it does not perform function-local or same-module declaration lookup. Its second identifier is resolved only in the aliased target module's declaration namespace under the exported-binding requirement owned by `names-modules.md`.

After qualified lookup selects the target binding, the consuming `ReferenceReferentType`, direct-call, record-construction, or record-pattern-head context validates the entity category. `SharedReferenceType`, `ReplacementReferenceType`, and `RawPointerType` all consume `ReferenceReferentType`, so the same qualified nominal-record spelling may supply a safe referent or raw pointee when that constructor's own semantic admission permits it. Lookup MUST NOT skip a private or wrong-category target to search for another entity.

A parameter/local binding MAY have the same lexical key as a module alias because the two participate in distinct lookup domains. Such a local controls ordinary unqualified spelling but does not block syntactically qualified `alias::member`.

The two-part qualification syntax is reused only in the explicitly represented `ReferenceReferentType`, direct-call target, record-construction target, and record-pattern-head positions. It does not create arbitrary member access, nested module paths, associated-item lookup, methods, re-export behavior, a qualified assignment root, qualified binding leaves, qualified field names, a qualified safe-reference root/reborrow/dereference target, or a qualified raw address/move target. A qualified direct call may appear as a producer-backed record-pattern scrutinee or as the receiver of a `ProducerFieldValueUse`; a qualified record construction may likewise appear wherever the existing `RecordConstruction` producer category is admitted. Any resulting record value may then undergo ordinary `.` field selection through `field-access.md`. A qualified record-pattern head uses the same lookup relation only to select its nominal record and does not turn its field selectors or binding leaves into qualified module members.

## Deliberate boundaries

This revision does not define:

- string, byte, character, or other literal syntax beyond the represented boolean, signed decimal integer, and bounded decimal floating forms; decimal scientific notation; hexadecimal/binary/octal floating notation; `.5`/`1.` floating shorthand; explicit infinity or NaN spellings; literal suffixes or digit separators; alternate numeric radices; or a leading-plus numeric form;
- floating unary negation, unary plus, increment/decrement, another numeric unary operator beyond bounded plain fixed-width integer negation and integer bitwise complement, arithmetic beyond bounded plain fixed-width integer negation/multiplication/addition/subtraction and same-format binary floating multiplication/division/addition/subtraction, floating remainder or mixed/cross-format arithmetic, binary bitwise operations beyond bounded plain fixed-width integer exclusive-or and bitwise OR, shifts, equality/inequality for any source type other than the represented exact-Bool relations, ordering or other comparison, short-circuit logical operators beyond represented Boolean conjunction, compound-assignment, conversion/cast, source `standard`/`reproducible` selector spellings, block/function/module numeric-contract selection scopes or defaults, lexical/dynamic contract inheritance, caller-to-callee contract propagation, generic `@` annotations/attributes/pragmas, or other operator forms beyond represented Boolean negation/conjunction, plain fixed-width integer negation/bitwise complement/multiplication/addition/subtraction/exclusive-or/bitwise-OR, same-format floating multiplication/division/addition/subtraction with the bounded operation-local `fast` selector, and Boolean equality/inequality;
- Unit/empty-group, tuple/comma-expression, parenthesized-type, or general expression grammar beyond the bounded one-value contextual grouping and selected-value forms above; multiplicative/additive/exclusive-or/bitwise-OR/equality/logical-conjunction/comparison chaining; or a binary precedence/associativity hierarchy beyond the bounded multiplicative, additive, exclusive-or, bitwise-OR, non-associative equality, and bounded logical-conjunction tiers;
- assignment expressions, assignment-as-value, compound or destructuring assignment, arbitrary assignment targets, field assignment beyond the represented bare binding-root `FieldAssignmentTarget`, reference-relative field/subregion assignment, raw field/path assignment, or general place/lvalue syntax beyond represented whole-binding assignment, bounded binding-root field assignment, bounded complete-referent replacement `*r = Value;`, and the bounded raw-pointee replacement statement;
- uninitialized locals, type inference, mutable parameters, mutable safe-reference locals, or mutable record-pattern binding modifiers;
- conditional expressions, direct `else if`, unrestricted nonterminal-within-block return or arbitrary unreachable tails, additional loop forms (`loop`, `for`, do/while), loop `else`, labels or a label namespace, labeled `break`/`continue`, transfer values, loop values, refutable/literal/alternative/guard patterns, `match`, wildcard/ignore/shorthand patterns, range patterns, catch/recovery, or other control-transfer forms beyond represented statement-level `if`, bounded statement-level `while`, bounded unlabeled `break;`/`continue;`, terminal return, payload-free explicit `fault;`, and lexical unsafe blocks;
- record-pattern scrutinees beyond the represented bare direct binding root and dedicated `DirectCall`, `RecordConstruction`, and bounded `FieldValueUse` producer-backed forms; in particular no literal (including decimal floating), bare `IdentifierUse`-as-value, represented operator/safe-reference/raw-pointer value, grouping, numeric-contract-selected value, other operator expression, conversion, arbitrary postfix/member expression, or other general expression is admitted there;
- source-visible module identities, dependency locators, package paths, nested module paths beyond the represented alias/member pair, selective imports, glob imports, re-exports, implicit preludes, or transitive import lookup;
- inferred/anonymous, positional, shorthand, defaulted, update/spread/base, constructor-body, method-based, or partial record construction, nor a constructor namespace or separate public-constructor capability; record-pattern `..` does not authorize constructor rest, update, spread, or general spread syntax;
- arbitrary-receiver member/postfix access beyond the explicit binding-root/direct-call/record-construction field-value forms, the bounded binding-root field-assignment target, the bounded Shared root-target selector chain after `&`, and the bounded Shared reborrow-target selector chain after `&*r`; direct safe-reference-relative or raw-pointer-relative field value access; field accessibility beyond the represented module-private/exported direct relation; package/friend/protected accessibility; methods; properties; or associated-item lookup;
- qualified binding leaves or qualified field names inside record patterns;
- explicit copy/clone value operations, custom copy constructors, or duplicability-selection syntax beyond the record-specific `copy` selection;
- safe-reference semantics beyond bounded `&T`, `&mut T`, Shared root `&x`/`&x.field...`, complete-root replacement-capable `&mut x`, complete-referent `*r`, explicit Shared child `&*r`/`&*r.field...`, complete-referent replacement-capable child `&mut *r`, bounded `*r = Value;`, ordinary carrier transport, lexical carrier/authority lifetime, replacement-capable external referent state/restoration, and the bounded identity-preserving/direct-child Shared-reference result contract: no plain `Exclusive` source class/spelling, replacement-capable field/path root formation, replacement-capable field/subregion reborrow through an existing reference, producer/transient reference target, reference-containing record field/aggregate, replacement-capable result, projected/subregion or arbitrary-descendant Shared result contracts, multiple/explicit/static result origins, explicit result-contract or result-origin selector spelling, direct reference-relative field value access such as `*r.field`, reference pattern/binding mode, named lifetime/parameter/outlives syntax, non-lexical authority shortening, implicit call reborrow, reference-to-raw conversion, or general dereference place;
- source raw-pointer semantics beyond the bounded activation-local `raw T`, `raw &x`, contextual `raw move p`, contextual `raw assign p = Value;`, ordinary raw-pointer local duplication/retargeting, and lexical `unsafe` block described above: no raw-pointer parameter/result transfer, pointer-containing record field/aggregate, pointer-to-pointer or pointer-to-safe-reference type, safe reference to a raw-pointer value, null/fabricated/integer pointer, source `RawRead`/non-consuming owned raw load, field/path raw address, general raw dereference syntax, pointer arithmetic/offset/one-past rule, pointer equality/ordering/hash/identity observation, pointer/integer conversion, target-sized integer, physical address/layout/alignment/endian/representation/relocation/stability/pinning contract, heap/global/static raw storage, unsafe function/callable/call contract, caller proof obligation, user-written proof contract, or raw/reference conversion;
- indirect calls, function values, closures, or any closure/function-value role for standalone `|`;
- pattern alternatives or any pattern-alternative role for standalone `|` beyond the explicitly absent future refutable/alternative-pattern family above;
- Boolean `||`; adjacent `||` remains two standalone `|` tokens and is not a represented operator form;
- generics, traits, or coherence;
- const/static forms or a general constant-expression category;
- fault payload/message/code/site/value/type forms, `fault(...)`, `fault Value;`, panic/throw syntax, catch/recovery, backtrace syntax, or another fault spelling beyond the represented payload-free `fault;`;
- ABI, layout, FFI, or linkage forms;
- Exec or Model source forms;
- package or filesystem discovery;
- malformed-source recovery, syntax-tree structure, source-range representation, or diagnostic wording;
- source-to-Core lowering or backend behavior.

Those concerns require their own accepted semantic owners and concrete consumers before this grammar is extended.