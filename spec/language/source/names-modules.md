# Source Names and Modules

Status: **provisional normative; incomplete**

This document owns the represented source module identity, module binding, module alias, visibility, and qualified cross-module lookup relations. It consumes lexical identifier keys from [Source lexical foundation](lexical.md) and does not redefine identifier formation or equivalence.

It does not define concrete declaration or import syntax, local lexical scopes, member lookup, overload resolution, package discovery, or an implementation representation.

## Source modules

A **source module** is an opaque source-language organization identity established as part of the source compilation context.

The source compilation context is an input to source-language validation. For the rules in this document it supplies the source-module identities that participate in the compilation, the assignment of directly supplied source units to those module identities, and the target module identity named by any represented module-import relation. It is not Runen program state.

Source-module identity is not derived from source-unit bytes, a lexical identifier spelling, a filesystem path or file name, a directory, a package coordinate, source-unit presentation order, or another physical storage convention.

Every directly supplied Runen source unit in one source compilation MUST be assigned to exactly one source module. A source module MAY have more than one directly supplied source unit.

Source-unit presentation order, file-system order, and build-system processing order are not semantic inputs to module-level name resolution.

This document does not define how a build system discovers source units, maps files or directories to modules, resolves dependency coordinates, loads module interfaces, caches compilation results, or serializes module identity or exported binding information.

## Module declaration namespace

Every source module has one **module declaration namespace** keyed by lexical identifier keys.

When a later source-language rule establishes that a declaration introduces a module-level name, that declaration contributes one binding with its lexical identifier key to this namespace unless the declaration's canonical owner explicitly places that name only in a member-specific or otherwise distinct lookup domain.

Two distinct module-level bindings in the same source module MUST NOT have the same lexical identifier key.

Module-level bindings are available to module-level name resolution independently of source-unit presentation order and textual declaration order. Reordering directly supplied source units or reordering module-level declarations MUST NOT change which module-level binding a given lexical identifier key denotes.

A module-level binding is identified by its source module and binding identity, not by the original source spelling of its identifier. The namespace key is the lexical identifier key defined by `lexical.md`.

For this foundation, name resolution first identifies one binding/entity. A later source-language rule then determines whether that resolved entity category is valid for the applicable type, value, declaration, or other semantic context. This document does not define separate module-level type and value namespaces or context-dependent searches across such namespaces.

This section does not define fields, methods, associated items, local bindings, parameters, generic parameters, pattern bindings, lifetime names, labels, macros, or overload sets. A later rule that permits one source name to denote an overload set or another multi-entity binding MUST define that binding relation explicitly; duplicate module-level binding keys do not become an overload set merely because the declarations have different signatures or categories.

## Module binding accessibility

Each module-level binding represented by this document has one of two accessibility classes for source-module lookup:

- **module-private** — usable by same-module lookup but not by cross-module lookup;
- **exported** — usable by same-module lookup and eligible for cross-module lookup from another module.

Accessibility is a source semantic fact. It is not inferred from identifier case, original spelling, physical symbol visibility, linkage, ABI export status, file placement, or build-system metadata unless a later source-language rule explicitly establishes such a source relation.

This revision does not define package-scoped, friend, subtree-restricted, protected, FFI-linkage, or other accessibility classes.

## Same-module lookup

For a source unit assigned to source module `M`, **module-scope lookup** of lexical identifier key `k` consults only `M`'s module declaration namespace.

If that namespace contains the binding keyed by `k`, module-scope lookup resolves to that binding regardless of whether the binding is module-private or exported.

If the namespace contains no binding keyed by `k`, this module-scope lookup does not resolve a binding. This rule does not cause imported modules, future preludes, enclosing local scopes, member scopes, or another namespace to be searched implicitly.

A later source construct with a more-local binding model may define when its own lookup is attempted before module-scope lookup. This document does not define a universal nested-scope or shadowing order.

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

Different source units assigned to one source module MAY use different alias keys for the same target module. One source unit MAY also use multiple distinct alias keys for the same target module.

A module alias is available only within the source unit whose module import relation introduces it. Another source unit in the same source module does not acquire that alias merely because the target module or alias exists elsewhere in the module.

This document does not define concrete import syntax, alias syntax, target-locator spelling, import placement, unused-import diagnostics, or a module object's source-level value representation.

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

Concrete grammar may later choose a path separator or other spelling that lowers to this relation. That spelling does not alter the binding relation defined here.

Module aliases themselves are not exported module-level bindings under this revision and therefore do not re-export their target modules or target bindings.

## Cyclic module-import relations

Source name resolution under this document does not reject a finite cycle of module import relations between distinct source-module identities merely because it is cyclic.

The represented import relation creates only source-unit-local module aliases; it does not copy or re-export bindings. Module declaration namespaces are order-independent. Therefore resolving a qualified lookup does not recursively search through imported modules: it follows one alias to one target module and performs one lookup in that target module's declaration namespace.

This permission concerns source name resolution only. A later const/static initialization, runtime initialization, linking, package/build dependency, environment-admission, or other canonical owner may impose an independently justified cycle restriction for its own semantics. Physical compilation or build order does not by itself create a source name-resolution restriction.

## Deliberate boundaries

This revision defines only module declaration namespaces and source-unit module-alias scopes. It does not define:

- local or block scopes, local shadowing, parameters, generic parameters, pattern bindings, closure captures, lifetime names, or labels;
- nested or parent/child module hierarchy, module path segments, `self`/`super`-like relations, or a source-visible canonical module name;
- fields, methods, associated items, extension lookup, trait lookup, overload resolution, argument-dependent lookup, or member precedence;
- implicit/predeclared names or a standard-library prelude;
- concrete declarations or their grammar;
- module/import keywords, path punctuation, comments, or literals;
- package management, dependency solving, filesystem layout, source discovery, or interface serialization;
- const/static initialization order or runtime module initialization;
- ABI, linkage, FFI export/import, or physical symbol visibility;
- parser, lossless syntax, HIR, Core MIR lowering, backend, or another implementation representation.

Those concerns require their own canonical owners when their first concrete consumers are accepted.
