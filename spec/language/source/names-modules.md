# Source Names and Modules

Status: **provisional normative; incomplete**

This document owns the represented source module identity, module binding, module alias, visibility, and qualified cross-module lookup relations. It consumes lexical identifier keys from [Source lexical foundation](lexical.md) and does not redefine identifier formation or equivalence.

Function-local lexical scopes, local binding identity, local lookup precedence, and local shadowing are owned by [Source function-local bindings](local-bindings.md). The represented concrete record/function/marker-trait/constant accessibility, module-import alias, and two-part qualified module-member forms, including their use as record-pattern heads, marker references, and constant values, are owned by [Source concrete syntax](concrete-syntax.md). Marker trait entity/category semantics, unnamed implementation propositions, and coherence are owned by [Source marker traits](traits.md). Constant declaration/value/use semantics are owned by [Source constants](constants.md). Captureless function-value formation and bounded indirect-call target classification consume these lookup relations through [Source function values and indirect calls](function-values.md). This document does not define member lookup, overload resolution, package discovery, marker implementation selection, constant evaluation beyond that owner, or an implementation representation.

## Source modules

A **source module** is an opaque source-language organization identity established as part of the source compilation context.

The source compilation context is an input to source-language validation. For the rules in this document it supplies the source-module identities that participate in the compilation, the assignment of directly supplied source units to those module identities, and the target module identity named by each represented module-import relation. It is not Runen program state.

For the concrete `import A;` form in `concrete-syntax.md`, the compilation context supplies exactly one target source-module identity associated with lexical alias key `A` for that source unit. The alias key selects that context entry; it does not identify or derive the target module identity.

Source-module identity is not derived from source-unit bytes, a lexical identifier spelling, a filesystem path or file name, a directory, a package coordinate, source-unit presentation order, or another physical storage convention.

Every directly supplied Runen source unit in one source compilation MUST be assigned to exactly one source module. A source module MAY have more than one directly supplied source unit.

Source-unit presentation order, file-system order, and build-system processing order are not semantic inputs to module-level name resolution.

This document does not define how a build system discovers source units, maps files or directories to modules, resolves dependency coordinates, loads module interfaces, caches compilation results, or serializes module identity or exported binding information.

## Module declaration namespace

Every source module has one **module declaration namespace** keyed by lexical identifier keys.

When an accepted source-language rule establishes that a declaration introduces a module-level name, that declaration contributes one binding with its lexical identifier key to this namespace unless the declaration's canonical owner explicitly places that name only in a member-specific or otherwise distinct lookup domain.

The represented record and function definitions in `concrete-syntax.md` each establish their accepted record/function declaration and therefore contribute one module binding through this relation.

The represented marker trait declaration owned semantically by `traits.md` and spelled by `concrete-syntax.md` likewise contributes one ordinary module binding through this relation.

The represented source constant declaration owned semantically by `constants.md` and spelled by `concrete-syntax.md` likewise contributes one ordinary module binding through this relation. Record, function, marker-trait, and constant bindings share this one declaration namespace and the same duplicate-key prohibition.

A represented marker implementation declaration contributes **no module binding**. After its trait/type operands resolve through the applicable relations here, it establishes only the unnamed implementation proposition owned by `traits.md`.

Two distinct module-level bindings in the same source module MUST NOT have the same lexical identifier key.

Module-level bindings are available to module-level name resolution independently of source-unit presentation order and textual declaration order. Reordering directly supplied source units or reordering module-level declarations MUST NOT change which module-level binding a given lexical identifier key denotes.

A module-level binding is identified by its source module and binding identity, not by the original source spelling of its identifier. The namespace key is the lexical identifier key defined by `lexical.md`.

For this foundation, name resolution first identifies one binding/entity. A consuming source-language rule then determines whether that resolved entity category is valid for the applicable type, value, declaration, call, pattern-head, marker-reference, constant-value, or other semantic context. This document does not define separate module-level type, value, and trait namespaces or context-dependent searches across such namespaces.

Function-local parameter/local bindings are owned by `local-bindings.md` and do not become members of this module declaration namespace. This section also does not define fields, methods, associated items, generic parameters, pattern bindings, lifetime names, labels, macros, overload sets, or marker implementation names. Top-level marker trait and constant declarations are ordinary module bindings; trait member/associated lookup and broader constant-expression name use remain separately owned or unrepresented. A later rule that permits one source name to denote an overload set or another multi-entity binding MUST define that binding relation explicitly; duplicate module-level binding keys do not become an overload set merely because the declarations have different signatures or categories.

## Module binding accessibility

Each module-level binding represented by this document has one of two accessibility classes for source-module lookup:

- **module-private** — usable by same-module lookup but not by cross-module lookup;
- **exported** — usable by same-module lookup and eligible for cross-module lookup from another module.

Accessibility is a source semantic fact. It is not inferred from identifier case, original spelling, physical symbol visibility, linkage, ABI export status, file placement, or build-system metadata unless a source-language rule explicitly establishes such a source relation.

For the represented record, function, marker trait, and constant declarations in `concrete-syntax.md`, absence of the concrete `export` modifier establishes module-private accessibility and presence of that modifier establishes exported accessibility. The concrete modifier changes only this source accessibility fact; it does not establish ABI export, linkage, FFI visibility, runtime publication, runtime storage, or realization behavior.

Marker implementation declarations have no independent binding accessibility because they introduce no module binding. Their resolved propositions participate in the compilation-global relation owned by `traits.md` rather than this lookup namespace.

This revision does not define package-scoped, friend, subtree-restricted, protected, FFI-linkage, or other accessibility classes.

## Same-module lookup

For a source unit assigned to source module `M`, **module-scope lookup** of lexical identifier key `k` consults only `M`'s module declaration namespace.

If that namespace contains the binding keyed by `k`, module-scope lookup resolves to that binding regardless of whether the binding is module-private or exported.

If the namespace contains no binding keyed by `k`, this module-scope lookup does not resolve a binding. This rule does not cause imported modules, future preludes, member scopes, or another namespace to be searched implicitly.

Within represented function bodies, `local-bindings.md` owns when active function-local bindings are consulted before this same-module relation. The consuming source form validates the category of the selected entity; same-module lookup does not skip bindings based on the category desired by that context. Explicit unqualified record-construction targets and unqualified record-pattern heads consume same-module declaration lookup directly under their own owners without introducing local-binding participation. A bare constant value use consumes the local-first relation from `local-bindings.md` and reaches this same-module relation only when no active function-local value binding selects the key; `constants.md` then requires the selected module binding to be a constant. A bare function-value formation under an exact required function-value type uses the same local-first boundary and reaches this same-module relation only when no active local resolves; `function-values.md` then requires the selected module binding to be one exact non-generic function entity. The same same-module function lookup continues to select the existing direct-call target when no active local resolves an unqualified call target.

A bare marker trait reference likewise consumes same-module declaration lookup directly under `traits.md`; function-local generic type-parameter lookup does not participate in that marker-reference domain. The selected binding must then satisfy the marker-trait category requirement under `traits.md`; this lookup does not bypass a wrong-category binding.

## Source-unit module aliases

A represented **module import relation** belongs to exactly one source unit. It associates:

- one lexical identifier key, the **module alias**; and
- exactly one target source-module identity supplied by the source compilation context.

The source unit containing the relation MUST be assigned to a source module distinct from the target source module. The current module is not imported through this relation.

A module alias is static source name-resolution structure only. It is not a Runen program value, runtime module object, initialization operation, side effect, task, capability, or realization choice.

Within one source unit:

- two represented module import relations MUST NOT introduce the same module-alias key;
- a module-alias key MUST NOT equal any module-level binding key in the source unit's own source module.

Consequently, a module alias does not hide or replace a same-module declaration.

Different source units assigned to one source module MAY use different alias keys for the same target module. One source unit MAY also use multiple distinct alias keys for the same target module. Because module aliases are source-unit-local relations, the same alias key in two distinct source units MAY designate different target source modules.

A module alias is available only within the source unit whose module import relation introduces it. Another source unit in the same source module does not acquire that alias merely because the target module or alias exists elsewhere in the module.

The concrete `import A;` form in `concrete-syntax.md` introduces one represented module import relation whose module-alias key is the lexical identifier key of `A`. The source compilation context MUST supply exactly one target source-module identity for that alias key in that source unit. If it does not, that import relation is invalid for the supplied source compilation context.

The concrete import spelling contains no source module name or target locator. An alias spelling therefore cannot be used to infer the target module identity. Additional host or build-system mappings for alias keys not declared by that source unit do not create source module aliases and have no source lookup effect under this document.

This document does not define target-locator spelling, unused-import diagnostics, or a module object's source-level value representation.

## Qualified cross-module lookup

The module import relation imports module identity only. It does not copy the target module's module-level bindings into the importing source module or source-unit alias scope.

A **qualified cross-module lookup** is given:

1. the source unit in which lookup occurs;
2. one module-alias lexical identifier key `a`; and
3. one target-member lexical identifier key `m`.

The lookup succeeds only when all of the following hold:

- the source unit has exactly one represented module alias keyed by `a`;
- that alias names one target source module `T`;
- `T`'s module declaration namespace contains exactly one binding keyed by `m`; and
- that binding is exported.

When those conditions hold, the qualified lookup resolves to that target binding.

An unqualified lookup MUST NOT search imported modules merely because they are aliased in the source unit. This revision defines no selective direct imports, wildcard or glob imports, dot imports, re-exports, implicit preludes, transitive import visibility, or imported-member precedence rules.

The concrete `a::m` form in `concrete-syntax.md` maps exactly to this lookup relation. `::` does not by itself define arbitrary member access, nested module paths, associated-item lookup, or another name-resolution domain. The consuming concrete type, direct-call, record-construction-target, record-pattern-head, marker-reference, implementation-target, or constant-value context validates the category of the resolved binding after this lookup; qualified lookup does not skip an inaccessible or wrong-category binding.

A qualified marker trait reference therefore consumes the same one-hop exported-binding relation as the already represented qualified type/call/construction/pattern/constant contexts. `traits.md` separately requires the selected binding to denote one marker trait entity. A qualified nominal implementation target likewise consumes this relation and is then required by `traits.md` to denote one nominal record type.

A qualified constant value consumes this same one-hop exported-binding relation. `constants.md` separately requires the selected binding to denote one source constant and defines its effect-free owned-value production. A qualified function-value formation consumes the same exported-binding relation and `function-values.md` separately requires one non-generic function entity of the exact required function-value type. A qualified call target likewise remains the existing direct module-function call category because function-local bindings do not participate in this explicitly qualified lookup.

A qualified record-pattern head consumes the same lookup relation as the already represented qualified type/call/construction/constant contexts. The lookup establishes only the exported target module binding; `patterns.md` separately requires the resolved binding to denote one nominal record and applies direct record-field accessibility through `field-access.md` to the fields explicitly opened by the pattern. This document does not create pattern-field lookup or a second visibility relation.

Module aliases themselves are not exported module-level bindings under this revision and therefore do not re-export their target modules or target bindings.

A marker implementation proposition is not a module alias/member and is not imported through this qualified relation. Once an implementation declaration has resolved its operands and established a valid proposition under `traits.md`, that proposition is compilation-global evidence under that owner rather than a name exposed by module lookup.

## Cyclic module-import relations

Source name resolution under this document does not reject a finite cycle of module import relations between distinct source-module identities merely because it is cyclic.

The represented import relation creates only source-unit-local module aliases; it does not copy or re-export bindings. Module declaration namespaces are order-independent. Therefore resolving a qualified lookup does not recursively search through imported modules: it follows one alias to one target module and performs one lookup in that target module's declaration namespace.

This permission concerns source name resolution only. The represented literal-initialized constants from `constants.md` create no declaration dependency edge and therefore add no initialization/evaluation cycle rule. A later constant-expression dependency, static/runtime initialization, linking, package/build dependency, environment-admission, or other canonical owner may impose an independently justified cycle restriction for its own semantics. Physical compilation or build order does not by itself create a source name-resolution restriction.

## Deliberate boundaries

This revision defines module declaration namespaces, binding accessibility, same-module lookup, source-unit module-alias scopes, and qualified cross-module lookup. Represented function-local value-binding scopes and precedence are owned by `local-bindings.md`. The current concrete record/function/marker-trait/constant/import/export/qualification forms are owned by `concrete-syntax.md`.

This document does not define:

- additional local binding classes such as pattern bindings, closure captures, generic parameters, lifetime names, or labels;
- nested or parent/child module hierarchy, module path segments beyond the represented alias/member pair, `self`/`super`-like relations, or a source-visible canonical module name;
- fields, trait member lookup, methods, associated items, extension lookup, supertrait lookup, implementation selection by name, overload resolution, argument-dependent lookup, or member precedence;
- implicit/predeclared names or a standard-library prelude;
- dependency-locator syntax, selective direct imports, wildcard/glob imports, dot imports, re-exports, or transitive import visibility;
- package management, dependency solving, filesystem layout, source discovery, interface serialization, or a module/package orphan policy for marker implementations;
- constant expressions/dependencies beyond the represented self-contained literal initializer, static storage initialization order, or runtime module initialization;
- ABI, linkage, FFI export/import, or physical symbol visibility;
- parser, lossless syntax, HIR, Core MIR lowering, backend, or another implementation representation.

Top-level marker trait and constant bindings use the ordinary declaration lookup represented above. Exact marker implementation/coherence and compilation-global evidence are owned by `traits.md` and introduce no lookup namespace. Exact constant value/initializer/use semantics are owned by `constants.md` and introduce no runtime module object or storage namespace.

Those concerns require their own canonical owners when their first concrete consumers are accepted.
