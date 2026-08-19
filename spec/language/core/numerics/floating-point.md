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

This value-format section by itself does not define signed zero, signed infinity, NaN identity or payloads, operation-specific subnormal handling, rounding, overflow or underflow result selection, flushing, conversions, or literals. Those concerns have semantics only where another applicable rule explicitly defines them.

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

## Interior finite rounding

For an otherwise-defined floating arithmetic operation whose applicable contract determines an exact finite real result `x`, `standard` uses the following rounding rule when the operation's result type has the binary finite value format above.

If `x` is a nonzero finite value exactly representable by that result format, the rounded result is exactly `x`.

Otherwise, this section supplies a rounded result only when there are two nonzero finite representable values `a < x < b` such that `a`, `x`, and `b` have the same sign and no other nonzero finite representable value lies strictly between `a` and `b`. In that case:

- if `|x - a| < |b - x|`, the rounded result is `a`;
- if `|b - x| < |x - a|`, the rounded result is `b`;
- if `|x - a| = |b - x|`, the rounded result is the one of `a` or `b` whose magnitude has an even canonical format significand integer `m`.

For every positive nonzero finite representable value, its **canonical format significand integer** is the unique `m` supplied by its normal or subnormal representation in the value-format definition above. A negative value uses the canonical `m` of its positive magnitude. For adjacent same-sign representable values in a tie, exactly one candidate has even canonical `m`.

By the contract-refinement rule, `reproducible` and `fast` follow this `standard` rounding rule unless a later contract-specific rule explicitly narrows or relaxes this exact numerical behavior.

This section includes rounding between adjacent normal values, between adjacent subnormal values, and across the nonzero subnormal/normal boundary. The interior rule does not itself supply a result when zero is a bounding candidate; that lower boundary is defined separately below. It also does not supply a result when the exact result lies beyond the largest finite magnitude. Operation-specific exact-zero sign, infinity production and overflow result selection, NaN behavior, conversions, literals, transcendental accuracy, contraction or FMA, reduced precision, floating exception/status behavior, and any contract-specific flushing relaxation remain open.

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

## Reassociation

Under `standard` and `reproducible`, semantic grouping of separately represented floating-point additions or multiplications is result-significant. A realization MUST NOT use real-number associativity to regroup those operations when doing so can change Runen-observable behavior permitted by the selected contract. A realization MAY physically restructure the computation when it proves that the resulting behavior still satisfies the selected contract.

Under `fast`, a realization MAY reassociate a pure finite tree of already-established floating-point operand values when every internal operation in that tree is addition, or when every internal operation in that tree is multiplication. Reassociation may change only the grouping of those operations: the ordered leaf-value sequence and the operation kind MUST remain unchanged.

This `fast` permission does not authorize operand permutation, omission, duplication, substitution of another operation, reciprocal replacement, contraction or fused multiply-add formation, reduced precision, approximate functions, assumptions about NaN, infinity, or signed zero, or changes to the evaluation or effects by which the leaf values were obtained. Every other applicable `fast` rule continues to constrain the reassociated computation and its result.

Reassociation under this section is not authority to choose an Exec unordered-reduction tree or to treat floating addition or multiplication as satisfying a reduction combination law. Exec reduction participation, contribution coverage, and combination obligations remain independently applicable.

Exact operation accuracy beyond the finite rounding rules above, contraction or FMA behavior, transcendental behavior, NaN handling, operation-specific infinity behavior and overflow selection, operation-specific exact-zero sign, contract-specific subnormal handling, remaining rounding and conversion behavior, reduction-specific numeric equivalence, the remaining detailed `standard`/`reproducible`/`fast` result sets, source contract selection/defaulting, and the concrete hard requirements for unsupported direct realization are not defined by this revision.
