# Core Faults

Status: **provisional normative; incomplete**

Defined faults are distinct from undefined behavior and from ordinary recoverable result values.

A defined fault may participate in cleanup rules where the applicable Core semantics say so.

[Core functions and direct calls](functions.md) owns propagation of an already selected defined fault through the represented direct-call activation relation. That owner preserves the same defined fault while each terminating activation performs its applicable Core cleanup.

This document does not independently define direct-call propagation, and the represented direct-call relation does not complete the broader panic/fault model.

The complete panic syntax, fault payload representation, catch/recovery model, exception object model, backtrace model, and physical unwind mechanism remain undefined by this revision.
