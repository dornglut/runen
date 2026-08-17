# Runen Agent Guide

Start with `README.md`, `ARCHITECTURE.md`, `TESTING.md`, and `spec/annex-a-memory.md`.

## Repository mission

Runen is proving a language semantic kernel before committing to surface syntax or production lowering. The current A0 authority is the narrow value/place/initialization/move/copy/assignment/destruction/fault model.

## Required workflow

1. Bind nontrivial work to an owning repository issue and an explicit accepted `main` revision.
2. Preserve the A0 boundary unless the owning issue explicitly authorizes a later proving slice.
3. Keep `crates/runen-core-ir` semantic-data-only and `crates/runen-reference` the executable conformance oracle.
4. Do not let Rust ownership, `Drop`, addresses, allocation identity, or container iteration accidentally define Runen semantics.
5. Add or revise the semantic annex and conformance tests together when observable semantics change.
6. Prefer explicit semantic errors over host panics for invalid Runen programs represented by the current MIR.
7. Do not add compatibility layers or duplicate semantic authority.
8. Run `cargo validate` before proposing acceptance.

## Current exclusions

Do not introduce parser/source syntax, borrowing, raw pointers/provenance, traits, native backends, Exec, GPU, or Model as part of A0 maintenance unless a separately accepted issue opens that proving slice.
