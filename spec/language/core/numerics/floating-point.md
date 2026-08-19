# Core Floating-Point Semantics

Status: **provisional normative; incomplete**

Runen distinguishes three floating-point contracts:

- **standard** — portable baseline without arbitrary fast-math transformations;
- **reproducible** — stronger cross-realization repeatability, with emulation or rejection when required;
- **fast** — explicit documented numerical relaxations.

## Numeric-contract authority

For every floating-point operation governed by this contract system, an applicable Runen semantic context MUST establish exactly one of `standard`, `reproducible`, or `fast` before physical realization. How source selects that contract and which contract, if any, is the source default are not defined by this revision.

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

This revision does not determine whether a type's NaN set contains one or multiple semantic values. It does not assign semantic sign, payload, quiet or signaling state, canonical or preferred identity, ordering, or representation to NaN values. Equality, comparison, hashing, NaN production, propagation, and result selection remain owned by later applicable rules.

A later representation or ABI contract may refine how physical NaN encodings map to semantic NaN values while preserving every accepted semantic rule. This section requires neither an injective nor a canonical mapping and does not define source NaN literals, bitcasts, bytes, ABI layout, or serialization.

## Finite basic arithmetic

For an already-admitted basic binary floating operation, suppose each operand value is either a nonzero finite floating value or a signed zero, and an applicable contract has already established a binary floating result type governed by this document. This section defines only the numerical result relation after those facts are established; it does not define operand typing, promotions, conversions, or result-type selection.

Interpret each admitted nonzero finite operand by the exact real value supplied by its applicable finite value format. For exact real arithmetic, either signed zero contributes mathematical value `0`; its semantic zero sign remains available for the exact-zero result rules below. A nonzero finite operand has the positive or negative sign of its exact real value.

Under `standard`, floating addition, subtraction, multiplication, and division use the exact real relations `x + y`, `x - y`, `x * y`, and `x / y`, respectively. The division relation in this section applies only when the divisor is a nonzero finite value; a signed-zero divisor remains outside this section.

When that exact mathematical result is nonzero, it is a mathematical semantic quantity consumed by the applicable rounding rules below for the already-established result type. It is not a separately observable Runen value and need not itself be representable in that result type. In particular, a nonzero exact real result beyond the largest finite representable magnitude is handled by the upper-bound rounding rule rather than by host or backend overflow behavior.

When the exact mathematical result is zero, `standard` selects a signed-zero result as follows:

- **addition:** two signed-zero operands of the same sign produce that signed zero; every other exact-zero addition case produces `+0`;
- **subtraction:** `-0` minus `+0` produces `-0`; every other exact-zero subtraction case produces `+0`;
- **multiplication:** when at least one operand is signed zero, the result is `-0` exactly when the operand signs differ, and `+0` otherwise;
- **division:** when the numerator is signed zero and the divisor is nonzero finite, the result is `-0` exactly when the operand signs differ, and `+0` otherwise.

These exact-zero rules do not apply when a nonzero exact mathematical result merely rounds to zero. That case remains governed by the zero-boundary rounding rule, which preserves the sign of the nonzero exact result.

By the contract-refinement rules, `reproducible` and `fast` follow these basic-operation rules unless a later contract-specific rule explicitly narrows or relaxes the named numerical behavior. The existing `fast` reassociation permission may change grouping only where that permission applies; it does not by itself alter the numerical relation or signed-zero result rule of an individual basic operation. Backend `nsz`, target latitude around signed zero, or aggregate fast-math behavior supplies no implicit Runen relaxation.

Additional determinate infinity and zero-divisor cases are defined separately below. This section does not define NaN-producing basic-operation forms, behavior when an operand is already NaN, unary negation, remainder, fused operations, source operator spellings, comparison or hashing behavior, sign-inspection APIs, or physical instructions.

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

The following non-NaN input forms are not defined by this revision because their result requires a NaN-result contract: opposite-sign infinity addition, same-sign infinity subtraction, signed zero multiplied by infinity in either order, infinity divided by infinity, and signed zero divided by signed zero. This section also does not define behavior when either operand is already NaN, NaN production or propagation, equality, comparison, hashing, total ordering, sign-observation APIs, unary negation, remainder, fused operations, source operator spellings, or physical instructions.

## Interior finite rounding

For an otherwise-defined floating arithmetic operation whose applicable contract determines an exact finite real result `x`, `standard` uses the following rounding rule when the operation's result type has the binary finite value format above.

If `x` is a nonzero finite value exactly representable by that result format, the rounded result is exactly `x`.

Otherwise, this section supplies a rounded result only when there are two nonzero finite representable values `a < x < b` such that `a`, `x`, and `b` have the same sign and no other nonzero finite representable value lies strictly between `a` and `b`. In that case:

- if `|x - a| < |b - x|`, the rounded result is `a`;
- if `|b - x| < |x - a|`, the rounded result is `b`;
- if `|x - a| = |b - x|`, the rounded result is the one of `a` or `b` whose magnitude has an even canonical format significand integer `m`.

For every positive nonzero finite representable value, its **canonical format significand integer** is the unique `m` supplied by its normal or subnormal representation in the value-format definition above. A negative value uses the canonical `m` of its positive magnitude. For adjacent same-sign representable values in a tie, exactly one candidate has even canonical `m`.

By the contract-refinement rule, `reproducible` and `fast` follow this `standard` rounding rule unless a later contract-specific rule explicitly narrows or relaxes this exact numerical behavior.

This section includes rounding between adjacent normal values, between adjacent subnormal values, and across the nonzero subnormal/normal boundary. The interior rule does not itself supply a result when zero is a bounding candidate; that lower boundary is defined separately below. It also does not itself supply a result beyond the largest finite magnitude; that upper boundary is defined separately below. Determinate basic infinity and zero-divisor results are defined above. Exact-zero sign outside the basic operations defined above, NaN-producing and NaN-input operation behavior, conversions, literals, transcendental accuracy, contraction or FMA, reduced precision, and any contract-specific flushing relaxation remain open.

## Zero-boundary rounding

Let `q = 2^(emin - (p - 1))`, the smallest positive subnormal magnitude of a binary floating result format.

For an otherwise-defined floating arithmetic operation whose exact finite real result `x` is nonzero and satisfies `|x| < q`, `standard` extends the nearest/ties-to-even rule to the signed-zero boundary:

- if `0 < x < q / 2`, the rounded result is `+0`;
- if `q / 2 < x < q`, the rounded result is `+q`;
- if `x = q / 2`, the rounded result is `+0`;
- if `-q / 2 < x < 0`, the rounded result is `-0`;
- if `-q < x < -q / 2`, the rounded result is `-q`;
- if `x = -q / 2`, the rounded result is `-0`.

At either halfway point, signed zero is the ties-to-even choice because this lower-bound lattice may equivalently index the magnitudes `0` and `q` by integers `0` and `1`; index `0` is even. This is a mathematical tie rule and does not define a physical significand encoding for zero.

Therefore, when a nonzero exact finite result rounds to zero under this boundary rule, the rounded zero has the sign of the exact result. This consequence does not determine the sign of an operation whose exact mathematical result is zero.

By the contract-refinement rule, `reproducible` and `fast` follow this `standard` zero-boundary rule unless a later contract-specific rule explicitly narrows or relaxes this exact numerical behavior. A backend flush-to-zero or denormal mode supplies no such relaxation by itself.

## Upper-bound rounding

For a binary floating result format, define:

- the maximum finite magnitude `L = (2^p - 1) * 2^(emax - (p - 1))`;
- the positive upper limit `U = 2^(emax + 1)`, the next power-of-two value in the mathematical continuation of the format spacing beyond `L`;
- the overflow midpoint `H = (L + U) / 2 = U - 2^(emax - p)`.

For an otherwise-defined floating arithmetic operation whose exact real result `x` is finite and satisfies `|x| > L`, `standard` extends nearest/ties-to-even to the upper finite boundary:

- if `L < |x| < H`, the rounded result is the maximum finite value with the sign of `x`;
- if `|x| >= H`, the rounded result is the signed infinity with the sign of `x`.

At `|x| = H`, the infinity side is the ties-to-even choice. The maximum finite candidate has canonical significand integer `2^p - 1`, which is odd. The mathematical continuation at `U` has significand integer `2^(p - 1)`, which is even; selecting that continuation candidate yields signed infinity. This tie construction is semantic mathematics and does not define a physical exponent or significand encoding beyond the accepted value format.

By the contract-refinement rule, `reproducible` and `fast` follow this `standard` upper-bound rule unless a later contract-specific rule explicitly narrows or relaxes this exact numerical behavior. Backend finite-only, saturation, overflow-mode, or fast-math behavior supplies no such relaxation by itself.

This section rounds only an otherwise-defined finite exact real result. Determinate basic infinity and zero-divisor results are defined separately above. This section does not define NaN-producing or NaN-input operation behavior, conversions, literals, or transcendental accuracy.

## Reassociation

Under `standard` and `reproducible`, semantic grouping of separately represented floating-point additions or multiplications is result-significant. A realization MUST NOT use real-number associativity to regroup those operations when doing so can change Runen-observable behavior permitted by the selected contract. A realization MAY physically restructure the computation when it proves that the resulting behavior still satisfies the selected contract.

Under `fast`, a realization MAY reassociate a pure finite tree of already-established floating-point operand values when every internal operation in that tree is addition, or when every internal operation in that tree is multiplication. Reassociation may change only the grouping of those operations: the ordered leaf-value sequence and the operation kind MUST remain unchanged.

This `fast` permission does not authorize operand permutation, omission, duplication, substitution of another operation, reciprocal replacement, contraction or fused multiply-add formation, reduced precision, approximate functions, assumptions about NaN, infinity, or signed zero, or changes to the evaluation or effects by which the leaf values were obtained. Every other applicable `fast` rule continues to constrain the reassociated computation and its result.

Reassociation under this section is not authority to choose an Exec unordered-reduction tree or to treat floating addition or multiplication as satisfying a reduction combination law. Exec reduction participation, contribution coverage, and combination obligations remain independently applicable.

NaN-producing and NaN-input basic-operation semantics, operation semantics outside the basic `+`, `-`, `*`, and `/` relations above, contraction or FMA behavior, transcendental behavior, exact-zero sign outside the basic operations above, contract-specific subnormal handling, remaining conversion behavior, reduction-specific numeric equivalence, the remaining detailed `standard`/`reproducible`/`fast` result sets, source contract selection/defaulting, and the concrete hard requirements for unsupported direct realization are not defined by this revision.
