# Core Floating-Point Semantics

Status: **provisional normative; incomplete**

Runen distinguishes three floating-point contracts:

- **standard** — portable baseline without arbitrary fast-math transformations;
- **reproducible** — stronger cross-realization repeatability, with emulation or rejection when required;
- **fast** — explicit documented numerical relaxations.

## Numeric-contract authority

For every floating-point operation governed by this contract system, exactly one of `standard`, `reproducible`, or `fast` MUST be established before physical realization.

If an applicable Runen semantic rule explicitly establishes a numeric contract for an operation, that explicit semantic selection applies subject to the exactly-one-contract requirement above. Otherwise, `standard` is established for that operation by default.

This fallback defines only semantic contract selection. It does not define where an explicit semantic selection may originate or how source requests, scopes, inherits, or spells one. Those source-language mechanisms are not defined by this revision.

The selected numeric contract is semantic input. It is not a target class, backend capability, compiler optimization level, runtime mode, hardware feature, or realization choice. A realization MUST preserve every applicable rule of the selected contract and MUST NOT silently substitute another numeric contract because a target lacks a preferred operation or exposes additional numerical transformation freedom.

When direct realization is unavailable, an implementation MAY use emulation when that emulation preserves the selected contract. If satisfying the selected contract requires a hard environment capability, the applicable contract MUST expose that requirement to environment admission, where it is either admitted or rejected according to the language lifecycle. Once admitted, inability to realize the contract directly is not permission to weaken it.

A physical implementation may use greater internal precision or another stronger implementation technique only when the resulting Runen-observable behavior remains permitted by the selected contract. Physical mechanism does not strengthen, weaken, or replace the semantic contract by itself.

The `fast` contract authorizes only numerical relaxations that Runen explicitly grants to `fast`; enabling a backend's aggregate fast-math mode does not by itself make all of that mode's transformations legal. Conversely, `standard` and `reproducible` gain no implicit transformation freedom merely because a target or backend supports it.

## Contract refinement

`standard` supplies the baseline floating-point numerical obligations and permissions.

`reproducible` follows every applicable `standard` rule, except only to the extent that a `reproducible`-specific rule strengthens an obligation or narrows the numerical behavior permitted by that rule. A `reproducible`-specific rule MUST NOT weaken an applicable `standard` requirement or admit numerical behavior that the applicable `standard` rule forbids.

`fast` follows every applicable `standard` rule, except only to the extent that a `fast`-specific rule explicitly grants an additional numerical relaxation for behavior that rule names. In the absence of such a `fast`-specific relaxation, the applicable `standard` rule remains controlling. Selecting `fast` is therefore not a general license to ignore unspecified baseline requirements.

A `fast` numerical relaxation does not by implication relax evaluation authority, effects, control flow, memory semantics, Exec ordering or synchronization, resource rules, or another non-numeric semantic contract. Any change to those semantics requires authority from their own normative owners.

These refinement relationships compose the obligations of an already-selected numeric contract. They do not select a contract for an operation and do not authorize a realization to switch contracts; the numeric-contract authority rules above remain controlling.

## Floating exception observability

For floating operations governed by these numeric contracts, Runen uses non-stop result semantics with respect to physical floating exception mechanisms.

Whether a physical realization would classify an operation as overflow, underflow, inexact, divide-by-zero, invalid operation, or an analogous floating exception condition does not by itself produce a Core Fault, cancellation, environment failure, or another abnormal Runen outcome. An operation's Runen result remains determined only by its applicable semantic and numeric rules.

Floating exception or status flags maintained by a realization are not Runen-observable state under these contracts and are not semantic inputs to later floating operations. A realization MUST NOT let incidental hardware trap enablement, sticky status flags, host floating-environment state, or backend exception metadata change Runen-observable behavior.

This rule does not itself define the result of division by zero, an invalid operation, arithmetic involving infinity, signaling or quiet NaNs, or another operation whose result semantics remain open. An independently defined operation may still have a Runen Fault or ordinary error result where its own canonical semantic owner explicitly establishes one; a physical floating exception mechanism is not authority for that outcome.

A realization MAY configure physical floating exception controls, ignore non-observable status, or emulate where needed to preserve this contract. Any required hard capability remains governed by the existing numeric-contract and environment-admission rules. The `fast` contract does not make traps or floating status observable and gains no extra numerical result freedom merely from backend exception or fast-math modes.

## Binary floating finite value format

For every binary floating type governed by these contracts, the applicable semantic type contract fixes three value-format parameters:

- the complete significand precision `p`, an integer with `p >= 2`;
- the minimum normal exponent `emin`;
- the maximum normal exponent `emax`, with `emin <= emax`.

The nonzero finite values of that semantic format are exactly the following values and their negatives:

- **normal values:** `m * 2^(e - (p - 1))`, where `e` is an integer in `[emin, emax]` and `m` is an integer in `[2^(p - 1), 2^p - 1]`;
- **subnormal values:** `m * 2^(emin - (p - 1))`, where `m` is an integer in `[1, 2^(p - 1) - 1]`.

The normal and subnormal sets above are semantic value sets. Their membership, `p`, `emin`, and `emax` are fixed by the type contract and MUST NOT change with numeric-contract selection or physical realization. A backend MUST NOT silently substitute another finite value lattice merely because another native format is preferred or available.

This value-format definition does not prescribe a storage width, byte layout, exponent-field or significand-field encoding, ABI representation, NaN payload encoding, or address-level representation. It also does not define which source types select a binary floating format or which concrete `(p, emin, emax)` triples those types use.

This value-format section by itself does not define signed zero, signed infinity, NaN class structure or identity, operation-specific subnormal handling, rounding, overflow or underflow result selection, flushing, conversions, or literals. Those concerns have semantics only where another applicable rule explicitly defines them.

## Signed zero value domain

Every binary floating type governed by these contracts contains two signed zero values, written `+0` and `-0` in this specification. Both have mathematical magnitude zero, but they are different members of the floating value domain with different zero signs.

The zero-sign distinction is a semantic floating-value fact fixed by the type contract. Numeric-contract selection and physical realization MUST NOT merge, remove, or silently change those signed-zero alternatives merely because a backend or host treats zero sign as insignificant.

This value-domain distinction does not define whether a language equality, comparison, hashing, or another operation treats `+0` and `-0` identically or differently. It likewise does not define which signed zero is produced by arithmetic, rounding, conversion, literal evaluation, or another operation. Those results remain owned by their applicable operation-specific contracts.

No physical sign bit, storage layout, ABI encoding, byte representation, or source spelling follows from the existence of the two semantic zero values. A future `fast` rule may relax zero-sign significance only if it explicitly grants that numerical relaxation under the contract-refinement rules above; backend `nsz` or aggregate fast-math behavior is not semantic authority by itself.

## Signed infinity value domain

Every binary floating type governed by these contracts contains two signed infinity values, written `+∞` and `-∞` in this specification. They are special floating values outside the finite normal/subnormal value lattice and are different members of the floating value domain with different signs.

The existence and sign distinction of these infinity values are semantic type facts fixed independently of numeric-contract selection and physical realization. A realization MUST NOT remove, merge, or assume away these infinity alternatives merely because a backend or target can operate under a no-infinity assumption.

This value-domain distinction does not define equality, comparison, hashing, mathematical ordering, arithmetic involving infinity, overflow production, division by zero, conversion or literal behavior, or interaction with NaN. Those semantics remain owned by their applicable operation-specific contracts.

No physical exponent pattern, sign bit, storage layout, ABI encoding, byte representation, or source spelling follows from the two semantic infinity values. A future `fast` rule may relax infinity-related numerical behavior only if it explicitly grants that relaxation under the contract-refinement rules above; backend `ninf` or aggregate fast-math behavior is not semantic authority by itself.

## NaN value class

Every binary floating type governed by these contracts contains a non-empty set of **NaN values**. Every NaN value is a special floating value distinct from every finite value, signed zero, and signed infinity value of that type.

A NaN value does not denote an exact real-number value or real magnitude for the arithmetic and rounding rules in this document that operate on exact real results. Those rules therefore do not apply to a NaN merely because a physical representation uses the same floating format.

NaN membership is a semantic type fact. A realization MUST NOT remove the NaN value class or assume that NaN values are absent merely because a backend or target can operate under a no-NaN assumption. The `fast` contract gains no implicit no-NaN relaxation from backend `nnan` or aggregate fast-math behavior.

This revision does not determine whether a type's NaN set contains one or multiple semantic values. It does not assign semantic sign, payload, quiet or signaling state, canonical or preferred identity, ordering, or representation to NaN values.

Whenever a numerical result rule in this document requires a result to belong to the result type's NaN value class and does not state a stronger member-selection rule, **any** NaN member of that result type is a permitted result. No relation to an input NaN member is required: preservation, propagation, canonicalization, sign, payload, and quiet/signaling identity are not implied. Member choice under such a class-only result rule is explicit permitted variation, not an open semantic gap, and backend or host NaN behavior is not an additional semantic input.

For purposes of the `standard`, `reproducible`, and `fast` numeric result contracts defined by this document, any two NaN values of the same semantic floating type are **numerically equivalent** unless a later operation-specific numeric rule explicitly defines a stronger member-sensitive relation. This numeric equivalence does not define language equality, comparison, hashing, total ordering, representation identity, bit equality, serialization equality, or observational equivalence under a future representation-sensitive operation.

A later representation or ABI contract may refine how physical NaN encodings map to semantic NaN values while preserving every accepted semantic rule. This section requires neither an injective nor a canonical mapping and does not define source NaN literals, bitcasts, bytes, ABI layout, or serialization.

## Finite basic arithmetic

For an already-admitted basic binary floating operation, suppose each operand value is either a nonzero finite floating value or a signed zero, and an applicable contract has already established a binary floating result type governed by this document. This section defines only the numerical result relation after those facts are established; it does not define operand typing, promotions, conversions, or result-type selection.

Interpret each admitted nonzero finite operand by the exact real value supplied by its applicable finite value format. For exact real arithmetic, either signed zero contributes mathematical value `0`; its semantic zero sign remains available for the exact-zero result rules below. A nonzero finite operand has the positive or negative sign of its exact real value.

Under `standard`, floating addition, subtraction, multiplication, and division use the exact real relations `x + y`, `x - y`, `x * y`, and `x / y`, respectively. The division relation in this section applies only when the divisor is a nonzero finite value; a signed-zero divisor remains outside this section.

When that exact mathematical result is nonzero, the operation explicitly supplies it as a nonzero exact finite real quantity to the binary floating rounding relation below for the already-established result type exactly once. It is not a separately observable Runen value and need not itself be representable in that result type. In particular, a nonzero exact real result beyond the largest finite representable magnitude is handled by the upper-bound rounding rule rather than by host or backend overflow behavior.

When the exact mathematical result is zero, `standard` selects a signed-zero result as follows:

- **addition:** two signed-zero operands of the same sign produce that signed zero; every other exact-zero addition case produces `+0`;
- **subtraction:** `-0` minus `+0` produces `-0`; every other exact-zero subtraction case produces `+0`;
- **multiplication:** when at least one operand is signed zero, the result is `-0` exactly when the operand signs differ, and `+0` otherwise;
- **division:** when the numerator is signed zero and the divisor is nonzero finite, the result is `-0` exactly when the operand signs differ, and `+0` otherwise.

These exact-zero rules do not apply when a nonzero exact mathematical result merely rounds to zero. That case remains governed by the zero-boundary rounding rule, which preserves the sign of the nonzero exact result.

By the contract-refinement rules, `reproducible` and `fast` follow these basic-operation rules unless a later contract-specific rule explicitly narrows or relaxes the named numerical behavior. The existing `fast` reassociation permission may change grouping only where that permission applies; it does not by itself alter the numerical relation or signed-zero result rule of an individual basic operation. Backend `nsz`, target latitude around signed zero, or aggregate fast-math behavior supplies no implicit Runen relaxation.

Additional determinate infinity and zero-divisor cases are defined separately below. NaN-class outcomes for basic arithmetic are also defined separately below. This section does not define unary negation, remainder, fused operations, source operator spellings, comparison or hashing behavior, sign-inspection APIs, or physical instructions.

## Determinate special-value basic arithmetic

For an already-admitted basic binary floating operation, this section defines additional cases whose non-NaN operands include signed infinity or a signed-zero divisor and whose result is determinately a signed infinity or signed zero. The operation's binary floating result type is already established by another applicable contract; this section does not define operand typing, promotions, conversions, or result-type selection.

For this section, two signed non-NaN values have **equal signs** when both are positive or both are negative, and **opposite signs** otherwise. A nonzero finite value has the sign of its exact real value; signed zero and signed infinity have their semantic signs defined above. This terminology is local to numerical result selection and does not define a source sign-observation operation.

Under `standard`, cases already covered by finite basic arithmetic and its rounding rules remain unchanged. The additional determinate cases are:

### Addition

- infinities of the same sign produce that signed infinity;
- one infinity plus any nonzero finite or signed-zero operand produces that infinity.

### Subtraction

- infinities of opposite signs produce the left-hand infinity;
- an infinity minus any nonzero finite or signed-zero operand produces the left-hand infinity;
- any nonzero finite or signed-zero operand minus an infinity produces the infinity with the opposite sign from the right-hand infinity.

### Multiplication

- when one operand is an infinity and the other is an infinity or a nonzero finite value, the result is `+∞` when the operand signs are equal and `-∞` when they are opposite.

### Division

- infinity divided by a nonzero finite value or signed zero produces `+∞` when the operand signs are equal and `-∞` when they are opposite;
- a nonzero finite value divided by signed zero produces `+∞` when the operand signs are equal and `-∞` when they are opposite;
- a nonzero finite value or signed zero divided by infinity produces `+0` when the operand signs are equal and `-0` when they are opposite.

By the contract-refinement rules, `reproducible` and `fast` follow these determinate special-value rules unless a later contract-specific rule explicitly narrows or relaxes the named numerical behavior. Backend `nnan`, `ninf`, `nsz`, aggregate fast-math behavior, physical floating exception behavior, or target latitude supplies no implicit Runen relaxation.

The remaining basic-operation forms whose result belongs to the NaN class are defined separately below. This section does not define equality, comparison, hashing, total ordering, sign-observation APIs, unary negation, remainder, fused operations, source operator spellings, or physical instructions.

## NaN-class basic arithmetic outcomes

For an already-admitted basic binary floating operation with an already-established binary floating result type governed by this document, `standard` requires the operation result to belong to that result type's NaN value class when either operand is already a NaN value.

When both operands are non-NaN, `standard` likewise requires a NaN-class result for exactly the following basic-operation forms not covered by the finite or determinate special-value rules above:

- addition of opposite-sign infinities;
- subtraction of same-sign infinities;
- multiplication of signed zero by signed infinity, in either operand order;
- division of signed infinity by signed infinity;
- division of signed zero by signed zero.

These requirements define the result's **value class**. By the NaN class rule above, any NaN member of the result type is permitted for each such occurrence and no relation to an input NaN member is required. The choice does not assign or preserve sign, payload, quiet/signaling state, preferred or canonical identity, equality, ordering, hashing, or representation properties.

By the contract-refinement rules, `reproducible` and `fast` inherit the NaN-class outcome and its same-type numeric-equivalence relation unless a later contract-specific rule explicitly narrows or relaxes the named numerical behavior. Backend `nnan`, canonicalization, payload propagation, aggregate fast-math behavior, or physical signaling behavior supplies no additional Runen rule.

## Fixed-width integer to binary floating conversion

For an already-admitted conversion whose source is a semantic fixed-width integer value `n` and whose result type is an already-established binary floating type governed by this document, interpret `n` as its exact mathematical integer value and therefore as an exact finite real quantity.

Under `standard`, integer zero converts to `+0`. A nonzero integer value explicitly supplies its exact finite real quantity to the binary floating rounding relation below for the established result type exactly once. An exactly representable integer therefore converts exactly; a nonrepresentable integer is rounded only by that relation, including its lower and upper format boundaries.

By the contract-refinement rules, `reproducible` and `fast` follow this `standard` conversion unless a later conversion-specific rule explicitly narrows or relaxes it. The `fast` subnormal input- and result-flushing permissions below apply only to the basic floating `+`, `-`, `*`, and `/` operation occurrences they name and do not by themselves flush this conversion's source or result.

This conversion uses the semantic integer value, not a physical integer encoding or bit pattern. It does not define source cast syntax, implicit conversion or promotion, conversion insertion, concrete integer or floating source type spellings, vector conversion, literals, constant-evaluation diagnostics, physical representation, ABI, backend instructions, or target behavior.

## Binary floating to binary floating conversion

For an already-admitted conversion whose source value belongs to a binary floating type governed by this document and whose result type is an already-established binary floating type governed by this document, `standard` determines the result from the source semantic value rather than from a physical representation.

A nonzero finite source value is interpreted by the exact real value supplied by its source type's accepted finite value format and explicitly supplies that nonzero exact finite real quantity to the binary floating rounding relation below for the destination type exactly once.

The remaining source value classes convert as follows:

- source `+0` produces destination `+0`;
- source `-0` produces destination `-0`;
- source `+∞` produces destination `+∞`;
- source `-∞` produces destination `-∞`;
- a source NaN value produces a value belonging to the destination type's NaN value class.

For a NaN source, any NaN member of the destination type is permitted by the NaN class rule above; no propagation, preservation, canonicalization, sign, payload, or quiet/signaling relation to the source member is required. Backend or physical target behavior supplies no additional member-selection rule.

By the contract-refinement rules, `reproducible` and `fast` follow this `standard` conversion unless a later conversion-specific rule explicitly narrows or relaxes it. The `fast` subnormal input- and result-flushing permissions below apply only to the basic floating `+`, `-`, `*`, and `/` operation occurrences they name and do not by themselves flush a subnormal source value or subnormal destination result of this conversion.

A same-format conversion follows these same semantic value rules. For a nonzero finite source, the destination has the same exact real value because that value is exactly representable in the same format; signed zeros and signed infinities retain their sign. No physical bit-identity rule follows for NaN values merely because source and destination semantic formats are the same.

This conversion does not define source cast syntax, implicit conversion or promotion, conversion insertion, concrete binary floating source type spellings, vector conversion, literals, constant-evaluation diagnostics, physical representation, bitcasts, ABI, backend instructions, or target behavior.

## Finite in-range binary floating to fixed-width integer conversion

For an already-admitted conversion whose source is a semantic binary floating value governed by this document and whose result type is an already-established fixed-width integer type of width `N` and signedness, this section defines the `standard` result only when the source is finite and truncation toward zero produces an integer admitted by the destination value interval.

Interpret a nonzero finite source by the exact real value `x` supplied by its source floating format. For either signed-zero source, let `x = 0` for this conversion.

The **truncation toward zero** of `x`, written `t`, is the unique integer satisfying:

- when `x >= 0`, `t <= x < t + 1`;
- when `x < 0`, `t - 1 < x <= t`.

The destination integer value interval is:

- `[0, 2^N - 1]` for an unsigned result type;
- `[-2^(N-1), 2^(N-1) - 1]` for a signed result type.

If `t` belongs to that destination interval, the Runen conversion result is exactly destination integer value `t`.

The admissibility test is applied to `t`, not directly to `x`. A finite source whose exact real value lies slightly beyond a destination bound may therefore still have a defined result under this section when truncation yields the boundary integer. Likewise, a negative finite source strictly between `-1` and `0` truncates to integer zero and is admitted by an unsigned destination.

Both `+0` and `-0` convert to integer `0`; this section introduces no integer zero-sign distinction.

By the contract-refinement rules, `reproducible` and `fast` follow this `standard` finite in-range conversion unless a later conversion-specific rule explicitly narrows or relaxes it. The `fast` subnormal input- and result-flushing permissions below apply only to the basic floating `+`, `-`, `*`, and `/` operation occurrences they name and do not alter the source value for this conversion.

This section by itself does not define the conversion outcome when the source is NaN or signed infinity, or when truncation produces an integer outside the destination interval. The non-NaN cases are extended by the clamping rule below, and the NaN case is defined by the subsequent NaN conversion rule.

This section also does not define source cast syntax, implicit conversion or promotion, conversion insertion, vector conversion, literals, constant-evaluation diagnostics, physical representation, ABI, backend instructions, target behavior, compiler IR, or executable oracle representation.

## Non-NaN floating to fixed-width integer clamping

For the same already-admitted conversion and destination integer interval `[lo, hi]` defined above, `standard` extends floating-to-integer conversion to every non-NaN source value.

For a finite source, compute the toward-zero truncated mathematical integer `t` exactly as defined above. The conversion result is:

- `lo` when `t < lo`;
- exactly `t` when `lo <= t <= hi`;
- `hi` when `t > hi`.

Thus every result already defined by the finite in-range section is preserved exactly, while a finite truncated result outside the destination interval clamps to the nearest destination bound.

For signed infinity:

- source `+∞` produces destination integer `hi`;
- source `-∞` produces destination integer `lo`.

For an unsigned destination, `lo = 0`, so `-∞` converts to integer zero. For a signed destination, `lo = -2^(N-1)`.

This clamping is a conversion rule over mathematical destination values. It does not first wrap modulo `2^N`, does not invoke the explicit integer saturating-arithmetic mode, and does not depend on whether `lo` or `hi` is exactly representable in the source floating format.

By the contract-refinement rules, `reproducible` and `fast` follow this `standard` non-NaN conversion unless a later conversion-specific rule explicitly narrows or relaxes it. The `fast` subnormal input- and result-flushing permissions below remain scoped only to basic floating `+`, `-`, `*`, and `/` operation occurrences and do not become conversion authority.

A NaN source remains outside this clamping section and is governed only by the separate NaN conversion rule below. This clamping section supplies no NaN member, payload, sign, or integer-result authority.

This section does not define explicit checked, trapping, or separately named saturating conversion forms; source cast syntax or implicit conversion policy; vector conversion; literals or constant-evaluation diagnostics; physical representation or ABI; backend instructions or target taxonomy; compiler IR; or executable oracle representation.

## NaN floating to fixed-width integer conversion

For the same already-admitted binary-floating-to-fixed-width-integer conversion under `standard`, if the source value belongs to the source floating type's NaN value class, the conversion result is exactly destination integer value `0`.

This result is independent of which semantic NaN member is supplied. Every source NaN member converts to integer zero; this rule does not inspect or infer a NaN sign, payload, signaling state, preferred or canonical identity, or physical representation.

By the contract-refinement rules, `reproducible` and `fast` follow this `standard` NaN conversion result unless a later conversion-specific rule explicitly narrows or relaxes it. Backend `nnan`, poison or undefined behavior, host-language casts, target conversion conventions, and physical NaN encodings do not alter the result.

Together with the finite and non-NaN clamping rules above, this defines the numerical result of the already-admitted binary-floating-to-fixed-width-integer conversion for every source floating value class.

This rule selects only an integer conversion result. It does not define NaN equality/comparison/hash/order, NaN representation or payload semantics, source conversion syntax or policy, explicit checked/trapping/saturating conversion forms, vector conversion, literals, constant-evaluation diagnostics, physical representation, ABI, backend instructions, target taxonomy, compiler IR, or executable oracle representation.

## Finite multiply-add contraction

An **eligible finite multiply-add contraction occurrence** is one dynamic evaluation of an already-established floating addition for which one consumed operand value is the result value of an already-established floating multiplication evaluation and the other consumed addition operand is an already-established value `z`. Let `x` and `y` be the operand values consumed by that multiplication evaluation. Eligibility requires all of the following for that occurrence:

- `x`, `y`, and `z` are nonzero finite floating values;
- the established multiplication result type and addition result type are the same binary floating type governed by this document;
- the exact mathematical value `(x * y) + z` is nonzero;
- no conversion, semantic operation, or other value-producing boundary transforms the multiplication result value before that value is consumed as the addition operand.

Without contraction, evaluation remains the already-defined basic-operation semantics: the multiplication result is rounded according to its result type, and the addition then consumes that rounded value and produces its own rounded result.

Under `fast`, a realization MAY instead contract an eligible occurrence. Contracted evaluation computes the exact real quantity `(x * y) + z` as if with unbounded intermediate range and precision. No Runen rounding, overflow-result selection, underflow-result selection, or finite-format truncation occurs between multiplication and addition. The one exact result is then explicitly supplied as a nonzero exact finite real quantity to the binary floating rounding relation below for the established addition result type exactly once.

Therefore an eligible `fast` occurrence explicitly permits either the ordinary uncontracted result or the contracted one-round result. Choosing whether to contract is permitted numerical variation under the `fast` contract; backend behavior is not additional semantic input.

Eligibility and permission are occurrence-local. A realization that performs contraction before the operand values of affected dynamic evaluations are known MUST establish that every affected occurrence satisfies these eligibility conditions, or otherwise prove that the transformed realization preserves the applicable Runen semantics for ineligible occurrences. This rule does not make an ineligible occurrence eligible merely because it shares source, IR, or physical instruction structure with an eligible one.

The multiplication result value may occupy either operand position of the addition. This permission does not reorder the addition operands and does not authorize reassociation, operand permutation, omission, duplication, reciprocal replacement, reduced precision, approximation, or changes to the evaluation or effects that establish `x`, `y`, or `z`. The multiplication evaluation may still have other semantic consumers; contracting this use does not by itself remove or alter obligations for those other consumers. This rule does not manufacture a contraction opportunity by regrouping a mixed arithmetic expression. Any other transformation composed with contraction requires independent Runen authority.

`standard` and `reproducible` gain no result-changing contraction permission from this section. A realization may physically fuse their operations only when it proves that the resulting Runen-observable behavior remains permitted by the already-applicable contract.

Backend fused instructions, LLVM-style contraction flags, WGSL fusion latitude, or another physical realization mechanism do not widen the eligible set and are not semantic input.

This section does not define an exact-zero contracted result, contraction involving signed-zero, signed-infinity, or NaN operand values, including which result class such a contraction would have, multiply-subtract or negated fused variants, a standalone fused operation, source syntax, compiler IR, physical instructions, or environment capability requirements.

The **binary floating rounding relation** is the result relation supplied collectively by the interior finite, zero-boundary, and upper-bound rounding sections below. An applicable operation consumes this relation only when its own canonical semantics explicitly supplies a nonzero exact finite real quantity `x` and an already-established binary floating result type to it. Merely having a real-valued mathematical interpretation does not make another operation consume this relation.

## Interior finite rounding

For a nonzero exact finite real quantity `x` explicitly supplied to the binary floating rounding relation, `standard` uses the following rounding rule when the already-established result type has the binary finite value format above.

If `x` is a nonzero finite value exactly representable by that result format, the rounded result is exactly `x`.

Otherwise, this section supplies a rounded result only when there are two nonzero finite representable values `a < x < b` such that `a`, `x`, and `b` have the same sign and no other nonzero finite representable value lies strictly between `a` and `b`. In that case:

- if `|x - a| < |b - x|`, the rounded result is `a`;
- if `|b - x| < |x - a|`, the rounded result is `b`;
- if `|x - a| = |b - x|`, the rounded result is the one of `a` or `b` whose magnitude has an even canonical format significand integer `m`.

For every positive nonzero finite representable value, its **canonical format significand integer** is the unique `m` supplied by its normal or subnormal representation in the value-format definition above. A negative value uses the canonical `m` of its positive magnitude. For adjacent same-sign representable values in a tie, exactly one candidate has even canonical `m`.

By the contract-refinement rule, `reproducible` and `fast` follow this `standard` rounding rule unless a later contract-specific rule explicitly narrows or relaxes this exact numerical behavior.

This section includes rounding between adjacent normal values, between adjacent subnormal values, and across the nonzero subnormal/normal boundary. The interior rule does not itself supply a result when zero is a bounding candidate; that lower boundary is defined separately below. It also does not itself supply a result beyond the largest finite magnitude; that upper boundary is defined separately below. Determinate basic infinity and zero-divisor results and NaN-class basic outcomes are defined above. Exact-zero sign outside the basic operations defined above, contraction outside the finite multiply-add case above, literals, transcendental accuracy except where another section explicitly defines it, reduced precision, and any contract-specific flushing relaxation remain open.

## Zero-boundary rounding

Let `q = 2^(emin - (p - 1))`, the smallest positive subnormal magnitude of a binary floating result format.

For a nonzero exact finite real quantity `x` explicitly supplied to the binary floating rounding relation and satisfying `|x| < q`, `standard` extends nearest/ties-to-even to the signed-zero boundary:

- if `0 < x < q / 2`, the rounded result is `+0`;
- if `q / 2 < x < q`, the rounded result is `+q`;
- if `x = q / 2`, the rounded result is `+0`;
- if `-q / 2 < x < 0`, the rounded result is `-0`;
- if `-q < x < -q / 2`, the rounded result is `-q`;
- if `x = -q / 2`, the rounded result is `-0`.

At either halfway point, signed zero is the ties-to-even choice because this lower-bound lattice may equivalently index the magnitudes `0` and `q` by integers `0` and `1`; index `0` is even. This is a mathematical tie rule and does not define a physical significand encoding for zero.

Therefore, when a nonzero exact finite result rounds to zero under this boundary rule, the rounded zero has the sign of the exact result. This consequence does not determine the sign of an operation whose exact mathematical result is zero.

By the contract-refinement rule, `reproducible` and `fast` follow this `standard` zero-boundary rule unless a later contract-specific rule explicitly narrows or relaxes this exact numerical behavior. A backend flush-to-zero or denormal mode supplies no such relaxation by itself.

## Fast subnormal input flushing

For one already-defined basic floating `+`, `-`, `*`, or `/` operation occurrence under `fast`, before applying that operation's numerical and result-class rules, each consumed operand occurrence whose value is a nonzero subnormal MAY independently be replaced by the signed-zero value having the same sign. Leaving the operand as its original subnormal value remains permitted.

After the input-flush choices for that operation occurrence have been made, the existing Runen rules apply to the resulting operand values. A subnormal divisor replaced by signed zero is therefore governed by the existing signed-zero divisor rules; a subnormal operand replaced by zero next to infinity is governed by the existing zero/infinity and NaN-class rules. This section does not create a duplicate result table for those consequences.

Input flushing under this section preserves sign. A negative subnormal input can flush only to `-0`, and a positive subnormal input only to `+0`. This section grants no general permission to ignore or merge signed zero.

For a use considered for finite multiply-add contraction, the input-flush choices affecting the multiplication operands `x` and `y` and the other addition operand `z` are made before the finite contraction eligibility of that `fast` evaluation choice is tested. The contraction rule consumes those post-substitution operand values. Because the accepted contraction rule requires `x`, `y`, and `z` to be nonzero finite values, replacing any of them by signed zero makes that contraction choice ineligible. `fast` may instead leave a subnormal operand unflushed and contract when every other contraction condition holds.

If the multiplication evaluation has other semantic consumers, its input-flush choices are the same choices that govern its ordinary multiplication result. Contracting one addition use does not authorize that contracted use to recover a pre-flush `x` or `y` value that the multiplication evaluation did not consume.

For an ordinary basic operation after input substitution, the existing arithmetic/result rules determine its result first; the separate `fast` subnormal result-flushing rule below may then apply when that result is nonzero subnormal. For a multiply-add use that remains eligible and is contracted, the contraction rule determines its one final result and result flushing may then apply to that final result when subnormal.

`standard` and `reproducible` gain no subnormal input-flushing permission from this section.

Backend FTZ/DAZ modes, WGSL latitude, SPIR-V denormal execution modes, LLVM denormal environment settings, or another physical mechanism do not widen this permission. A realization using such a mechanism MUST preserve the ordering and every Runen obligation not explicitly relaxed by an applicable rule.

This section does not define positive-zero replacement of a negative subnormal input, flushing of normal, infinity, NaN, or already-zero inputs, conversion or literal input flushing, transcendental or reduction input flushing, contraction of post-flush zero/special-value multiply-add cases, reduced precision, approximation, source selection of `fast`, compiler IR, physical instructions, or environment capability requirements.

## Fast subnormal result flushing

For one already-defined basic floating `+`, `-`, `*`, or `/` operation occurrence under `fast`, first determine its result using every otherwise-applicable Runen numerical rule, including any separately permitted finite multiply-add contraction choice that changes how that addition result is computed.

If that otherwise-determined result is a nonzero subnormal value `r`, `fast` additionally MAY replace `r` with the signed-zero value having the same sign as `r`. The unflushed subnormal value remains permitted. This is explicit permitted numerical variation under `fast`; physical flush-to-zero behavior is not additional semantic input.

This relaxation applies independently to each basic-operation result occurrence. In an uncontracted multiply-add sequence, the multiplication result and the later addition result are separate operation results and each may be flushed when it is subnormal. For a use contracted under the finite multiply-add contraction rule, that contracted use has no separately rounded intermediate multiplication result; this rule can flush only the one final contracted addition result for that use. Any independently required ordinary multiplication result, including a result consumed elsewhere, remains governed by its own operation and may separately be flushed only as its own result occurrence permits.

Flushing under this section preserves sign. A negative subnormal result can flush only to `-0`, and a positive subnormal result only to `+0`. This section grants no general permission to ignore or merge signed zero.

`standard` and `reproducible` gain no subnormal result-flushing permission from this section. A realization MUST NOT treat a subnormal input as zero under authority of this result-flushing rule; input flushing is governed only by the separate rule above.

Backend FTZ/DAZ modes, WGSL latitude, SPIR-V denormal execution modes, LLVM denormal environment settings, or another physical mechanism do not widen this permission. A realization using such a mechanism MUST preserve every applicable Runen obligation not explicitly relaxed by an applicable rule.

This section does not define positive-zero replacement of a negative subnormal result, source selection of `fast`, conversion or literal flushing, transcendental or reduction flushing, reduced precision, approximation, compiler IR, physical instructions, or environment capability requirements.

## Upper-bound rounding

For a binary floating result format, define:

- the maximum finite magnitude `L = (2^p - 1) * 2^(emax - (p - 1))`;
- the positive upper limit `U = 2^(emax + 1)`, the next power-of-two value in the mathematical continuation of the format spacing beyond `L`;
- the overflow midpoint `H = (L + U) / 2 = U - 2^(emax - p)`.

For a nonzero exact finite real quantity `x` explicitly supplied to the binary floating rounding relation and satisfying `|x| > L`, `standard` extends nearest/ties-to-even to the upper finite boundary:

- if `L < |x| < H`, the rounded result is the maximum finite value with the sign of `x`;
- if `|x| >= H`, the rounded result is the signed infinity with the sign of `x`.

At `|x| = H`, the infinity side is the ties-to-even choice. The maximum finite candidate has canonical significand integer `2^p - 1`, which is odd. The mathematical continuation at `U` has significand integer `2^(p - 1)`, which is even; selecting that continuation candidate yields signed infinity. This tie construction is semantic mathematics and does not define a physical exponent or significand encoding beyond the accepted value format.

By the contract-refinement rule, `reproducible` and `fast` follow this `standard` upper-bound rule unless a later contract-specific rule explicitly narrows or relaxes this exact numerical behavior. Backend finite-only, saturation, overflow-mode, or fast-math behavior supplies no such relaxation by itself.

This section rounds only a nonzero exact finite real quantity explicitly supplied to the binary floating rounding relation. Determinate basic infinity and zero-divisor results and NaN-class basic outcomes are defined separately above. This section does not define literals or transcendental accuracy except where another section explicitly defines it.

## Correctly rounded sine baseline

For an already-admitted unary sine operation, suppose another applicable contract has already established the semantic floating operand value and an already-established binary floating result type `T` governed by this document. This section defines only the numerical result relation after those facts are established; it does not define operand typing, overload resolution, promotions, source spelling, or result-type selection.

Under `standard`, signed-zero and special-value inputs produce:

- input `+0` produces result `+0` of `T`;
- input `-0` produces result `-0` of `T`;
- input `+∞` or `-∞` produces a result belonging to `T`'s NaN value class;
- any NaN input produces a result belonging to `T`'s NaN value class.

For a nonzero finite input, interpret the input by the exact real value `x` supplied by its semantic floating format and define the exact real reference result `y = sin(x)`. A nonzero finite binary floating input is a nonzero dyadic rational; because the only real zeros of sine are integer multiples of irrational `π`, `y` is nonzero. Mathematical sine is finite, so the operation explicitly supplies this one nonzero exact finite real quantity `y` to the binary floating rounding relation for `T` exactly once.

The accepted rounding relation therefore determines the `standard` result across interior, signed-zero, and upper-format boundaries. Although mathematical sine lies in `[-1, 1]`, this rule does not assume that every possible Runen binary floating result format can represent `+1` or `-1`; the accepted upper-bound rounding rule remains authoritative when needed.

For either NaN-class sine outcome, any NaN member of `T` is permitted by the NaN class rule above. No propagation or relationship to an input NaN member is required, and no sign, payload, quiet/signaling, canonical-identity, or representation property is implied.

By the contract-refinement rules, `reproducible` and `fast` follow this `standard` sine result unless a later sine-specific rule explicitly narrows or relaxes the named numerical behavior. This slice grants no sine-specific approximation, subnormal input or result flushing, reduced precision, range-reduction latitude, reassociation, contraction, or other fast-math permission. Existing `fast` rules for basic `+`, `-`, `*`, `/`, finite multiply-add contraction, and unordered-sum trees do not automatically apply to sine.

A realization MAY use a native or library sine implementation only when it proves that the produced semantic result satisfies this contract, and MAY emulate when needed to preserve the selected contract. Backend approximate-function flags, shader accuracy latitude, host math-library behavior, target-native range reduction, or aggregate fast-math modes are not semantic authority and do not widen the permitted result.

This section does not define cosine, exponential, logarithmic, power, or another transcendental operation; a sine-specific `fast` approximation contract; source function syntax or library placement; vector forms; constant evaluation; a physical range-reduction algorithm; compiler IR; backend instructions; or target taxonomy.

## Finite unordered floating sum reduction

For an already-admitted unordered floating sum reduction, suppose one binary floating type `T` governed by this document is already established as the type of every submitted floating value and as the final floating result type. This section defines the `standard` numerical rule only for a normally completed reduction occurrence whose submitted values are all nonzero finite values of `T` or signed zero values of `T`, and whose semantic contribution collection is finite.

This numerical rule consumes the participant, cohort, contribution-occurrence, exact-once coverage, identity, and normal-completion obligations of the Exec unordered-reduction contract. It does not reinterpret participant selection, hierarchy, contribution occurrence identity, completion, scheduling, or physical reduction structure.

The floating-sum operation supplies Exec with an internal semantic combination domain of **finite sum states**. A finite sum state is operation-private semantic structure used as the contribution and partial-result value of this reduction contract. It is not a source-visible Runen type, an ordinary program value, a storage representation, an ABI value, or a required physical accumulator.

A finite sum state is a pair `(s, z)` where:

- `s` is the exact mathematical finite real sum represented by the state, interpreting either signed zero as mathematical zero;
- `z` is exactly one of:
  - `empty` — no submitted floating value is represented by the state;
  - `negative-zero-only` — at least one submitted value is represented and every represented value is `-0`;
  - `other` — every other non-empty represented collection.

Each submitted floating value is injected into exactly one singleton semantic contribution state before Exec combines contributions:

- a nonzero finite value with exact real value `x` becomes `(x, other)`;
- `+0` becomes `(0, other)`;
- `-0` becomes `(0, negative-zero-only)`.

The reduction identity value `e` is the state `(0, empty)`.

Combining `(s1, z1)` and `(s2, z2)` produces a finite sum state whose exact sum is the mathematical real value `s1 + s2`, computed with unbounded semantic range and precision and with no floating rounding. Its zero-sign classification is combined by these rules:

- `empty` is a two-sided identity for the classification;
- `negative-zero-only` combined with `negative-zero-only` yields `negative-zero-only`;
- every other combination of two non-empty classifications yields `other`.

Under exact equality of both state components, this combination is normally completing, closed over finite sum states, result-only, has the two-sided identity `e`, and is associative and commutative. It therefore supplies a combination domain satisfying the existing Exec unordered-reduction law boundary. This statement does not make ordinary binary floating `+` associative or commutative under its rounded operation semantics and does not make ordinary floating values a generic lawful unordered-reduction combination domain.

After Exec has incorporated every singleton semantic contribution exactly once, the numeric operation finalizes the resulting state `(s, z)` to `T`:

- if `s != 0`, it explicitly supplies that one exact finite real quantity to the binary floating rounding relation for `T` exactly once;
- if `s = 0` and `z = negative-zero-only`, it produces `-0`;
- otherwise, when `s = 0`, it produces `+0`.

Consequently an empty floating sum produces `+0`; a non-empty sum whose submitted values are all `-0` produces `-0`; mixed signed zeros produce `+0`; and exact cancellation involving any nonzero finite contribution produces `+0`. A nonzero exact sum may lie beyond the finite representable range of `T`; the accepted upper-bound rounding rule then determines whether the final result is finite or signed infinity. No intermediate floating rounding occurs in the semantic sum state.

By the contract-refinement rules, `reproducible` and `fast` follow this `standard` finite same-format sum-reduction rule unless a later reduction-specific rule explicitly narrows or relaxes the named numerical behavior. Existing `fast` permissions for basic-operation subnormal input flushing, subnormal result flushing, reassociation, and finite multiply-add contraction apply only to the operation occurrences they explicitly name and do not alter singleton injection, exact sum-state combination, or finalization under this reduction rule.

Physical tree shape, contribution permutation, lane or worker identity, queue or chunk order, native reduction instructions, host floating addition, and backend aggregate fast-math behavior are not semantic input. A realization MAY use an exact, binned, superaccumulator, or another implementation technique when it proves that technique preserves the result permitted here; absence of a direct native exact-sum primitive is not permission to substitute a result-changing ordinary floating-addition tree.

This section does not define submitted NaN or signed-infinity behavior, mixed floating formats or promotions, products, dot products, min/max, scans, ordered reductions, generic user-defined floating combination operators, a source-visible exact accumulator type, physical accumulator width or layout, a required summation algorithm, a `fast` reduction-specific tree/approximation/flushing relaxation, source reduction syntax or placement, hierarchy selection syntax, numeric-contract selection syntax, or abnormal reduction completion.

## Signed-infinity extension for unordered floating sum reduction

For the same already-admitted same-format unordered floating sum reduction, `standard` extends the accepted finite rule to submitted signed infinity values of `T`. Submitted NaN values remain outside this section.

Keep the accepted finite sum state `f = (s, z)` unchanged. The broader operation-private **sum state** is the product `(f, i)`, where `i` records the set of signed infinity values represented by the state and is exactly one of:

- `none` — no signed infinity is represented;
- `positive` — one or more `+∞` values and no `-∞` value are represented;
- `negative` — one or more `-∞` values and no `+∞` value are represented;
- `both` — both infinity signs are represented.

Singleton injection is:

- a finite or signed-zero submitted value `v` becomes `(inj_f(v), none)`, where `inj_f` is exactly the accepted finite-state injection above;
- `+∞` becomes `(e_f, positive)`;
- `-∞` becomes `(e_f, negative)`, where `e_f = (0, empty)` is the accepted finite-state identity.

The broader reduction identity is `(e_f, none)`.

Combine the product state componentwise. The `f` component uses exactly the accepted finite-state combine. The infinity-presence component combines by set union of represented infinity signs: `none` is the two-sided identity, combining identical single-sign classes preserves that class, and combining `positive` with `negative` yields `both`.

Under exact product-state equality, the infinity-presence component is a commutative monoid and its product with the already-accepted finite-state commutative monoid remains normally completing, closed, result-only, associative, commutative, and equipped with the true two-sided identity `(e_f, none)`. The existing Exec unordered-reduction law boundary therefore remains satisfied without making ordinary floating addition the combination operation.

After exact-once contribution coverage, finalization is:

- `i = positive` produces `+∞`;
- `i = negative` produces `-∞`;
- `i = both` produces a value belonging to `T`'s NaN value class;
- `i = none` uses the accepted finite-state finalization unchanged.

When `i = both`, any NaN member of `T` is permitted by the NaN class rule above. No input NaN exists in this slice, and no sign, payload, quiet/signaling, canonical-identity, or representation property is implied by the selected member.

Finite contributions do not change a result determined by one submitted infinity sign. A finite exact sum that later rounds to signed infinity when `i = none` remains a finite-state finalization result; it does not become an infinity-presence contribution and does not interact with the presence component retroactively.

By the contract-refinement rules, `reproducible` and `fast` follow this `standard` signed-infinity extension unless a later reduction-specific rule explicitly narrows or relaxes it. Existing `fast` permissions for basic-operation subnormal flushing, reassociation, and finite multiply-add contraction remain scoped to the operation occurrences they name and do not alter singleton injection, finite-state combine, infinity-presence combine, or finalization here.

Physical reduction tree or permutation, native infinity handling, lane or worker order, backend fast-math flags, and target conventions are not semantic input.

A submitted NaN value remains outside the same-format unordered sum semantics defined so far. Whether such a contribution makes the reduction result NaN-class or has another operation-specific effect is an open semantic obligation, not implementation freedom. If a future reduction rule requires a NaN-class result, the general NaN member-variation rule above applies unless that future rule explicitly strengthens member selection.

This section also does not define mixed floating formats or promotions, other reduction operations, a source-visible accumulator type, physical accumulator representation, a `fast` reduction-specific native-tree/FTZ/approximation relaxation, source reduction syntax or placement, hierarchy selection syntax, numeric-contract selection syntax, or abnormal completion.

## Fast tree-rounded unordered floating sum relaxation

For an already-admitted same-format unordered floating sum reduction under `fast`, when every submitted value belongs to the currently defined sum input domain above, `fast` retains the inherited exact-state result and additionally MAY select any **tree-rounded candidate** defined by this section. Submitted NaN values remain outside the sum input domain and are not made admissible by this relaxation.

For a non-empty reduction occurrence, a tree-rounded candidate is constructed as follows:

1. choose any full binary tree whose leaves are in bijection with the semantic contribution occurrences selected by the existing Exec reduction contract;
2. assign each contribution occurrence to exactly one leaf; because this is an unordered sum and this `fast` result rule explicitly admits permutation, any bijection between the contribution occurrences and the tree leaves is permitted;
3. each leaf initially carries the submitted semantic floating value of `T` for that contribution occurrence;
4. evaluate each internal node from its two child outcomes using the baseline `standard` basic floating **addition** result relation already defined for `T`, rather than any `fast` basic-operation relaxation;
5. the root outcome is one tree-rounded candidate.

If an internal addition is a NaN-class case, that node may select any NaN member of `T` under the general NaN class rule. Every ancestor is then also a NaN-class outcome because baseline addition with a NaN operand has a NaN-class result; each such class-only node independently has the same permitted member variation unless a stronger rule is later defined.

For an empty reduction there is no tree-rounded alternative and the inherited exact-state result `+0` remains controlling. For a singleton reduction, the one leaf value is its tree-rounded candidate.

Therefore the `fast` result set for this slice contains the inherited exact-state result plus every tree-rounded candidate constructed above. Tree shape and contribution permutation may change the selected result only because this section names that variation explicitly. Exact-once contribution coverage is unchanged: omission, duplication, or introduction of another contribution is not permitted.

This result-set relaxation does not replace the operation-private exact sum state as the semantic Exec combination domain and does not assert that rounded floating addition satisfies Exec's associativity/commutativity requirements. A physical realization may use a native parallel addition tree or permutation when its produced result belongs to this explicitly permitted `fast` result set.

`standard` and `reproducible` gain no tree-rounded result variation from this section.

Internal tree nodes deliberately use the baseline `standard` addition relation so this rule grants only tree/permutation rounding variation. It does not additionally permit reduction-internal subnormal input flushing, subnormal result flushing, reduced precision, approximate addition, contraction, operand omission or duplication, or another backend fast-math behavior. Existing `fast` basic-operation FTZ, reassociation, and contraction permissions apply only to semantic basic operation occurrences already governed by those sections and do not automatically apply to these reduction-internal tree nodes.

A later reduction-specific rule may explicitly add FTZ, reduced-precision, approximation, or another numerical relaxation if justified. Backend aggregate fast-math mode, physical FTZ state, native reduction instruction behavior, or target latitude supplies no such permission by itself.

This section does not define submitted NaN reduction semantics, mixed floating formats or promotions, other reduction operations, source-visible tree or accumulator APIs, physical tree topology, source reduction syntax or placement, hierarchy selection syntax, or abnormal completion.

## Reassociation

Under `standard` and `reproducible`, semantic grouping of separately represented floating-point additions or multiplications is result-significant. A realization MUST NOT use real-number associativity to regroup those operations when doing so can change Runen-observable behavior permitted by the selected contract. A realization MAY physically restructure the computation when it proves that the resulting behavior still satisfies the selected contract.

Under `fast`, a realization MAY reassociate a pure finite tree of already-established floating-point operand values when every internal operation in that tree is addition, or when every internal operation in that tree is multiplication. Reassociation may change only the grouping of those operations: the ordered leaf-value sequence and the operation kind MUST remain unchanged.

This `fast` reassociation permission does not by itself authorize operand permutation, omission, duplication, substitution of another operation, reciprocal replacement, contraction, reduced precision, approximate functions, assumptions about NaN, infinity, or signed zero, or changes to the evaluation or effects by which the leaf values were obtained. Contraction is permitted only where the finite multiply-add contraction rule above applies. Every other applicable `fast` rule continues to constrain the reassociated computation and its result.

Reassociation under this section is not authority to choose an Exec unordered-reduction tree or to treat floating addition or multiplication as satisfying a reduction combination law. The separate fast tree-rounded sum rule above owns the specific tree/permutation latitude it grants. Exec reduction participation, contribution coverage, and combination obligations remain independently applicable.

NaN equality/comparison/hash/order and representation-sensitive identity, operation semantics outside the basic `+`, `-`, `*`, and `/` relations above, contraction outside the finite multiply-add case above, including exact-zero and special-value cases and multiply-subtract variants, transcendental behavior outside the correctly rounded sine baseline above, exact-zero sign outside the basic operations above, subnormal handling outside the `fast` basic input/result rules above, submitted-NaN and other reduction-specific numeric behavior outside the same-format unordered sum and explicit fast tree-rounded rules above, the remaining detailed `standard`/`reproducible`/`fast` result sets, explicit source contract selection and scoping, and the concrete hard requirements for unsupported direct realization are not defined by this revision.
