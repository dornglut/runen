# Source Control Flow

Status: **provisional normative; incomplete**

This document owns the represented source semantics for statement-level conditional control flow: condition admission and selection, validation of both conditional arms, explicit arm lexical-scope composition, omitted-else behavior, definite structural ownership at a normal conditional successor, and the source-to-Core conditional refinement boundary.

It consumes represented source type identity and the intrinsic `Bool` type from [Source type foundation](types.md); owned-value producers, producer evaluation, lexical-block execution, cleanup, defined-fault propagation, and divergence from [Source function execution](function-execution.md); binding identity, lexical scope, lookup, and binding structural lifecycle from [Source function-local bindings](local-bindings.md); structural ownership state from [Source structural ownership](structural-ownership.md); and represented Core Bool branching and CFG path-state validity from [Core control flow](../core/control-flow.md). Concrete `if`/`else` spelling and the represented conditional-value grammar are owned by [Source concrete syntax](concrete-syntax.md).

This document does not redefine owned-value producer semantics, structural path/state mathematics, binding scope rules, lexical cleanup order, fault cleanup, Core path state, or concrete grammar.

## Represented conditional statement

One represented source conditional statement consists of:

- exactly one represented conditional-value producer;
- exactly one explicit **then arm** block; and
- zero or one explicit **else arm** block.

The concrete form is owned by `concrete-syntax.md`.

A represented conditional is a statement. It produces no source value, introduces no Unit/Void value, and is not an owned-value producer.

This revision defines no conditional expression, direct `else if` form, pattern condition, guard, loop, match, catch, label, break, continue, or early/nested return.

## Conditional-value admission

The represented condition is one concrete `ConditionalValue` from `concrete-syntax.md`.

The condition MUST produce exactly one owned source value whose source type is exactly the intrinsic `Bool` type under `types.md`.

No truthiness, implicit conversion, coercion, integer-to-Bool relation, structural conversion, or second Bool-like type is introduced.

Concrete syntax deliberately excludes record construction from `ConditionalValue`; that grammar restriction is owned by `concrete-syntax.md`. This semantic owner does not infer conditional admissibility from parser lookahead.

A syntactically represented conditional value whose resolved/produced type is not exactly `Bool` is source-invalid.

## Condition validation state

Source validation of one represented conditional begins in the enclosing function-local binding environment that exists immediately before the condition producer.

Validate the condition through its existing producer owner with exact required source type `Bool`.

Every source ownership transition caused by validating/producing the condition is applied exactly once before conditional arm state splitting.

The resulting enclosing binding environment is the **post-condition environment**.

Both conditional arms are source-validated from semantically identical copies of that same post-condition environment.

The condition value itself is not part of the enclosing binding environment unless its existing producer semantics operated on a binding. Its successful result is one owned Bool transient held by the conditional operation for branch selection.

## Validation does not prune by Bool value

Both normal conditional outcomes MUST be source-validated independently of the semantic Bool value that one concrete runtime execution may observe.

In particular:

- a condition spelled as the literal `true` does not exempt the false/else arm from source validation; and
- a condition spelled as the literal `false` does not exempt the then arm from source validation.

This is a source-validity rule. It does not assert that both arms execute in one concrete activation and does not create an unknown or three-valued Bool.

This revision therefore does not require source constant propagation, constant folding, symbolic execution, unreachable-arm weakening, or a value lattice merely to validate a conditional statement.

## Condition execution and runtime selection

When execution reaches one represented conditional statement:

1. evaluate the condition producer exactly once under its existing source execution semantics with required type `Bool`;
2. preserve every ownership transition, transient consequence, fault possibility, and divergence consequence of that producer evaluation;
3. on successful production, hold exactly one owned Bool condition transient;
4. consume that transient for conditional selection;
5. when its Bool value is `true`, execute only the then arm;
6. when its Bool value is `false` and an explicit else arm exists, execute only that else arm; and
7. when its Bool value is `false` and no explicit else arm exists, take the omitted-else normal false outcome defined below.

Conditional selection itself performs no additional binding read, move, duplicate, assignment, structural consumption, cleanup, call, fault selection, or hidden source state transition beyond ending ownership of the successful Bool condition transient used for selection.

The represented condition is therefore evaluated once even when validation considered both arms.

## Condition fault

If condition evaluation yields one defined fault `F` before successful Bool production:

- no then or else arm begins;
- no conditional normal join occurs;
- ownership transitions already completed during condition evaluation remain effective; and
- the active activation follows the existing defined-fault cleanup and propagation relation from `function-execution.md` with the same fault `F`.

The conditional statement does not add a second fault cleanup boundary.

## Condition divergence

If condition evaluation diverges before successful Bool production:

- no then or else arm begins;
- no conditional normal join occurs; and
- no lexical-scope, activation, or conditional cleanup occurs merely because execution remains suspended.

Producer-owned transients and completed ownership transitions persist exactly as required by their existing owners.

## Explicit arm lexical scopes

Every explicit conditional arm is exactly one represented nested block and therefore exactly one child lexical scope under `local-bindings.md` and `function-execution.md`.

An explicit then arm and explicit else arm are sibling lexical scopes of the same enclosing scope.

Consequently:

- bindings introduced in one arm do not enter the other arm;
- bindings introduced in one arm do not survive the normal end of that arm;
- sibling arms MAY independently introduce the same lexical identifier key because their scopes do not overlap;
- neither arm may introduce a key that illegally shadows an active enclosing function-local binding;
- ordinary locals, record-pattern bindings, nested blocks, assignments, and calls inside an arm retain their existing semantics; and
- nested represented conditionals may occur because `IfStatement` is itself a represented body statement inside an arm block.

This document does not create an abstract source-visible branch-scope identity beyond the ordinary child lexical scopes already owned by `local-bindings.md`.

## Explicit arm normal completion

When the selected explicit arm completes normally:

1. finish its contained body-statement sequence under `function-execution.md`;
2. normally exit that child lexical scope;
3. clean bindings declared directly in that arm using the existing lexical-scope cleanup relation; and
4. only after that cleanup does the arm produce its **normal enclosing outcome** for conditional join purposes.

Arm-local bindings have ended before normal join comparison and therefore are not members of the enclosing environment compared at the join.

Normal cleanup of one arm does not itself clean or reset enclosing bindings merely to make their states match another arm.

## Explicit arm fault and divergence

If execution inside the selected arm yields a defined fault, that arm does not produce a normal enclosing outcome.

The active child scope participates exactly once in the existing activation fault cleanup from `function-execution.md`. It does not first perform independent normal arm cleanup and then fault cleanup.

If execution inside the selected arm diverges, that arm produces no normal enclosing outcome and performs no normal arm or join cleanup merely because execution remains suspended.

The normal ownership join below applies only to normal arm outcomes.

## Omitted else

When no explicit else arm is present, runtime `false` selects the **omitted-else normal false outcome**.

That outcome:

- executes no body statement;
- introduces no lexical binding;
- creates no source-visible synthetic lexical scope;
- performs no cleanup; and
- carries every enclosing binding's structural ownership state exactly as it exists in the post-condition environment.

For normal-join validation, the omitted-else false outcome is therefore the unchanged post-condition enclosing environment.

An explicit `else {}` is an actual child lexical scope, even though its empty body yields the same enclosing structural ownership outcome.

## Enclosing environment at a conditional join

Let `E` be the post-condition function-local binding environment.

`E` contains exactly the active parameter/local binding identities that remain in the enclosing lexical environment after successful condition production. Condition evaluation cannot introduce one new function-local binding because represented conditional values are owned-value producers rather than declarations.

Each explicit arm is source-validated from its own copy of `E`. An omitted else uses unchanged `E` as its false normal outcome.

After normal completion and local cleanup of every explicit arm, the conditional compares only the binding identities belonging to `E`.

Binding identity, declared type, and assignment-mutability classification are unchanged by conditional branching itself. The conditional therefore compares each enclosing binding's structural ownership state.

## Exact structural-ownership state equality

For one represented enclosing binding, two normal arm outcomes have equal structural ownership state exactly when their prefix-free consumed-path sets under `structural-ownership.md` are equal.

A represented conditional has a valid normal successor only when **every** binding identity in `E` has equal structural ownership state on both normal outcomes.

When all enclosing binding states are equal:

- that common state is the one definite structural ownership state of the binding at the normal continuation; and
- subsequent source validation proceeds through the existing single-state binding relation in `local-bindings.md` and `structural-ownership.md`.

When any enclosing binding has unequal normal arm states, the conditional statement is source-invalid at its normal join boundary. No normal post-conditional ownership state is established.

The equality requirement is semantic equality of source structural state. It is not equality of runtime scalar values, record values, Core local state, parser nodes, HIR data structures, compiler hashes, or physical storage.

## Definite-state consequences

The exact-state rule has these required consequences.

### Equal complete consumption

If both normal outcomes consume the complete root of the same enclosing non-duplicable binding and do not reinitialize it, both consumed-path sets contain the complete root path and the binding may join as unavailable.

If one normal outcome consumes the complete root while the other leaves it fully available, the states differ and the conditional is source-invalid.

### Equal partial consumption

If both normal outcomes consume exactly the same represented nested structural paths of one enclosing binding, the equal resulting consumed-path set may join.

If the outcomes leave different consumed sibling or nested paths, the states differ and the conditional is source-invalid.

No structural similarity, equal remaining-frontier shape, or equal number of consumed paths substitutes for exact consumed-path-set equality.

### Duplicable uses

A represented non-consuming duplicate use leaves the consumed-path set unchanged under `structural-ownership.md` and therefore does not by itself prevent an equal-state join.

### Assignment and reinitialization

Whole-binding assignment retains its existing source-first replacement semantics.

A successful assignment may begin from a fully available, partially available, or unavailable binding state and ends by establishing fresh complete structural ownership for the replacement value.

Consequently, arms with different earlier ownership histories may still produce equal normal ownership states when accepted whole-binding assignments explicitly re-establish the same complete-state classification before normal arm completion.

The conditional does not special-case assignment and does not require runtime values assigned by different arms to be equal.

### Branch-dependent runtime values

Two normal outcomes may produce different runtime values in the same mutable enclosing binding while still having equal structural ownership state.

Structural ownership definiteness is not value equality, constant propagation, or SSA value merging.

### Omitted else

With no explicit else arm, the then arm's normal enclosing structural ownership state MUST equal the unchanged post-condition state for every enclosing binding.

Therefore an omitted-else then arm may not leave one enclosing binding consumed or partially consumed unless condition evaluation had already established exactly that same state before both outcomes were split or accepted operations inside the then arm restore the post-condition state before normal completion.

### Zero-field and zero-leaf values

Zero-field and recursively zero-leaf source values participate in consumed-path-state equality exactly like other structural owned values.

Their source ownership state MUST NOT be inferred from whether a Core representation has a scalar destruction leaf or emits a physical destruction operation.

## Post-join source operations

After a valid normal join, every represented enclosing binding has exactly one committed structural ownership state.

Subsequent whole-binding use, field-value use, record-pattern use, assignment, nested conditional, lexical cleanup, return cleanup, or fault cleanup consumes that one ordinary state through its existing source owner.

No post-join operation performs a second conditional-state analysis merely because the value was previously used inside a represented conditional.

## No implicit join normalization

This conditional relation does not derive a common state by:

- union of consumed paths;
- intersection of consumed paths;
- prefix minimization;
- meet, join, widening, or another lattice operation;
- automatically consuming still-owned values on one branch edge;
- inventing a maybe-owned state; or
- consulting lower Core liveness.

In particular, source-invalid unequal arm states are not made valid by silently cleaning additional owned subvalues on an arm that retained them.

This avoids introducing path-specific source destruction timing as an implicit consequence of merely reaching a branch join.

## No conditional ownership flags

This revision introduces no source drop flag, runtime moved-state flag, hidden path tag, conditional cleanup bit, or source-visible ownership test.

Every binding that reaches the represented normal continuation has one definite structural ownership state fixed statically by the exact-state rule above.

Future source control-flow forms or later extensions may introduce additional accepted relations only through their own normative changes. Their existence MUST NOT retroactively alter the semantics of conditionals already accepted by this exact-state relation.

## Constant conditions

A concrete source execution with condition value `true` executes only the then arm. A concrete source execution with condition value `false` executes only the false outcome.

Source validation nevertheless validates both normal outcomes and applies the exact-state join requirement to both.

This difference between runtime selection and conservative source validation does not introduce nondeterministic source execution.

## Nested conditionals

A represented conditional may occur inside any explicit arm block because it is a body statement under `concrete-syntax.md`.

The inner conditional must independently establish one definite normal ownership state before the containing arm may continue.

Consequently, exact-state joins compose recursively without a general source CFG or state-set relation.

This revision defines no direct `else if` grammar. Equivalent nested selection may be written only through the represented explicit block nesting admitted by `concrete-syntax.md`.

## Result-bearing function boundary

This conditional statement does not change the existing result-bearing function completion requirement from `function-execution.md`.

`ReturnStatement` remains a root-terminal form rather than a represented body statement in this revision. Conditional arms therefore contain no represented early return.

A result-bearing function must still have the accepted root terminal result return after any preceding normally completing conditional statements.

Future early-return control flow requires its own treatment of non-normal successors and does not follow from this normal-join relation.

## Source/Core refinement

A faithful source-to-Core lowering MAY refine one source-valid represented conditional to the accepted Bool-valued `Branch` relation in [Core control flow](../core/control-flow.md).

After source validation has fixed the condition type and exact normal source ownership join, a lowering may:

1. lower the existing condition producer exactly once;
2. materialize its Bool result in a compiler-owned Core temporary when useful;
3. consume that temporary once as the Core `Branch` condition operand;
4. lower the then arm to one or more Core blocks;
5. lower explicit else-arm blocks when present, or use the normal join target directly as the false target when no else body needs lower execution;
6. refine each explicit arm's retained source normal cleanup before that arm's normal join edge; and
7. transfer normally completing arms to one Core join block before lowering subsequent source statements.

The exact shape or number of Core blocks is not source-observable.

A compiler temporary used for condition selection is not a source binding.

## Lower path states do not define the source join

Core CFG validation may preserve multiple distinct implementation states at the lower join even when source enclosing ownership is definite and equal.

For example, a source local declared only in the then-arm child scope may be represented by one Core local that is Dead after normal then-arm cleanup but Never-initialized on a false execution that never entered that arm. Those distinct lower states remain valid implementation facts under Core control flow.

They do not make the ended arm-local source binding visible after the arm and do not create path-dependent source ownership for enclosing bindings.

Lowering MUST NOT:

- use Core path-state union/intersection to reconstruct source structural ownership;
- accept an unequal source ownership join merely because every lower continuation operation happens to validate under multiple Core states;
- infer source cleanup from lower scalar liveness; or
- turn Core worklist behavior into source semantic authority.

Source join validity is established before lowering by this document and `structural-ownership.md`.

## Determinism

For one fixed source-valid represented program and one fixed activation state, conditional runtime selection is deterministic after successful condition evaluation.

The condition producer has its existing deterministic or otherwise accepted source behavior. Once it yields one of the two semantic Bool values, exactly one runtime outcome is selected.

Validation of both arms is a static validity obligation, not runtime nondeterminism.

## Further boundaries

This revision does not define general expressions, grouping expressions, comparisons, logical operators, arithmetic operators, truthiness, coercions, record-construction conditions, conditional values/expressions, direct `else if`, early/nested return, loops, match, refutable patterns, catch/recovery, labels, break, continue, source state lattices, path-dependent ownership after a normal join, automatic join cleanup, drop flags, custom destructors, must-consume policy, references, borrows, lifetime inference, optimizer transformations, ABI/linkage, backend branches, Exec, Model, or stable serialized HIR/Core control-flow identity.

Those concerns require their own accepted owners or later extensions and MUST NOT be inferred from the represented conditional relation here.
