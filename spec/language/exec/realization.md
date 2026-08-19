# Exec Realization

Status: **provisional normative; incomplete**

Runen distinguishes:

- an **algorithm or implementation**, which defines a computation;
- a **schedule**, which changes physical arrangement while preserving permitted behavior;
- a **specialization**, which provides an alternative realization of the same public semantic operation under stated assumptions.

## Physical execution agents

A **physical execution agent** is an opaque realization entity used only where a physical placement or resource contract must distinguish which physical entity may service an operation.

One physical execution agent has one opaque identity for the applicable realization. Distinct physical execution agents within that realization have distinct agent identities. Agent identity is physical-realization identity. It is not source-visible program state, a Core or Exec semantic execution context, task identity, dynamic `each` or iteration identity, hierarchy/group/subgroup identity, a worker or lane index, queue identity, device class, memory-space identity, scheduling order, concurrency relation, synchronization relation, or progress guarantee.

Assigning an applicable Exec operation to a physical execution agent is a realization choice. That choice MUST preserve every applicable language semantic contract and admitted environment contract.

When the canonical owner of a physical resource defines an accessibility relation for execution agents, a realization MUST NOT claim that an agent directly services an access through that resource unless the applicable accessibility relation admits that exact agent for the required physical-access interval.

Choosing the same physical agent for two semantic actions does not by itself establish semantic order, synchronization, shared state, or any other language-visible interaction. Choosing distinct physical agents likewise does not by itself make two semantic actions concurrent or unordered; those relations remain defined by their semantic owners.

This revision does not define source target or placement syntax, device enumeration, CPU/GPU/host/device classes, worker topology, queue topology, scheduling policy, progress or fairness, execution-agent discovery, environment admission APIs, or backend instructions.

## Realization preservation

A schedule transformation or specialization MUST preserve every applicable semantic contract.

Physical placement and transfer may be selected automatically only where the resource and program semantics permit that choice.

The source-level authoring model for schedules, placement preferences, or specializations is not defined by this revision.
