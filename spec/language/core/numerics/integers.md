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

Selecting an explicit checked, wrapping, saturating, or other overflow mode is not plain arithmetic under this rule. The separate semantics and source forms of those modes remain to be defined by their applicable contracts.

The modular result above is a semantic value-domain rule. It does not require a physical integer representation, byte layout, ABI representation, or address-level interpretation.
