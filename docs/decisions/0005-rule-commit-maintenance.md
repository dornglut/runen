# 0005 — Staged Rule Commit and Explicit Maintenance Contracts

Status: **accepted**  
Recorded: **2026-08-17**  
Normative owners: [`spec/language/model/rules.md`](../../spec/language/model/rules.md), [`spec/language/model/maintenance.md`](../../spec/language/model/maintenance.md)  
Supersedes: none  
Superseded by: none

## Context

Reactive logical state becomes incoherent if pre-commit effects escape as though committed or if a maintenance request silently promises unspecified freshness, retry, or distributed transaction behavior.

## Decision

Stage rule proposals and logical events until state-domain admission, couple transition state and logical events at commit, and require maintenance targets to publish the contracts needed to define correspondence.

## Alternatives considered

Immediate mutation or event emission would make reaction ordering part of observable semantics. A universal synchronization promise for `maintain` would overstate portable guarantees.

## Consequences

Rule evaluation has a clear commit boundary and maintenance semantics can vary by target without becoming undefined.