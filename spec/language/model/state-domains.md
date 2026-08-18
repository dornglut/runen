# Model State Domains

Status: **provisional normative; incomplete**

A **state domain** controls a coherent set of logical state, invariants, revisions, admission, and commits.

Model state-domain semantics use state-domain control, admission, and commit terminology for those responsibilities rather than using **authority** as their primary ownership term.

No state domain is implicitly process-global.

A **state revision** identifies version or progress according to a state-domain contract. A state revision is not a clock domain or causal frontier.

The exact state-domain interface, revision ordering, visibility, durability, failure, and replication contracts are not defined by this revision.