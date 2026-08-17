# Repository Testing

This document owns the mechanical repository validation contract. Semantic assurance strategy is documented separately under `docs/verification/`.

## Canonical command

```text
cargo validate
```

The command performs:

1. locked Cargo metadata resolution;
2. Markdown link integrity and normative `spec/` dependency-boundary checks;
3. workspace formatting verification;
4. locked all-target workspace tests;
5. all-target workspace Clippy with warnings denied;
6. Git diff hygiene;
7. before/after checkout-state comparison.

Focused commands may be used during development but do not replace `cargo validate` before acceptance.

GitHub Actions invokes the same repository-owned command through the pinned Dornglut reusable Rust validation workflow and validates the exact reviewed feature-head revision.