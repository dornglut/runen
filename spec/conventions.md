# Specification Conventions

Status: **normative meta-specification**

This file defines how Runen specification documents are interpreted. It does not define language operations.

## Normative terms

Capitalized **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, **MAY**, and **OPTIONAL** express normative requirements.

## Document status

- **normative** — constrains conforming implementations;
- **provisional normative** — currently constrains the accepted pre-stability design and may change only through an explicit normative revision;
- **illustrative** — communicates intent without defining required syntax or API spelling;
- **non-normative** — explanation, rationale, implementation guidance, planning, or verification guidance;
- **unspecified in this revision** — this specification version deliberately provides no rule for the named detail.

## Precedence

When normative artifacts overlap, the more specific accepted normative rule governs its stated scope. A normative annex may therefore refine a general language rule for the subset the annex explicitly covers.

Non-normative material never overrides normative text.

Compiler behavior, repository code, tests, host-language behavior, examples, issue discussion, and research sources do not silently define Runen semantics.

## Illustrative syntax

Unless a normative grammar document explicitly says otherwise, source spellings and examples are illustrative. Semantic names such as `task`, `each`, `Relation`, or `maintain` identify concepts; they do not by themselves freeze lexical or concrete grammar.

## Unspecified behavior versus implementation freedom

A detail described as unspecified in this revision is a specification gap for that detail, not permission to invent a conflicting language rule and call it conforming.

Implementation freedom exists only inside behavior permitted by the normative specification.