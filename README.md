# Runen

Runen is a programming language under semantic-kernel development.

## Start here

- [Language specification](spec/README.md)
- [Project roadmap](ROADMAP.md)
- [Repository architecture](ARCHITECTURE.md)
- [Repository testing](TESTING.md)
- [Contributor and agent workflow](AGENTS.md)
- [Non-normative engineering/design documentation](docs/README.md)

## Source tree

- `crates/runen-core-ir` — semantic IR data structures for the currently implemented Core subset;
- `crates/runen-reference` — executable reference semantics for the currently implemented subset;
- `tools/xtask` — repository tooling;
- `spec/` — normative Runen specification;
- `docs/` — non-normative implementation, verification, rationale, and research documentation.

The repository intentionally develops semantics before committing to a full frontend or production backend.