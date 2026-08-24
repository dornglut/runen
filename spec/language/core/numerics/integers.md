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

## Plain fixed-width integer addition

This revision represents one concrete plain fixed-width arithmetic operation: **integer addition**.

Its represented operand and result scalar kinds are exactly:

- signed fixed-width integer kinds `I8`, `I16`, `I32`, and `I64`; and
- unsigned fixed-width integer kinds `U8`, `U16`, `U32`, and `U64`.

For one selected fixed-width integer kind of width `N`, let `l` and `r` be two semantic integer values of that same kind. The **exact mathematical addition result** is the mathematical integer

```text
x = l + r
```

with no intermediate fixed-width truncation, host overflow, backend overflow flag, or physical representation involved in forming `x`.

The successful **plain fixed-width addition result** is then exactly the value of that same selected fixed-width integer kind obtained by applying the plain fixed-width overflow mapping above to `x`.

Consequently addition is total over every pair of values in one represented fixed-width integer domain. An exact sum outside the destination interval wraps by the accepted modulo-`2^N` mapping; it does not by itself produce a defined fault, undefined behavior, checked-overflow outcome, or saturated value.

This numerical relation does not make distinct Core type identities interchangeable merely because they have the same scalar kind. The Core operation that consumes this relation owns exact operand/destination type-identity validation and result storage. Likewise, source-language typing and `+` spelling are owned by their source semantic and concrete-syntax owners rather than by this Core numerical relation.

The represented Core operation consuming this relation evaluates its two operands left-to-right and writes the resulting value through the non-replacing destination relation owned by [Core value and storage semantics](../value-storage.md). Those operand-access, storage, lifetime, and initialization rules are not duplicated here.

The addition step after both semantic operand values are available is deterministic, non-faulting, and non-diverging. It introduces no borrow, reference, pointer, storage identity, layout, ABI, target, instruction-selection, or source-visible contract-selection fact.

A conforming implementation MAY use native arithmetic only when it preserves the exact mathematical-addition-plus-plain-overflow relation above. Host signed-overflow behavior, backend `nsw`/`nuw`-like assumptions, debug/release mode, instruction width, or physical two's-complement representation is never semantic authority.

This represented operation defines no multiplication, division, remainder, shift, bitwise operation, comparison, floating operation, vector operation, conversion, constant-evaluation rule, checked addition, saturating addition, or explicitly wrapping addition.

## Plain fixed-width integer subtraction

This revision represents one concrete plain fixed-width arithmetic operation: **integer subtraction**.

Its represented operand and result scalar kinds are exactly:

- signed fixed-width integer kinds `I8`, `I16`, `I32`, and `I64`; and
- unsigned fixed-width integer kinds `U8`, `U16`, `U32`, and `U64`.

For one selected fixed-width integer kind of width `N`, let `l` and `r` be two semantic integer values of that same kind. The **exact mathematical subtraction result** is the mathematical integer

```text
x = l - r
```

with no intermediate fixed-width truncation, host overflow, backend overflow flag, or physical representation involved in forming `x`.

The successful **plain fixed-width subtraction result** is then exactly the value of that same selected fixed-width integer kind obtained by applying the plain fixed-width overflow mapping above to `x`.

Consequently subtraction is total over every pair of values in one represented fixed-width integer domain. An exact difference outside the destination interval wraps by the accepted modulo-`2^N` mapping; it does not by itself produce a defined fault, undefined behavior, checked-overflow outcome, or saturated value.

This numerical relation does not make distinct Core type identities interchangeable merely because they have the same scalar kind. The Core operation that consumes this relation owns exact operand/destination type-identity validation and result storage. Likewise, source-language typing and binary `-` spelling are owned by their source semantic and concrete-syntax owners rather than by this Core numerical relation.

The represented Core operation consuming this relation evaluates its two operands left-to-right and writes the resulting value through the non-replacing destination relation owned by [Core value and storage semantics](../value-storage.md). Those operand-access, storage, lifetime, and initialization rules are not duplicated here.

The subtraction step after both semantic operand values are available is deterministic, non-faulting, and non-diverging. It introduces no borrow, reference, pointer, storage identity, layout, ABI, target, instruction-selection, numeric-contract, or source-visible contract-selection fact.

A conforming implementation MAY use native arithmetic only when it preserves the exact mathematical-subtraction-plus-plain-overflow relation above. Host signed-overflow behavior, backend `nsw`/`nuw`-like assumptions, debug/release mode, instruction width, or physical two's-complement representation is never semantic authority.

This represented operation defines no unary negation, multiplication, division, remainder, shift, bitwise operation, comparison, floating operation, vector operation, conversion, constant-evaluation rule, checked subtraction, saturating subtraction, or explicitly wrapping subtraction.

## Plain fixed-width integer multiplication

This revision represents one concrete plain fixed-width arithmetic operation: **integer multiplication**.

Its represented operand and result scalar kinds are exactly:

- signed fixed-width integer kinds `I8`, `I16`, `I32`, and `I64`; and
- unsigned fixed-width integer kinds `U8`, `U16`, `U32`, and `U64`.

For one selected fixed-width integer kind of width `N`, let `l` and `r` be two semantic integer values of that same kind. The **exact mathematical multiplication result** is the mathematical integer

```text
x = l * r
```

with no intermediate fixed-width truncation, host overflow, backend overflow flag, or physical representation involved in forming `x`.

The successful **plain fixed-width multiplication result** is then exactly the value of that same selected fixed-width integer kind obtained by applying the plain fixed-width overflow mapping above to `x`.

Consequently multiplication is total over every pair of values in one represented fixed-width integer domain. An exact product outside the destination interval wraps by the accepted modulo-`2^N` mapping; it does not by itself produce a defined fault, undefined behavior, checked-overflow outcome, or saturated value.

This numerical relation does not make distinct Core type identities interchangeable merely because they have the same scalar kind. The Core operation that consumes this relation owns exact operand/destination type-identity validation and result storage. Likewise, source-language typing and binary `*` spelling are owned by their source semantic and concrete-syntax owners rather than by this Core numerical relation.

The represented Core operation consuming this relation evaluates its two operands left-to-right and writes the resulting value through the non-replacing destination relation owned by [Core value and storage semantics](../value-storage.md). Those operand-access, storage, lifetime, and initialization rules are not duplicated here.

The multiplication step after both semantic operand values are available is deterministic, non-faulting, and non-diverging. It introduces no borrow, reference, pointer, storage identity, layout, ABI, target, instruction-selection, numeric-contract, or source-visible contract-selection fact.

A conforming implementation MAY use native arithmetic only when it preserves the exact mathematical-multiplication-plus-plain-overflow relation above. Host signed-overflow behavior, backend `nsw`/`nuw`-like assumptions, debug/release mode, instruction width, or physical two's-complement representation is never semantic authority.

This represented operation defines no unary negation, division, remainder, shift, bitwise operation, comparison, floating operation, vector operation, conversion, constant-evaluation rule, checked multiplication, saturating multiplication, explicitly wrapping multiplication, or generic arithmetic operation family.

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
