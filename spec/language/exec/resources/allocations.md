# Exec Allocations

Status: **provisional normative; incomplete**

An allocation is physical storage in a specified or selected memory space.

Allocation identity is distinct from the identity of any higher-level logical resource backed by that allocation.

An allocation is not transparently migratable merely because another execution agent can access the same logical data.

## Allocation identity and extent

One physical allocation has one opaque **allocation identity** for the duration of its allocation extent. Distinct physical allocations whose extents overlap in time have distinct allocation identities.

Allocation identity is physical-realization identity. It is not Buffer identity, logical element identity, a numeric or physical address, pointer provenance, a source handle, an execution agent, or an ordering token.

An **allocation extent** is the interval during which that physical allocation exists and may provide backing required by an applicable resource contract.

When an active typed Buffer mapping uses an allocation as its physical backing, that mapping MUST end before the allocation extent ends. Ending an allocation extent while such a mapping still requires that allocation would violate the mapping's physical-accessibility contract.

This lifetime relation does not make allocation extent observable as program time, establish semantic execution order, or define allocation creation/destruction operations.

## Relationship to Buffer mapping

Typed Buffer mapping is defined by [Exec Buffers](buffers.md). This allocation owner supplies only physical allocation identity and the extent within which that mapping's backing must remain available.

A mapping does not make allocation identity equal to the logical Buffer or region it backs, and allocation identity does not by itself grant Buffer access permission, synchronization, or address stability.

## Not yet defined

This revision deliberately does **not** define:

- allocation creation or destruction APIs;
- allocation-space taxonomy or accessibility matrices;
- allocation capacity, size units, byte layout, alignment, or contiguity;
- mapping forms other than the address-free typed Buffer mapping owned by `buffers.md`;
- allocation interoperability or import/export;
- relocation rules or stable-address guarantees;
- raw, numeric, or physical address exposure;
- pointer provenance or pointer formation from an allocation.
