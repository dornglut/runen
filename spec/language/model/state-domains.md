# Model State Domains

Status: **provisional normative; incomplete**

A **state domain** controls a coherent set of logical state, invariants, revisions, admission, and commits.

Model state-domain semantics use state-domain control, admission, and commit terminology for those responsibilities rather than using **authority** as their primary ownership term.

No state domain is implicitly process-global.

A state-domain identity is abstract semantic identity. It is not a Model value, Core storage place or storage instance, source declaration, physical database or storage identity, address, index, ECS world or entity identity, clock, transaction, state revision, or other realization token merely because one implementation can associate those things with a domain.

## First represented observed-root profile

The first represented state-backed Model evaluation profile is deliberately bounded.

A state domain participates in this profile by fixing exactly one represented Model logical type `T` as its **observed logical-root type**. An admitted observation of that domain determines the logical root value consumed by state-backed from-scratch Model evaluation under the observation relation in [Model observation](observation.md).

The observed logical-root type is a Model logical type from [Model logical data](data.md). Associating the domain with `T` does not make the root a Core value, source binding or declaration, physical row set, storage object, index, allocation, ECS query result, or serialized representation. If `T` is a record or collection type, it retains exactly the existing Model data semantics, including the absence of hidden row, entity, allocation, address, or storage identity.

This one-root shape is the first proving profile only. It does not require every future state-domain contract to expose exactly one logical root, use one universal state interface, or represent richer logical source/catalog structure by this same profile. A later consumer that requires multiple logical sources or another state-domain shape needs its own accepted contract.

The profile defines only the logical root type and its relationship to admitted observations. It does not define an acquisition API, storage lifetime, retention duration, reacquisition guarantee, unavailable-observation failure, durability, replication, recovery, physical snapshot mechanism, materialization strategy, or incremental realization.

A **state revision** identifies version or progress according to a state-domain contract. A state revision is not a clock domain or causal frontier.

The observed-root profile does not identify an observation with a state revision and does not define any required mapping, order, predecessor/successor relation, timestamp, frame, transaction, freshness position, ECS change cursor, or causal relation between them.

Except for the bounded observed-root profile above, the exact general state-domain interface, revision ordering, visibility, durability, failure, and replication contracts are not defined by this revision.
