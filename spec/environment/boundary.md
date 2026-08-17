# Standard Environment Boundary

Status: **provisional normative boundary**

The Runen Standard Environment contains portable facilities that are standardized without becoming fundamental language primitives.

A library namespace, runtime service, or common realization technique does not become a language primitive merely because a standard implementation provides it.

A profile that requires a Standard Environment facility MUST state that dependency explicitly.

A Standard Environment contract may expose language-level concepts but MUST preserve their normative language semantics rather than redefine them.

A facility belongs in the Standard Environment only when interoperability or common portable source requires a shared contract and the contract can be standardized without making one incidental realization architecture normative.