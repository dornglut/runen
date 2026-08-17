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

## Lifecycle terms

A program is **well-formed** when it satisfies the applicable structural and grammar rules needed to form a candidate program for language validation.

A program is **valid** when it satisfies the applicable language and claimed-profile validation rules.

A valid program is **admissible** for an environment when every applicable hard environment requirement has been accepted by that environment.

An admitted program is **realized** when a legal physical implementation has been selected subject to the language semantics and admitted environment contracts.

These terms describe distinct boundaries. Well-formedness does not imply validity, validity does not imply environment admission, and admission does not make every physical realization legal.

## Language validation

Language validation determines whether a program satisfies the rules of the claimed language/profile, including applicable syntax, names, types, ownership, effects, resources, and statically checkable unsafe preconditions.

## Environment admission

Admission checks hard environment requirements such as execution features, authority, memory capabilities, ABI requirements, realtime guarantees, or other profile-defined facilities.

A hard requirement MUST either be admitted or rejected. It MUST NOT silently degrade into an optimization preference.

## Realization

Realization chooses a legal physical implementation subject to language semantics and admitted environment contracts.

Placement, scheduling, transfer, layout, specialization, materialization, and incremental maintenance are realization choices only where the specification permits them.

## Requirement and preference

A **requirement** constrains correctness or admission.

A **preference** is an optimization request that an implementation MAY ignore.
