# 0005 — Staged Rule Commit and Maintenance Contracts

Status: **accepted design rationale**

## Context

If rule mutations, irreversible effects, and logical events become visible independently, observers may see events for state that never committed or state whose defining event is missing. Likewise, “keep synchronized” is meaningless without freshness, failure, and visibility rules.

## Decision

Stage rule state proposals and logical events before admission. Make transition state changes and their logical events acquire logical existence together at successful commit. Require `maintain` targets to define observation identity, admission, freshness, progress, failure/reconciliation, and visibility contracts.

## Consequences

Ordinary rule semantics do not become a universal distributed transaction system, and maintenance does not silently promise reliable zero-latency synchronization.