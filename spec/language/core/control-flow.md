# Core Control Flow

Status: **provisional normative; incomplete**

This document owns the represented Core semantics for finite function-body basic blocks, activation entry into a body, statement-order progression at the control-flow level, unconditional intra-activation transfer, scalar-`Bool` conditional branching, control-flow-graph path-state validity, and cyclic intra-activation execution.

It consumes Core values and value/type compatibility, local storage, stored-value lifetime, operand ownership transfer/copy, assignment, destruction domains, and termination cleanup from [Core value and storage semantics](value-storage.md); loan/access authority from [Core borrowing](borrowing.md); raw-pointer operand behavior from [Core pointers and provenance](pointers.md); unsafe-operation outcomes and undefined-behavior separation from [Core unsafe semantics](unsafe.md); direct-call activation, caller suspension, result transfer, normal return, and call fault propagation from [Core functions and direct calls](functions.md); and defined-fault classification from [Core faults](faults.md). It does not redefine those owners.

The represented Core data model and validator may use numeric block identifiers, vectors, worklists, hashes, or other implementation structures. Those representations are not Core program values or source-language semantics.

## Function-body control-flow structure

A represented Core function body contains one finite non-empty collection of **basic blocks** and designates exactly one of those blocks as its **entry block**.

Every represented basic block contains:

1. one finite ordered sequence of represented Core statements; and
2. exactly one represented Core terminator.

A basic-block identity distinguishes one block from another only within its containing function body. It is not:

- a Core scalar or aggregate value;
- a dynamic function-activation identity;
- a local-storage identity;
- a source-language label or source-observable identity;
- a physical code address;
- an ABI or linkage identity; or
- a stable serialized numeric handle.

The order in which an implementation stores basic blocks does not itself define execution order. Execution order is selected only by the entry block and the terminator transfer relations defined here or delegated to another accepted terminator owner.

## Activation entry

Creation of a dynamic Core function activation and transfer into its parameter locals are owned by `functions.md`.

After that relation has completed successfully, execution of the activation begins at the body's designated entry block.

Entering a basic block does not itself:

- create another activation;
- create another local storage instance;
- initialize, move, copy, destroy, or replace a value;
- create or end a loan; or
- perform cleanup.

Those effects occur only through the represented statements, operands, or terminators whose accepted owners define them.

## Basic-block execution

Within one entered basic block, represented statements execute strictly in their stored block order.

For each statement:

1. its existing semantic owner determines its value, storage, borrowing, pointer, unsafe, and other state consequences;
2. if that statement has a defined continuation, the next statement in the block begins with exactly the resulting state; and
3. if that statement has no defined continuation, no later statement or terminator in that block is reached in that execution.

After the final statement completes with a defined continuation, execute the block's terminator exactly once.

A terminator either:

- selects a successor basic block in the same activation under this document;
- delegates to the accepted direct-call relation in `functions.md`, which may later resume at its represented normal continuation block;
- delegates to the accepted normal-return relation in `functions.md`; or
- delegates to the accepted defined-fault termination relation and call-fault propagation owned by `faults.md` and `functions.md`.

This document defines only control-flow composition. It does not duplicate the semantic rules of the statements or delegated terminators.

## Unconditional transfer

The represented unconditional intra-activation terminator is **`Goto(target)`**.

`target` MUST identify one existing basic block in the same function body.

When a block reaches `Goto(target)` with a defined continuation:

1. `Goto` evaluates no operand and produces no value;
2. `Goto` changes no local initialization state, stored-value lifetime, storage-instance identity, pointer value, or active-loan state; and
3. execution continues by entering `target` in the same function activation with exactly the state produced by the preceding block.

The target MAY be the same basic block as the current block. Unconditional edges MAY participate in a control-flow cycle.

## Bool-kind conditional branch

The represented conditional intra-activation terminator is one branch with:

- exactly one represented Core condition operand;
- exactly one true-target basic block; and
- exactly one false-target basic block.

The abstract semantic shape is:

```text
Branch {
    condition: Operand,
    true_target: BasicBlock,
    false_target: BasicBlock,
}
```

This spelling is explanatory only. A conforming implementation may represent the same semantic fields differently while preserving the relation below.

Both targets MUST identify existing basic blocks in the same function body. The two targets MAY identify the same block.

The condition operand's resolved Core type MUST be a scalar type whose scalar kind is `Bool`. The branch contract does not designate or require one globally distinguished Bool type identity. No condition whose resolved type has another scalar kind or structural aggregate kind is admitted.

The two semantic Bool values consumed by this branch relation are owned by [Core value and storage semantics](value-storage.md).

A branch executes in this order:

1. after all statements in the current block complete with a defined continuation, evaluate `condition` exactly once under its existing operand semantics;
2. preserve every state consequence of that operand evaluation;
3. if the produced Bool value is `true`, enter `true_target` in the same activation with the resulting state;
4. if the produced Bool value is `false`, enter `false_target` in the same activation with the resulting state.

The branch operation itself performs no additional read, move, copy, write, destruction, cleanup, borrow, pointer operation, fault selection, or hidden storage transition.

Consequently:

- a condition represented by `Move` has the ordinary ownership-consuming effect of that `Move` before the selected successor begins;
- a condition represented by `Copy` has the ordinary non-consuming effect of that `Copy` before the selected successor begins;
- any raw-pointer or unsafe operand behavior remains governed by its existing owner; and
- if condition evaluation has no defined continuation because undefined behavior has been selected by an existing unsafe rule, neither branch target is reached in that defined execution.

This branch relation defines no truthiness, integer-to-Bool conversion, comparison, predicate operation, implicit conversion, coercion, or second Bool-like type.

## Runtime branch selection and validation branch edges

Concrete Core execution and Core path-state validation have distinct responsibilities.

### Runtime execution

A concrete defined execution observes exactly one of the two semantic Bool condition values owned by `value-storage.md` and therefore takes exactly one branch target as defined above.

This control-flow relation introduces no unknown, symbolic, three-valued, or validation-only Bool value.

### Path-state validation

Program-level Core validation MUST establish operation state preconditions for every state propagated along the represented control-flow graph from the entry validation state.

For validation purposes, a conditional branch contributes **both** of its target edges to that graph independently of the concrete Bool value that one runtime execution would observe.

After validating the condition operand's type and applying the condition operand's existing validation-state effects, the resulting validation state is propagated to both branch targets.

This is an intentional validation over-approximation. It does not redefine concrete branch execution and does not assert that both targets execute in one activation.

Core validation therefore does not require constant evaluation, constant propagation, symbolic execution, or general value analysis merely to prune a branch edge. In particular, a branch whose condition operand is the constant Bool value `true` still contributes both branch edges to path-state validation unless another accepted transformation has removed the false edge before the program is validated.

## CFG-reachable validation states

The **entry validation state** is the existing validation state established for a function body after parameter locals receive their validation values under the represented program-validation relation.

A validation state is **CFG-reachable** at a block when it can be propagated from that entry state by repeatedly applying the accepted validation-state effects of statements/operands and these control-flow edges:

- `Goto` contributes its one target edge;
- `Branch` contributes both target edges after condition-operand validation-state effects;
- a normally continuing direct call contributes its existing normal continuation edge after the call's already-defined state effects and result initialization facts;
- `Return` and `Fault` contribute no intra-activation successor edge.

When an existing unsafe operation has no defined continuation under the validation relation, that validation execution contributes no successor state beyond that point.

A basic block may receive more than one distinct CFG-reachable validation state.

Those states MUST remain semantically distinct for validation whenever merging them could change whether a later represented operation satisfies its preconditions.

In particular, this document does not establish a union, intersection, meet, join, widening, or other merged state for:

- Never-initialized/Live/Dead structural storage state;
- raw-pointer target identity;
- active-loan identity, kind, place, or parent relation; or
- another state component owned by an existing Core semantic relation.

Every represented statement or terminator reached under multiple CFG-reachable validation states MUST satisfy its applicable state preconditions independently for every such incoming state.

A conforming validator MAY use a worklist, graph traversal, memoization, or another implementation strategy. It MAY treat an already-validated pair consisting of the same basic block and semantically equal complete validation state as requiring no second validation traversal.

This allowance does not create a runtime state identity and does not make validator hashes, allocation addresses, or traversal order semantic.

## Disconnected and statically invalid blocks

Static/structural Core validity and path-state execution validity are separate obligations.

Every represented basic block MUST satisfy applicable static structural/type rules independently of whether that block is CFG-reachable from the entry block. This includes, where applicable:

- valid local, loan, place, projection, function, type, and block identities;
- valid operand and destination type structure;
- a branch condition whose resolved type has scalar kind `Bool`; and
- valid terminator targets.

A structurally valid basic block with no CFG path from the entry validation state need not be executed by path-state validation merely because it is present in the body.

Once a CFG-reachable branch has an outgoing edge to a target, that target receives the propagated validation state even when a concrete runtime condition would always select the other target.

This distinction permits validation to remain CFG-based without turning static structural validation into execution or path-state validation into constant evaluation.

## Cycles and divergence

A represented Core control-flow graph MAY contain cycles formed by `Goto`, `Branch`, normal direct-call continuation edges, or combinations of those edges where the applicable owner permits them.

A cycle is not itself a validation error.

A concrete execution that continues indefinitely through such a cycle **diverges**.

Divergence:

- does not become a defined `Fault` merely because an implementation or test harness has run for a long time;
- does not implicitly perform function termination cleanup under the storage relation in `value-storage.md`;
- does not end local storage extents merely because the same basic block is revisited; and
- does not introduce an implicit iteration or execution-step limit.

The existing function-call recursion relation in `functions.md` remains independently capable of divergence. Intra-activation control-flow cycles and recursive-call cycles may compose.

## Storage, destruction, and cleanup boundary

Control-flow transfer does not create a second destruction or cleanup relation.

At any point reached through one successor path, local initialization and stored-value lifetime state are exactly those established by the operations that actually executed on that path.

Explicit `Drop`, assignment/replacement destruction, normal-return cleanup, defined-fault cleanup, and any later accepted termination cleanup continue to determine their destruction domains from then-current storage state under `value-storage.md`.

A value moved or destroyed on one runtime branch is therefore Dead on executions that took that branch and is not retroactively Dead on executions that took another branch.

The CFG-based validator checks each propagated validation state independently and MUST NOT reconstruct a single merged liveness state merely to simplify later cleanup validation.

This document introduces no:

- drop flag;
- runtime moved-state flag distinct from existing Core storage state;
- custom destructor body;
- source lexical cleanup relation;
- source ownership state; or
- automatic branch-join destruction.

## Borrowing and pointer-state boundary

Branching itself creates, ends, upgrades, or merges no loan.

If two CFG paths reach the same block with different active-loan states, those incoming validation states remain distinct and every later operation must be valid under each state that reaches it.

Likewise, if represented operations on distinct CFG paths establish different raw-pointer targets or different structural initialization state, path-state validation preserves those distinctions rather than replacing them with one invented target or liveness relation.

Concrete execution follows only the state produced by the branch actually taken.

## Determinism

For one fixed validated Core program and one fixed concrete activation state, unconditional transfer and Bool-kind branch selection are deterministic.

`Goto` has one successor.

`Branch` evaluates its condition once and selects exactly the target corresponding to the resulting Bool value.

The validator's deliberate propagation over both branch edges is an assurance relation, not nondeterministic Core execution.

## Implementation boundary

This document defines no Rust enum, parser, serializer, vector layout, worklist algorithm, hash key, reference-machine data structure, or backend branch instruction.

A canonical implementation may represent control flow with block indices and may validate CFG-reachable states with a finite worklist over complete block/storage/loan validation states. It may retain a value-erased abstraction for ordinary non-pointer scalars because this control-flow validity relation does not require Bool constant propagation.

An implementation MUST NOT use its traversal order, host recursion behavior, hash iteration order, optimizer constant folding, or backend branch behavior as semantic authority.

## Further boundaries

This revision does not define:

- source-language `if`, loops, `match`, catch/recovery, labels, `break`, or `continue`;
- source structural-ownership joins, definite source availability, source drop elaboration, or source cleanup flags;
- general Core comparison, logical, or predicate operators;
- Core constant-evaluation or constant-propagation semantics;
- exception/unwind edges beyond already accepted defined-fault propagation;
- custom destruction;
- optimizer transformation legality or CFG canonicalization;
- ABI, linkage, physical code layout, branch prediction, or backend instruction selection;
- stable serialized Core IR.

Those concerns require their own accepted owners or later consumers.