# Core Floating-Point Semantics

Status: **provisional normative; incomplete**

Runen distinguishes three floating-point contracts:

- **standard** — portable baseline without arbitrary fast-math transformations;
- **reproducible** — stronger cross-realization repeatability, with emulation or rejection when required;
- **fast** — explicit documented numerical relaxations.

## Numeric-contract authority

For every floating-point operation governed by this contract system, an applicable Runen semantic context MUST establish exactly one of `standard`, `reproducible`, or `fast` before physical realization. How source selects that contract and which contract, if any, is the source default are not defined by this revision.

The selected numeric contract is semantic input. It is not a target class, backend capability, compiler optimization level, runtime mode, hardware feature, or realization choice. A realization MUST preserve every applicable rule of the selected contract and MUST NOT silently substitute another numeric contract because a target lacks a preferred operation or exposes additional numerical transformation freedom.

When direct realization is unavailable, an implementation MAY use emulation when that emulation preserves the selected contract. If satisfying the selected contract requires a hard environment capability, the applicable contract MUST expose that requirement to environment admission, where it is either admitted or rejected according to the language lifecycle. Once admitted, inability to realize the contract directly is not permission to weaken it.

A physical implementation may use greater internal precision or another stronger implementation technique only when the resulting Runen-observable behavior remains permitted by the selected contract. Physical mechanism does not strengthen, weaken, or replace the semantic contract by itself.

The `fast` contract authorizes only numerical relaxations that Runen explicitly grants to `fast`; enabling a backend's aggregate fast-math mode does not by itself make all of that mode's transformations legal. Conversely, `standard` and `reproducible` gain no implicit transformation freedom merely because a target or backend supports it.

Exact operation accuracy, contraction or FMA behavior, transcendental behavior, NaN handling, subnormal handling, rounding and conversion behavior, reduction and reassociation rules, the detailed `standard`/`reproducible`/`fast` result sets, source contract selection/defaulting, and the concrete hard requirements for unsupported direct realization are not defined by this revision.
