# Core Callable Values and Indirect Calls

Status: **provisional normative; incomplete**

This document owns the currently represented Core semantics for callable scalar types, captureless function values, static function-value formation, exact callable-value typing, indirect-call static target typing, callee-operand evaluation, dynamic function-target selection, and the representation-neutral boundary around those relations.

It consumes same-program function identity, the canonical callable-interface relation, parameter/result transfer admission, safe-reference result contracts, result-destination admission, ordinary argument evaluation, final call-entry reference admission, function activation, caller suspension, result transfer, defined-fault propagation, divergence, and termination cleanup from [Core functions and calls](functions.md). It consumes scalar/aggregate storage, stored-value lifetime, `Move`, `Copy`, initialization, assignment, destruction, and cleanup from [Core value and storage semantics](value-storage.md); safe-reference value/authority/carrier semantics from [Core references](references.md); raw-pointer value/provenance semantics from [Core pointers and provenance](pointers.md); and the absence of an implicit stable physical callable representation from [Core layout and ABI](layout-abi.md). It does not redefine those owners.

This relation is independent of source syntax, source function-value policy, source closure/capture semantics, ABI, linkage, calling convention, physical code addresses, backend realization, or a particular compiler representation.

## Callable scalar type

The represented Core type domain contains one **callable scalar kind** parameterized by exactly one canonical callable interface from `functions.md`.

A callable interface contains exactly:

1. one finite ordered sequence of parameter Core `TypeId`s;
2. either no result value or exactly one result Core `TypeId`; and
3. exactly one `SafeReferenceResultContract` from `functions.md`.

The callable scalar kind does not create a second callable-interface relation. Interface structure, parameter-transfer-safe admission, result admission, and safe-reference result-contract validity remain owned by `functions.md`.

Scalar kind and exact per-program Core type identity remain distinct facts under `value-storage.md`. Consequently:

- two distinct callable `TypeId`s MAY carry equal callable interfaces;
- equal callable interfaces do not make the two `TypeId`s equal;
- ordinary places, stored values, `Move`, `Copy`, operand typing, parameter/result typing, and indirect-call static typing continue to use exact `TypeId` identity; and
- this revision introduces no implicit conversion, coercion, subtyping, structural type equivalence, or canonical interning relation between distinct callable `TypeId`s.

A callable type is one scalar/leaf value-shape type. Its interface parameter/result `TypeId` references are semantic signature edges, not structural value-containment edges. Therefore a callable value does not recursively contain values of its parameter or result types.

A finite Core type table may contain callable-interface reference cycles, including a callable interface that directly or indirectly refers to its own callable type, without thereby creating an infinitely recursive runtime value shape. Existing structural aggregate recursion restrictions remain unchanged because only structural field-containment edges contribute to structural value recursion.

For the structural value-shape and call-transfer predicates consumed from `functions.md`, a callable type is an ordinary non-pointer, non-reference scalar leaf. In particular:

- a callable leaf itself contains no safe-reference or raw-pointer leaf merely because its interface accepts or returns such types;
- recursive parameter-transfer-safe, reference-parameter-referent-safe, and result-transfer-safe structural checks stop at the callable leaf;
- an aggregate may contain callable leaves under the ordinary structural value/storage rules; and
- a callable value itself may cross ordinary parameter and result boundaries when its exact callable type is otherwise admitted.

The callable interface carried by every callable type is validated independently through the canonical callable-interface validity relation in `functions.md`. Treating the callable value as a scalar leaf therefore does not weaken or bypass validation of the interface it denotes.

## Callable-type validity

A callable Core type is language-valid exactly when:

1. its exact `TypeId` exists in the program-wide Core type domain;
2. its scalar kind is callable and contains exactly one finite callable interface; and
3. that interface is valid under the canonical callable-interface validity relation in `functions.md`.

Callable-interface validity establishes the existence and applicable transfer/result-contract requirements of every referenced parameter/result type. This document does not restate those rules.

A type table containing callable-interface cycles is validated as a finite graph of type identities and semantic signature edges. Validation MUST NOT reject such a table merely because recursively following callable-interface edges revisits a callable `TypeId`. Structural aggregate recursion remains governed by the structural owner and is not weakened by this rule.

## Function values

A **Core function value** is one semantic scalar value whose payload is exactly one `FunctionId` identifying one represented function entity in the same Core program.

Distinct represented function entities denote distinct function values even when their derived callable interfaces are equal.

A function value has no independent dynamic identity beyond the selected function entity. Forming, copying, moving, storing, passing, returning, or destroying a function value does not create, clone, enter, suspend, or terminate a function activation.

A function value inhabits exact callable type `C` exactly when:

1. `C` is a valid callable Core type;
2. the function value's `FunctionId` identifies an existing function entity in the same represented Core program; and
3. that function entity's exact derived callable interface under `functions.md` equals the callable interface carried by `C`.

Exact callable-interface equality requires:

- the same parameter count;
- exact corresponding parameter `TypeId` equality in order;
- the same no-result/result structure;
- exact result `TypeId` equality when a result exists; and
- the same `SafeReferenceResultContract` variant and, when present, the same origin parameter-slot index.

Function name, body structure, source declaration identity, source accessibility, source spelling, physical address, symbol identity, linkage, backend realization, call-graph position, and current activation state are not callable-interface matching dimensions.

The semantic payload does not duplicate the callable `TypeId`. Exact callable type is supplied by the typed place or operand context in which the value exists. A value stored as callable type `C1` therefore remains a value of exact type `C1` under ordinary value transport even when another callable type `C2` has an equal callable interface.

## Static function-value formation

The Core semantics provides one static semantic producer, written here as `FunctionValue(f)`, that produces the function value naming existing same-program function entity `f`.

This spelling names the semantic producer and does not require one particular proving-MIR enum variant, constant carrier, instruction encoding, or compiler representation.

`FunctionValue(f)` is context-typed. For one required exact callable type `C`, formation is valid exactly when the function value naming `f` inhabits `C` under the rule above. The producer itself need not carry or duplicate `C`; the exact expected type is supplied by the surrounding typed operand context.

For a valid target/type pair, static function-value formation is:

- effect-free;
- non-faulting;
- non-diverging;
- independent of storage lookup;
- independent of safe-reference authority or carrier state;
- independent of raw-pointer provenance;
- independent of source spelling; and
- independent of a physical code address or ABI representation.

Formation creates no storage identity, activation identity, reference authority, pointer provenance, symbol, allocation, or physical address.

This revision defines no null, invalid, uninitialized, external, unresolved, or signature-mismatched function value that can be fabricated as an ordinary semantic alternative. Unknown targets and interface mismatches are program-validity failures.

## Ordinary value and storage behavior

A captureless Core function value is **copyable**.

Ordinary value/storage rules from `value-storage.md` apply to callable values and to structural aggregates containing callable leaves:

- `Move` transfers the exact function value and consumes the source stored value under the ordinary move relation;
- `Copy` preserves the source and produces another owned function value naming the same exact function entity;
- local storage may contain callable values;
- `Init`, ordinary assignment, and interior assignment may write callable values when their independently owned destination requirements are satisfied;
- callable stored-value lifetimes begin and end under the ordinary scalar stored-value lifecycle;
- destruction of a callable value ends only that stored-value lifetime and has no callable-specific cleanup action; and
- callable values may be passed and returned under their exact callable `TypeId` using the ordinary transfer relations from `functions.md`.

Copying a function value creates no new function entity, activation, storage identity, alias authority, reference carrier, pointer provenance, symbol, or physical address.

The copyability rule in this document applies only to this captureless Core function-value form. It does not establish copyability for future closure/environment values; such a future value form may have different value shape and capability requirements.

This revision defines no callable-specific equality, inequality, ordering, hashing, pattern matching, arithmetic, conversion, pointer conversion, serialization, or observation operation. Distinct function entities remain semantically distinct values even though this slice adds no operation that compares them.

## Indirect-call form

The represented Core relation contains one indirect call whose semantic structure contains exactly:

1. one explicit exact callable `TypeId` `C`;
2. one callee operand required to produce a value of exact type `C`;
3. one finite ordered sequence of ordinary argument operands;
4. either no result destination or exactly one direct result destination place; and
5. exactly one normal continuation block in the caller body.

The explicit callable `TypeId` is semantically required. The current Core operand relation is context-typed and a semantic function-value payload carries only `FunctionId`; therefore the callable type cannot in general be recovered uniquely from the callee operand. Two distinct callable `TypeId`s may also have equal callable interfaces.

The call's exact static callable type is fixed before operand evaluation. The call does not infer or change that type from the callee value, the first argument, a result destination, or implementation convenience.

## Indirect-call static validation

Let `C` be the indirect call's explicit callable `TypeId` and let `I` be its callable interface.

Before execution, static validation requires all of the following:

- `C` exists and is a valid callable type;
- `I` is the exact callable interface carried by `C` and is valid under `functions.md`;
- the result destination presence, exact type, vacancy/access admission, and result-contract compatibility satisfy the common result-destination relation from `functions.md` for `I`;
- the callee operand is valid under exact expected type `C`;
- ordinary argument count equals the parameter count of `I`;
- each ordinary argument operand is valid under the exact corresponding parameter `TypeId` from `I`;
- the normal continuation block exists in the caller body; and
- every independently owned operand/place/reference requirement is satisfied.

Static validation consumes the callable interface, not a dynamically selected target body, to establish argument count/types, result structure/type, safe-reference result contract, and parameter/result transfer admission.

The result destination is admitted at the call point before any callee-operand or ordinary argument-operand state transition. Operand effects cannot make an initially inadmissible destination valid for that same indirect call.

The callee operand is validated under exact type `C`. Equal callable-interface structure in a distinct callable type does not admit that operand and introduces no implicit conversion.

## Indirect-call evaluation and target selection

For a statically valid indirect call, execution proceeds in this exact order:

1. retain the already-established static callable type/interface and result-destination admission facts;
2. evaluate the callee operand completely under exact expected callable type `C`, producing and holding one owned callable value;
3. preserve all state consequences of callee-operand evaluation;
4. evaluate each ordinary argument operand strictly left to right under the corresponding parameter type from the callable interface, holding each produced owned transient argument value as required by `functions.md`;
5. after every ordinary argument effect, apply the existing final call-entry authority/liveness requirements from `functions.md` to every recursively contained safe-reference carrier in the held argument values;
6. select the exact same-program `FunctionId` carried by the held callable value;
7. require the selected function entity's derived callable interface to equal the already-established static callable interface; and
8. consume target selection into the common function-call execution relation from `functions.md`.

Callee-operand evaluation therefore precedes ordinary argument evaluation. A `Move` used as the callee operand commits its ordinary source-state transition before the first ordinary argument is evaluated. A `Copy` callee operand preserves its source under the ordinary copy relation.

The held callee value is a transient target-selection value. After its `FunctionId` is selected it is not retained as activation-local state, transferred into the callee, or stored by the call. Consuming it for target selection has no callable-specific cleanup consequence. Any source storage consumed by a `Move` was already changed by ordinary operand evaluation.

A directly formed `FunctionValue(f)` callee is effect-free, so its callee-evaluation step changes no storage state before argument evaluation.

The represented operand set in this revision adds no new defined-fault or divergence outcome merely for callable-value formation or target selection. If a future operand owner permits an abnormal callee-operand outcome, that owner must define its interaction with the call; this relation does not invent one.

## Dynamic target invariant

For every validated execution reaching target selection, the held callable value names exactly one existing function entity in the same represented Core program.

The callable-value typing invariant guarantees that the selected function's derived callable interface equals the exact static interface used to validate the indirect call. Therefore a validated execution has no ordinary runtime alternative for:

- an unknown function target;
- a null callable target;
- a signature mismatch;
- a result-presence/type mismatch;
- a safe-reference result-contract mismatch; or
- a target from another represented Core program.

Those cases are language/program validity failures, not represented runtime faults.

No global finite target-set analysis is required for an indirect call beyond the ordinary finite same-program function identities that valid function values may carry. Function bodies remain independently validated and are not recursively expanded into each indirect call site.

## Common call execution after target selection

After target selection, indirect invocation reuses the existing `functions.md` relations for:

- fresh dynamic activation identity;
- fresh local storage instances;
- parameter transfer;
- establishment of safe-reference result origins;
- caller suspension;
- external safe-reference referent domains and final call-entry admission;
- direct or mutual recursion and call-graph cycles;
- normal return and result transfer;
- `SharedIdentity` and `SharedDirectChild` result consequences;
- defined-fault outward propagation;
- divergence;
- termination cleanup; and
- caller continuation.

This document defines no second activation machine, parameter-transfer mechanism, reference-result summary, fault propagation relation, or cleanup order for indirect calls.

The dynamic target may itself execute direct or indirect calls according to the independently validated body. Call-graph cycles through indirect calls are not invalid merely because they may recurse or diverge.

## Direct calls remain represented

The existing direct-call relation in `functions.md` remains a distinct represented Core call form.

A direct call identifies its function target statically and therefore performs no callable-value formation, callee-operand evaluation, or dynamic target-selection step. It continues to use the same common callable-interface, destination, argument, activation, result, fault, divergence, and cleanup relations where applicable.

This revision does not redefine direct calls as syntactic sugar, require them to lower through `FunctionValue`, or remove their statically embedded function identity.

## Reference, pointer, and ABI boundary

A function value is neither a safe reference nor a raw pointer.

It has no storage target, referent lifetime, alias authority, reference carrier, raw-pointer origin, pointer provenance, address arithmetic, pointer conversion, or pointer-observable identity. Existing safe-reference parameter/result contracts are facts of the callable interface and do not turn the callable value itself into a reference.

Callable values and indirect target selection are representation-neutral. They establish or expose none of the following:

- code addresses;
- physical function-pointer representations;
- pointer equality or integer conversion;
- stable byte layout, size, or alignment;
- calling convention;
- register or stack placement;
- linker symbol identity;
- export/import linkage;
- FFI compatibility;
- dynamic-library or plugin loading;
- dispatch-table or vtable layout; or
- serialized callable identity.

`layout-abi.md` remains controlling for stable representation and external ABI policy. A backend may realize this semantic relation using any mechanism that preserves the accepted Core behavior; that realization is not additional language semantics.

## Validation requirements

A represented Core program using callable values and indirect calls is language-valid only when, in addition to all existing Core validity requirements:

- every callable `TypeId` has one callable scalar kind containing one valid canonical callable interface;
- callable-interface parameter/result edges are treated as semantic signature references rather than structural containment edges;
- every `FunctionValue(f)` is validated under one exact expected callable type and names one existing same-program function whose derived interface exactly equals that callable type's interface;
- ordinary storage/transport preserves exact callable `TypeId` identity and does not infer conversions from equal interfaces;
- every indirect call carries one exact valid callable `TypeId` explicitly;
- every indirect callee operand produces that exact callable type;
- indirect result destination and ordinary argument validation consume the callable type's exact interface;
- result destination admission is established before callee or argument operand effects;
- callee evaluation completes before the first ordinary argument evaluation;
- ordinary arguments evaluate strictly left to right;
- final safe-reference call-entry admission is applied after all ordinary argument effects and before activation creation;
- the selected function target exists in the same program and its derived callable interface equals the static callable interface;
- common activation, parameter/result/reference, fault, divergence, cleanup, and continuation rules are reused from `functions.md`; and
- no null/invalid/mismatched target is represented as an ordinary runtime alternative.

Validation MUST NOT reject a finite program merely because callable-interface signature edges form a cycle, because multiple callable `TypeId`s have equal interfaces, because an indirect target set cannot be reduced to one static function, or because indirect recursion may diverge.

## Deliberate boundaries

This revision does not define:

- source-language function types, function values, or indirect-call syntax;
- bare source function names as value producers;
- closures, capture sets, environments, or capture modes;
- closure escape/lifetime/copyability policy;
- closure recursion or self-reference;
- generic function values or indirect generic calls;
- type-parameter-bearing Core function entities;
- polymorphic or higher-ranked callable values;
- runtime generic witnesses or specialization selection;
- callable traits, trait methods, trait objects, vtables, virtual dispatch, or dynamic method lookup;
- methods or associated functions as values;
- external or FFI callable values;
- null function pointers or optional callable values;
- code/address values or pointer casts/conversions;
- callable equality, inequality, ordering, hashing, arithmetic, or pattern operations;
- ABI, calling-convention, linkage, export/import, or symbol mechanisms;
- package, plugin, or dynamic-library loading;
- async/coroutine/generator callables;
- variadics, default arguments, or new effect signatures;
- a global call-graph target-set inference requirement;
- Core implementation, reference-machine, compiler, backend, or conformance-test changes; or
- a universal source-to-Core refinement proof.

Those concerns require their own accepted semantic owner or consumer before this relation is extended.
