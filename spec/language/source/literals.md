# Source Literal Semantics

Status: **provisional normative; incomplete**

This document owns the represented source semantics for boolean literals and signed decimal fixed-width integer literals: their semantic values, integer required-type materialization, representability/source-validity rule, and literal owned-value production.

It consumes the represented intrinsic source type identities and semantic value domains from [Source type foundation](types.md). [Source concrete syntax](concrete-syntax.md) owns the represented literal spellings, reserved-key roles, decimal token forms, and grammar. [Source function execution](function-execution.md) consumes the owned values produced here and owns the surrounding initialization, assignment, call-argument, and return transfer relations. This document does not redefine those owners.

Literal semantics are independent of parser representation, typed HIR, Core MIR constant representation, host numeric parsing, physical machine integers, backend behavior, and target ABI.

## Boolean literals

The represented boolean literal forms map to the two existing `Bool` semantic values:

- concrete `true` denotes boolean value true and has source type `Bool`;
- concrete `false` denotes boolean value false and has source type `Bool`.

A boolean literal therefore produces a value only of source type `Bool`. It does not define truthiness, integer conversion, numeric promotion, ordering, representation bits, or another boolean-like source type.

Whether the concrete spellings are reserved identifier keys and how their token extent is determined are owned by `concrete-syntax.md` and `lexical.md`.

## Decimal integer literal datum

A represented decimal integer literal denotes one exact mathematical integer before required-type materialization.

For the concrete decimal magnitude consisting of decimal digits `d_0 ... d_n`, let `M` be the unique non-negative mathematical integer represented by those base-10 digits. Leading zeroes do not alter the value and have no radix significance.

The represented unsigned-sign form denotes `M`.

The represented negative-sign form denotes `-M`.

Consequently, the concrete forms `0`, `00`, `-0`, and `-00` all denote the same mathematical integer zero. Integer semantics contain no distinct negative-zero value.

The mathematical integer denoted at this stage is a literal datum used only by the materialization relation below. It is not an additional source type, an owned source value before materialization, a runtime arbitrary-precision integer, or a conversion source value.

The concrete decimal grammar, including which sign and digit spellings are represented, is owned by `concrete-syntax.md`.

## Required-type materialization

A represented decimal integer literal becomes an owned source value only under one **required source type** supplied by its consuming source construct.

Let `Z` be the exact mathematical integer denoted by the literal and let `T` be that required source type.

Materialization is valid exactly when both conditions hold:

1. `T` is one of the represented fixed-width integer source types `I8`, `I16`, `I32`, `I64`, `U8`, `U16`, `U32`, or `U64`;
2. `Z` belongs to the complete semantic value domain of `T` defined by `types.md` and its consumed Core integer semantics.

When valid, materialization produces exactly one owned source value whose source type is `T` and whose semantic integer value is exactly `Z`.

When either condition fails, the literal cannot produce a value for that consuming position and the source is invalid.

In particular:

- every represented signed type accepts its complete negative-through-positive value domain, including its minimum and maximum values;
- every represented unsigned type accepts exactly its non-negative value domain;
- a negative nonzero literal therefore cannot materialize at an unsigned required type;
- integer literals cannot materialize at `Bool`, any represented floating type, or a nominal record type;
- the same decimal spelling may validly materialize at multiple distinct fixed-width integer types when each consuming position independently supplies that required type and the mathematical integer is representable there.

This materialization relation is not type inference, defaulting, subtyping, promotion, widening, narrowing, coercion, or conversion. The literal datum has no prior concrete source type that is transformed into `T`.

This revision defines no fallback when a future construct does not provide exactly one required concrete source type. Such a construct must establish its own accepted rule rather than inheriting an implicit default from literal syntax.

## Representability is source validity

Integer literal representability is a static source-validity condition.

An accepted decimal integer literal preserves its exact mathematical integer through materialization. An integer outside the required fixed-width type's semantic value domain is source-invalid.

Literal materialization does not:

- wrap modulo a fixed width;
- saturate or truncate;
- produce a checked-overflow classification;
- produce a runtime defined fault;
- invoke arithmetic overflow behavior;
- depend on debug/release configuration or backend overflow flags.

The plain, checked, wrapping, and saturating integer rules owned by Core govern applicable arithmetic operations, not source literal range validation.

The decimal magnitude may contain arbitrarily many digits. A compiler may determine non-representability incrementally against the required type and need not allocate an arbitrary-precision runtime integer. Host parser limits, host integer widths, and host conversion behavior have no semantic authority.

## Owned-value production

A source-valid boolean literal or successfully materialized decimal integer literal is an **owned value producer**.

Evaluating such a literal:

- yields exactly one owned source value established above;
- consumes no parameter or local binding;
- changes no binding availability;
- has no source-visible side effect;
- does not yield a defined fault;
- does not diverge;
- creates no source-visible storage identity, stored-value lifetime, or independently addressable temporary.

The produced intrinsic value is duplicable because duplicability of the represented intrinsic source types is owned by `types.md`. Literal formation does not create a separate ownership class.

Transfer of the produced value into a local, assignment target, direct-call argument position, or return result is owned by `function-execution.md` and does not alter the literal value defined here.

## Concrete sign and operator boundary

The represented negative decimal literal form is a literal form owned concretely by `concrete-syntax.md`. Its `-` token denotes the negative sign only inside that literal production.

This document does not define a general unary-negation operation, subtraction, arithmetic operator, operator overloading, precedence, associativity, grouping, or general expression grammar.

A later operator owner may add independently defined dynamic negation or subtraction only while preserving the accepted meaning of the represented decimal literal forms. It must not reinterpret an already represented signed decimal literal through host-language operator behavior.

## Floating and other literal boundary

This revision defines no source literal semantics for `F16`, `F32`, or `F64`.

A floating-literal owner must independently define at least its represented textual forms, exact text-to-semantic-binary conversion, rounding at literal formation, overflow and underflow, subnormal behavior, signed zero, any NaN or infinity forms, and interaction with applicable `standard`, `reproducible`, and `fast` numeric contracts before such literals are accepted.

This revision also defines no string, byte, character, aggregate, pointer, reference, function, type, or other literal category.

The absence of those literal forms does not narrow the semantic value domains of their existing source types or pre-authorize any future concrete spelling.

## Conversion, inference, and constant boundary

Literal materialization grants no general implicit conversion or coercion between represented source types.

This document introduces no:

- abstract or unbounded integer source type;
- compile-time-only numeric source type;
- default integer type;
- local or global type inference;
- numeric promotion relation;
- literal suffix semantics;
- const/static declaration or evaluation model;
- general constant-expression category.

A literal may be compile-time-known to an implementation without acquiring const/static source semantics.

## Implementation boundary

This document defines no parser, syntax-tree node, diagnostic wording, typed-HIR representation, Core constant-value representation, reference-machine storage form, lowering strategy, runtime encoding, or backend instruction selection.

A lower representation used to realize these semantics must preserve every represented fixed-width integer value required by valid source. Existing implementation limitations are not permission to narrow the source literal family or its value domains.