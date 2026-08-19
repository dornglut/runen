# Core Integer Semantics

Status: **provisional normative; incomplete**

Fixed-width integer arithmetic MUST have language-defined semantics.

Signed overflow MUST NOT become undefined behavior merely because a backend uses machine integers, and debug or release mode MUST NOT change language meaning.

Checked, wrapping, and saturating operations are part of the intended arithmetic model.

## Plain fixed-width overflow

A **plain fixed-width integer arithmetic operation** is one whose applicable operation contract does not select an explicit checked, wrapping, saturating, or other overflow mode.

When such an operation is otherwise defined and its operation-specific semantics determine an exact mathematical integer result `x` for a fixed-width integer result type of width `N`, its Runen result is the value of that same result type obtained by reduction modulo `2^N`:

- for an unsigned result type, the result is the unique value in `[0, 2^N - 1]` congruent to `x` modulo `2^N`;
- for a signed result type, the result is the unique value in `[-2^(N-1), 2^(N-1) - 1]` congruent to `x` modulo `2^N`.

This rule applies whether or not `x` is already in range. An out-of-range exact result therefore does not by itself produce a defined fault and is not undefined behavior. A realization MUST preserve this result independently of host integer-overflow behavior, debug or release configuration, backend overflow metadata, or optimizer assumptions.

This is an overflow-result rule, not an operation-domain rule. It does not make an operation defined when another applicable contract supplies no exact mathematical integer result, and it does not define division-by-zero behavior, shift-count validity, conversions, source operator spelling, constant-evaluation diagnostics, or which integer operations or widths exist.

Selecting an explicit checked, wrapping, saturating, or other overflow mode is not plain arithmetic under this rule. Explicit checked, wrapping, and saturating overflow semantics are defined separately below; the source forms of all explicit modes remain to be defined by their applicable contracts.

The modular result above is a semantic value-domain rule. It does not require a physical integer representation, byte layout, ABI representation, or address-level interpretation.

## Explicit checked overflow

An **explicit checked fixed-width integer arithmetic operation** is one whose applicable operation contract selects checked overflow behavior.

When such an operation is otherwise defined and its operation-specific semantics determine an exact mathematical integer result `x` for a fixed-width integer result type of width `N`, let `[lo, hi]` be the mathematical value interval of that result type:

- for an unsigned result type, `lo = 0` and `hi = 2^N - 1`;
- for a signed result type, `lo = -2^(N-1)` and `hi = 2^(N-1) - 1`.

The operation's **checked arithmetic outcome** is exactly one of:

- `value(v)` when `lo <= x <= hi`, where `v` is destination integer value `x`;
- `overflow` when `x < lo` or `x > hi`.

The `overflow` outcome is an operation-local semantic outcome. It is not a Core Fault, undefined behavior, a wrapped value, a saturated value, or by itself a source-visible Runen value. A later source or executable representation of a checked operation MUST explicitly define how this outcome is represented or consumed and MUST preserve this numerical classification.

This is an overflow-result and classification rule, not an operation-domain rule. It does not make an operation defined when another applicable contract supplies no exact mathematical integer result, and it does not define division-by-zero behavior, shift-count validity, conversions, source spelling, result-container types, panic or trap behavior, constant-evaluation diagnostics, which integer operations or widths exist, compiler IR, or a physical integer representation.

A realization MAY use physical or backend overflow detection when it proves that the resulting checked outcome is the one required above. Host overflow behavior, backend overflow flags or metadata, optimizer assumptions, and debug or release configuration are not semantic authority for the classification.

## Explicit wrapping overflow

An **explicit wrapping fixed-width integer arithmetic operation** is one whose applicable operation contract selects wrapping overflow behavior.

When such an operation is otherwise defined and its operation-specific semantics determine an exact mathematical integer result `x` for a fixed-width integer result type of width `N`, its Runen result is exactly the value obtained by applying the modulo-`2^N` signed or unsigned value-domain mapping defined for plain fixed-width overflow above. This rule applies whether or not `x` is already in range.

Overflow of an explicit wrapping operation therefore does not by itself produce a defined fault and is not undefined behavior. The numerical result intentionally coincides with plain fixed-width overflow; wrapping remains a distinct explicitly selected operation contract rather than reliance on the plain default.

This is an overflow-result rule, not an operation-domain or source-form rule. It does not define checked arithmetic, division-by-zero behavior, shift-count validity, conversions, source spelling, constant-evaluation diagnostics, which operations or widths exist, or a physical integer representation. A realization MUST NOT derive a different wrapping result from host behavior, backend overflow metadata, optimizer assumptions, or physical representation.

## Explicit saturating overflow

An **explicit saturating fixed-width integer arithmetic operation** is one whose applicable operation contract selects saturating overflow behavior.

When such an operation is otherwise defined and its operation-specific semantics determine an exact mathematical integer result `x` for a fixed-width integer result type of width `N`, let `[lo, hi]` be the mathematical value interval of that result type:

- for an unsigned result type, `lo = 0` and `hi = 2^N - 1`;
- for a signed result type, `lo = -2^(N-1)` and `hi = 2^(N-1) - 1`.

The Runen result is `lo` when `x < lo`, exactly `x` when `lo <= x <= hi`, and `hi` when `x > hi`. Lower and upper out-of-range exact results therefore clamp to the nearest destination bound.

An out-of-range exact result of an explicit saturating operation does not by itself produce a defined fault and is not undefined behavior. Saturation is defined over the mathematical value interval and does not require a physical integer representation.

This is an overflow-result rule, not an operation-domain or source-form rule. It does not define checked arithmetic, division-by-zero behavior, shift-count validity, conversions, source spelling, constant-evaluation diagnostics, which operations or widths exist, or a physical saturation instruction. A realization MUST NOT derive a different saturating result from host behavior, backend overflow metadata, optimizer assumptions, or physical representation.
