# Exec Resources

Status: **provisional normative**

## Allocation

An allocation is physical storage in a specified or selected memory space. It is not transparently migratable merely because another device can access the same logical data.

## Buffer

`Buffer<T>` is a logical coherent Exec resource. Its logical identity is distinct from any one physical backing allocation or raw address.

A legal realization may maintain, migrate, or replicate physical backing only according to the Buffer coherence contract.

`View` and `ViewMut` are logical permission-bearing views. They do not promise permanently stable raw physical addresses.

Exposing a raw physical address requires a stable allocation or an explicit mapped/pinned realization whose contract preserves address validity for the required duration.

The precise Buffer version/coherence, mapping, relocation, and multi-realization state machine is unspecified in this revision.

## Transfer and placement

Physical transfer or placement may be chosen automatically when semantics permit it. Such choices do not change logical resource identity or ownership.