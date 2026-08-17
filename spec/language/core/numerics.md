# Core Numerics

Status: **provisional normative**

## Integers

Fixed-width integer arithmetic MUST have language-defined semantics. Signed overflow MUST NOT become undefined behavior merely because a backend uses machine integers, and debug or release mode MUST NOT change language meaning.

Checked, wrapping, and saturating operations are part of the intended arithmetic model.

The default overflow behavior of plain fixed-width arithmetic is not defined by this revision.

## Floating point

Runen distinguishes three numeric contracts:

- **standard** — portable baseline without arbitrary fast-math transformations;
- **reproducible** — stronger cross-realization repeatability, with emulation or rejection when required;
- **fast** — explicit documented numerical relaxations.

Exact operation accuracy, contraction or FMA behavior, transcendental behavior, NaN handling, subnormal handling, rounding and conversion behavior, reduction and reassociation rules, and unsupported-realization behavior are not defined by this revision.