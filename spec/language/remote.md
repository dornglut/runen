# Remote Boundaries

Status: **provisional normative**

Remote interaction is not shared memory.

A Network or distributed profile must define the message/protocol, failure, ordering, identity, serialization, observation, authority, and consistency contracts it claims.

Ordinary references, borrows, raw pointers, or Buffer physical addresses MUST NOT silently acquire remote-shared-memory meaning merely because an implementation can communicate with another machine.

Network protocols, CRDTs, replication strategies, RPC systems, and distributed transactions are optional mechanisms or profile facilities rather than universal Core or Model semantics.