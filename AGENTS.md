# Runen Agent Guide

Start with:

1. `README.md`;
2. `spec/README.md`;
3. `spec/language.md`;
4. the applicable normative annex, currently `spec/annex-a-memory.md` for A0;
5. `spec/semantic-closure.md`;
6. `ARCHITECTURE.md` and `TESTING.md`.

## Repository mission

Runen is proving a language semantic kernel before committing to surface syntax or production lowering.

The provisional language architecture is Core · Exec · Model with explicit bridge laws. The current executable implementation is narrower: A0 value/place/initialization/move/copy/assignment/destruction/fault semantics only.

Do not confuse the broader normative North Star with implemented feature availability.

## Required workflow

1. Bind nontrivial work to an owning repository issue and an explicit accepted `main` revision.
2. Identify which normative specification section/annex owns the semantics being changed.
3. Preserve `spec/semantic-closure.md` dependency order unless an accepted issue explicitly revises it.
4. Keep `crates/runen-core-ir` semantic-data-only and `crates/runen-reference` the executable conformance oracle for the subset it implements.
5. Do not let Rust ownership, `Drop`, addresses, allocation identity, container iteration, compiler/backend behavior, or test convenience accidentally define Runen semantics.
6. Add or revise normative semantics and applicable conformance tests together when observable implemented semantics change.
7. Mark unfrozen surface syntax as illustrative rather than treating examples as grammar authority.
8. Prefer explicit semantic errors over host panics for invalid Runen programs represented by the current executable subset.
9. Do not add compatibility layers or duplicate semantic authority.
10. Run `cargo validate` before proposing acceptance.

## Specification discipline

Normative text lives in `spec/language.md` and accepted normative annexes.

`spec/standard-environment.md` is normative only for claimed standard-environment/profile facilities.

`spec/implementation-architecture.md` and `spec/rationale.md` are non-normative and MUST NOT be cited as permission to change language behavior without a normative change.

When prior design material conflicts, preserve the accepted normative invariant and record unresolved choices rather than silently choosing an implementation-friendly answer.

## Current implementation exclusions

Do not introduce parser/source syntax, borrowing, raw pointers/provenance, traits, native backends, Exec, GPU, or Model implementation as part of A0 maintenance unless a separately accepted issue opens that proving slice.

The existence of provisional normative Exec/Model architecture does not by itself authorize implementing those strata.