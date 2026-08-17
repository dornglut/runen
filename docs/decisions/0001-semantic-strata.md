# 0001 — Core, Exec, and Model Semantic Strata

Status: **accepted design rationale**

## Context

Runen must cover ordinary systems values/memory, heterogeneous execution, and logical/declarative state without forcing one domain's implementation mechanism into another domain's source model.

## Decision

Use exactly three top-level semantic strata: Core, Exec, and Model. Require explicit semantics where values/resources cross their boundaries. Treat the strata as semantic responsibilities, not required runtimes or compiler crates.

## Consequences

The language can preserve domain-specific reasoning while allowing implementations to erase boundaries physically when behavior is preserved. Cross-stratum questions must be designed explicitly rather than delegated to runtime convenience.