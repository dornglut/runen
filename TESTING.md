# Repository Testing

This document owns the mechanical repository validation contract. Semantic conformance strategy is documented separately under `docs/verification/`.

## Canonical command

```text
cargo validate
```

The command performs:

1. locked Cargo metadata resolution;
2. workspace formatting verification;
3. locked all-target workspace tests;
4. all-target workspace Clippy with warnings denied;
5. Git diff hygiene;
6. before/after checkout-state comparison.

Focused commands may be used during development but do not replace `cargo validate` before acceptance.

GitHub Actions invokes the same repository-owned command through the pinned Dornglut reusable Rust validation workflow and validates the exact reviewed feature-head revision.

Repository validation is mechanical evidence only. It does not define Runen semantics.