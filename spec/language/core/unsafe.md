# Core Unsafe Semantics

Status: **provisional normative; incomplete**

Safe Runen MUST NOT require a safe caller to satisfy hidden undefined-behavior preconditions absent from its safe contract.

An unsafe operation may expose proof obligations that cannot be established automatically.

A safe abstraction implemented using unsafe operations MUST discharge those obligations for every use permitted by its safe public contract.

The complete value-validity model, unsafe-operation set, unsafe preconditions, and undefined-behavior taxonomy are not defined by this revision.