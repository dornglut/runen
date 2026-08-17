# Core Floating-Point Semantics

Status: **provisional normative; incomplete**

Runen distinguishes three floating-point contracts:

- **standard** — portable baseline without arbitrary fast-math transformations;
- **reproducible** — stronger cross-realization repeatability, with emulation or rejection when required;
- **fast** — explicit documented numerical relaxations.

Exact operation accuracy, contraction or FMA behavior, transcendental behavior, NaN handling, subnormal handling, rounding and conversion behavior, reduction and reassociation rules, and unsupported-realization behavior are not defined by this revision.