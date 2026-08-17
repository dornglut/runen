# Compiler Architecture

Status: **non-normative implementation design**

This document owns the intended compiler/realization decomposition. It does not define language semantics, repository package ownership, project sequencing, or verification policy.

## Target decomposition

```text
Source
  ↓
Lossless syntax
  ↓
Typed HIR
  ├───────────────┬────────────────┐
  ↓               ↓                ↓
Core MIR       Exec IR         Logical IR
  └───────────────┼────────────────┘
                  ↓
             Realization
                  ↓
             Execution IR
                  ↓
             Target IR(s)
```

Names and exact boundaries may change when implementation evidence warrants it.

## Responsibilities

**Lossless syntax** preserves source structure needed by parsing, diagnostics, formatting, and source tooling.

**Typed HIR** owns resolved source-level structure and type-checked author intent before lower semantic forms erase syntax.

**Core MIR** should make ordinary value/place/control/ownership semantics explicit.

**Exec IR** should preserve execution-visible tasks, resources, parallel legality, execution requirements, and numeric contracts.

**Logical IR** should preserve Model logical types, queries, state-domain observations, rules, and maintenance meaning.

**Realization** selects legal physical schedules, placements, representations, transfers, and specializations.

**Execution/target IRs** express increasingly physical computation suitable for native or accelerator backends.

## Dependency principle

Implementation layers may refine or lower normative semantics; they do not create semantics that the specification leaves unspecified.

The compiler may share internal infrastructure across semantic domains, but a universal node vocabulary should not erase domain-specific information merely for implementation convenience.