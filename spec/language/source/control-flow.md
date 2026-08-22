# Source Control Flow

Status: **provisional normative; incomplete**

This document owns the represented source semantics for statement-level conditional control flow: condition admission and selection, validation of both represented conditional outcomes, explicit arm lexical-scope composition, omitted-else behavior, normal-continuation composition, definite structural ownership at any normal conditional successor, and the source-to-Core conditional refinement boundary.

It consumes represented source type identity and the intrinsic `Bool` type from [Source type foundation](types.md); owned-value producers, producer evaluation, lexical-block execution, normal-continuation presence, return execution, cleanup, defined-fault propagation, and divergence from [Source function execution](function-execution.md); binding identity, lexical scope, lookup, and binding structural lifecycle from [Source function-local bindings](local-bindings.md); structural ownership state from [Source structural ownership](structural-ownership.md); and represented Core Bool branching and CFG path-state validity from [Core control flow](../core/control-flow.md). Concrete `if`/`else` spelling and the represented conditional-value grammar are owned by [Source concrete syntax](concrete-syntax.md).

This document does not redefine owned-value producer semantics, return execution, structural path/state mathematics, binding scope rules, lexical cleanup order, fault cleanup, Core path state, or concrete grammar.

## Represented conditional statement

One represented source conditional statement consists of:

- exactly one represented conditional-value producer;
- exactly one explicit **then arm** block; and
- zero or one explicit **else arm** block.

The concrete form is owned by `concrete-syntax.md`.

A represented conditional is a statement. It produces no source value, introduces no Unit/Void value, and is not an owned-value producer.

This revision defines no conditional expression, direct `else if` form, pattern condition, guard, loop, match, catch, label, break, continue, or unrestricted nonterminal-within-block return.

## Conditional-value admission

The represented condition is one concrete `ConditionalValue` from `concrete-syntax.md`.

The condition MUST produce exactly one owned source value whose source type is exactly the intrinsic `Bool` type under `types.md`.

No truthiness, implicit conversion, coercion, integer-to-Bool relation, structural conversion, or second Bool-like type is introduced.

Concrete syntax deliberately excludes record construction from `ConditionalValue`; that grammar restriction is owned by `concrete-syntax.md`. This semantic owner does not infer conditional admissibility from parser lookahead.

A syntactically represented conditional value whose resolved/produced type is not exactly `Bool` is source-invalid.

## Condition validation state

Source validation of one represented conditional begins in the enclosing function-local binding environment that exists immediately before the condition producer.

Validate the condition through its existing producer owner with exact required source type `Bool`.

During source validation, apply each semantic ownership consequence selected by that condition producer exactly once before conditional outcome state splitting.

The resulting enclosing binding environment is the **post-condition environment**.

The explicit then arm and, when present, the explicit else arm are each source-validated from semantically identical copies of that same post-condition environment. When else is omitted, the false normal outcome is the unchanged post-condition environment as defined below.

The successful condition result is one owned Bool transient held by the conditional operation for branch selection. That transient is not a function-local binding and is not a member of the post-condition environment. Any ownership consequences that condition production applied to pre-existing bindings are already reflected in the post-condition environment.

## Validation does not prune by Bool value

Both represented conditional outcomes MUST be considered for source validity independently of the semantic Bool value that one concrete runtime execution may observe.

In particular:

- a condition spelled as the literal `true` does not exempt the false outcome—explicit else arm or omitted-else outcome—from source validity; and
- a condition spelled as the literal `false` does not exempt the then arm from source validation.

An explicit arm that contains a represented terminal return is still validated in full even when a constant condition would prevent that arm from executing in one concrete run.

This is a source-validity rule. It does not assert that both outcomes execute in one concrete activation and does not create an unknown or three-valued Bool.

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

Conditional selection itself performs no additional binding read, move, duplicate, assignment, structural consumption, cleanup, call, fault selection, return, or hidden source state transition beyond ending ownership of the successful Bool condition transient used for selection.

The represented condition is therefore evaluated once even though source validation considers both outcomes.

## Condition fault

If condition evaluation yields one defined fault `F` before successful Bool production:

- no explicit conditional arm begins;
- no conditional normal successor is selected;
- ownership transitions already completed during condition evaluation remain effective; and
- the active activation follows the existing defined-fault cleanup and propagation relation from `function-execution.md` with the same fault `F`.

The conditional statement does not add a second fault cleanup boundary.

## Condition divergence

If condition evaluation diverges before successful Bool production:

- no explicit conditional arm begins;
- no conditional normal successor is selected; and
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
- ordinary locals, record-pattern bindings, nested blocks, assignments, calls, and the arm's optional terminal return retain their existing semantics; and
- nested represented conditionals may occur because `IfStatement` is itself a represented body statement inside an arm block.

This document does not create an abstract source-visible branch-scope identity beyond the ordinary child lexical scopes already owned by `local-bindings.md`.

## Explicit arm normal completion

When an explicit arm has a normal continuation and the selected execution reaches that normal completion:

1. finish its contained body-statement sequence under `function-execution.md`;
2. normally exit that child lexical scope;
3. clean bindings declared directly in that arm using the existing lexical-scope cleanup relation; and
4. only after that cleanup does the arm produce its **normal enclosing outcome** for conditional-successor purposes.

Arm-local bindings have ended before normal-successor comparison and therefore are not members of the enclosing environment compared when two normal outcomes meet.

Normal cleanup of one arm does not itself clean or reset enclosing bindings merely to make their states match another arm.

## Explicit arm return

When source validation determines that an explicit arm has no normal continuation because every represented path through that arm returns, that arm contributes no normal enclosing outcome to the conditional.

At runtime, when the selected arm reaches one of those represented returns:

- the return terminates the current source function activation under `function-execution.md`;
- the arm does not first perform independent normal child-scope cleanup;
- every then-active lexical scope participates exactly once in return-induced activation cleanup; and
- no conditional normal join or normal successor is taken on that execution.

A returning arm may consume the complete root or an arbitrary source-valid structural subvalue of an enclosing binding while evaluating its return value. That returning state does not have to equal a normal sibling outcome merely to make the conditional source-valid. Return cleanup uses the returning path's actual then-current structural ownership state.

This rule does not introduce path-dependent ownership at a normal successor because the returning outcome has no normal successor state.

## Explicit arm fault and divergence

If execution inside the selected arm yields a defined fault, that execution produces no normal enclosing outcome.

The active child scope participates exactly once in the existing activation fault cleanup from `function-execution.md`. It does not first perform independent normal arm cleanup and then fault cleanup.

If execution inside the selected arm diverges, that execution produces no runtime normal outcome and performs no normal arm or successor cleanup merely because execution remains suspended.

Fault and divergence remain dynamic execution outcomes. They do not alter the static normal-continuation-presence classification owned by `function-execution.md`.

## Omitted else

When no explicit else arm is present, runtime `false` selects the **omitted-else normal false outcome**.

That outcome:

- executes no body statement;
- introduces no lexical binding;
- creates no source-visible synthetic lexical scope;
- performs no cleanup; and
- carries every enclosing binding's structural ownership state exactly as it exists in the post-condition environment.

For normal-successor validation, the omitted-else false outcome is therefore the unchanged post-condition enclosing environment and always contributes one normal outcome.

An explicit `else {}` is an actual child lexical scope, even though its empty body yields the same enclosing structural ownership outcome.

## Enclosing environment at a conditional successor

Let `E` be the post-condition function-local binding environment.

`E` contains exactly the active parameter/local binding identities that remain in the enclosing lexical environment after successful condition production. Condition evaluation cannot introduce one new function-local binding because represented conditional values are owned-value producers rather than declarations.

Each explicit arm is source-validated from its own copy of `E`. An omitted else uses unchanged `E` as its normal false outcome.

After normal completion and local cleanup of an explicit arm that has a normal continuation, its normal outcome contains only the binding identities belonging to `E`. An arm with no normal continuation contributes no enclosing environment for normal-successor composition.

Binding identity, declared type, and assignment-mutability classification are unchanged by conditional branching itself. Where two normal outcomes meet, the conditional therefore compares each enclosing binding's structural ownership state.

## Conditional normal-continuation composition

After both represented outcomes have been source-validated, compose only their **normal** outcomes:

- **two normal outcomes:** the conditional has a normal successor only when the exact structural-ownership-state equality rule below succeeds for every binding in `E`; the equal common state is the one definite normal successor;
- **exactly one normal outcome:** that sole outcome is the conditional's one definite normal successor without any ownership-equality comparison against the returning outcome;
- **zero normal outcomes:** the conditional has no normal continuation and no normal join; and
- **omitted else:** the false outcome is always normal, so a conditional without explicit else always has at least one normal outcome.

This is only composition of the two-case normal-continuation-presence relation from `function-execution.md`. It introduces no source state set, completion lattice, maybe-owned state, implicit cleanup edge, or runtime completion tag.

A sole normal outcome is not a union, intersection, widening, or normalization of branch states. It is exactly the enclosing environment produced by that one normally completing outcome after its ordinary arm-local cleanup.

## Exact structural-ownership state equality for two normal outcomes

For one represented enclosing binding, two normal outcomes have equal structural ownership state exactly when their prefix-free consumed-path sets under `structural-ownership.md` are equal.

When **two** normal outcomes meet, a represented conditional has a valid normal successor only when every binding identity in `E` has equal structural ownership state on those two outcomes.

When all enclosing binding states are equal:

- that common state is the one definite structural ownership state of the binding at the normal continuation; and
- subsequent source validation proceeds through the existing single-state binding relation in `local-bindings.md` and `structural-ownership.md`.

When two normal outcomes exist and any enclosing binding has unequal normal outcome states, the conditional statement is source-invalid at its normal join boundary. No normal post-conditional ownership state is established.

The equality requirement is semantic equality of source structural state. It is not equality of runtime scalar values, record values, Core local state, parser nodes, HIR data structures, compiler hashes, or physical storage.

No equality comparison is performed between a normal outcome and an outcome that has no normal continuation because it returns.

## Definite-state consequences

The exact-state rule for two normal outcomes has these required consequences.

### Equal complete consumption

If both normal outcomes consume the complete root of the same enclosing non-duplicable binding and do not reinitialize it, both consumed-path sets contain the complete root path and the binding may join as unavailable.

If two normal outcomes exist and one consumes the complete root while the other leaves it fully available, the states differ and the conditional is source-invalid.

A returning outcome may independently consume that complete root; it contributes no normal state to compare against a sole normal sibling.

### Equal partial consumption

If both normal outcomes consume exactly the same represented nested structural paths of one enclosing binding, the equal resulting consumed-path set may join.

If two normal outcomes leave different consumed sibling or nested paths, the states differ and the conditional is source-invalid.

No structural similarity, equal remaining-frontier shape, or equal number of consumed paths substitutes for exact consumed-path-set equality.

A returning outcome may independently consume a different valid structural path; return cleanup handles the resulting state rather than normalizing it toward a normal sibling.

### Duplicable uses

A represented non-consuming duplicate use leaves the consumed-path set unchanged under `structural-ownership.md` and therefore does not by itself prevent an equal-state join when two normal outcomes meet.

### Assignment and reinitialization

Whole-binding assignment retains its existing source-first replacement semantics.

A successful assignment may begin from a fully available, partially available, or unavailable binding state and ends by establishing fresh complete structural ownership for the replacement value.

Consequently, two normal arms with different earlier ownership histories may still produce equal normal ownership states when accepted whole-binding assignments explicitly re-establish the same complete-state classification before normal arm completion.

The conditional does not special-case assignment and does not require runtime values assigned by different arms to be equal.

### Branch-dependent runtime values

Two normal outcomes may produce different runtime values in the same mutable enclosing binding while still having equal structural ownership state.

Structural ownership definiteness is not value equality, constant propagation, or SSA value merging.

### Omitted else

With no explicit else arm, the false normal outcome is the unchanged post-condition state.

When the then arm also completes normally, its normal enclosing structural ownership state MUST equal that unchanged post-condition state for every enclosing binding. Therefore a normally completing then arm may not leave one enclosing binding consumed or partially consumed unless condition evaluation had already established exactly that same state before both outcomes were split or accepted operations inside the then arm restore the post-condition state before normal completion.

When the then arm has no normal continuation because it returns, the omitted-else false outcome is the sole normal successor and no ownership equality is required against the returning then arm.

### Zero-field and zero-leaf values

Zero-field and recursively zero-leaf source values participate in consumed-path-state equality exactly like other structural owned values when two normal outcomes meet.

Their source ownership state MUST NOT be inferred from whether a Core representation has a scalar destruction leaf or emits a physical destruction operation.

## Post-successor source operations

After a conditional with one valid normal successor, every represented enclosing binding has exactly one committed structural ownership state: either the equal state of two normal outcomes or the exact state of the sole normal outcome.

Subsequent whole-binding use, field-value use, record-pattern use, assignment, nested conditional, lexical cleanup, return cleanup, or fault cleanup consumes that one ordinary state through its existing source owner.

No post-successor operation performs a second conditional-state analysis merely because the value was previously used inside a represented conditional.

A conditional with zero normal outcomes admits no subsequent statement in the same containing sequence under `function-execution.md`.

## No implicit join normalization

When two normal outcomes meet, this conditional relation does not derive a common state by:

- union of consumed paths;
- intersection of consumed paths;
- prefix minimization;
- meet, join, widening, or another lattice operation;
- automatically consuming still-owned values on one branch edge;
- inventing a maybe-owned state; or
- consulting lower Core liveness.

In particular, source-invalid unequal two-normal-outcome states are not made valid by silently cleaning additional owned subvalues on an outcome that retained them.

When exactly one normal outcome exists, using that outcome directly is not normalization and performs no branch-edge cleanup.

This avoids introducing path-specific source destruction timing as an implicit consequence of merely reaching a branch successor.

## No conditional ownership flags

This revision introduces no source drop flag, runtime moved-state flag, hidden path tag, conditional cleanup bit, or source-visible ownership test.

Every binding that reaches a represented normal continuation has one definite structural ownership state fixed statically by the rules above.

Future source control-flow forms or later extensions may introduce additional accepted relations only through their own normative changes. Their existence MUST NOT retroactively alter the semantics of two-normal-outcome conditionals already accepted by the exact-state relation.

## Constant conditions

A concrete source execution with condition value `true` executes only the then arm. A concrete source execution with condition value `false` executes only the false outcome.

Source validation nevertheless validates both represented outcomes and computes each arm's normal-continuation presence independently of that concrete Bool value. When both outcomes are normal, the exact-state equality requirement applies to both even for a constant condition. When only one outcome is normal, it is the sole static normal successor even if a particular constant Bool execution selects the returning outcome instead.

This difference between runtime selection and conservative source validation does not introduce nondeterministic source execution.

## Nested conditionals

A represented conditional may occur inside any explicit arm block because it is a body statement under `concrete-syntax.md`.

The inner conditional independently establishes zero or one definite normal successor by the composition rules above. If it has one, the containing arm may continue from exactly that state. If it has none, no later statement or terminal return in that same arm sequence is source-valid.

Consequently, nested conditionals compose recursively through normal-continuation presence without a general source CFG or state-set relation.

This revision defines no direct `else if` grammar. Equivalent nested selection may be written only through the represented explicit block nesting admitted by `concrete-syntax.md`.

## Result-bearing function boundary

This conditional statement consumes the result-bearing normal-path completion requirement from `function-execution.md`.

A conditional whose two explicit arms both have no normal continuation because they return has no normal successor. Such a conditional may therefore discharge the remaining result-path obligation without a redundant root terminal return after it.

If exactly one arm has a normal continuation, that sole continuation remains subject to the ordinary result-bearing requirement. A later source-valid result return is required before that path can reach the root closing boundary normally.

A conditional without explicit else always has the omitted-else normal false outcome and therefore cannot by itself eliminate every normal root path.

## Source/Core refinement

A faithful source-to-Core lowering MAY refine one source-valid represented conditional to the accepted Bool-valued `Branch` relation in [Core control flow](../core/control-flow.md).

After source validation has fixed the condition type, each arm's normal-continuation presence, and any required two-normal-outcome source ownership equality, a lowering may:

1. lower the existing condition producer exactly once;
2. materialize its Bool result in a compiler-owned Core temporary when useful;
3. consume that temporary once as the Core `Branch` condition operand;
4. lower the then arm to one or more Core blocks;
5. lower explicit else-arm blocks when present, or use the following normal continuation directly as the false target when no else body needs lower execution;
6. for each normally completing explicit arm, refine that arm's retained source normal cleanup before its normal successor edge;
7. for each returning arm, terminate its lower path through the existing Core `Return` relation without first emitting that arm's normal cleanup or a normal `Goto` edge;
8. when two normal outcomes exist, transfer both to one lower normal join before subsequent source statements;
9. when exactly one normal outcome exists, continue subsequent source lowering from that sole normal path, with or without a dedicated lower join block; and
10. when zero normal outcomes exist, emit no lower normal join merely to create an unreachable continuation.

The exact shape or number of Core blocks is not source-observable.

A compiler temporary used for condition selection is not a source binding.

A result-producing return whose producer is a direct call may first create the existing call continuation block and then terminate that continuation with Core `Return`; this does not create a second source continuation.

## Lower path states do not define the source successor

Core CFG validation may preserve multiple distinct implementation states at a lower join even when source enclosing ownership is definite and equal.

For example, a source local declared only in the then-arm child scope may be represented by one Core local that is Dead after normal then-arm cleanup but Never-initialized on a false execution that never entered that arm. Those distinct lower states remain valid implementation facts under Core control flow.

They do not make the ended arm-local source binding visible after the arm and do not create path-dependent source ownership for enclosing bindings.

Lowering MUST NOT:

- use Core path-state union/intersection to reconstruct source structural ownership;
- accept an unequal two-normal-outcome source ownership join merely because every lower continuation operation happens to validate under multiple Core states;
- invent a source normal successor from lower reachability after source validation determined none;
- infer source cleanup from lower scalar liveness; or
- turn Core worklist behavior into source semantic authority.

Source normal-continuation presence, any required source join validity, and the definite source successor state are established before lowering by this document, `function-execution.md`, and `structural-ownership.md`.

## Determinism

For one fixed source-valid represented program and one fixed activation state, conditional runtime selection is deterministic after successful condition evaluation.

The condition producer has its existing deterministic or otherwise accepted source behavior. Once it yields one of the two semantic Bool values, exactly one runtime outcome is selected. That selected outcome may normal-complete, return, fault, or diverge according to its existing owners.

Validation of both represented outcomes is a static validity obligation, not runtime nondeterminism.

## Further boundaries

This revision does not define general expressions, grouping expressions, comparisons, logical operators, arithmetic operators, truthiness, coercions, record-construction conditions, conditional values/expressions, direct `else if`, unrestricted nonterminal-within-block return or arbitrary unreachable tails, loops, match, refutable patterns, catch/recovery, labels, break, continue, source state lattices, path-dependent ownership after a two-normal-outcome join, automatic join cleanup, drop flags, custom destructors, must-consume policy, references, borrows, lifetime inference, optimizer transformations, ABI/linkage, backend branches, Exec, Model, or stable serialized HIR/Core control-flow identity.

Those concerns require their own accepted owners or later extensions and MUST NOT be inferred from the represented conditional relation here.
