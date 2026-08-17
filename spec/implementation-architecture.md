# Runen Implementation Architecture

Status: **non-normative implementation guidance**

This document describes a recommended implementation decomposition for proving and eventually compiling Runen. It is not part of the language semantics.

A conforming implementation MAY use a completely different internal architecture if it implements the normative language/profile contracts.

## 1. Recommended long-term shape

```text
Source
  ↓
Lossless Syntax
  ↓
Typed HIR
  ├───────────────┬────────────────┐
  ↓               ↓                ↓
Core MIR      Compute/Exec IR   Logical IR
  └───────────────┼────────────────┘
                  ↓
             Realization
                  ↓
             Execution IR
                  ↓
             Target IR(s)
```

Names and boundaries may change as implementation proving teaches us more.

## 2. Why multiple semantic IR views

Core, Exec, and Model have different semantic information worth preserving:

- Core needs explicit values, places, ownership, control flow, unsafe/validity facts, and ordinary effects;
- Exec needs resource access, task structure, parallel legality, execution requirements, numeric contracts, and realization opportunities;
- Model needs logical typing, relations, queries, state-domain observations, rule proposals, and incremental/materialization meaning.

Forcing all three into one universal IR node vocabulary risks either losing semantic structure too early or creating an enormous universal compiler object model.

The implementation may share infrastructure where useful, but semantic ownership should remain explicit.

## 3. Realization

Realization maps semantic computation/resources onto admitted physical execution while preserving language behavior.

Potential realization choices include:

- direct call versus scheduled task;
- scalar versus vector versus multicore;
- CPU versus GPU/accelerator;
- allocation/memory space;
- Buffer physical backing;
- transfer/staging;
- schedule transformations;
- specialization;
- query plan;
- materialization strategy;
- incremental maintenance mechanism.

These choices are not source semantics merely because a particular compiler represents them in an IR.

## 4. Current repository architecture

The repository intentionally implements only the A0 semantic kernel:

```text
spec/language.md
       │
       └── spec/annex-a-memory.md
                    │
              runen-core-ir
                    │
                    ▼
              runen-reference
                    │
                    ▼
              conformance tests
```

`runen-core-ir` is semantic data only.

`runen-reference` is the executable conformance oracle for A0. It is not a production interpreter architecture commitment.

`tools/xtask` is repository tooling and owns no Runen semantics.

## 5. Implementation sequence

The recommended proving sequence follows `semantic-closure.md` rather than frontend convenience:

1. finish value/memory/safety semantics;
2. finish Exec resource/concurrency semantics;
3. close numeric contracts;
4. complete source-language grammar/name/type/trait/closure/ABI/fault rules;
5. define the minimal Model algebra;
6. prove cross-stratum composition;
7. only then broaden ecosystem/backends/profiles aggressively.

A parser or LLVM backend may be useful during later stages, but neither should become the oracle for unresolved semantics.

## 6. Reference versus production implementation

The reference implementation should optimize for semantic clarity, determinism, diagnostics, and testability.

A production implementation may use very different data structures and algorithms.

Production optimization is valid only when it preserves the applicable normative contract.

## 7. Verification tooling

No proof technology is mandatory today.

Possible future assurance may combine:

- executable reference semantics;
- property-based tests;
- adversarial litmus suites;
- differential execution;
- translation validation;
- model checking;
- theorem proving for narrow high-risk kernels.

Verification tools are implementation/assurance choices until a specific normative conformance obligation requires a standardized artifact.