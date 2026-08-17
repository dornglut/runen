# Runen Specification Set

Status: **provisional language-design authority**

This directory contains the Runen specification set. It separates language semantics from standard facilities, implementation guidance, rationale, and conformance evidence so that none of those accidentally defines another by implication.

The current specification is intentionally incomplete. A statement being present here does not imply that every downstream semantic detail has been closed. Unresolved requirements are tracked explicitly in [Semantic Closure](semantic-closure.md).

## Authority order

When two repository artifacts appear to disagree, use this order:

1. the applicable accepted normative language specification or normative annex;
2. later accepted normative amendments to that specification;
3. executable conformance artifacts only for the semantic subset their governing normative text says they realize;
4. standard-environment contracts for implementations claiming the corresponding profile;
5. non-normative rationale and implementation architecture.

Repository code, compiler behavior, Rust behavior, examples, tests, research papers, issue discussion, and implementation convenience do **not** silently override normative Runen semantics.

## Specification artifacts

### [Runen Language Specification](language.md)

**Normative, provisional.**

Defines the currently accepted semantic architecture of Runen:

- program behavior and validity boundaries;
- Core, Exec, and Model semantic strata;
- cross-stratum bridge laws;
- safety and resource principles;
- execution, state, time, and observation concepts;
- correctness obligations;
- conformance profiles.

The document intentionally marks unresolved P0 semantics rather than inventing answers.

### [Semantic Annex A — Values, Places, Initialization, Move, Copy, and Destruction](annex-a-memory.md)

**Normative for the accepted A0 executable subset.**

This is currently the most detailed executable part of the language specification. Where `language.md` gives a broad Core invariant and Annex A0 gives a more precise A0 rule for the same accepted subset, Annex A0 governs that subset.

Annex A0 does not define later borrowing, provenance, atomics, ABI, parser, Exec, or Model semantics.

### [Semantic Closure](semantic-closure.md)

**Normative status ledger; not semantics by itself.**

Records what must still be specified before Runen can claim a complete implementable language specification. Work is dependency-ordered P0-A through P0-F.

### [Runen Standard Environment](standard-environment.md)

**Normative only for claimed environment/profile facilities; currently skeletal.**

Separates standard libraries and hosted facilities from language primitives.

### [Runen Conformance & Assurance](conformance.md)

**Normative conformance boundary plus assurance guidance.**

Defines what an implementation may claim, how profiles compose, and how reference semantics/tests relate to the language specification.

### [Runen Implementation Architecture](implementation-architecture.md)

**Non-normative.**

Provides a recommended compiler/realization decomposition. Implementations may use a different architecture if they satisfy the normative specification.

### [Runen Rationale](rationale.md)

**Non-normative.**

Records design intent, research pressure, rejected universal mechanisms, and terminology rationale. External research is evidence and pressure testing only; it is never Runen normative authority.

## Normative language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, **MAY**, and **OPTIONAL** are used as normative requirement terms when capitalized.

Lower-case descriptive prose is not automatically weaker, but specification work should prefer explicit requirement language where independent implementations need an interoperable rule.

## Status vocabulary

The specification uses these status terms deliberately:

- **normative** — constrains conforming implementations;
- **provisional normative** — currently constrains the accepted design, but remains eligible for deliberate revision before a compatibility commitment;
- **illustrative** — communicates intent but does not freeze source syntax or API spelling;
- **non-normative** — explanation, rationale, or implementation guidance only;
- **open** — no normative answer has yet been adopted.

## Surface syntax

Unless a section explicitly states otherwise, source syntax in the current specification is **illustrative**. The repository is proving semantic contracts before freezing lexical grammar, parser grammar, module syntax, or complete surface forms.

## Current accepted semantic implementation

The accepted implementation currently proves A0 only:

```text
Language Specification
        │
        └── Annex A0: value/place/init/move/copy/assign/drop/fault
                │
                ├── runen-core-ir
                └── runen-reference + conformance tests
```

Exec and Model are specification architecture, not implemented language features yet.

## Change discipline

A semantic change MUST update all affected normative artifacts together. If an executable conformance oracle exists for the changed semantic subset, its tests MUST be revised in the same accepted change.

If a design question depends on a later unresolved semantic layer, record that dependency in `semantic-closure.md` rather than smuggling a provisional implementation choice into normative prose.