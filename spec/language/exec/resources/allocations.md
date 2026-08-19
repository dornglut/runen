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

## Allocation accessibility

Physical execution-agent identity is defined by [Exec Realization](../realization.md).

For the duration required by one concrete typed Buffer mapping, an allocation is **accessible to a physical execution agent** when that exact agent may physically service the applicable typed mapped accesses through that exact allocation throughout the mapping's required physical-access interval.

Allocation accessibility is a physical realization fact. It does not grant logical Buffer or view permission, ownership, authority, semantic order, synchronization, logical coherence, progress, or environment admission. Those concerns remain owned by their applicable semantic or lifecycle contracts.

Distinct allocations may have different accessibility relationships to the same physical execution agent. One allocation may be accessible to more than one physical execution agent. Neither case merges allocation identities, execution-agent identities, or the identities of logical resources backed by those allocations.

If an allocation is not accessible to a selected physical execution agent for the duration required by a mapping occurrence, that allocation MUST NOT directly back that agent's mapping occurrence. A realization MAY instead select another legal allocation and mapping or use a staging/copying strategy permitted by the applicable Buffer and realization contracts. It MUST NOT silently treat the inaccessible allocation as accessible or retarget an existing exact mapping occurrence to another allocation or execution agent.

Accessibility of one allocation to multiple execution agents does not by itself establish physical cache coherence, shared-address semantics, address equality, pointer validity across agents, zero-copy behavior, a common address space, semantic synchronization, or semantic memory visibility.

This revision does not define how accessibility is derived from memory spaces, hardware, runtime capabilities, or environment admission; it defines only the relationship required by an applicable mapping occurrence.

## Relationship to Buffer mapping

Typed Buffer mapping is defined by [Exec Buffers](buffers.md). This allocation owner supplies physical allocation identity, allocation extent, and the physical execution-agent accessibility relation consumed by that mapping.

A mapping does not make allocation identity equal to the logical Buffer or region it backs, and allocation identity or accessibility does not by itself grant Buffer access permission, synchronization, or address stability.

## Not yet defined

This revision deliberately does **not** define:

- allocation creation or destruction APIs;
- allocation-space taxonomy, topology, or accessibility derivation;
- execution-agent or memory-capability discovery and environment-admission mechanisms;
- allocation capacity, size units, byte layout, alignment, or contiguity;
- mapping forms other than the address-free typed Buffer mapping owned by `buffers.md`;
- allocation interoperability or import/export;
- relocation rules or stable-address guarantees;
- raw, numeric, or physical address exposure;
- pointer provenance or pointer formation from an allocation.
