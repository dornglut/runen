# Core Unsafe Semantics

Status: **provisional normative; incomplete**

Safe Runen MUST NOT require a safe caller to satisfy hidden undefined-behavior preconditions absent from its safe contract.

An unsafe operation may expose proof obligations that cannot be established automatically.

A safe abstraction implemented using unsafe operations MUST discharge those obligations for every use permitted by its safe public contract.

## Currently defined unsafe operations

The proving-MIR `RawRead`, `RawMove`, and `RawAssign` operations defined by [Core pointers and provenance](pointers.md) are unsafe.

For `RawRead`, the concrete target-liveness precondition is defined by the pointer specification and the active-loan shared-access compatibility requirement is defined by [Core borrowing](borrowing.md). A `RawRead` whose pointer operand is structurally and language-valid but whose concrete target violates either required access precondition has **undefined behavior**.

For `RawMove`, the pointer specification defines target selection, the complete-target liveness requirement, and reuse of the ordinary ownership-transfer lifecycle, while [Core borrowing](borrowing.md) defines its raw target exclusive-access compatibility requirement. A `RawMove` whose pointer operand is structurally and language-valid but whose concrete target is not fully Live or violates that compatibility requirement has **undefined behavior**.

For `RawAssign`, the pointer specification defines target selection, source-first ordering, and reuse of the ordinary replacement lifecycle, while [Core borrowing](borrowing.md) defines its raw target exclusive-access compatibility requirement. A `RawAssign` whose explicit operands are structurally and language-valid but whose concrete target violates that compatibility requirement has **undefined behavior**.

`RawAssign` does not have a target-liveness unsafe precondition. Its defined replacement domain includes Never-initialized, partially initialized, fully Live, and Dead target storage according to the value/storage replacement semantics.

Undefined behavior from any currently defined raw operation is not malformed MIR, a language-validation error, a defined `Fault`, or an ordinary recoverable result. A validator's exact symbolic pointer-target bookkeeping for defined path-state propagation does not reclassify these unsafe target proof obligations as validation rules.

Once execution violates one of these unsafe preconditions, this specification defines no continuing execution state or operand result for subsequent Core operations. A verifier MAY therefore stop propagating defined path-state when its exact verification state establishes such a violation, but that verification choice is not a Runen-observable safety result and does not remove the execution-time unsafe precondition.

This revision makes no defined termination-cleanup guarantee after execution has violated one of these unsafe preconditions.

Source `unsafe` syntax and unsafe-block admission are not defined by this revision.

The complete value-validity model, unsafe-operation set, additional unsafe preconditions, and undefined-behavior taxonomy are not defined by this revision.
