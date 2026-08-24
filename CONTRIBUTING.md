# Contributing

This document owns the repository contribution process. Documentation ownership is defined separately in [Documentation Architecture](docs/documentation-architecture.md).

## Before changing the repository

1. Bind nontrivial work to an owning issue.
2. Record the accepted `main` revision from which the work starts.
3. Identify the artifact that owns the concern being changed.
4. Keep the change inside the owning issue's semantic and implementation scope.

## Licensing and inbound contributions

Runen's intended commercial dual-licensing model is described in [LICENSING.md](LICENSING.md). Any separate commercial grant depends on sufficient rights held by the granting copyright holder(s).

Until reviewed inbound contribution terms exist that preserve the commercial relicensing path, external pull requests that add tracked repository content are not accepted, including code, documentation, tests, examples, build scripts, or assets.

Issue reports, design discussion, reviews, reproducible cases supplied through discussion, and other participation that does not add third-party repository content remain welcome.

## Change discipline

Change the canonical owner rather than copying its rules into another document or package.

When a normative semantic change affects an executable oracle, update the corresponding conformance tests in the same accepted change.

Do not use host-language behavior, implementation convenience, or an existing test as authority for a semantic detail the normative specification has not defined.

## Acceptance

Run `cargo validate`, review the exact patch against its owning issue, and require exact-head pull-request validation before acceptance.

Normative semantic changes additionally require exact-head semantic assurance under [Verification Strategy](docs/verification/strategy.md). Before acceptance, that assurance must account for every owning-issue acceptance criterion, identify exact verification evidence where a criterion requires it, and establish that the changed semantics remain consistent with the accepted normative rules and ownership boundaries they directly affect.

The pull request must record the reviewed head and semantic-assurance verdict. A material change to the semantically reviewed head makes that verdict stale and requires renewed exact-head semantic assurance before acceptance.
