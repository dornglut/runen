# Semantic Strata

Status: **provisional normative**

Runen defines three top-level semantic strata: **Core**, **Exec**, and **Model**.

The strata are semantic responsibilities. They are not required runtime layers, processes, compiler crates, or deployment units.

## Core

Core owns ordinary values, storage, ownership and access, control, effects, faults, numerics, and low-level interaction.

## Exec

Exec owns execution-visible work and logical executable resources whose legal physical realization may vary across sequential, parallel, vector, accelerator, or other execution environments.

## Model

Model owns logical data, queries, state domains, observations, reactive transitions, and maintained logical correspondence independently of one mandatory physical storage model.

## Boundary rule

A value, resource, observation, or authority crossing between strata has meaning only through an explicit semantic bridge. Shared physical representation does not erase a semantic boundary.