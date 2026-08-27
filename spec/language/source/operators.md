# Source Operator Semantics

Status: **provisional normative; incomplete**

This document owns the represented source-semantic operator relations: each represented operator's operand and result source types, successful semantic value transformation, operator-local ownership consequence, and operation-specific source-to-Core refinement boundary.

It consumes the represented source type identities and semantic value domains from [Source type foundation](types.md), including the exact two-value `Bool` domain, represented fixed-width integer domains, and represented `F16`/`F32`/`F64` binary-floating domains. Plain fixed-width integer negation, addition, subtraction, and multiplication consume the exact mathematical operation-specific and modulo-overflow value relations from [Core integer semantics](../core/numerics/integers.md). Plain fixed-width integer exclusive-or and plain fixed-width integer bitwise OR consume the exact representation-neutral fixed-width exclusive-or and bitwise-OR relations from that same Core integer owner. Plain fixed-width integer bitwise complement is defined here as an operation-specific source relation over those same fixed-width integer value domains and consumes the accepted fixed-width modulo mapping only for its equivalent exact `-1 - v` characterization and Core refinement proof. Same-format binary floating addition and subtraction consume numeric-contract authority, the applicable selected-contract operation result relations, rounding, signed-zero, subnormal, infinity, NaN-class, and multi-operation composition semantics from [Core floating-point semantics](../core/numerics/floating-point.md). Their represented Core refinements consume Bool-valued conditional branching from [Core control flow](../core/control-flow.md) and constant values, wholly-vacant non-replacing initialization, and represented Core `IntegerAdd`/`IntegerSub`/`IntegerMul`/`IntegerXor`/`IntegerOr`/`FloatAdd`/`FloatSub` result initialization from [Core value and storage semantics](../core/value-storage.md).

[Source concrete syntax](concrete-syntax.md) owns the punctuation, concrete prefix/multiplicative/additive/exclusive-or/bitwise-OR/equality/logical-conjunction grammar, signed-literal priority, bounded contextual grouping grammar, and represented operation-local `fast` selected-value form that map source forms to the operator relations defined here. [Source function execution](function-execution.md) consumes these relations when validating and executing operand producers, sequencing multiple operands, establishing selected-value applicability before operand ownership can commit, propagating fault or divergence behavior, managing any operation-owned produced operand value that must remain live while a later eager operand executes, validating the definite ownership boundary of short-circuit conjunction, transparently executing any surrounding grouped-value wrapper, and transferring a successful operator result into an existing receiving position. [Source control flow](control-flow.md) consumes a completed operator result only through its existing exact-`Bool` `ConditionalValue` relation. This document does not redefine those owners.

This revision does not define a universal source expression taxonomy, parser implementation strategy, implementation HIR layout, runtime operator object, or backend instruction selection.

## Represented operator family

The represented source operator family contains exactly thirteen operations in this revision:

- **Boolean logical negation**;
- **plain fixed-width integer negation**;
- **plain fixed-width integer bitwise complement**;
- **plain fixed-width integer multiplication**;
- **plain fixed-width integer addition**;
- **same-format binary floating addition**;
- **plain fixed-width integer subtraction**;
- **same-format binary floating subtraction**;
- **plain fixed-width integer exclusive-or**;
- **plain fixed-width integer bitwise OR**;
- **Boolean equality**;
- **Boolean inequality**; and
- **Boolean short-circuit conjunction**.

Boolean logical negation, plain fixed-width integer negation, and plain fixed-width integer bitwise complement are prefix value-producing operations. Plain fixed-width integer multiplication, plain fixed-width integer addition, same-format binary floating addition, plain fixed-width integer subtraction, same-format binary floating subtraction, exclusive-or, bitwise OR, and Boolean equality/inequality are bounded eager binary value-producing operations. Boolean short-circuit conjunction is one bounded binary value-producing operation whose right operand is evaluated only after a successful `true` left operand. Integer addition and floating addition share the concrete `+` placement, while integer subtraction and floating subtraction share the concrete binary `-` placement; in each case the semantic operation is selected by the exact surrounding required type, and syntax does not define a generic numeric operation relation. Their represented concrete placements, `!`, prefix `-`, `~`, `*`, `+`, binary `-`, `^`, `|`, `==`, `!=`, and `&&` spellings, bounded prefix/multiplicative/additive/exclusive-or/bitwise-OR/equality/logical-conjunction tiers, signed-literal relationship, grouping relationship, and operation-local `fast` selected-value wrapper are owned by `concrete-syntax.md`; the semantic operations do not depend on the original punctuation tokens after source validation.

No arithmetic beyond the represented plain fixed-width integer negation, multiplication, addition, and subtraction and the represented same-format binary floating addition and subtraction, no binary bitwise operation beyond the represented plain fixed-width integer exclusive-or and bitwise OR, no shift, ordering, numeric comparison, structural or record comparison, floating comparison, pointer comparison, short-circuit logical operation beyond the represented Boolean conjunction, conversion, cast, compound-assignment, floating negation/multiplication/division/remainder, unary plus, postfix, member, or other operator is introduced by these relations. Plain fixed-width integer bitwise complement, plain fixed-width integer exclusive-or, and plain fixed-width integer bitwise OR are the only represented bitwise operations in this revision.

## Boolean logical negation typing

Boolean logical negation has exactly one operand and exactly one result.

The operand required source type is exactly the intrinsic source type `Bool`.

The result source type is intrinsically exactly `Bool`.

No other source type is accepted as the operand type. In particular, this relation introduces no truthiness, integer-to-Bool conversion, numeric interpretation, coercion, promotion, defaulting, subtyping, structural conversion, or second Bool-like type.

The result type is an intrinsic fact of the operator relation; it is not inferred from the surrounding receiving position. Validation/evaluation sequencing between that intrinsic result fact, the surrounding required type, and the operand producer is owned by `function-execution.md`.

## Plain fixed-width integer negation typing

Plain fixed-width integer negation has exactly one operand and exactly one result.

Like the represented binary integer arithmetic relations, integer negation is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Integer negation is source-admissible only when `T` is exactly one of these intrinsic source types:

- `I8`, `I16`, `I32`, or `I64`; or
- `U8`, `U16`, `U32`, or `U64`.

When `T` is admitted:

- the operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand does not independently infer, choose, or alter `T`. A surrounding non-integer required type makes integer negation source-invalid before operand validation may commit a binding ownership consequence. A surrounding integer type different from an operand producer's own exact type causes that operand to fail its existing exact required-type validation rather than causing a conversion or promotion.

This relation defines no mixed-width or mixed-signedness arithmetic, integer promotion, widening, narrowing, coercion, conversion, default numeric type, overload resolution, trait dispatch, generic arithmetic, or result-type inference from operand syntax. Decimal integer literals may materialize as `T` only through their existing context-required literal relation; integer negation introduces no second literal-typing rule and MUST NOT reinterpret a concrete signed decimal literal as application of this operator. The signed-literal-versus-prefix-operator distinction is owned by `concrete-syntax.md` and `literals.md` and remains semantically significant before operator validation.

The context-directed result/operand requirement and complete one-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Plain fixed-width integer bitwise complement typing

Plain fixed-width integer bitwise complement has exactly one operand and exactly one result.

Like plain fixed-width integer negation, complement is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Complement is source-admissible only when `T` is exactly one of these intrinsic source types:

- `I8`, `I16`, `I32`, or `I64`; or
- `U8`, `U16`, `U32`, or `U64`.

When `T` is admitted:

- the operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand does not independently infer, choose, or alter `T`. A surrounding non-integer required type makes complement source-invalid before operand validation may commit a binding ownership consequence. A surrounding integer type different from an operand producer's own exact type causes that operand to fail its existing exact required-type validation rather than causing a conversion or promotion.

This relation defines no mixed-width or mixed-signedness operation, integer promotion, widening, narrowing, coercion, conversion, default numeric type, overload resolution, trait dispatch, generic bitwise abstraction, or result-type inference from operand syntax. Decimal integer literals may materialize as `T` only through their existing context-required literal relation; complement introduces no second literal-typing rule.

The context-directed result/operand requirement and complete one-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Plain fixed-width integer addition typing

Plain fixed-width integer addition has exactly two operands, ordered **left** and **right**, and exactly one result.

Unlike the represented Boolean operators, the addition relation is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Integer addition is source-admissible only when `T` is exactly one of these intrinsic source types:

- `I8`, `I16`, `I32`, or `I64`; or
- `U8`, `U16`, `U32`, or `U64`.

When `T` is admitted:

- the left operand required source type is exactly `T`;
- the right operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand types do not independently infer, choose, or alter `T`. A surrounding non-integer required type makes integer addition source-invalid before operand validation may commit a binding ownership consequence. A surrounding integer type different from an operand producer's own exact type causes that operand to fail its existing exact required-type validation rather than causing a conversion or promotion.

This relation defines no mixed-width or mixed-signedness arithmetic, integer promotion, widening, narrowing, coercion, conversion, default numeric type, overload resolution, trait dispatch, generic arithmetic, or result-type inference from operand syntax. Decimal integer literals may materialize as `T` only through their existing context-required literal relation; integer addition introduces no second literal-typing rule.

The context-directed result/operand requirement and complete two-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Same-format binary floating addition typing

Same-format binary floating addition has exactly two operands, ordered **left** and **right**, and exactly one result.

Like represented integer addition, floating addition is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Floating addition is source-admissible only when `T` is exactly one of the intrinsic source types `F16`, `F32`, or `F64`.

When `T` is admitted:

- the left operand required source type is exactly `T`;
- the right operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand types do not independently infer, choose, or alter `T`. A surrounding required type that is not one of `F16`, `F32`, or `F64` makes this floating-addition relation source-invalid before operand validation may commit a binding ownership consequence. The same concrete `+` form selects the existing integer-addition relation instead when the exact surrounding required type is one of the eight represented fixed-width integer types. No operand syntax, literal shape, or operand type independently selects between those two semantic operations.

Every source-valid floating-addition occurrence has exactly one selected numeric contract before its operand transaction executes. An unqualified occurrence establishes no explicit source selection, so the accepted Core floating numeric-contract fallback establishes `standard`. A source-valid operation-local selected-value wrapper establishes `fast` for exactly its selected root floating-addition occurrence under the applicability relation in `function-execution.md`. This revision introduces no source spelling for `standard` or `reproducible`; the absence of a source `reproducible` spelling does not narrow the accepted lower semantic contract domain.

Numeric-contract selection does not alter the context-directed type relation above. It introduces no cross-format floating arithmetic, integer/floating mixed arithmetic, promotion, widening, narrowing, coercion, conversion, default numeric type, overload search, trait dispatch, generic numeric abstraction, or result-type inference from operand syntax. A decimal floating literal may materialize as `T` only through its existing exact context-required relation in `literals.md`; floating addition introduces no second literal-typing rule. A decimal integer literal is not reclassified as a floating literal merely because this operator supplies a floating required type. Signed decimal floating literal priority remains the existing literal relation and does not introduce floating unary negation.

The context-directed result/operand requirement, selected-contract applicability boundary, and complete two-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Plain fixed-width integer subtraction typing

Plain fixed-width integer subtraction has exactly two operands, ordered **left** and **right**, and exactly one result.

Like the represented addition relation, subtraction is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Subtraction is source-admissible only when `T` is exactly one of these intrinsic source types:

- `I8`, `I16`, `I32`, or `I64`; or
- `U8`, `U16`, `U32`, or `U64`.

When `T` is admitted:

- the left operand required source type is exactly `T`;
- the right operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand types do not independently infer, choose, or alter `T`. A surrounding non-integer required type makes the subtraction source-invalid before operand validation may commit a binding ownership consequence. A surrounding integer type different from an operand producer's own exact type causes that operand to fail its existing exact required-type validation rather than causing a conversion or promotion.

This relation defines no mixed-width or mixed-signedness arithmetic, integer promotion, widening, narrowing, coercion, conversion, default numeric type, overload resolution, trait dispatch, generic arithmetic, or result-type inference from operand syntax. Decimal integer literals may materialize as `T` only through their existing context-required literal relation; subtraction introduces no second literal-typing rule and does not reinterpret the accepted signed-literal relation.

The context-directed result/operand requirement and complete two-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Same-format binary floating subtraction typing

Same-format binary floating subtraction has exactly two operands, ordered **left** and **right**, and exactly one result.

Like represented integer subtraction, floating subtraction is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Floating subtraction is source-admissible only when `T` is exactly one of the intrinsic source types `F16`, `F32`, or `F64`.

When `T` is admitted:

- the left operand required source type is exactly `T`;
- the right operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand types do not independently infer, choose, or alter `T`. A surrounding required type that is not one of `F16`, `F32`, or `F64` makes this floating-subtraction relation source-invalid before operand validation may commit a binding ownership consequence. The same concrete binary `-` form selects the existing integer-subtraction relation instead when the exact surrounding required type is one of the eight represented fixed-width integer types. No operand syntax, literal shape, or operand type independently selects between those two semantic operations.

Every source-valid floating-subtraction occurrence has exactly one selected numeric contract before its operand transaction executes. An unqualified occurrence establishes no explicit source selection, so the accepted Core floating numeric-contract fallback establishes `standard`. A source-valid operation-local selected-value wrapper establishes `fast` for exactly its selected root floating-subtraction occurrence under the applicability relation in `function-execution.md`. This revision introduces no source spelling for `standard` or `reproducible`; the absence of a source `reproducible` spelling does not narrow the accepted lower semantic contract domain.

Numeric-contract selection does not alter the context-directed type relation above. It introduces no cross-format floating arithmetic, integer/floating mixed arithmetic, promotion, widening, narrowing, coercion, conversion, default numeric type, overload search, trait dispatch, generic numeric abstraction, or result-type inference from operand syntax. A decimal floating literal may materialize as `T` only through its existing exact context-required relation in `literals.md`; floating subtraction introduces no second literal-typing rule. A decimal integer literal is not reclassified as a floating literal merely because this operator supplies a floating required type. Signed decimal floating literals remain complete literals under the existing signed-literal priority and are not decomposed into a floating-negation operation. This relation introduces no floating unary negation.

The context-directed result/operand requirement, selected-contract applicability boundary, and complete two-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Plain fixed-width integer multiplication typing

Plain fixed-width integer multiplication has exactly two operands, ordered **left** and **right**, and exactly one result.

Like represented addition and subtraction, multiplication is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Multiplication is source-admissible only when `T` is exactly one of these intrinsic source types:

- `I8`, `I16`, `I32`, or `I64`; or
- `U8`, `U16`, `U32`, or `U64`.

When `T` is admitted:

- the left operand required source type is exactly `T`;
- the right operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand types do not independently infer, choose, or alter `T`. A surrounding non-integer required type makes multiplication source-invalid before operand validation may commit a binding ownership consequence. A surrounding integer type different from an operand producer's own exact type causes that operand to fail its existing exact required-type validation rather than causing a conversion or promotion.

This relation defines no mixed-width or mixed-signedness arithmetic, integer promotion, widening, narrowing, coercion, conversion, default numeric type, overload resolution, trait dispatch, generic arithmetic, or result-type inference from operand syntax. Decimal integer literals may materialize as `T` only through their existing context-required literal relation; multiplication introduces no second literal-typing rule and does not reinterpret the accepted signed-literal relation.

The context-directed result/operand requirement and complete two-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Plain fixed-width integer exclusive-or typing

Plain fixed-width integer exclusive-or has exactly two operands, ordered **left** and **right**, and exactly one result.

Like the represented same-type integer arithmetic relations, exclusive-or is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Exclusive-or is source-admissible only when `T` is exactly one of these intrinsic source types:

- `I8`, `I16`, `I32`, or `I64`; or
- `U8`, `U16`, `U32`, or `U64`.

When `T` is admitted:

- the left operand required source type is exactly `T`;
- the right operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand types do not independently infer, choose, or alter `T`. A surrounding non-integer required type makes exclusive-or source-invalid before operand validation may commit a binding ownership consequence. A surrounding integer type different from an operand producer's own exact type causes that operand to fail its existing exact required-type validation rather than causing a conversion or promotion.

This relation defines no mixed-width or mixed-signedness bitwise operation, integer promotion, widening, narrowing, coercion, conversion, default numeric type, overload resolution, trait dispatch, generic bitwise abstraction, or result-type inference from operand syntax. Decimal integer literals may materialize as `T` only through their existing context-required literal relation; exclusive-or introduces no second literal-typing rule.

The context-directed result/operand requirement and complete two-operand transactional validation/eager execution are consumed by `function-execution.md`.

## Plain fixed-width integer bitwise-OR typing

Plain fixed-width integer bitwise OR has exactly two operands, ordered **left** and **right**, and exactly one result.

Like the represented exclusive-or relation, bitwise OR is deliberately **context-directed by one exact surrounding required source type**. Let `T` be that required type. Bitwise OR is source-admissible only when `T` is exactly one of these intrinsic source types:

- `I8`, `I16`, `I32`, or `I64`; or
- `U8`, `U16`, `U32`, or `U64`.

When `T` is admitted:

- the left operand required source type is exactly `T`;
- the right operand required source type is exactly `T`; and
- the successful result source type is exactly `T`.

The operand types do not independently infer, choose, or alter `T`. A surrounding non-integer required type makes bitwise OR source-invalid before operand validation may commit a binding ownership consequence. A surrounding integer type different from an operand producer's own exact type causes that operand to fail its existing exact required-type validation rather than causing a conversion or promotion.

This relation defines no mixed-width or mixed-signedness bitwise operation, integer promotion, widening, narrowing, coercion, conversion, default numeric type, overload resolution, trait dispatch, generic bitwise abstraction, or result-type inference from operand syntax. Decimal integer literals may materialize as `T` only through their existing context-required literal relation; bitwise OR introduces no second literal-typing rule.

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

## Boolean short-circuit conjunction typing

Boolean short-circuit conjunction has exactly two ordered operands, **left** and **right**, and exactly one result.

For conjunction:

- the left operand required source type is exactly intrinsic `Bool`;
- the right operand required source type is exactly intrinsic `Bool`; and
- the result source type is intrinsically exactly `Bool`.

No other source type is accepted for either operand or the result. In particular, conjunction introduces no truthiness, integer-to-Bool or numeric conversion, coercion, promotion, defaulting, subtyping, overload resolution, trait dispatch, generic logical abstraction, or second Bool-like type.

The result type is intrinsic rather than inferred from either operand or the surrounding receiver. The surrounding required type MUST first accept that intrinsic `Bool` result before source validation may commit any operand-producer ownership consequence. Complete short-circuit operand validation, the exact skipped-versus-executed-right definite-ownership requirement, and dynamic execution sequencing are owned by `function-execution.md`.

## Boolean logical negation value relation

Let `b` be the successfully produced semantic `Bool` operand value.

Boolean logical negation consumes that owned operand value exactly once and produces exactly one distinct owned `Bool` result with the opposite semantic value:

- when `b` is `true`, the result is `false`;
- when `b` is `false`, the result is `true`.

These two cases are exhaustive because `types.md` defines exactly two semantic `Bool` values.

Ownership of the successful operand result ends at this operator application. The consumed operand result is not duplicated and receives no independent cleanup after the result has been produced.

The operation is deterministic. Equal semantic Bool operands produce equal semantic Bool results independently of source spelling, implementation representation, target, backend, optimization level, or host-language behavior.

## Plain fixed-width integer negation value relation

Let `T` be the admitted exact fixed-width integer source type selected by the surrounding required type, and let `v` be the successfully produced semantic operand value of `T`.

Plain integer negation consumes that owned operand value exactly once. It first forms the exact mathematical integer additive inverse

```text
x = -v
```

and then produces exactly one distinct owned result of source type `T` by applying the plain fixed-width modulo-`2^N` signed/unsigned mapping owned by `../core/numerics/integers.md` for the width and signedness corresponding to `T`.

There is no intermediate fixed-width truncation before the exact negation. If `x` lies outside the value interval of `T`, the accepted plain-overflow mapping determines the wrapped semantic result; overflow does not by itself select a source fault, undefined behavior, checked outcome, or saturated result. Consequently negating the minimum value of a represented signed type is defined by the same plain mapping, and negating any represented unsigned value is likewise defined, with nonzero values producing their modulo-`2^N` additive inverses.

This relation is total after one valid `T` operand value has been produced. Its result is independent of host unary-negation overflow, physical representation, two's-complement storage, optimizer assumptions, debug/release configuration, backend flags, or target instructions.

This operator relation is distinct from signed literal formation. A concrete signed decimal literal that denotes `-M` is materialized according to `literals.md` and is not semantically decomposed into this operator applied to `M`. This distinction is observable for unsigned required types and MUST survive source validation.

The relation does not define floating negation, unary plus, decrement, a source checked/saturating/explicitly wrapping mode, conversion, or another arithmetic operation.

## Plain fixed-width integer bitwise complement value relation

Let `T` be the admitted exact fixed-width integer source type selected by the surrounding required type, let `N` be that type's fixed width, and let `v` be the successfully produced semantic operand value of `T`.

Define the **canonical width residue** `r_N(v)` as the unique mathematical integer `r` satisfying

```text
0 <= r < 2^N
r ≡ v (mod 2^N).
```

Define the complement residue

```text
c = (2^N - 1) - r_N(v).
```

Plain fixed-width integer bitwise complement consumes the owned operand value exactly once and produces exactly one distinct owned result of source type `T`: the unique semantic value of `T` congruent to `c` modulo `2^N` under the accepted fixed-width signed/unsigned value mapping.

Equivalently, the same result is obtained by first forming the exact mathematical integer

```text
x = -1 - v
```

and then applying the same fixed-width modulo-`2^N` signed/unsigned mapping corresponding to `T`. The equivalence follows because `v` and `r_N(v)` are congruent modulo `2^N`, so `-1 - v` is congruent to `(2^N - 1) - r_N(v)` modulo `2^N`.

This relation is total after one valid `T` operand value has been produced. Complement of zero yields the same-type semantic value congruent to `2^N - 1`; complement of the same-type semantic value congruent to `2^N - 1` yields zero. Signed and unsigned source types use the same width-residue transformation and retain their distinct exact type identities when the result is mapped back to `T`.

The canonical width residue and complement residue are mathematical semantic relations only. They do not define or imply physical two's-complement storage, a physical all-ones representation, byte order, layout, alignment, ABI representation, serialization, bit-addressable storage, representation validity, pointer reinterpretation, host integer representation, target instructions, or backend flags.

The complement relation introduces no overflow fault, undefined behavior, checked outcome, saturation, numeric-contract selection, conversion, binary bitwise operation, shift, or physical representation rule. A host-language bitwise-complement operator is not semantic authority for this relation.

## Plain fixed-width integer addition value relation

Let `T` be the admitted exact fixed-width integer source type selected by the surrounding required type, and let `l` and `r` be the two successfully produced semantic operand values of `T`.

Plain integer addition consumes both owned operand values exactly once. It first forms the exact mathematical integer sum

```text
x = l + r
```

and then produces exactly one distinct owned result of source type `T` by applying the plain fixed-width modulo-`2^N` signed/unsigned mapping owned by `../core/numerics/integers.md` for the width and signedness corresponding to `T`.

There is no intermediate fixed-width truncation before the exact sum. If `x` lies outside the value interval of `T`, the accepted plain-overflow mapping determines the wrapped semantic result; overflow does not by itself select a source fault, undefined behavior, checked outcome, or saturated result.

This relation is total after two valid `T` operand values have been produced. Its result is independent of host integer overflow, physical representation, optimizer assumptions, debug/release configuration, backend flags, or target instructions.

The relation does not define a source checked, saturating, or explicitly wrapping mode. It also does not define floating addition, multiplication, division, remainder, shifts, binary bitwise operations, conversions, or numeric comparison.

## Same-format binary floating addition value relation

Let `T` be the admitted exact floating source type selected by the surrounding required type, let `C` be the one already-established numeric contract for this floating-addition occurrence, and let `l` and `r` be the two successfully produced semantic operand values of `T`.

Same-format binary floating addition consumes both owned operand values exactly once and produces exactly one distinct owned result of source type `T` whose semantic value belongs to the accepted same-format binary floating-addition result relation in `../core/numerics/floating-point.md` for type `T` under selected contract `C`.

The Core floating owner is the sole numerical authority for this source operation. The selected contract is semantic input to that owner; this source relation does not restate rounding, subnormal, signed-zero, infinity, NaN-class, reassociation, contraction, or other numerical permissions and does not make a backend mode additional authority.

Current represented source establishes `C = standard` for an unqualified floating addition through the accepted fallback, or `C = fast` for a source-valid operation-local selected-value form. No current source form establishes `reproducible`. This source restriction does not alter the accepted lower `standard | reproducible | fast` contract domain.

A result that belongs to the accepted semantic NaN class remains an ordinary semantic floating value of exact source type `T`. This source relation does not introduce one singleton source value named `NaNClass` and does not select, expose, compare, or preserve a NaN member identity, sign, payload, quiet/signaling state, canonical member, physical encoding, bit pattern, hash identity, or literal spelling. The reference semantics may use a class-level observation carrier for verification without making that observation representation a source semantic value.

This relation is total, non-faulting, and non-diverging after two valid `T` operand values have been produced. Any numerical result variation permitted by selected contract `C` remains ordinary numerical semantic latitude. A finite result, signed zero, subnormal, infinity, or permitted NaN-class result is a numerical result rather than a source fault, panic, undefined behavior, or alternate control-flow outcome. Host floating arithmetic, host floating environment, physical IEEE encoding, target instruction behavior, backend flags, optimizer assumptions, or implementation NaN behavior are not semantic authority.

The relation does not define floating negation, subtraction, multiplication, division, remainder, fused arithmetic, comparison, conversion, a source contract other than the bounded selection described above, vector arithmetic, or another floating operation.

## Plain fixed-width integer subtraction value relation

Let `T` be the admitted exact fixed-width integer source type selected by the surrounding required type, and let `l` and `r` be the two successfully produced semantic operand values of `T`.

Plain integer subtraction consumes both owned operand values exactly once. It first forms the exact mathematical integer difference

```text
x = l - r
```

and then produces exactly one distinct owned result of source type `T` by applying the plain fixed-width modulo-`2^N` signed/unsigned mapping owned by `../core/numerics/integers.md` for the width and signedness corresponding to `T`.

There is no intermediate fixed-width truncation before the exact difference. If `x` lies outside the value interval of `T`, the accepted plain-overflow mapping determines the wrapped semantic result; overflow does not by itself select a source fault, undefined behavior, checked outcome, or saturated result.

This relation is total after two valid `T` operand values have been produced. Its result is independent of host integer overflow, physical representation, optimizer assumptions, debug/release configuration, backend flags, or target instructions.

The relation does not define source-level rewriting through unary negation, a source checked/saturating/explicitly wrapping mode, floating subtraction, multiplication, division, remainder, shifts, binary bitwise operations, conversions, or numeric comparison.

## Same-format binary floating subtraction value relation

Let `T` be the admitted exact floating source type selected by the surrounding required type, let `C` be the one already-established numeric contract for this floating-subtraction occurrence, and let `l` and `r` be the two successfully produced semantic operand values of `T`.

Same-format binary floating subtraction consumes both owned operand values exactly once and produces exactly one distinct owned result of source type `T` whose semantic value belongs to the accepted same-format binary floating-subtraction result relation in `../core/numerics/floating-point.md` for type `T` under selected contract `C`.

The Core floating owner is the sole numerical authority for this source operation. The selected contract is semantic input to that owner; this source relation does not restate rounding, subnormal, signed-zero, infinity, NaN-class, or other numerical permissions and does not make a backend mode additional authority. In particular, the accepted finite multiply-add contraction rule does not define multiply-subtract or a negated fused variant, and this subtraction relation adds no such permission.

Current represented source establishes `C = standard` for an unqualified floating subtraction through the accepted fallback, or `C = fast` for a source-valid operation-local selected-value form. No current source form establishes `reproducible`. This source restriction does not alter the accepted lower `standard | reproducible | fast` contract domain.

A result that belongs to the accepted semantic NaN class remains an ordinary semantic floating value of exact source type `T`. This source relation does not introduce one singleton source value named `NaNClass` and does not select, expose, compare, or preserve a NaN member identity, sign, payload, quiet/signaling state, canonical member, physical encoding, bit pattern, hash identity, or literal spelling. The reference semantics may use a class-level observation carrier for verification without making that observation representation a source semantic value.

This relation is total, non-faulting, and non-diverging after two valid `T` operand values have been produced. Any numerical result variation permitted by selected contract `C` remains ordinary numerical semantic latitude. A finite result, signed zero, subnormal, infinity, or permitted NaN-class result is a numerical result rather than a source fault, panic, undefined behavior, or alternate control-flow outcome. Host floating arithmetic, host floating environment, physical IEEE encoding, target instruction behavior, backend flags, optimizer assumptions, or implementation NaN behavior are not semantic authority.

This is direct subtraction semantics. It is not defined by rewriting the right operand through floating unary negation and then applying floating addition. The relation does not define floating negation, multiplication, division, remainder, fused arithmetic, comparison, conversion, a source contract other than the bounded selection described above, vector arithmetic, or another floating operation.

## Plain fixed-width integer multiplication value relation

Let `T` be the admitted exact fixed-width integer source type selected by the surrounding required type, and let `l` and `r` be the two successfully produced semantic operand values of `T`.

Plain integer multiplication consumes both owned operand values exactly once. It first forms the exact mathematical integer product

```text
x = l * r
```

and then produces exactly one distinct owned result of source type `T` by applying the plain fixed-width modulo-`2^N` signed/unsigned mapping owned by `../core/numerics/integers.md` for the width and signedness corresponding to `T`.

There is no intermediate fixed-width truncation before the exact product. If `x` lies outside the value interval of `T`, the accepted plain-overflow mapping determines the wrapped semantic result; overflow does not by itself select a source fault, undefined behavior, checked outcome, or saturated result.

This relation is total after two valid `T` operand values have been produced. Its result is independent of host integer overflow, physical representation, optimizer assumptions, debug/release configuration, backend flags, or target instructions.

The relation does not define a source checked/saturating/explicitly wrapping mode, floating multiplication, division, remainder, shifts, binary bitwise operations, conversions, numeric comparison, dereference, or another meaning of `*`.

## Plain fixed-width integer exclusive-or value relation

Let `T` be the admitted exact fixed-width integer source type selected by the surrounding required type, and let `l` and `r` be the two successfully produced semantic operand values of `T`.

Plain integer exclusive-or consumes both owned operand values exactly once and produces exactly one distinct owned result of source type `T` whose semantic value is the **plain fixed-width integer exclusive-or result** defined by `../core/numerics/integers.md` for the corresponding fixed-width integer kind.

The Core relation is the sole numerical authority for this source operation. In particular, its canonical-width-residue and mathematical-binary-digit definition determines the result independently of physical signed representation, byte layout, endianness, host-language `^` behavior, optimizer assumptions, backend instructions, or target details.

This relation is total after two valid `T` operand values have been produced. It introduces no overflow classification, defined fault, undefined behavior, checked outcome, saturation, conversion, numeric-contract selection, binary AND/OR relation, shift, or physical representation rule.

## Plain fixed-width integer bitwise-OR value relation

Let `T` be the admitted exact fixed-width integer source type selected by the surrounding required type, and let `l` and `r` be the two successfully produced semantic operand values of `T`.

Plain integer bitwise OR consumes both owned operand values exactly once and produces exactly one distinct owned result of source type `T` whose semantic value is the **plain fixed-width integer bitwise-OR result** defined by `../core/numerics/integers.md` for the corresponding fixed-width integer kind.

The Core relation is the sole numerical authority for this source operation. In particular, its canonical-width-residue and mathematical-binary-digit definition determines the result independently of physical signed representation, byte layout, endianness, host-language `|` behavior, optimizer assumptions, backend instructions, or target details.

This relation is total after two valid `T` operand values have been produced. It introduces no overflow classification, defined fault, undefined behavior, checked outcome, saturation, conversion, numeric-contract selection, binary AND relation, XOR/complement/arithmetic rewrite, shift, or physical representation rule.

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

## Boolean short-circuit conjunction value relation

Let `l` be the successfully produced semantic left `Bool` operand value.

Boolean short-circuit conjunction consumes `l` exactly once for selection.

- When `l` is `false`, no right semantic operand value is required or produced by the conjunction execution. The conjunction produces exactly one distinct owned `Bool` result `false`.
- When `l` is `true`, the complete right producer is evaluated under `function-execution.md`. Let `r` be its successfully produced semantic `Bool` value. The conjunction consumes `r` exactly once and produces exactly one distinct owned `Bool` result with the same semantic value as `r`.

These cases are exhaustive because `types.md` defines exactly two semantic `Bool` values. The relation is deterministic and is not an eager two-operand truth table: a right semantic operand value exists only on the successful left-`true` path.

The successful result therefore equals ordinary Boolean conjunction where both operand values exist, while the dynamic evaluation relation additionally requires the accepted short-circuit omission of the right producer when left is `false`.

A host-language `&&`, Boolean bitwise operation, optimizer rewrite, constant folder, backend branch, or speculative evaluation is not semantic authority for this relation.

## Operator-local ownership and execution effects

Boolean logical negation receives one already successfully produced owned `Bool` operand, consumes that operand as defined above, and produces one owned `Bool` result.

Plain fixed-width integer negation receives one already successfully produced owned value of the admitted fixed-width integer source type `T`, consumes that operand exactly once as defined above, and produces one owned `T` result.

Plain fixed-width integer bitwise complement likewise receives one already successfully produced owned value of the admitted fixed-width integer source type `T`, consumes that operand exactly once as defined above, and produces one owned `T` result.

Plain fixed-width integer addition receives two already successfully produced owned values of the same admitted fixed-width integer source type `T`, consumes both operands exactly once as defined above, and produces one owned `T` result.

Same-format binary floating addition receives two already successfully produced owned values of the same admitted floating source type `T`, consumes both operands exactly once under its already-established numeric contract, and produces one owned `T` result.

Plain fixed-width integer subtraction likewise receives two already successfully produced owned values of the same admitted fixed-width integer source type `T`, consumes both operands exactly once as defined above, and produces one owned `T` result.

Same-format binary floating subtraction likewise receives two already successfully produced owned values of the same admitted floating source type `T`, consumes both operands exactly once under its already-established numeric contract, and produces one owned `T` result.

Plain fixed-width integer multiplication likewise receives two already successfully produced owned values of the same admitted fixed-width integer source type `T`, consumes both operands exactly once as defined above, and produces one owned `T` result.

Plain fixed-width integer exclusive-or likewise receives two already successfully produced owned values of the same admitted fixed-width integer source type `T`, consumes both operands exactly once as defined above, and produces one owned `T` result.

Plain fixed-width integer bitwise OR likewise receives two already successfully produced owned values of the same admitted fixed-width integer source type `T`, consumes both operands exactly once as defined above, and produces one owned `T` result.

Boolean equality and inequality each receive two already successfully produced owned `Bool` operand values, consume both operands exactly once as defined above, and produce one owned `Bool` result.

Boolean short-circuit conjunction receives one successfully produced left `Bool`, consumes it for selection, and receives/consumes a right `Bool` only on the successful left-`true` path. It produces one owned `Bool` result on either successful normal path.

Each represented operator itself:

- consumes no parameter or local binding directly;
- duplicates or transfers no binding-owned structural path directly;
- changes no binding structural-ownership state;
- creates no source binding, address, reference, place, or source-visible storage identity;
- introduces no defined-fault reason;
- introduces no divergence possibility after the operand value or values required by the selected successful semantic path have been produced; and
- introduces no runtime flag or hidden source state.

Any binding ownership transition, defined fault, divergence, transient lifetime, or other effect needed to produce an operand remains the consequence of that operand producer and the sequencing relation in `function-execution.md`.

For each represented **eager** binary operator, successful left production necessarily precedes right production under `function-execution.md`, so the in-progress operation owns the produced left value until right production either succeeds, faults, or remains suspended by divergence. For equality/inequality that value is `Bool`; for integer multiplication/addition/subtraction/exclusive-or/bitwise OR it is the selected exact integer type `T`; for same-format floating addition and subtraction it is the selected exact floating type `T`. That bounded lifetime is an execution/cleanup sequencing fact owned by `function-execution.md`, not a new source binding or storage identity. Unary Boolean negation, integer negation, and integer complement have no held-left transient because their complete single operand is consumed immediately after successful operand production. Boolean short-circuit conjunction likewise has no held-left transient across right evaluation because its successful left Bool is consumed for selection before the right producer may begin.

A successfully validated represented operator adds no binding structural-ownership transition beyond the committed consequences of its operand producer or producers. For conjunction specifically, `function-execution.md` additionally requires the successful right-producer structural-ownership state to equal the skipped-right post-left state before one definite normal operator state may be committed.

Every successful result is one ordinary owned value of its operator's exact result type. Intrinsic Bool, fixed-width integer, and represented binary-floating result duplicability is the existing intrinsic duplicability classification from `types.md`; these operators introduce no second duplicability or copyability rule.

## Contextual grouping relationship

Parenthesized grouping is concrete syntax around one already represented value producer; it is not an operator and defines no operator-local type, value, ownership, fault, divergence, or Core relation. `concrete-syntax.md` owns the ordinary and conditional grouped-value grammar, and `function-execution.md` owns its semantic transparency.

When a group contains a represented operator, the contained operator retains exactly its typing, semantic value relation, operand ordering, ownership consequences, held-left lifetime where applicable, short-circuit behavior where applicable, fault/divergence behavior, selected numeric contract where applicable, and source-to-Core refinement defined in this document. The parentheses add no operator step before, between, or after those relations.

Grouping may make explicit a syntax-tree nesting that the unparenthesized bounded grammar does not represent. For example, `!(a == b)` contains one grouped equality value as the operand of logical negation, `-(a + b)` contains one grouped addition value as the operand of integer negation when the surrounding required type is integer, `~(a + b)` contains one grouped addition value as the operand of integer complement when the surrounding required type is integer, `(a == b) == c` contains one grouped inner equality as the left operand of an outer equality, and `a == (b != c)` contains one grouped inner inequality as the right operand. For additive operators, `(a + b) - c`, `(a - b) + c`, `a - (b - c)`, and `a + (b - c)` contain explicitly grouped inner additive values; each concrete `+` retains the integer-addition or floating-addition semantic identity selected by its exact required type, and each concrete binary `-` likewise retains the integer-subtraction or floating-subtraction identity selected by that exact required type. For multiplication, `(a * b) * c` and `a * (b * c)` explicitly represent repeated multiplication that the ungrouped multiplicative tier does not admit. Mixed-tier forms such as `(a + b) * c` and `a * (b + c)` explicitly override the ungrouped multiplicative-over-additive nesting. For exclusive-or, `(a ^ b) ^ c` and `a ^ (b ^ c)` explicitly represent repeated exclusive-or that the ungrouped exclusive-or tier does not admit. For bitwise OR, `(a | b) | c` and `a | (b | c)` explicitly represent repeated bitwise OR that the ungrouped bitwise-OR tier does not admit. For conjunction, `(a && b) && c` and `a && (b && c)` explicitly represent repeated conjunction that the ungrouped conjunction tier does not admit. None of these forms introduces associativity or chaining.

No precedence number, associativity metadata, grouping operator identity, runtime parenthesis object, or Core grouping operation follows from this concrete nesting. A faithful typed frontend may erase the grouping wrapper while retaining the already required contained operator/value facts.

## Operation-local numeric-contract selection relationship

The represented numeric-contract-selected value is a qualification wrapper around one existing governed operation, not an operator or a new owned-value producer. `concrete-syntax.md` owns its exact spelling and grouping shape; `function-execution.md` owns target discovery and the pre-operand applicability transaction.

For the current represented source operation family, same-format floating-addition and floating-subtraction roots are governed by the Core floating numeric-contract system. An unqualified occurrence obtains `standard` from the accepted default. A source-valid selected-value wrapper establishes `fast` for exactly one directly contained governed floating root after ordinary grouping transparency. It does not recursively select nested governed operations, and another selected-value wrapper is not transparent for target discovery.

The selector creates no source `standard` or `reproducible` spelling, no block/function/module default, no lexical or dynamic inheritance, no callable-signature dimension, and no ambient runtime or backend numeric mode. Nested floating additions/subtractions therefore retain independent selected-contract facts, including mixed `standard`/`fast` trees governed by the accepted Core composition rules. For example, `@fast(a - b)` selects one floating-subtraction occurrence when the exact surrounding required type is floating; `@fast((a - b) + c)` selects only the outer floating addition, while the grouped inner unqualified floating subtraction remains `standard`; and `@fast((a + b) - c)` analogously selects only the outer floating subtraction.

## Conditional-use relationship

When a completed represented operator result is used as the condition of a represented `if` or `while`, `control-flow.md` requires the resulting owned value to have exact source type `Bool`.

Logical negation changes only one successful operand's semantic Bool value. Its post-condition binding environment is exactly the operand producer's successful post-evaluation environment as established by `function-execution.md`.

Boolean equality/inequality eagerly execute the complete left producer and then the complete right producer before producing their Bool result. Their successful post-condition binding environment is therefore exactly the right producer's successful post-evaluation environment after the already-completed left producer consequences. The Bool truth relation adds no further binding transition.

Boolean short-circuit conjunction executes the complete left producer first. Its left-`false` normal path skips the right producer and retains the post-left environment; its left-`true`/right-success normal path retains the successful post-right environment. `function-execution.md` requires those two complete structural-ownership environments to be exactly equal before conjunction is source-valid, so a completed conjunction exposes that one definite common environment to `control-flow.md`. The Bool short-circuit truth relation adds no binding transition of its own.

The concrete conditional grammar may syntactically contain the bounded integer-negation/integer-complement prefix, multiplicative/additive/exclusive-or/bitwise-OR/equality, and logical-conjunction tiers so that one grammar hierarchy remains explicit and context-preserving. Plain integer negation, integer complement, multiplication, integer addition, floating addition, integer subtraction, floating subtraction, exclusive-or, or bitwise OR nevertheless cannot satisfy the condition's surrounding exact required type `Bool`. `function-execution.md` therefore rejects such a numeric operation at outer required-type admission before its operand or operands are validated in a way that may commit ownership. No completed integer-neg, integer-complement, integer-mul, integer-add, floating-add, integer-sub, floating-sub, integer-xor, or integer-or result reaches conditional selection.

The selected-value wrapper is not directly represented by `ConditionalValue` in this revision. Since the currently governed source operations produce exact `F16`, `F32`, or `F64`, adding a parallel conditional selector grammar would have no source-valid current consumer and would not alter the exact-`Bool` rejection above.

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

For plain fixed-width integer negation, a minimal typed representation may be equivalent to:

```text
ValueKind::IntegerNeg {
    operand: Box<Value>,
}
```

where the outer value and operand both carry the same exact admitted fixed-width integer source type `T` selected by the surrounding receiving requirement. This retained operator identity is distinct from the representation of a signed decimal literal; a frontend MUST NOT normalize an accepted signed literal into `IntegerNeg` before literal materialization and source validation.

For plain fixed-width integer bitwise complement, a minimal typed representation may be equivalent to:

```text
ValueKind::IntegerComplement {
    operand: Box<Value>,
}
```

where the outer value and operand both carry the same exact admitted fixed-width integer source type `T` selected by the surrounding receiving requirement.

For plain fixed-width integer addition, a minimal typed representation may be equivalent to:

```text
ValueKind::IntegerAdd {
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all carry the same exact admitted fixed-width integer source type `T` selected by the surrounding receiving requirement.

For same-format binary floating addition, a minimal typed representation may be equivalent to:

```text
NumericContract::{Standard, Reproducible, Fast}

ValueKind::FloatAdd {
    contract: NumericContract,
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all carry the same exact admitted floating source type `T` selected by the surrounding receiving requirement, and `contract` retains the one already-established numeric contract for that operation occurrence. Current source validation constructs `Standard` for an unqualified FloatAdd and `Fast` for a source-valid selected FloatAdd. A frontend may use the complete accepted lower contract domain shown above without thereby introducing source `reproducible` syntax; current source validation does not manufacture that selection. The retained operation identity remains distinct from `IntegerAdd`; the shared concrete `+` token is not a reason to erase either the semantic operation distinction or the selected contract fact.

For plain fixed-width integer subtraction, a minimal typed representation may be equivalent to:

```text
ValueKind::IntegerSub {
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all carry the same exact admitted fixed-width integer source type `T` selected by the surrounding receiving requirement.

For same-format binary floating subtraction, a minimal typed representation may be equivalent to:

```text
ValueKind::FloatSub {
    contract: NumericContract,
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all carry the same exact admitted floating source type `T` selected by the surrounding receiving requirement, and `contract` retains the one already-established numeric contract for that operation occurrence. Current source validation constructs `Standard` for an unqualified FloatSub and `Fast` for a source-valid selected FloatSub. The retained operation identity remains distinct from `IntegerSub`; the shared concrete binary `-` token is not a reason to erase either the semantic operation distinction or the selected contract fact. This explanatory shape does not require the current implementation to retain its existing integer-specific syntax-node name.

For plain fixed-width integer multiplication, a minimal typed representation may be equivalent to:

```text
ValueKind::IntegerMul {
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all carry the same exact admitted fixed-width integer source type `T` selected by the surrounding receiving requirement.

For plain fixed-width integer exclusive-or, a minimal typed representation may be equivalent to:

```text
ValueKind::IntegerXor {
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all carry the same exact admitted fixed-width integer source type `T` selected by the surrounding receiving requirement.

For plain fixed-width integer bitwise OR, a minimal typed representation may be equivalent to:

```text
ValueKind::IntegerOr {
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

For Boolean short-circuit conjunction, a minimal typed representation may be equivalent to:

```text
ValueKind::BooleanAnd {
    left: Box<Value>,
    right: Box<Value>,
}
```

where the outer value, left operand, and right operand all have source type `Bool`. The retained right value identifies the statically validated right producer; it does not imply eager runtime execution when the left value is `false`.

A faithful implementation MAY instead use another explicit typed layout when it retains exactly the same source-semantic facts and no speculative generalized abstraction.

These explanatory shapes are not implementation-layout mandates. Source locations may be retained by diagnostics/tooling but are not part of an operator's semantic identity. Token spelling, numeric precedence values, associativity metadata, grouping delimiters, selected-value delimiters, Core block identities, source-CFG identities, source ownership-state sets, runtime operator objects, and backend opcodes are not semantic facts required after validation.

A faithful lowerer MUST reject internally inconsistent retained operator facts rather than repairing them from lower/Core type information. This includes a non-`Bool` Boolean operator result or operand, an integer-neg or integer-complement result whose type is not one of the eight admitted fixed-width integer types, an integer-neg or integer-complement operand whose retained exact source type differs from the retained operation result type, an integer-add/integer-sub/integer-mul/integer-xor/integer-or result whose type is not one of the eight admitted fixed-width integer types, integer-add/integer-sub/integer-mul/integer-xor/integer-or operands whose retained exact source type differs from the retained operation result type, a floating-add or floating-sub result whose type is not one of `F16`, `F32`, or `F64`, floating-add or floating-sub operands whose retained exact source type differs from their retained operation result type, or a floating-add or floating-sub numeric-contract fact outside the accepted `Standard | Reproducible | Fast` domain. A lowerer MUST preserve each validated floating-operation contract fact rather than re-defaulting, inferring, or replacing it from target/backend state.

## Source-to-Core refinement

Boolean logical negation, Boolean equality/inequality, and Boolean short-circuit conjunction require no new Core operation. Plain fixed-width integer negation and plain fixed-width integer bitwise complement also require no new Core operation: after each complete source operand has lowered, each refines through the already represented Core `IntegerSub` relation. Plain fixed-width integer addition refines to exactly the represented Core `IntegerAdd` relation, same-format binary floating addition refines to exactly the represented Core `FloatAdd` relation with its selected contract retained, plain fixed-width integer subtraction refines to exactly the distinct represented Core `IntegerSub` relation, same-format binary floating subtraction refines to exactly the distinct represented Core `FloatSub` relation with its selected contract retained, plain fixed-width integer multiplication refines to exactly the distinct represented Core `IntegerMul` relation, plain fixed-width integer exclusive-or refines to exactly the distinct represented Core `IntegerXor` relation, and plain fixed-width integer bitwise OR refines to exactly the distinct represented Core `IntegerOr` relation, all owned by `../core/value-storage.md`. Integer numerical relations remain owned by `../core/numerics/integers.md`; the floating numerical relations, contract domain/default/refinement rules, and mixed-operation numerical permissions remain owned by `../core/numerics/floating-point.md`.

### Plain fixed-width integer negation refinement

A faithful integer-neg lowerer first lowers the complete source operand exactly once under its existing producer semantics. The lowered operand local MUST carry the exact Core type identity corresponding to the admitted source type `T`; malformed retained source facts are rejected rather than converted or repaired.

Only after complete operand-producer lowering succeeds:

1. create one fresh Core result local of that same exact Core integer type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `IntegerSub` whose destination is that result local, whose left operand is semantic integer constant zero matching that exact destination Core type identity, and whose right operand is an ownership-transferring `Move` of the operand-result local; and
3. continue lowering with the result local live and available as the lowered source integer-negation result.

This refinement is valid under the accepted Core owners because:

- the complete source operand producer has already finished before the `IntegerSub` operation is reached, so any source binding ownership transition, call, fault possibility, divergence possibility, or nested producer effect has already occurred in source order;
- if operand producer execution faults or diverges, the result local and `IntegerSub` are never reached;
- the typed semantic zero constant has no source producer, binding, ownership transition, fault, divergence, or source-visible evaluation position and exists only in the lower refinement after the source operand has completed;
- the fresh result local satisfies the operation's wholly-vacant non-replacing destination rule;
- `IntegerSub` admits a semantic constant only when it matches the exact destination Core type identity and then evaluates `0 - v` under that exact type;
- moving the lowered operand-result local consumes the already-produced owned source operand representation exactly once;
- the Core numerical relation computes the exact mathematical difference `0 - v = -v` and applies the same accepted plain fixed-width result mapping required by the source integer-negation relation;
- the Core operation's internal left-constant-before-right-move operand order cannot reorder the already completed source operand producer and adds no source-visible effect; and
- successful `IntegerSub` leaves exactly the result local live and introduces no negation-only branch, join, fault, divergence, or new Core operation.

This is an operation-specific source-to-Core refinement, not semantic authority to rewrite source signed literals or existing source binary subtraction. A conforming frontend or optimizer MUST NOT replace a source signed decimal literal with an integer-negation operator before source validation merely because this lower refinement uses Core subtraction. It likewise MUST NOT infer a generic arithmetic identity, source unary-plus form, decrement relation, or generic Core arithmetic opcode from this refinement.

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source integer negation MUST still complete the source operand once before its negation refinement, create one fresh wholly-vacant same-type result local, apply exactly one existing `IntegerSub` from exact same-type zero and ownership-transferring operand move, and add no Core branch or join solely for negation. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, signed-literal distinction, and plain-overflow result.

### Plain fixed-width integer bitwise complement refinement

A faithful integer-complement lowerer first lowers the complete source operand exactly once under its existing producer semantics. The lowered operand local MUST carry the exact Core type identity corresponding to the admitted source type `T`; malformed retained source facts are rejected rather than converted or repaired.

Only after complete operand-producer lowering succeeds:

1. create one fresh Core result local of that same exact Core integer type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `IntegerSub` whose destination is that result local, whose left operand is the semantic constant of that exact destination type congruent to `-1` modulo `2^N`, and whose right operand is an ownership-transferring `Move` of the operand-result local; and
3. continue lowering with the result local live and available as the lowered source integer-complement result.

The required left constants are semantic values, not physical bit patterns: `I8(-1)`, `I16(-1)`, `I32(-1)`, `I64(-1)`, `U8(255)`, `U16(65535)`, `U32(4294967295)`, and `U64(18446744073709551615)` for the corresponding exact destination type.

This refinement is valid under the accepted Core owners because:

- the complete source operand producer has already finished before `IntegerSub` is reached, so every source binding ownership transition, call, fault possibility, divergence possibility, or nested producer effect has already occurred in source order;
- if operand producer execution faults or diverges, the result local and `IntegerSub` are never reached;
- the same-type left constant has no source producer, binding, ownership transition, fault, divergence, source-visible evaluation position, or physical-representation meaning and exists only in the lower refinement after source operand completion;
- the fresh result local satisfies the wholly-vacant non-replacing destination rule;
- `IntegerSub` admits the semantic constant only when it matches the exact destination Core type identity;
- moving the lowered operand-result local consumes the already-produced owned source operand representation exactly once;
- the Core numerical relation computes exact mathematical subtraction from the same-type value congruent to `-1`, and its accepted fixed-width result mapping therefore yields the same semantic value as exact `-1 - v` and the canonical-residue complement relation above;
- the Core operation's internal left-constant-before-right-move order cannot reorder the already completed source operand producer and adds no source-visible effect; and
- successful `IntegerSub` leaves exactly the result local live and introduces no complement-only branch, join, fault, divergence, validator/reference extension, or new Core operation.

This is an operation-specific source-to-Core refinement, not semantic authority to rewrite source complement as source subtraction, to infer a generic arithmetic/bitwise identity, or to make the left constant a physical all-ones representation guarantee.

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source integer complement MUST still complete the source operand once before its complement refinement, create one fresh wholly-vacant same-type result local, apply exactly one existing `IntegerSub` from the exact same-type semantic value congruent to `-1 mod 2^N` and ownership-transferring operand move, and add no Core branch or join solely for complement. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, representation independence, and complement result relation.

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

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source integer addition MUST still lower the complete left producer, then the complete right producer, create one fresh wholly-vacant result local, apply exactly one `IntegerAdd` to that local, and add no Core branch or join solely for the addition. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, and plain-overflow result, and MUST NOT replace the accepted semantics with host arithmetic assumptions or implicit conversions.

### Same-format binary floating addition refinement

A faithful floating-add lowerer first lowers the complete left producer exactly once and then lowers the complete right producer exactly once from the successful left continuation. The lowered operand locals MUST carry the same exact Core type identity corresponding to the admitted source type `T`, where `T` is exactly `F16`, `F32`, or `F64`; malformed retained source facts are rejected rather than converted or repaired. The lowerer MUST also consume the already-validated selected numeric contract `C` retained on this FloatAdd occurrence and MUST NOT re-default or infer `C` from target/backend state.

After both operand-producer lowerings succeed:

1. create one fresh Core result local of that same exact Core floating type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `FloatAdd` whose destination is that result local, whose left operand is an ownership-transferring `Move` of the left operand-result local, whose right operand is an ownership-transferring `Move` of the right operand-result local, and whose explicit numeric contract is exactly retained `C`; and
3. continue lowering with the result local live and available as the lowered source floating-addition result.

This refinement is valid under the accepted Core owners because:

- source left-producer execution completes before source right-producer execution begins;
- the lowered left result local remains live while the complete right producer lowers/executes, representing the source operation's held-left floating value;
- if left producer execution faults or diverges, right producer execution and `FloatAdd` are never reached;
- if right producer execution faults after left success, `FloatAdd` is never reached and the existing Core activation fault cleanup ends then-live compiler locals, including the local retaining the produced left floating value;
- if right producer execution diverges, the caller remains suspended and the left operand-result local remains live;
- once both producers succeed, `FloatAdd` evaluates its two `Move` operands left-to-right, consuming each produced operand local exactly once;
- the fresh result local satisfies the operation's wholly-vacant non-replacing destination rule;
- accepted Core `FloatAdd` uses the exact same-format floating type identity and exactly selected contract `C` retained from source validation;
- operation-produced infinity or semantic NaN-class results and any other numerical alternatives explicitly permitted by `C` are ordinary runtime floating results and require no fabricable NaN constant, source-visible NaN identity, or ambient runtime numeric mode;
- successful `FloatAdd` leaves exactly the result local live and introduces no branch, join, defined fault, divergence, selector wrapper, runtime contract state, or additional Core operation; and
- nested represented `+` operations may recursively lower their complete operand producers, retaining integer-add or floating-add identity and an independent selected contract for every floating-add occurrence at every typed nesting level.

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source floating addition MUST still lower the complete left producer, then the complete right producer, create one fresh wholly-vacant exact floating result local, apply exactly one `FloatAdd` carrying the validated selected contract to that local, and add no Core branch or join solely for the addition. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, the result set permitted by selected contract `C`, and NaN-class freedom, and MUST NOT replace the accepted semantics with host floating arithmetic, physical floating encodings, implicit conversions, an ambient source/runtime contract mode, or a speculative generic arithmetic opcode.

### Plain fixed-width integer subtraction refinement

A faithful integer-sub lowerer first lowers the complete left producer exactly once and then lowers the complete right producer exactly once from the successful left continuation. The lowered operand locals MUST carry the same exact Core type identity corresponding to the admitted source type `T`; malformed retained source facts are rejected rather than converted or repaired.

After both operand-producer lowerings succeed:

1. create one fresh Core result local of that same exact Core integer type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `IntegerSub` whose destination is that result local, whose left operand is an ownership-transferring `Move` of the left operand-result local, and whose right operand is an ownership-transferring `Move` of the right operand-result local; and
3. continue lowering with the result local live and available as the lowered source subtraction result.

This refinement is valid under the accepted Core owners because:

- source left-producer execution completes before source right-producer execution begins;
- the lowered left result local remains live while the complete right producer lowers/executes, representing the source operation's held-left value;
- if left producer execution faults or diverges, right producer execution and `IntegerSub` are never reached;
- if right producer execution faults after left success, `IntegerSub` is never reached and the existing Core activation fault cleanup ends then-live compiler locals, including the local retaining the produced left integer value;
- if right producer execution diverges, the caller remains suspended and the left operand-result local remains live;
- once both producers succeed, `IntegerSub` evaluates its two `Move` operands left-to-right, consuming each produced operand local exactly once;
- the fresh result local satisfies the operation's wholly-vacant non-replacing destination rule;
- the Core numerical relation computes the exact mathematical difference and accepted plain fixed-width result for the same type `T`;
- successful `IntegerSub` leaves exactly the result local live and introduces no comparison branch, join, fault, or divergence; and
- nested represented additive operators may recursively lower complete operand producers before their enclosing operation emits its own one Core `IntegerAdd`, `FloatAdd`, `IntegerSub`, or `FloatSub` according to the retained typed operation identity.

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source integer subtraction MUST still lower the complete left producer, then the complete right producer, create one fresh wholly-vacant result local, apply exactly one `IntegerSub` to that local, and add no Core branch or join solely for the subtraction. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, and plain-overflow result, and MUST NOT replace the accepted semantics with host arithmetic assumptions, implicit conversions, unary-negation rewriting, or a speculative generic arithmetic opcode.

### Same-format binary floating subtraction refinement

A faithful floating-sub lowerer first lowers the complete left producer exactly once and then lowers the complete right producer exactly once from the successful left continuation. The lowered operand locals MUST carry the same exact Core type identity corresponding to the admitted source type `T`, where `T` is exactly `F16`, `F32`, or `F64`; malformed retained source facts are rejected rather than converted or repaired. The lowerer MUST also consume the already-validated selected numeric contract `C` retained on this FloatSub occurrence and MUST NOT re-default or infer `C` from target/backend state.

After both operand-producer lowerings succeed:

1. create one fresh Core result local of that same exact Core floating type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `FloatSub` whose destination is that result local, whose left operand is an ownership-transferring `Move` of the left operand-result local, whose right operand is an ownership-transferring `Move` of the right operand-result local, and whose explicit numeric contract is exactly retained `C`; and
3. continue lowering with the result local live and available as the lowered source floating-subtraction result.

This refinement is valid under the accepted Core owners because:

- source left-producer execution completes before source right-producer execution begins;
- the lowered left result local remains live while the complete right producer lowers/executes, representing the source operation's held-left floating value;
- if left producer execution faults or diverges, right producer execution and `FloatSub` are never reached;
- if right producer execution faults after left success, `FloatSub` is never reached and the existing Core activation fault cleanup ends then-live compiler locals, including the local retaining the produced left floating value;
- if right producer execution diverges, the caller remains suspended and the left operand-result local remains live;
- once both producers succeed, `FloatSub` evaluates its two `Move` operands left-to-right, consuming each produced operand local exactly once;
- the fresh result local satisfies the operation's wholly-vacant non-replacing destination rule;
- accepted Core `FloatSub` uses the exact same-format floating type identity and exactly selected contract `C` retained from source validation;
- operation-produced infinity or semantic NaN-class results and any other numerical alternatives explicitly permitted by `C` are ordinary runtime floating results and require no fabricable NaN constant, source-visible NaN identity, or ambient runtime numeric mode;
- successful `FloatSub` leaves exactly the result local live and introduces no branch, join, defined fault, divergence, selector wrapper, floating-negation rewrite, runtime contract state, or additional Core operation; and
- nested represented additive operations may recursively lower their complete operand producers, retaining integer-add, floating-add, integer-sub, or floating-sub identity and an independent selected contract for every governed floating occurrence at every typed nesting level.

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source floating subtraction MUST still lower the complete left producer, then the complete right producer, create one fresh wholly-vacant exact floating result local, apply exactly one `FloatSub` carrying the validated selected contract to that local, and add no Core branch or join solely for the subtraction. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, the result set permitted by selected contract `C`, and NaN-class freedom, and MUST NOT replace the accepted semantics with host floating arithmetic, physical floating encodings, implicit conversions, a rewrite through floating negation/addition, multiply-subtract contraction, an ambient source/runtime contract mode, or a speculative generic arithmetic opcode.

### Plain fixed-width integer multiplication refinement

A faithful integer-mul lowerer first lowers the complete left producer exactly once and then lowers the complete right producer exactly once from the successful left continuation. The lowered operand locals MUST carry the same exact Core type identity corresponding to the admitted source type `T`; malformed retained source facts are rejected rather than converted or repaired.

After both operand-producer lowerings succeed:

1. create one fresh Core result local of that same exact Core integer type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `IntegerMul` whose destination is that result local, whose left operand is an ownership-transferring `Move` of the left operand-result local, and whose right operand is an ownership-transferring `Move` of the right operand-result local; and
3. continue lowering with the result local live and available as the lowered source multiplication result.

This refinement is valid under the accepted Core owners because:

- source left-producer execution completes before source right-producer execution begins;
- the lowered left result local remains live while the complete right producer lowers/executes, representing the source operation's held-left value;
- if left producer execution faults or diverges, right producer execution and `IntegerMul` are never reached;
- if right producer execution faults after left success, `IntegerMul` is never reached and the existing Core activation fault cleanup ends then-live compiler locals, including the local retaining the produced left integer value;
- if right producer execution diverges, the caller remains suspended and the left operand-result local remains live;
- once both producers succeed, `IntegerMul` evaluates its two `Move` operands left-to-right, consuming each produced operand local exactly once;
- the fresh result local satisfies the operation's wholly-vacant non-replacing destination rule;
- the Core numerical relation computes the exact mathematical product and accepted plain fixed-width result for the same type `T`;
- successful `IntegerMul` leaves exactly the result local live and introduces no comparison branch, join, fault, or divergence; and
- nested represented multiplicative, additive, or mixed-tier operators may recursively lower complete operand producers before their enclosing arithmetic operation emits its own distinct Core operation.

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source multiplication MUST still lower the complete left producer, then the complete right producer, create one fresh wholly-vacant result local, apply exactly one `IntegerMul` to that local, and add no Core branch or join solely for multiplication. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, and plain-overflow result, and MUST NOT replace the accepted semantics with host arithmetic assumptions, implicit conversions, or a speculative generic arithmetic opcode.

### Plain fixed-width integer exclusive-or refinement

A faithful integer-exclusive-or lowerer first lowers the complete left producer exactly once and then lowers the complete right producer exactly once from the successful left continuation. The lowered operand locals MUST carry the same exact Core type identity corresponding to the admitted source type `T`; malformed retained source facts are rejected rather than converted or repaired.

After both operand-producer lowerings succeed:

1. create one fresh Core result local of that same exact Core integer type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `IntegerXor` whose destination is that result local, whose left operand is an ownership-transferring `Move` of the left operand-result local, and whose right operand is an ownership-transferring `Move` of the right operand-result local; and
3. continue lowering with the result local live and available as the lowered source exclusive-or result.

This refinement is valid under the accepted Core owners because:

- source left-producer execution completes before source right-producer execution begins;
- the lowered left result local remains live while the complete right producer lowers/executes, representing the source operation's held-left value;
- if left producer execution faults or diverges, right producer execution and `IntegerXor` are never reached;
- if right producer execution faults after left success, `IntegerXor` is never reached and the existing Core activation fault cleanup ends then-live compiler locals, including the local retaining the produced left integer value;
- if right producer execution diverges, the caller remains suspended and the left operand-result local remains live;
- once both producers succeed, `IntegerXor` evaluates its two `Move` operands left-to-right, consuming each produced operand local exactly once;
- the fresh result local satisfies the operation's wholly-vacant non-replacing destination rule;
- the Core numerical relation computes exactly the accepted representation-neutral fixed-width exclusive-or result for the same type `T`;
- successful `IntegerXor` leaves exactly the result local live and introduces no branch, join, fault, divergence, conversion, or additional numeric operation; and
- nested represented exclusive-or or tighter-tier operators may recursively lower complete operand producers before their enclosing exclusive-or emits its own one Core `IntegerXor`.

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source exclusive-or MUST still lower the complete left producer, then the complete right producer, create one fresh wholly-vacant result local, apply exactly one `IntegerXor` to that local, and add no Core branch or join solely for exclusive-or. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, and representation-neutral exclusive-or result, and MUST NOT replace the accepted semantics with host bitwise assumptions, implicit conversions, arithmetic rewrites, AND/OR/shift decompositions, or a speculative generic bitwise opcode.

### Plain fixed-width integer bitwise-OR refinement

A faithful integer-bitwise-OR lowerer first lowers the complete left producer exactly once and then lowers the complete right producer exactly once from the successful left continuation. The lowered operand locals MUST carry the same exact Core type identity corresponding to the admitted source type `T`; malformed retained source facts are rejected rather than converted or repaired.

After both operand-producer lowerings succeed:

1. create one fresh Core result local of that same exact Core integer type identity; the local is initially wholly vacant;
2. emit exactly one represented Core `IntegerOr` whose destination is that result local, whose left operand is an ownership-transferring `Move` of the left operand-result local, and whose right operand is an ownership-transferring `Move` of the right operand-result local; and
3. continue lowering with the result local live and available as the lowered source bitwise-OR result.

This refinement is valid under the accepted Core owners because:

- source left-producer execution completes before source right-producer execution begins;
- the lowered left result local remains live while the complete right producer lowers/executes, representing the source operation's held-left value;
- if left producer execution faults or diverges, right producer execution and `IntegerOr` are never reached;
- if right producer execution faults after left success, `IntegerOr` is never reached and the existing Core activation fault cleanup ends then-live compiler locals, including the local retaining the produced left integer value;
- if right producer execution diverges, the caller remains suspended and the left operand-result local remains live;
- once both producers succeed, `IntegerOr` evaluates its two `Move` operands left-to-right, consuming each produced operand local exactly once;
- the fresh result local satisfies the operation's wholly-vacant non-replacing destination rule;
- the Core numerical relation computes exactly the accepted representation-neutral fixed-width bitwise-OR result for the same type `T`;
- successful `IntegerOr` leaves exactly the result local live and introduces no branch, join, fault, divergence, conversion, or additional numeric operation; and
- nested represented bitwise-OR or tighter-tier operators may recursively lower complete operand producers before their enclosing bitwise OR emits its own one Core `IntegerOr`.

An implementation MAY represent this exact refinement differently internally, but the represented Core semantic program for one source bitwise OR MUST still lower the complete left producer, then the complete right producer, create one fresh wholly-vacant result local, apply exactly one `IntegerOr` to that local, and add no Core branch or join solely for bitwise OR. It MUST preserve the exact source type, successful operand consumption, ownership/fault/divergence behavior, and representation-neutral bitwise-OR result, and MUST NOT replace the accepted semantics with host bitwise assumptions, implicit conversions, arithmetic rewrites, AND/XOR/complement/shift decompositions, or a speculative generic bitwise opcode.

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

A faithful equality/inequality lowerer first lowers the complete left producer exactly once and then lowers the complete right producer exactly once. Both lowering operations complete before comparison branching begins. Each produced local MUST have Core Bool type; malformed retained source facts are rejected rather than repaired.

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

### Boolean short-circuit conjunction refinement

A faithful conjunction lowerer first lowers the complete left producer exactly once. The produced local MUST have Core Bool type; malformed retained source facts are rejected rather than repaired. The right producer is retained by typed source facts, and the Core operations emitted for that producer MUST be reachable only from the left-`true` target so that concrete left-`false` execution performs none of them.

After left-producer lowering succeeds:

1. create one fresh Core Bool result local, initially wholly vacant;
2. create a true-target block, a false-target block, and one join block;
3. terminate the current block with existing Core `Branch` whose condition is an ownership-transferring `Move` of the left-result local;
4. in the false-target block, `Init` the result local from semantic Bool constant `false`, then `Goto` the join block;
5. in the true-target block, lower the complete right producer exactly once under its existing producer semantics; after successful right completion, require the produced local to have Core Bool type, `Init` the same result local from an ownership-transferring `Move` of that right-result local, and `Goto` the join block; and
6. continue lowering from the join with the result local live and available as the lowered conjunction result.

This refinement is valid under the accepted Core owners because:

- the complete source left producer finishes before short-circuit selection;
- the `Branch` moves and consumes the produced left Bool exactly once;
- concrete left `false` execution reaches only the false target, performs no right-producer operation, and initializes the result exactly once to `false`;
- concrete left `true` execution reaches only the true target, where complete right-producer lowering/execution occurs before the result initialization;
- if the left producer faults or diverges, the branch, result paths, and right producer are never reached;
- if right producer execution faults or diverges on the true path, no conjunction result is produced and the false path is not executed;
- the fresh result local is wholly vacant before the one `Init` reached on each successful concrete path;
- Core path-state validation propagates the branch edges independently and therefore needs no source/Core state union, meet, join, widening, or new merge relation merely because both successful paths reach the same join block;
- source validation has already proved exact equality between the skipped-right normal structural-ownership state and successful-right normal structural-ownership state, so lowering MUST NOT add cleanup or other state repair merely to make the join possible; and
- every successful incoming Core state at the join has the same Bool result local live exactly once.

This refinement introduces no Core `And`, logical opcode, predicate operation, new branch kind, new fault edge, borrowing rule, state-merge rule, or reference-machine extension. An implementation MAY use another Core program shape only when it preserves the source short-circuit ordering, left/right successful consumption, exact Bool result, fault/divergence behavior, and absence of right execution on the left-`false` path.

The Boolean refinements introduce no Core `Not`, `Eq`, `Ne`, `And`, comparison/logical operation, grouping operation, reference-machine extension, source/Core state merge, host-language Boolean authority, or backend requirement. The integer-neg and integer-complement refinements each consume exactly one existing Core `IntegerSub` after complete source operand lowering and introduce no Core arithmetic or bitwise operation. The integer-add, float-add, integer-sub, float-sub, integer-mul, integer-xor, and integer-or refinements introduce exactly their distinct represented Core `IntegerAdd`, `FloatAdd`, `IntegerSub`, `FloatSub`, `IntegerMul`, `IntegerXor`, and `IntegerOr` operations and no additional Core control-flow or numeric operation. The source selected-value wrapper is erased after typed validation and introduces no Core wrapper operation or ambient contract state; its only surviving semantic fact is the selected contract carried by the governed `FloatAdd` or `FloatSub` occurrence.

## Deliberate boundaries

This revision defines no:

- floating-point unary negation, unary plus, increment/decrement, or other numeric unary operator beyond the represented plain fixed-width integer negation and integer bitwise complement;
- arithmetic operator other than plain same-type fixed-width integer negation, multiplication, addition, and subtraction and same-format binary floating addition and subtraction for exact `F16`, `F32`, or `F64`;
- binary bitwise operation beyond plain same-type fixed-width integer exclusive-or and bitwise OR, shift operation, physical bit-pattern operation, or generic bitwise abstraction;
- floating-point multiplication, division, remainder, fused operation, cross-format arithmetic, or mixed integer/floating arithmetic;
- checked, saturating, or explicitly wrapping source variant of the represented plain fixed-width integer arithmetic operations;
- integer, floating, nominal-record, pointer, reference, function, structural, representation, or generic value equality/inequality;
- ordering or other comparison operator;
- `||` or another short-circuit logical operator beyond the represented Boolean conjunction;
- conversion, cast, coercion, numeric promotion, or operand-derived/default arithmetic typing;
- Unit/tuple grouping form or general expression system beyond the bounded contextual grouping grammar owned by `concrete-syntax.md`;
- ungrouped multiplicative chaining, additive chaining, exclusive-or chaining, bitwise-OR chaining, equality chaining, logical-conjunction chaining, or comparison chaining;
- general binary precedence or associativity hierarchy beyond the bounded multiplicative, additive, exclusive-or, bitwise-OR, equality, and logical-conjunction tiers owned by `concrete-syntax.md`;
- arbitrary postfix/member/method expression;
- assignment expression, compound assignment, field assignment, or general place/lvalue operation;
- source numeric-contract selection beyond the represented operation-local `fast` qualification of one governed floating-addition or floating-subtraction root, including no explicit source `standard`/`reproducible` spelling, block/function/module default, lexical or dynamic inheritance, callable contract dimension, or ambient source/runtime numeric mode;
- refutable, literal, alternative, or guard pattern, or `match` relation;
- reference/borrow/lifetime/source-`unsafe` operation;
- generic/trait/coherence operation;
- function value, closure, or capture operation;
- const/static evaluation relation;
- additional fault/panic/catch/recovery operation;
- ABI/layout/FFI/linkage operation; or
- Exec, Model, runtime, or backend source operation.

Those concerns require their own accepted semantic owners and concrete consumers before this operator family is extended.
