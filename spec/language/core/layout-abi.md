# Core Layout and ABI

Status: **provisional normative**

Default native layout and ABI are not implicitly stable.

Source code that requires stable representation or an external ABI MUST use an explicit language mechanism whose contract establishes the required guarantees.

[Core external callable imports](external-calls.md) defines one representation-neutral semantic provider interface for intrinsic scalar values. That relation is an external-call semantic contract but **not** a binary ABI or linkage contract: it does not establish scalar bit encodings, size/alignment, endianness, calling convention, register/stack classification, symbol identity, physical function addresses, relocation, or stable data layout. A realization that uses an adapter or physical ABI to satisfy an external callable requirement MUST preserve the semantic provider contract; its chosen mechanism does not become language semantics merely through implementation use.

The concrete stable-layout, binary calling-convention, physical FFI binding, linkage/symbol, and entry-point mechanisms are not defined by this revision.
