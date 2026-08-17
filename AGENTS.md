# Contributor and Agent Workflow

This file owns repository work discipline for human and automated contributors. It does not restate language semantics, roadmap content, or compiler architecture.

## Before changing the repository

1. Read `README.md`.
2. Bind nontrivial work to an owning issue.
3. Record the accepted `main` revision from which the work starts.
4. Identify the document or code package that owns the concern being changed.

## Change discipline

- Change the owning artifact instead of duplicating its wording elsewhere.
- Normative language changes belong under `spec/`.
- Project sequencing belongs in `ROADMAP.md` and issue tracking.
- Compiler implementation design belongs under `docs/compiler/`.
- Verification strategy and conformance-oracle documentation belong under `docs/verification/`.
- Design rationale belongs under `docs/decisions/`.
- Research notes belong under `docs/research/`.
- Repository package ownership belongs in `ARCHITECTURE.md`.
- Mechanical validation belongs in `TESTING.md`.

When a normative semantic change affects an executable oracle, update the corresponding conformance tests in the same accepted change.

Do not let host-language behavior, implementation convenience, or tests silently define unspecified language semantics.

## Acceptance

Run `cargo validate`, review the exact patch against the owning issue, and use an exact-head pull-request validation result before acceptance.