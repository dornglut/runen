# Core Numerics

Status: **provisional normative**

## Integers

Fixed-width integer arithmetic MUST have language-defined semantics. Signed overflow MUST NOT become undefined behavior merely because a backend uses machine integers, and debug/release mode MUST NOT change language meaning.

Checked, wrapping, and saturating operations are part of the intended arithmetic model.

The default overflow behavior of plain fixed-width arithmetic is unspecified in this revision.

## Floating point

Runen distinguishes three numeric contracts:

- **standard** — portable baseline without arbitrary fast-math transformations;
- **reproducible** — stronger cross-realization repeatability, with emulation or rejection when required;
- **fast** — explicit documented numerical relaxations.

Exact operation accuracy, contraction/FMA, transcendental behavior, NaN handling, subnormal handling, rounding/conversion behavior, reduction/reassociation rules, and unsupported-realization behavior are unspecified in this revision.