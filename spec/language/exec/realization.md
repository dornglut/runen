# Exec Realization

Status: **provisional normative; incomplete**

Runen distinguishes:

- an **algorithm or implementation**, which defines a computation;
- a **schedule**, which changes physical arrangement while preserving permitted behavior;
- a **specialization**, which provides an alternative realization of the same public semantic operation under stated assumptions.

A schedule transformation or specialization MUST preserve every applicable semantic contract.

Physical placement and transfer may be selected automatically only where the resource and program semantics permit that choice.

The source-level authoring model for schedules, placement preferences, or specializations is not defined by this revision.