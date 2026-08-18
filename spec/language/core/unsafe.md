# Core Unsafe Semantics

Status: **provisional normative; incomplete**

Safe Runen MUST NOT require a safe caller to satisfy hidden undefined-behavior preconditions absent from its safe contract.

An unsafe operation may expose proof obligations that cannot be established automatically.

A safe abstraction implemented using unsafe operations MUST discharge those obligations for every use permitted by its safe public contract.

## Currently defined unsafe operation

The proving-MIR `RawRead` operation defined by [Core pointers and provenance](pointers.md) is unsafe.

Its concrete target-liveness precondition is defined by the pointer specification, and its active-loan compatibility requirement is defined by [Core borrowing](borrowing.md).

A `RawRead` whose pointer operand is structurally and language-valid but whose concrete target violates either of those required access preconditions has **undefined behavior**.

Such undefined behavior is not malformed MIR, a language-validation error, a defined `Fault`, or an ordinary recoverable result. This revision makes no defined termination-cleanup guarantee after that `RawRead` has violated its unsafe preconditions.

Source `unsafe` syntax and unsafe-block admission are not defined by this revision.

The complete value-validity model, unsafe-operation set, additional unsafe preconditions, and undefined-behavior taxonomy are not defined by this revision.
