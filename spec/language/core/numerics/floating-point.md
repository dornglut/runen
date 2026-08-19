# Core Floating-Point Semantics

Status: **provisional normative; incomplete**

Runen distinguishes three floating-point contracts:

- **standard** — portable baseline without arbitrary fast-math transformations;
- **reproducible** — stronger cross-realization repeatability, with emulation or rejection when required;
- **fast** — explicit documented numerical relaxations.

## Numeric-contract authority

For every floating-point operation governed by this contract system, an applicable Runen semantic context MUST establish exactly one of `standard`, `reproducible`, or `fast` before physical realization. How source selects that contract and which contract, if any, is the source default are not defined by this revision.

The selected numeric contract is semantic input. It is not a target class, backend capability, compiler optimization level, runtime mode, hardware feature, or realization choice. A realization MUST preserve every applicable rule of the selected contract and MUST NOT silently substitute another numeric contract because a target lacks a preferred operation or exposes additional numerical transformation freedom.

When direct realization is unavailable, an implementation MAY use emulation when that emulation preserves the selected contract. It MAY reject the realization only where an applicable conformance, environment, lifecycle, or operation contract permits that rejection. Inability to realize a contract directly is not permission to weaken it.

A physical implementation may use greater internal precision or another stronger implementation technique only when the resulting Runen-observable behavior remains permitted by the selected contract. Physical mechanism does not strengthen, weaken, or replace the semantic contract by itself.

The `fast` contract authorizes only numerical relaxations that Runen explicitly grants to `fast`; enabling a backend's aggregate fast-math mode does not by itself make all of that mode's transformations legal. Conversely, `standard` and `reproducible` gain no implicit transformation freedom merely because a target or backend supports it.

Exact operation accuracy, contraction or FMA behavior, transcendental behavior, NaN handling, subnormal handling, rounding and conversion behavior, reduction and reassociation rules, the detailed `standard`/`reproducible`/`fast` result sets, source contract selection/defaulting, and the concrete mechanism for unsupported-realization admission or rejection are not defined by this revision.
