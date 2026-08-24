# Source Operator Semantics

Status: **provisional normative; incomplete**

This document owns the represented source-semantic operator relations: each represented operator's operand and result source types, successful semantic value transformation, operator-local ownership consequence, and operation-specific source-to-Core refinement boundary.

It consumes the represented source type identities and semantic value domains from [Source type foundation](types.md), including the exact two-value `Bool` domain and the represented fixed-width integer domains. Plain fixed-width integer addition consumes the exact mathematical-addition and modulo-overflow value relation from [Core integer semantics](../core/numerics/integers.md). Its represented Core refinements consume Bool-valued conditional branching from [Core control flow](../core/control-flow.md) and constant values, wholly-vacant non-replacing initialization, and represented Core `IntegerAdd` result initialization from [Core value and storage semantics](../core/value-storage.md).

[Source concrete syntax](concrete-syntax.md) owns the punctuation, concrete prefix/additive/equality grammar, and bounded contextual grouping grammar that map source forms to the operator relations defined here. [Source function execution](function-execution.md) consumes these relations when validating and executing operand producers, sequencing multiple operands, propagating fault or divergence behavior, managing any operation-owned produced operand value that must remain live while a later operand executes, transparently executing any surrounding grouped-value wrapper, and transferring a successful operator result into an existing receiving position. [Source control flow](control-flow.md) consumes a completed operator result only through its existing exact-`Bool` `ConditionalValue` relation. This document does not redefine those owners.

This revision does not define a universal source expression taxonomy, parser implementation strategy, implementation HIR layout, runtime operator object, or backend instruction selection.

## Represented operator family

The represented source operator family contains exactly four operations in this revision:

- **Boolean logical negation**;
- **plain fixed-width integer addition**;
- **Boolean equality**; and
- **Boolean inequality**.

Boolean logical negation is a prefix value-producing operation. Plain fixed-width integer addition and Boolean equality/inequality are bounded binary value-producing operations. Their represented concrete placements, `!`, `+`, `==`, and `!=` spellings, bounded additive/equality tiers, and grouping relationship are owned by `concrete-syntax.md`; the semantic operations do not depend on the original punctuation tokens after source validation.

No arithmetic beyond the represented plain fixed-width integer addition, ordering, numeric comparison, structural or record comparison, floating comparison, pointer comparison, short-circuit logical, conversion, cast, compound-assignment, numeric-negation, subtraction, postfix, member, or other operator is introduced by these relations.

## Boolean logical negation typing

Boolean logical negation has exactly one operand and exactly one result.

The operand required source type is exactly the intrinsic source type `Bool`.

The result source type is intrinsically exactly `Bool`.

No other source type is accepted as the operand type. In particular, this relation introduces no truthiness, integer-to-Bool conversion, numeric interpretation, coercion, promotion, defaulting, subtyping, structural conversion, or second Bool-like type.

The result type is an intrinsic fact of the operator relation; it is not inferred from the surrounding receiving position. Validation/evaluation sequencing between that intrinsic result fact, the surrounding required type, and the operand producer is owned by `function-execution.md`.

## Plain fixed-width integer addition typing

Plain fixed-width integer addition has exactly two operands, ordered **left** and **right**, and exactly one result.

Unlike the represented Boolean operators, the addition relation is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Addition is source-admissible only when `T` is exactly one of these intrinsic source types:

- `I8`, `I16`, `I32`, or `I64`; or
- `U8`, `U16`, `U32`, or `U64`.

When `T` is admitted:

- the left operand required source type is exactly `T`;
- the right operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand types do not independently infer, choose, or alter `T`. A surrounding non-integer required type makes the addition source-invalid before operand validation may commit a binding ownership consequence. A surrounding integer type different from an operand producer's own exact type causes that operand to fail its existing exact required-type validation rather than causing a conversion or promotion.

This relation defines no mixed-width or mixed-signedness arithmetic, integer promotion, widening, narrowing, coercion, conversion, default numeric type, overload resolution, trait dispatch, generic arithmetic, or result-type inference from operand syntax. Decimal integer literals may materialize as `T` only through their existing context-required literal relation; addition introduces no second literal-typing rule.

The context-directed result/operand requirement and complete two-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Boolean equality and inequality typing

Boolean equality and Boolean inequality each have exactly two operands, ordered **left** and **right**, and exactly one result.

For both operations:

- the left operand required source type is exactly intrinsic `Bool`;
- the right operand required source type is exactly intrinsic `Bool`; and
- the result source type is intrinsically exactly `Bool`.

No other source type is accepted for either operand. In particular, these operations define no equality, inequality, ordering, or comparison over fixed-width integers, binary floating values, nominal records, raw pointers, future references, functions, or another source value family.

Source type equality from `types.md` is not source value equality. Nominal identity, structural similarity, duplicability, physical representation equality, bit equality, host equality, or backend comparison behavior does not make a value admissible to these Boolean operations.

The intrinsic Bool result is established before operand validation may commit binding ownership consequences. Complete two-operand transactional validation and eager left-to-right execution are owned by `function-execution.md`.

## Boolean logical negation value relation

Let `b` be the successfully produced semantic `Bool` operand value.

Boolean logical negation consumes that owned operand value exactly once and produces exactly one distinct owned `Bool` result with the opposite semantic value:

- when `b` is `true`, the result is `false`;
- when `b` is `false`, the result is `true`.

These two cases are exhaustive because `types.md` defines exactly two semantic `Bool` values.

Ownership of the successful operand result ends at this operator application. The consumed operand result is not duplicated and receives no independent cleanup after the result has been produced.

The operation is deterministic. Equal semantic Bool operands produce equal semantic Bool results independently of source spelling, implementation representation, target, backend, optimization level, or host-language behavior.

## Plain fixed-width integer addition value relation

Let `T` be the admitted exact fixed-width integer source type selected by the surrounding required type, and let `l` and `r` be the two successfully produced semantic operand values of `T`.

Plain integer addition consumes both owned operand values exactly once. It first forms the exact mathematical integer sum

```text
x = l + r
```

and then produces exactly one distinct owned result of source type `T` by applying the plain fixed-width modulo-`2^N` signed/unsigned mapping owned by `core/numerics/integers.md` for the width and signedness corresponding to `T`.

There is no intermediate fixed-width truncation before the exact sum. If `x` lies outside the value interval of `T`, the accepted plain-overflow mapping determines the wrapped semantic result; overflow does not by itself select a source fault, undefined behavior, checked outcome, or saturated result.

This relation is total after two valid `T` operand values have been produced. Its result is independent of host integer overflow, physical representation, optimizer assumptions, debug/release configuration, backend flags, or target instructions.

The relation does not define a source checked, saturating, or explicitly wrapping mode. It also does not define floating addition, subtraction, multiplication, division, remainder, shifts, bitwise operations, conversions, or numeric comparison.

## Boolean equality value relation

Let `l` and `r` be the successfully produced semantic left and right `Bool` operand values.

Boolean equality consumes both owned operand values exactly once and produces exactly one distinct owned `Bool` result according to this exhaustive table:

| `l` | `r` | result |
| --- | --- | --- |
| `false` | `false` | `true` |
| `false` | `true` | `false` |
| `true` | `false` | `false` |
| `true` | `true` | `true` |

Equivalently, the result is `true` exactly when `l` and `r` are the same member of the two-value Bool domain.

This equivalence statement is local to the accepted Bool domain. It does not establish a generic value-equality relation, a structural equality relation, or equality for any other represented source type.

## Boolean inequality value relation

Let `l` and `r` be the successfully produced semantic left and right `Bool` operand values.

Boolean inequality consumes both owned operand values exactly once and produces exactly one distinct owned `Bool` result according to this exhaustive table:

| `l` | `r` | result |
| --- | --- | --- |
| `false` | `false` | `false` |
| `false` | `true` | `true` |
| `true` | `false` | `true` |
| `true` | `true` | `false` |

Equivalently, the result is `true` exactly when `l` and `r` are different members of the two-value Bool domain.

The accepted truth tables are semantic authority. A host-language equality/inequality operator, target compare instruction, constant folder, optimizer, or backend convention is not semantic authority for either relation.

## Operator-local ownership and execution effects

Boolean logical negation receives one already successfully produced owned `Bool` operand, consumes that operand as defined above, and produces one owned `Bool` result.

Plain fixed-width integer addition receives two already successfully produced owned values of the same admitted fixed-width integer source type `T`, consumes both operands exactly once as defined above, and produces one owned `T` result.

Boolean equality and inequality each receive two already successfully produced owned `Bool` operand values, consume both operands exactly once as defined above, and produce one owned `Bool` result.

Each represented operator itself:

- consumes no parameter or local binding directly;
- duplicates or transfers no binding-owned structural path directly;
- changes no binding structural-ownership state;
- creates no source binding, address, reference, place, or source-visible storage identity;
- introduces no defined-fault reason;
- introduces no divergence possibility after all of its required operand values have been produced; and
- introduces no runtime flag or hidden source state.

Any binding ownership transition, defined fault, divergence, transient lifetime, or other effect needed to produce an operand remains the consequence of that operand producer and the sequencing relation in `function-execution.md`.

For each represented binary operator, successful left production necessarily precedes right production under `function-execution.md`, so the in-progress operation owns the produced left value until right production either succeeds, faults, or remains suspended by divergence. For equality/inequality that value is `Bool`; for integer addition it is the selected exact integer type `T`. That bounded lifetime is an execution/cleanup sequencing fact owned by `function-execution.md`, not a new source binding or storage identity.

A successfully validated represented operator adds no binding structural-ownership transition beyond the committed consequences of its operand producer or producers.

Every successful result is one ordinary owned value of its operator's exact result type. Intrinsic Bool and fixed-width integer result duplicability is the existing intrinsic duplicability classification from `types.md`; these operators introduce no second duplicability or copyability rule.

## Contextual grouping relationship

Parenthesized grouping is concrete syntax around one already represented value producer; it is not a fifth operator and defines no operator-local type, value, ownership, fault, divergence, or Core relation. `concrete-syntax.md` owns the ordinary and conditional grouped-value grammar, and `function-execution.md` owns its semantic transparency.

When a group contains a represented operator, the contained operator retains exactly its typing, semantic value relation, operand ordering, ownership consequences, held-left lifetime where applicable, fault/divergence behavior, and source-to-Core refinement defined in this document. The parentheses add no operator step before, between, or after those relations.

Grouping may make explicit a syntax-tree nesting that the unparenthesized bounded grammar does not represent. For example, `!(a == b)` contains one grouped equality value as the operand of logical negation, `(a == b) == c` contains one grouped inner equality as the left operand of an outer equality, and `a == (b != c)` contains one grouped inner inequality as the right operand. For addition, `(a + b) + c` and `a + (b + c)` contain explicitly grouped inner additions and do not make ungrouped `a + b + c` represented. These forms introduce neither equality nor addition associativity or chaining.

No precedence number, associativity metadata, grouping operator identity, runtime parenthesis object, or Core grouping operation follows from this concrete nesting. A faithful typed frontend may erase the grouping wrapper while retaining the already required contained operator/value facts.

## Conditional-use relationship

When a completed represented operator result is used as the condition of a represented `if` or `while`, `control-flow.md` requires the resulting owned value to have exact source type `Bool`.

Logical negation changes only one successful operand's semantic Bool value. Its post-condition binding environment is exactly the operand producer's successful post-evaluation environment as established by `function-execution.md`.

Boolean equality/inequality eagerly execute the complete left producer and then the complete right producer before producing their Bool result. Their successful post-condition binding environment is therefore exactly the right producer's successful post-evaluation environment after the already-completed left producer consequences. The Bool truth relation adds no further binding transition.

The concrete conditional grammar may syntactically contain the bounded additive tier so that one grammar hierarchy remains explicit and context-preserving. A plain integer addition nevertheless cannot satisfy the condition's surrounding exact required type `Bool`. `function-execution.md` therefore rejects such an addition at outer required-type admission before either addition operand is validated in a way that may commit ownership. No completed integer-add result reaches conditional selection.

None of these relationships adds truthiness, constant-branch pruning, a source state set, a join/meet/widening relation, implicit restoration, or a second conditional-selection rule.

## Typed frontend boundary

After successful source validation, a faithful typed frontend must retain enough information to identify every represented operator and its complete recursively contained operand values with their exact required/result type facts.

For Boolean logical negation, a minimal typed representation may be equivalent to:

```text
ValueKind::BooleanNot {
    operand: Box<Value>,
}
```

where both the outer value and operand have source type `Bool`.

For plain fixed-width integer addition, a minimal typed representation may be equivalent to:

```text
ValueKind::IntegerAdd {
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all carry the same exact admitted fixed-width integer source type `T` selected by the surrounding receiving requirement.

For Boolean equality/inequality, a minimal typed representation may be equivalent to:

```text
BooleanEqualityRelation::{Equal, NotEqual}

ValueKind::BooleanEquality {
    relation: BooleanEqualityRelation,
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all have source type `Bool`.

A faithful implementation MAY instead use another explicit typed layout when it retains exactly the same source-semantic facts and no speculative generalized abstraction.

These explanatory shapes are not implementation-layout mandates. Source locations may be retained by diagnostics/tooling but are not part of an operator's semantic identity. Token spelling, numeric precedence values, associativity metadata, grouping delimiters, Core block identities, source-CFG identities, source ownership-state sets, runtime operator objects, and backend opcodes are not semantic facts required after validation.

A faithful lowerer MUST reject internally inconsistent retained operator facts rather than repairing them from lower/Core type information. This includes a non-`Bool` Boolean operator result or operand, an integer-add result whose type is not one of the eight admitted fixed-width integer types, or integer-add operands whose retained exact source type differs from the retained addition result type.

## Source-to-Core refinement

The represented Boolean operators require no new Core operation. Plain fixed-width integer addition refines to exactly the represented Core `IntegerAdd` relation owned by `core/value-storage.md` and its numerical relation in `core/numerics/integers.md`.

### Plain fixed-width integer addition refinement

A faithful integer-add lowerer first lowers the complete left producer exactly once and then lowers the complete right producer exactly once from the successful left continuation. The lowered operand locals MUST carry the same exact Core type identity corresponding to the admitted source type `T`; malformed retained source facts are rejected rather than converted or repaired.

After both operand-producer lowerings succeed:

1. create one fresh Core result local of that same exact Core integer type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `IntegerAdd` whose destination is that result local, whose left operand is an ownership-transferring `Move` of the left operand-result local, and whose right operand is an ownership-transferring `Move` of the right operand-result local; and
3. continue lowering with the result local live and available as the lowered source addition result.

This refinement is valid under the accepted Core owners because:

- source left-producer execution completes before source right-producer execution begins;
- the lowered left result local remains live while the complete right producer lowers/executes, representing the source operation's held-left value;
- if left producer execution faults or diverges, right producer execution and `IntegerAdd` are never reached;
- if right producer execution faults after left success, `IntegerAdd` is never reached and the existing Core activation fault cleanup ends then-live compiler locals, including the local retaining the produced left integer value;
- if right producer execution diverges, the caller remains suspended and the left operand-result local remains live;
- once both producers succeed, `IntegerAdd` evaluates its two `Move` operands left-to-right, consuming each produced operand local exactly once;
- the fresh result local satisfies the operation's wholly-vacant non-replacing destination rule;
- the Core numerical relation computes the exact mathematical sum and accepted plain fixed-width result for the same type `T`;
- successful `IntegerAdd` leaves exactly the result local live and introduces no comparison branch, join, fault, or divergence; and
- nested represented additions may recursively lower complete operand producers before their enclosing addition emits its own one Core `IntegerAdd`.

A conforming implementation MAY use another Core program shape only when it is observationally equivalent under the accepted Core semantics and preserves the exact source type, left-to-right producer ordering, successful operand consumption, ownership/fault/divergence behavior, plain-overflow result, and one-result relation. It may not replace the accepted semantics with host arithmetic assumptions or implicit conversions.

### Boolean logical negation refinement

After the complete source operand has successfully lowered to one fresh Core local containing its owned Bool result, a faithful refinement MAY use the already represented Core control-flow/value-storage relations with this semantic shape:

1. create one fresh Core local of Bool type for the negation result; the local is initially wholly vacant;
2. create a true-target block, a false-target block, and a join block;
3. terminate the current block with one represented Core `Branch` whose condition is an ownership-transferring `Move` of the operand-result local;
4. in the true-target block, `Init` the result local from semantic Bool constant `false`, then `Goto` the join block;
5. in the false-target block, `Init` the result local from semantic Bool constant `true`, then `Goto` the join block;
6. continue lowering from the join block with the result local live and available as the lowered operator result.

This refinement is valid under the accepted Core owners because:

- `Branch` evaluates its Bool-valued operand exactly once;
- the `Move` consumes the lowered owned operand result exactly once, matching the source operator's operand-value consumption;
- the moved operand local is a compiler representation of the already completed source operand result, so that move adds no source binding structural-ownership transition;
- the fresh result local is wholly vacant on each branch path before that path's `Init`;
- each concrete execution takes exactly one branch and therefore executes exactly one of the two result initializations;
- Core path-state validation may propagate both branch paths to the same join while retaining their states independently rather than inventing a union, meet, join, or widening state;
- on every reachable incoming state at the join, the same result local is live with Bool type;
- any operand call fault, operand divergence, or other lack of successful operand completion prevents this negation branch/result sequence from being reached; and
- recursive logical negations may apply the same relation repeatedly to the previously lowered Bool result.

The source semantic truth relation is the authority for the opposite constants selected on the two branches.

### Boolean equality and inequality refinement

A faithful equality/inequality lowerer first lowers the complete left producer exactly once and then the complete right producer exactly once. Both lowering operations complete before comparison branching begins. Each produced local MUST have Core Bool type; malformed retained source facts are rejected rather than repaired.

After both Bool operand locals exist, a faithful refinement MAY use this four-leaf truth-table shape:

1. create one fresh Core Bool result local, initially wholly vacant;
2. create left-true and left-false blocks, four truth-table leaf blocks, and one join block;
3. terminate the then-current block with `Branch` whose condition is an ownership-transferring `Move` of the left operand local;
4. in each mutually exclusive left-successor block, terminate with `Branch` whose condition is an ownership-transferring `Move` of the right operand local;
5. in each of the four truth-table leaf blocks, `Init` the same result local from the semantic Bool constant required by the selected equality or inequality table, then `Goto` the common join;
6. continue lowering from the join with the result local live and available as the lowered operator result.

This refinement is valid under the accepted Core owners because:

- the source's eager left-to-right producer execution is complete before the first comparison `Branch`;
- each concrete successful execution moves the left operand local exactly once;
- after that branch, the right operand local remains live in both independently propagated Core path states;
- Core path-state validation retains those states independently, so each mutually exclusive left-successor may move the right operand local exactly once under its own incoming state without implying two moves in one concrete execution;
- each concrete successful execution therefore moves the right operand exactly once;
- the fresh result local is wholly vacant on every truth-table leaf before that leaf's `Init`;
- exactly one leaf executes on one concrete successful run and initializes the result exactly once;
- every reachable join state has both operand locals consumed and the same result local live with Bool type;
- if left producer execution faults or diverges, right producer execution and the comparison CFG are never reached;
- if right producer execution faults after left success, no comparison CFG or result is reached and the containing Core activation's existing fault cleanup ends then-live compiler locals, including the local retaining the already-produced left Bool;
- if right producer execution diverges, the caller remains suspended and the left operand local remains live, matching the source operation's held-left lifetime; and
- nested represented operators may recursively lower complete operand producers before their enclosing operator constructs its own comparison CFG.

A conforming implementation MAY use another Core program shape only when it is observationally equivalent under the accepted Core semantics and preserves the source relations above. In particular, an implementation may reduce the number of blocks but may not change operand evaluation order, successful operand consumption, truth mapping, fault/divergence behavior, or the one-result relation.

The Boolean refinements introduce no Core `Not`, `Eq`, `Ne`, comparison operation, grouping operation, reference-machine extension, source/Core state merge, host-language Boolean comparison authority, or backend requirement. The integer-add refinement introduces exactly the one represented Core `IntegerAdd` operation and no additional Core control-flow or numeric operation.

## Deliberate boundaries

This revision defines no:

- numeric unary negation or subtraction;
- arithmetic operator other than plain same-type fixed-width integer addition;
- floating-point addition or mixed-width/mixed-signedness integer addition;
- checked, saturating, or explicitly wrapping source addition mode;
- integer, floating, nominal-record, pointer, reference, function, structural, representation, or generic value equality/inequality;
- ordering or other comparison operator;
- `&&`, `||`, short-circuiting, or another logical operator;
- conversion, cast, coercion, numeric promotion, or operand-derived/default arithmetic typing;
- Unit/tuple grouping form or general expression system beyond the bounded contextual grouping grammar owned by `concrete-syntax.md`;
- ungrouped addition chaining, equality chaining, or comparison chaining;
- general binary precedence or associativity hierarchy beyond the bounded additive and equality tiers owned by `concrete-syntax.md`;
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
