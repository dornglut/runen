# Language Lifecycle

Status: **provisional normative**

Runen distinguishes these semantic phases:

```text
Parse
  ↓
Language validation
  ↓
Environment admission
  ↓
Realization
  ↓
Execution
```

An implementation need not literally implement five runtime/compiler stages.

## Language validation

Language validation determines whether a program satisfies the rules of the claimed language/profile, including applicable syntax, names, types, ownership, effects, resources, and statically checkable unsafe preconditions.

## Environment admission

Admission checks hard environment requirements such as execution features, authority, memory capabilities, ABI requirements, realtime guarantees, or other profile-defined facilities.

A hard requirement MUST either be admitted or rejected. It MUST NOT silently degrade into an optimization preference.

## Realization

Realization chooses a legal physical implementation subject to language semantics and admitted environment contracts.

Placement, scheduling, transfer, layout, specialization, materialization, and incremental maintenance may be realization choices only where the specification permits them.

## Requirement and preference

A **requirement** constrains correctness or admission.

A **preference** is an optimization request that an implementation MAY ignore.