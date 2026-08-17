# Standard Environment Boundary

Status: **provisional normative boundary**

The Runen Standard Environment contains portable facilities that are standardized without becoming fundamental language primitives.

A library namespace, module, runtime service, or common implementation technique is not a language primitive merely because a standard implementation ships it.

## Profile boundary

An implementation claiming only Runen Core does not need a Hosted Standard Environment.

A profile that requires a Standard Environment facility must state that requirement explicitly.

## Freestanding boundary

Freestanding Core MUST remain implementable without requiring:

- heap allocation;
- a filesystem;
- networking;
- threads;
- an async runtime;
- a GPU runtime;
- a Model runtime;
- rendering, UI, or asset systems.

## Semantic ownership

A Standard Environment API that exposes a language-level concept must obey the corresponding language semantics and does not redefine them.

A standard allocator does not redefine Core pointer provenance. A standard Buffer API does not redefine Exec resource semantics. A standard query API does not redefine Model query semantics. A standard network API does not turn remote state into shared memory.

## Standardization criterion

A facility should be standardized when interoperability or common portable source requires a shared contract and the contract is sufficiently mature not to freeze one incidental implementation architecture.