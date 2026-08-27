# Source Literal Semantics

Status: **provisional normative; incomplete**

This document owns the represented source semantics for boolean literals, signed decimal fixed-width integer literals, and context-typed decimal binary-floating literals: their semantic values, required-type materialization, integer representability/source-validity, floating formation rounding, and literal owned-value production.

It consumes the represented intrinsic source type identities and semantic value domains from [Source type foundation](types.md), including the exact `F16` / `F32` / `F64` binary-format parameters. For nonzero decimal floating materialization it consumes the semantic binary floating rounding relation from [Core floating-point semantics](../core/numerics/floating-point.md). [Source concrete syntax](concrete-syntax.md) owns the represented literal spellings, reserved-key roles, decimal token forms, and grammar. [Source function execution](function-execution.md) consumes the owned values produced here and owns their surrounding initialization, assignment, direct-call argument, record-construction-field, return, cleanup, and transfer execution. [Source control flow](control-flow.md) consumes literal producers admitted by its concrete `ConditionalValue` grammar and owns exact-`Bool` condition admission and conditional/loop selection. This document does not redefine those owners.

Literal semantics are independent of parser representation, typed HIR, Core MIR constant representation, host numeric parsing, physical machine integers or floating encodings, backend behavior, and target ABI.

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

## Integer required-type materialization

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

## Integer representability is source validity

Integer literal representability is a static source-validity condition.

An accepted decimal integer literal preserves its exact mathematical integer through materialization. An integer outside the required fixed-width type's semantic value domain is source-invalid.

Integer literal materialization does not:

- wrap modulo a fixed width;
- saturate or truncate;
- produce a checked-overflow classification;
- produce a runtime defined fault;
- invoke arithmetic overflow behavior;
- depend on debug/release configuration or backend overflow flags.

The plain, checked, wrapping, and saturating integer rules owned by Core govern applicable arithmetic operations, not source literal range validation.

The decimal magnitude may contain arbitrarily many digits. A compiler may determine non-representability incrementally against the required type and need not allocate an arbitrary-precision runtime integer. Host parser limits, host integer widths, and host conversion behavior have no semantic authority.

## Decimal floating literal datum

A represented decimal floating literal denotes one exact finite decimal rational before required-type materialization.

For one concrete `DecimalFloatingMagnitude` token with spelling `A.B`:

- `A` is the non-empty ASCII decimal digit sequence before the token's decimal point;
- `B` is the non-empty ASCII decimal digit sequence after the token's decimal point;
- let `D` be the unique non-negative mathematical integer denoted by concatenating `A` followed by `B`;
- let `k` be the number of digits in `B`;
- the unsigned-sign form denotes the exact rational real value `D / 10^k`;
- the represented negative-sign form denotes its additive inverse.

Leading zeroes in `A` and leading or trailing zeroes in `B` do not alter the exact rational value. They remain part of concrete spelling only.

The exact decimal rational denoted at this stage is a literal datum used only by the floating materialization relation below. It is not an additional source type, an owned source value before materialization, a runtime decimal or arbitrary-precision value, a conversion source value, or a physical decimal representation.

The concrete floating token extent, decimal point, and optional sign grammar are owned by `concrete-syntax.md`.

The floating magnitude may contain arbitrarily many digits. Host parser limits, host integer widths, host floating ranges, and host decimal-to-binary conversion behavior have no semantic authority. Any independently applicable whole-source admission limit does not become a literal-value or numeric-range rule.

## Floating required-type materialization

A represented decimal floating literal becomes an owned source value only under one **required source type** supplied by its consuming source construct.

Let `X` be the exact decimal rational denoted by the literal and let `T` be that required source type.

Materialization is valid only when `T` is exactly one of the represented floating source types `F16`, `F32`, or `F64`.

When the required type is `Bool`, one of the represented fixed-width integer types, or a nominal record type, the decimal floating literal cannot produce a value for that position and the source is invalid.

The represented decimal integer and decimal floating literal families remain distinct. In particular, an integer-looking `DecimalIntegerLiteral` such as `1` does not materialize as `F16`, `F32`, or `F64` merely because a consuming position requires a floating type; represented floating materialization requires a `DecimalFloatingLiteral` concrete form.

This relation introduces no abstract or unbounded floating source type, default float type, inference, suffix typing, subtyping, promotion, widening, narrowing, coercion, or conversion. The exact decimal rational datum has no prior concrete source type that is transformed into `T`.

This revision defines no fallback when a future construct does not provide exactly one required concrete source type. Such a construct must establish its own accepted rule rather than inheriting an implicit numeric default from literal syntax.

## Floating exact zero and sign

When the exact decimal rational datum is mathematical zero, materialization does not apply the nonzero binary floating rounding relation.

Instead:

- an unsigned-sign decimal floating literal produces semantic `+0` of the required floating type;
- a negative-sign decimal floating literal produces semantic `-0` of the required floating type.

Consequently `0.0`, `00.000`, and equivalent unsigned zero spellings produce `+0`, while `-0.0`, `-00.000`, and equivalent negative zero spellings produce `-0`.

This sign selection is part of decimal floating literal formation only. It defines neither a general unary-negation operation nor a physical floating sign bit.

## Floating nonzero formation and rounding

Let `X` be the nonzero exact finite rational real denoted by a source-valid decimal floating literal and let `T` be its required `F16`, `F32`, or `F64` source type.

`types.md` selects the exact binary floating format parameters for `T`. Literal materialization explicitly supplies `X` exactly once to the accepted binary floating rounding relation from Core `floating-point.md` for that format. The semantic result of that relation is exactly the produced source value of type `T`.

This imports the accepted semantic rounding relation, not a host conversion routine or physical IEEE representation.

Consequently:

- an exactly representable nonzero normal or subnormal datum materializes exactly;
- an interior nonrepresentable datum uses the accepted nearest/ties-to-even relation;
- a positive or negative nonzero datum below the minimum subnormal magnitude follows the accepted signed-zero boundary, including its exact halfway rule;
- rounding at the subnormal/normal boundary may produce the minimum normal value when the accepted relation requires it;
- a datum above the maximum finite magnitude follows the accepted maximum-finite/infinity boundary and its exact midpoint rule; and
- a finite decimal rational whose rounded result is `+∞` or `-∞` is source-valid and produces that semantic infinity.

Floating materialization therefore does **not** use the integer literal rule that every exact datum must lie in the finite target value domain. The accepted binary floating rounding relation supplies the floating result across underflow and overflow boundaries.

Floating literal materialization does not wrap, saturate, truncate, use host overflow/underflow, flush according to a backend mode, produce a checked-overflow classification, or produce a runtime defined fault.

## Floating numeric-contract boundary

Decimal floating literal formation has one deterministic semantic result and does not select or depend on source selection of `standard`, `reproducible`, or `fast`.

This literal owner consumes the accepted binary floating rounding relation directly for literal formation. Numeric-contract refinements or relaxations apply only where their canonical operation owners explicitly say they do. In particular, `fast` permissions for subnormal handling, reassociation, contraction, reduced precision, or other named operation behavior do not re-form, flush, or reinterpret an already materialized literal value.

The represented operation-local numeric-contract selected-value relation owned by `concrete-syntax.md`, `operators.md`, and `function-execution.md` preserves the materialized literal value defined here. A numeric-contract selector whose contained root producer is a literal is source-invalid; it does not re-form, flush, or reinterpret literal materialization. Backend fast-math settings, target floating modes, and host parsing are not additional semantic input.

## Floating infinity and NaN boundary

This revision defines no explicit source spelling for infinity.

Semantic `+∞` or `-∞` may nevertheless be produced by a finite decimal floating literal when the accepted upper-bound rounding relation yields that result. Such a result does not imply an `inf`/`infinity` token, reserved key, or additional literal family.

This revision defines no NaN literal form. Decimal floating literal data in this slice are exact finite real quantities, so materialization produces no NaN.

The absence of NaN literal syntax does not narrow the NaN value class of `F16`, `F32`, or `F64` and does not establish a singleton, canonical, signed, payload-bearing, quiet, or signaling NaN member. Those semantic member properties remain exactly as defined or deliberately left open by the Core floating owner.

## Owned-value production

A source-valid boolean literal, successfully materialized decimal integer literal, or successfully materialized decimal floating literal is an **owned value producer**.

Evaluating such a literal:

- yields exactly one owned source value established above;
- consumes no parameter or local binding;
- changes no binding availability;
- has no source-visible side effect;
- does not yield a defined fault;
- does not diverge;
- creates no source-visible storage identity, stored-value lifetime, or independently addressable temporary.

The produced intrinsic value is duplicable because duplicability of the represented intrinsic source types is owned by `types.md`. Literal formation does not create a separate ownership class.

Transfer of the produced value into a receiving position whose concrete grammar admits that literal is owned by the applicable source owner and does not alter the literal value defined here. Represented examples include local initialization, assignment RHS, direct-call arguments, record-construction fields, return results, and `ConditionalValue` positions.

Concrete syntax, not this semantic owner, determines which literal family is admitted in each represented receiving grammar. In particular, a decimal floating literal is included in the concrete `ConditionalValue` grammar, but `control-flow.md` rejects it semantically because a condition must produce exact source type `Bool`.

## Concrete sign and operator boundary

The represented negative decimal integer and decimal floating forms are literal forms owned concretely by `concrete-syntax.md`. Whenever the complete applicable signed-literal production is present, its standalone `-` token is the literal's negative sign and the signed-literal production takes priority over the separately represented integer-negation prefix form.

This literal owner does not define an arithmetic operator. Represented plain fixed-width integer negation and binary addition, subtraction, and multiplication are owned by `operators.md`. The integer-negation role for value-start `-` applies only when `concrete-syntax.md` does not consume that token as the sign of an applicable `DecimalIntegerLiteral` or `DecimalFloatingLiteral`. Represented binary subtraction likewise does not reinterpret an already represented signed decimal literal.

The distinction remains part of source validity and MUST NOT be normalized away before literal materialization and operator validation. For example, under required source type `U8`, concrete `-1` is the existing negative decimal integer literal and is source-invalid because its mathematical literal datum is not representable as `U8`, while concrete `-(1)` is a separately represented integer-negation operation whose unsigned operand may validly materialize as `U8` and whose result is governed by `operators.md`.

A parser, typed frontend, optimizer, lowerer, future operator owner, or host-language implementation MUST preserve the accepted meaning of a represented signed decimal literal rather than reinterpret it as application of an arithmetic operator or host-language unary negation.

## Other literal boundary

This revision defines no string, byte, character, aggregate, pointer, reference, function, type, NaN, explicit-infinity, scientific-notation, hexadecimal-floating, or other literal category beyond the represented boolean, decimal integer, and bounded decimal floating forms above.

The absence of those literal forms does not narrow the semantic value domains of their existing source types or pre-authorize any future concrete spelling.

## Conversion, inference, and constant boundary

Literal materialization grants no general implicit conversion or coercion between represented source types.

This document introduces no:

- abstract or unbounded integer or floating source type;
- compile-time-only numeric source type;
- default integer or floating type;
- local or global type inference;
- numeric promotion relation;
- literal suffix semantics;
- const/static declaration or evaluation model;
- general constant-expression category.

A literal may be compile-time-known to an implementation without acquiring const/static source semantics.

## Implementation boundary

This document defines no parser, syntax-tree node, diagnostic wording, typed-HIR representation, exact-decimal implementation algorithm, Core constant-value representation, reference-machine storage form, lowering strategy, runtime encoding, or backend instruction selection.

A lower representation used to realize these semantics must preserve every represented fixed-width integer value and every represented decimal floating result required by valid source. Existing implementation limitations, host parser capacity, or physical floating representation are not permission to narrow or alter the source literal families or their semantic values.
