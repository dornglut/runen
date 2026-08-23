# Source Operator Semantics

Status: **provisional normative; incomplete**

This document owns the represented source-semantic operator relations: each represented operator's operand and result source types, successful semantic value transformation, operator-local ownership consequence, and operation-specific source-to-Core refinement boundary.

It consumes the represented source type identities and semantic value domains from [Source type foundation](types.md), including the exact two-value `Bool` domain. Its represented Core refinement consumes Bool-valued conditional branching from [Core control flow](../core/control-flow.md) and constant values plus wholly-vacant non-replacing initialization from [Core value and storage semantics](../core/value-storage.md).

[Source concrete syntax](concrete-syntax.md) owns the punctuation and concrete prefix grammar that map to the operator relation defined here. [Source function execution](function-execution.md) consumes this relation when validating and executing the operand producer, propagating its fault or divergence behavior, and transferring a successful operator result into an existing receiving position. [Source control flow](control-flow.md) consumes a completed operator result only through its existing exact-`Bool` `ConditionalValue` relation. This document does not redefine those owners.

This revision does not define a universal source expression taxonomy, parser precedence model, implementation HIR shape, runtime operator object, or backend instruction selection.

## Represented operator family

The represented source operator family contains exactly one operation in this revision: **Boolean logical negation**.

Boolean logical negation is a prefix value-producing operation. The represented concrete placements and `!` spelling are owned by `concrete-syntax.md`; the semantic operation does not depend on the original punctuation token after source validation.

No arithmetic, comparison, equality, inequality, short-circuit logical, conversion, cast, compound-assignment, numeric-negation, subtraction, postfix, member, or other operator is introduced by this relation.

## Boolean logical negation typing

Boolean logical negation has exactly one operand and exactly one result.

The operand required source type is exactly the intrinsic source type `Bool`.

The result source type is intrinsically exactly `Bool`.

No other source type is accepted as the operand type. In particular, this relation introduces no truthiness, integer-to-Bool conversion, numeric interpretation, coercion, promotion, defaulting, subtyping, structural conversion, or second Bool-like type.

The result type is an intrinsic fact of the operator relation; it is not inferred from the surrounding receiving position. Validation/evaluation sequencing between that intrinsic result fact, the surrounding required type, and the operand producer is owned by `function-execution.md`.

## Boolean logical negation value relation

Let `b` be the successfully produced semantic `Bool` operand value.

Boolean logical negation produces exactly the opposite semantic `Bool` value:

- when `b` is `true`, the result is `false`;
- when `b` is `false`, the result is `true`.

These two cases are exhaustive because `types.md` defines exactly two semantic `Bool` values.

The operation is deterministic. Equal semantic Bool operands produce equal semantic Bool results independently of source spelling, implementation representation, target, backend, optimization level, or host-language behavior.

## Operator-local ownership and execution effects

Boolean logical negation receives one already successfully produced owned `Bool` operand and produces one owned `Bool` result.

The logical-negation operation itself:

- consumes no parameter or local binding directly;
- duplicates or transfers no binding-owned structural path directly;
- changes no binding structural-ownership state;
- creates no source binding, address, reference, or source-visible storage identity;
- introduces no defined-fault reason;
- introduces no divergence possibility after successful operand production; and
- introduces no runtime flag or hidden source state.

Any binding ownership transition, defined fault, divergence, transient lifetime, or other effect needed to produce the operand remains entirely the consequence of that operand producer and its existing owner. `function-execution.md` owns the sequencing and transactional source-validation boundary that determines when those operand consequences commit.

The successful result is one ordinary owned value of intrinsic type `Bool`. Its owned-value duplicability is the existing intrinsic duplicability classification from `types.md`; this operator introduces no second duplicability or copyability rule.

## Conditional-use relationship

When a completed Boolean-logical-negation result is used as the condition of a represented `if` or `while`, `control-flow.md` consumes the resulting owned `Bool` exactly like any other admitted `ConditionalValue` result.

Logical negation itself changes only the semantic Bool value. It does not change the enclosing binding environment after successful operand production. Consequently, the post-condition binding environment for such a condition is exactly the operand producer's successful post-evaluation binding environment as established by `function-execution.md`.

This relationship does not add truthiness, constant-branch pruning, a source state set, a join/meet/widening relation, or a second conditional-selection rule.

## Typed frontend boundary

After successful source validation, a faithful typed frontend must retain enough information to identify:

- one Boolean logical-negation value producer;
- its complete recursively contained operand value;
- exact operand type `Bool`;
- exact result type `Bool`; and
- the source location required by the implementation's diagnostic/source-mapping contract.

A minimal typed representation may be equivalent to:

```text
ValueKind::BooleanNot {
    operand: Box<Value>,
}
```

where both the outer value and operand have source type `Bool`.

This explanatory shape is not an implementation-layout mandate. Token spelling, a numeric precedence value, associativity metadata, Core block identities, source-CFG identities, source ownership-state sets, and runtime operator tags are not semantic facts required after validation.

A faithful lowerer MUST reject internally inconsistent retained operator facts, such as a non-`Bool` operand or result, rather than repairing them from lower/Core type information.

## Source-to-Core refinement

Boolean logical negation requires no new Core operation.

After the complete source operand has successfully lowered to one fresh Core local containing its owned Bool result, a faithful refinement MAY use the already represented Core control-flow/value-storage relations with this semantic shape:

1. create one fresh Core local of Bool type for the negation result; the local is initially wholly vacant;
2. create a true-target block, a false-target block, and a join block;
3. terminate the current block with one represented Core `Branch` whose condition is an ownership-transferring `Move` of the operand-result local;
4. in the true-target block, `Init` the result local from semantic Bool constant `false`, then `Goto` the join block;
5. in the false-target block, `Init` the result local from semantic Bool constant `true`, then `Goto` the join block;
6. continue lowering from the join block with the result local live and available as the lowered operator result.

This refinement is valid under the accepted Core owners because:

- `Branch` evaluates its Bool-valued operand exactly once;
- the moved operand local is a compiler representation of the already completed source operand result, so moving it adds no source binding ownership transition;
- the fresh result local is wholly vacant on each branch path before that path's `Init`;
- each concrete execution takes exactly one branch and therefore executes exactly one of the two result initializations;
- Core path-state validation may propagate both branch paths to the same join while retaining their states independently rather than inventing a union, meet, join, or widening state;
- on every reachable incoming state at the join, the same result local is live with Bool type;
- any operand call fault, operand divergence, or other lack of successful operand completion prevents this negation branch/result sequence from being reached; and
- recursive logical negations may apply the same relation repeatedly to the previously lowered Bool result.

The source semantic truth relation is the authority for the opposite constants selected on the two branches. A host-language Boolean-negation operator, constant folder, backend instruction, or target convention is not semantic authority.

An implementation MAY use another Core program shape only when it is observationally equivalent under the accepted Core semantics and preserves the source relation above. This section does not introduce a Core `Not` operation, Core numeric operation, reference-machine extension, or backend requirement.

## Deliberate boundaries

This revision defines no:

- numeric unary negation or subtraction;
- arithmetic operator;
- equality, inequality, ordering, or other comparison operator;
- `&&`, `||`, short-circuiting, or another logical operator;
- conversion, cast, coercion, or numeric promotion;
- grouping or parenthesized expression;
- general binary precedence or associativity hierarchy;
- arbitrary postfix/member/method expression;
- assignment expression, compound assignment, field assignment, or general place/lvalue operation;
- source numeric-contract selection or scoping;
- refutable, literal, alternative, or guard pattern, or `match` relation;
- reference/borrow/lifetime/source-`unsafe` operation;
- generic/trait/coherence operation;
- function value, closure, or capture operation;
- const/static evaluation relation;
- additional fault/panic/catch/recovery operation;
- ABI/layout/FFI/linkage operation; or
- Exec, Model, runtime, or backend source operation.

Those concerns require their own accepted semantic owners and concrete consumers before this operator family is extended.
