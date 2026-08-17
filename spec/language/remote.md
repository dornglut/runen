# Remote Boundaries

Status: **provisional normative**

Remote interaction is not shared memory.

Ordinary references, borrows, raw pointers, or Buffer physical addresses MUST NOT silently acquire remote-shared-memory meaning because communication is available.

A remote or distributed contract must define the failure, ordering, identity, serialization, observation, authority, and consistency properties on which programs may rely.

A **causal frontier** describes causal knowledge or ordering when such a contract defines one. A causal frontier is not a clock domain or state revision.