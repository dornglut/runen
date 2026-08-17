# Contributing

This document owns the repository contribution process. Documentation ownership is defined separately in [Documentation Architecture](docs/documentation-architecture.md).

## Before changing the repository

1. Bind nontrivial work to an owning issue.
2. Record the accepted `main` revision from which the work starts.
3. Identify the artifact that owns the concern being changed.
4. Keep the change inside the owning issue's semantic and implementation scope.

## Change discipline

Change the canonical owner rather than copying its rules into another document or package.

When a normative semantic change affects an executable oracle, update the corresponding conformance tests in the same accepted change.

Do not use host-language behavior, implementation convenience, or an existing test as authority for a semantic detail the normative specification has not defined.

## Acceptance

Run `cargo validate`, review the exact patch against its owning issue, and require exact-head pull-request validation before acceptance.