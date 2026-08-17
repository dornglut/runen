# Specification Conventions

Status: **normative meta-specification**

This document defines how normative Runen specification artifacts are interpreted. It does not define language operations.

## Requirement terms

Capitalized **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, **MAY**, and **OPTIONAL** express normative requirements.

## Document status

- **normative** — constrains conforming implementations;
- **provisional normative** — currently constrains the pre-stability design and may change only through an explicit normative revision;
- **illustrative** — communicates intent without defining required syntax or API spelling;
- **non-normative** — does not define Runen behavior.

## Open specification items

Text that says a rule is **not defined by this revision** marks an open specification item. It is not a grant of implementation freedom and cannot be used to justify a standardized conformance claim for that detail.

This is distinct from **permitted variation**: when normative text explicitly permits multiple outcomes, orders, or realizations, every permitted alternative is part of the language semantics.

## Ownership and conflicts

Each normative rule has one canonical owner. Normative files may reference each other, but they SHOULD NOT restate another file's rule merely for convenience.

If two normative artifacts appear to define conflicting rules for the same scope, that is a specification defect requiring explicit correction. There is no general "more specific file silently wins" rule.

Non-normative material never overrides normative text.

## Illustrative syntax

Unless a normative grammar document explicitly states otherwise, source spellings and examples are illustrative. Semantic names such as `task`, `each`, `Relation`, or `maintain` identify concepts; they do not by themselves freeze lexical or concrete grammar.