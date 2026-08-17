# Core Ownership and Safety

Status: **provisional normative**

Runen targets affine ownership: a value has one logical owner unless its type and the applicable operation permit copying or shared access.

## Borrows

Shared and exclusive borrowing are distinct semantic concepts. Illustrative spellings such as `&T` and `&mut T` do not freeze grammar.

The complete lifetime, reborrow, interior-mutability, and aliasing rules are unspecified in this revision.

## Address, pointer, provenance, and authority

Runen distinguishes:

- numeric address;
- language pointer or reference;
- provenance and validity information;
- resource permission;
- security authority.

A pointer is not semantically defined as merely an integer address.

Exact provenance rules are unspecified in this revision.

## Unsafe abstractions

Safe Runen MUST NOT require a safe caller to satisfy hidden undefined-behavior preconditions that are absent from the safe contract.

An unsafe operation may expose proof obligations that cannot be established automatically.

A safe abstraction implemented with unsafe operations MUST discharge those obligations for every use permitted by its safe public contract.

The complete unsafe-operation list, value-validity model, pointer rules, and undefined-behavior taxonomy are unspecified in this revision.