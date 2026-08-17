# Specification Conventions

Status: **provisional normative**

This document defines how normative Runen specification artifacts are interpreted. It does not define language operations.

## Requirement terms

Capitalized **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, **MAY**, and **OPTIONAL** express normative requirement levels.

Lowercase uses of `must`, `should`, and `may` are ordinary English and do not carry those requirement levels. A normative obligation or permission that relies on one of these levels MUST use the capitalized form.

## Document status

A status line combines an authority class with zero or more defined qualifiers.

Authority classes are:

- **normative** — constrains conforming implementations;
- **illustrative** — communicates intent without defining required syntax or behavior;
- **non-normative** — does not define Runen behavior.

Defined qualifiers are:

- **provisional** — the artifact constrains the current pre-stability design and may change only through an explicit normative revision;
- **incomplete** — the artifact owns some established rules while explicitly leaving identified semantic items open.

`incomplete` does not weaken rules that the artifact does define, and it is not a grant of implementation freedom for its open items. Unrecognized status qualifiers have no normative meaning and MUST NOT be used to imply one.

## Open specification items

Text that says a rule is **not defined by this revision** marks an open specification item. It is not a grant of implementation freedom and cannot be used to justify a standardized conformance claim for that detail.

This is distinct from **permitted variation**: when normative text explicitly permits multiple outcomes, orders, or realizations, every permitted alternative is part of the language semantics.

**Implementation-defined** behavior exists only where normative text explicitly permits an implementation choice and requires that choice to be documented. An open specification item does not become implementation-defined merely because an implementation must choose something internally.

**Undefined behavior** exists only where the applicable normative safety or unsafe contract explicitly designates behavior as undefined. It is not a synonym for an invalid program, an open specification item, or unsupported implementation behavior.

## Ownership and conflicts

Each normative rule has exactly one canonical owner.

A normative artifact MAY reference another normative owner or state a relationship between separately owned concepts. It MUST NOT independently restate, redefine, or duplicate another owner's normative rule merely for convenience or local completeness.

If two normative artifacts appear to define rules for the same semantic responsibility, that is a specification defect requiring explicit ownership correction. A conflict is not resolved by file order, path depth, apparent specificity, or implementation behavior.

Non-normative material never overrides normative text.

## Illustrative syntax

Unless a normative grammar document explicitly states otherwise, source spellings and examples are illustrative. Semantic names such as `task`, `each`, `Relation`, or `maintain` identify concepts; they do not by themselves freeze lexical or concrete grammar.
