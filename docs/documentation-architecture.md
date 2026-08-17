# Documentation Architecture

This document owns repository documentation boundaries and dependency direction. It does not define Runen semantics, project priorities, compiler design, or verification policy.

## Artifact ownership

- `spec/` — normative Runen specification only;
- `ROADMAP.md` — project sequencing and semantic-closure planning only;
- `ARCHITECTURE.md` — repository package and dependency structure only;
- `TESTING.md` — mechanical repository validation only;
- `docs/compiler/` — non-normative compiler and realization design;
- `docs/verification/` — non-normative assurance and oracle contracts;
- `docs/decisions/` — historical design decisions and rationale;
- `docs/research/` — external research records;
- `CONTRIBUTING.md` — contributor process;
- `AGENTS.md` — automation-specific contributor constraints;
- `README.md` and directory README files — navigation and orientation.

## Dependency direction

Normative specification artifacts may reference other normative specification artifacts. They MUST NOT depend on roadmap, repository implementation, verification documents, design-decision records, research notes, or contributor workflow for their meaning.

Non-normative documents may reference normative specification owners.

Roadmap and contributor documents may reference any artifact needed to identify work, but they do not acquire authority over the referenced concern.

## No duplicated authority

A document may summarize its own scope and link to another owner. It SHOULD NOT copy another owner's detailed rules merely to make the current file self-contained.

If a concept change would require editing multiple documents that claim to define the same rule, the documentation decomposition is defective. Select one canonical owner and turn the other occurrences into links or relationship statements.

## Index discipline

README files are indexes and orientation pages. They list destinations and may state the scope of a directory; they do not become shadow specifications, roadmaps, or architecture documents.

## Growth rule

Split a document when two parts can evolve independently under different correctness or review obligations. Do not split merely to reduce line count or pre-create empty taxonomy.