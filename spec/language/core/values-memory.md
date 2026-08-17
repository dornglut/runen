# Core Values and Memory

Status: **provisional normative**

Core owns ordinary language values and storage semantics.

A value and a storage place are distinct semantic categories.

Core native structural types have ordinary language value and memory semantics. A Model record is a logical value whose physical storage layout is not its semantic identity.

The accepted A0 rules for structural values, places, hierarchical initialization, partial initialization, move, copy, assignment, destruction, return cleanup, and fault cleanup are defined in [Core Annex A0](../../annexes/core/a0-values-places.md).

Later Core memory semantics must preserve those accepted rules unless an explicit normative revision changes them.

This revision does not yet specify the complete language semantics of heap/raw allocations, object lifetime/deallocation, references, or raw pointer access.