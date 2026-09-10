from pathlib import Path

p = Path('crates/runen-hir/src/build.rs')
s = p.read_text()

def once(old, new, label):
    global s
    n = s.count(old)
    assert n == 1, f'{label}: {n} matches'
    s = s.replace(old, new, 1)

# Bundle stable header-resolution dependencies rather than suppressing a new wide signature.
old = '''fn resolve_function_headers(
    syntax: &[FunctionSyntax],
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    records: &[Record],
    marker_traits: &[MarkerTrait],
    function_types: &RefCell<Vec<FunctionType>>,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FunctionHeader> {
'''
new = '''struct HeaderResolutionContext<'a> {
    modules: &'a BTreeMap<ModuleId, ModuleBuild>,
    imports: &'a [UnitImports],
    records: &'a [Record],
    marker_traits: &'a [MarkerTrait],
    function_types: &'a RefCell<Vec<FunctionType>>,
}

fn resolve_function_headers(
    syntax: &[FunctionSyntax],
    context: &HeaderResolutionContext<'_>,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FunctionHeader> {
    let HeaderResolutionContext {
        modules,
        imports,
        records,
        marker_traits,
        function_types,
    } = context;
'''
once(old, new, 'header signature')

old = '''    let headers = resolve_function_headers(
        &function_syntax,
        &modules,
        &imports,
        &records,
        &marker_traits,
        &function_types,
        &mut next_binding,
        &mut diagnostics,
    );
'''
new = '''    let header_context = HeaderResolutionContext {
        modules: &modules,
        imports: &imports,
        records: &records,
        marker_traits: &marker_traits,
        function_types: &function_types,
    };
    let headers = resolve_function_headers(
        &function_syntax,
        &header_context,
        &mut next_binding,
        &mut diagnostics,
    );
'''
once(old, new, 'header call')

# Name-only resolution owns module/import/type-parameter lookup. Full resolution layers
# concrete function-type structure over it. This removes all new too-many-argument expects.
old_start = s.index('fn resolve_type(\n')
full_start = s.index('fn type_ref_is_function(', old_start)
base = s[old_start:full_start]
old_sig = '''fn resolve_type(
    module: ModuleId,
    unit: usize,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    type_parameters: &[TypeParameter],
    allow_type_parameters: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
'''
assert old_sig in base
new_sig = '''struct TypeNameResolutionContext<'a> {
    module: ModuleId,
    unit: usize,
    modules: &'a BTreeMap<ModuleId, ModuleBuild>,
    imports: &'a [UnitImports],
    type_parameters: &'a [TypeParameter],
}

fn resolve_type(
    node: &SyntaxNode,
    context: &TypeNameResolutionContext<'_>,
    allow_type_parameters: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let TypeNameResolutionContext {
        module,
        unit,
        modules,
        imports,
        type_parameters,
    } = context;
'''
base = base.replace(old_sig, new_sig, 1)
# Dereference copied module/unit where APIs require values.
base = base.replace('resolve_qualified_entity(unit, &qualified, modules, imports, diagnostics)?',
                    'resolve_qualified_entity(*unit, &qualified, modules, imports, diagnostics)?')
base = base.replace('location(unit, &qualified)', 'location(*unit, &qualified)')
base = base.replace('            unit,\n            range:', '            unit: *unit,\n            range:')
base = base.replace('.get(&module)', '.get(module)')
s = s[:old_start] + base + s[full_start:]

# Replace the full resolver block up to qualified entity lookup with context-based forms.
full_start = s.index('fn type_ref_is_function(')
qualified_start = s.index('fn resolve_qualified_entity(', full_start)
old_full = s[full_start:qualified_start]
# Preserve type_ref_is_function helper, replace everything after it.
helper_end = old_full.index('\n}\n', old_full.index('fn type_ref_is_function(')) + 3
helper = old_full[:helper_end]
new_full = helper + r'''
struct FullTypeResolutionContext<'a> {
    names: TypeNameResolutionContext<'a>,
    records: &'a [Record],
    function_types: &'a RefCell<Vec<FunctionType>>,
}

fn resolve_full_type(
    node: &SyntaxNode,
    context: &FullTypeResolutionContext<'_>,
    allow_type_parameters: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if type_ref_is_function(node) {
        return resolve_function_type(node, context, diagnostics);
    }
    resolve_type(node, &context.names, allow_type_parameters, diagnostics)
}

fn resolve_function_type(
    node: &SyntaxNode,
    context: &FullTypeResolutionContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    debug_assert!(type_ref_is_function(node));
    let mut components = node
        .children()
        .filter(|child| child.kind() == SyntaxKind::TypeRef)
        .collect::<Vec<_>>();
    let has_result = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::Arrow);
    let result_node = has_result.then(|| {
        components
            .pop()
            .expect("syntax-clean result-bearing function type has one result TypeRef")
    });

    let mut parameters = Vec::with_capacity(components.len());
    for parameter_node in components {
        let parameter = resolve_full_type(&parameter_node, context, false, diagnostics)?;
        if matches!(parameter, Type::RawPointer(_)) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::RawPointerParameter,
                location: location(context.names.unit, &parameter_node),
            });
            return None;
        }
        if !validate_safe_reference_referent(
            parameter,
            context.records,
            location(context.names.unit, &parameter_node),
            diagnostics,
        ) {
            return None;
        }
        parameters.push(parameter);
    }

    let result = if let Some(result_node) = result_node {
        let result = resolve_full_type(&result_node, context, false, diagnostics)?;
        if matches!(result, Type::RawPointer(_)) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::RawPointerResult,
                location: location(context.names.unit, &result_node),
            });
            return None;
        }
        if matches!(
            result,
            Type::SafeReference {
                permission: ReferencePermission::ExclusiveReplace,
                ..
            }
        ) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ReplacementReferenceResult,
                location: location(context.names.unit, &result_node),
            });
            return None;
        }
        if !validate_safe_reference_referent(
            result,
            context.records,
            location(context.names.unit, &result_node),
            diagnostics,
        ) {
            return None;
        }
        Some(result)
    } else {
        None
    };

    let safe_reference_result_contract = if let Some(Type::SafeReference {
        referent,
        permission: ReferencePermission::Shared,
    }) = result
    {
        let shared = Type::SafeReference {
            referent,
            permission: ReferencePermission::Shared,
        };
        let mut matching_shared = parameters
            .iter()
            .enumerate()
            .filter(|(_, parameter)| **parameter == shared);
        match (matching_shared.next(), matching_shared.next()) {
            (Some((origin, _)), None) => SafeReferenceResultContract::SharedIdentity { origin },
            (Some(_), Some(_)) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::AmbiguousSharedReferenceResultOrigin,
                    location: location(context.names.unit, node),
                });
                return None;
            }
            (None, _) => {
                let replacement = Type::SafeReference {
                    referent,
                    permission: ReferencePermission::ExclusiveReplace,
                };
                let mut matching_replacement = parameters
                    .iter()
                    .enumerate()
                    .filter(|(_, parameter)| **parameter == replacement);
                match (matching_replacement.next(), matching_replacement.next()) {
                    (Some((origin, _)), None) => {
                        SafeReferenceResultContract::SharedDirectChild { origin }
                    }
                    (None, _) => {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::MissingSharedReferenceResultOrigin,
                            location: location(context.names.unit, node),
                        });
                        return None;
                    }
                    (Some(_), Some(_)) => {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::AmbiguousSharedReferenceResultOrigin,
                            location: location(context.names.unit, node),
                        });
                        return None;
                    }
                }
            }
        }
    } else {
        SafeReferenceResultContract::None
    };

    Some(Type::Function(intern_function_type(
        context.function_types,
        FunctionType {
            parameters,
            result,
            safe_reference_result_contract,
        },
    )))
}

fn intern_function_type(
    function_types: &RefCell<Vec<FunctionType>>,
    candidate: FunctionType,
) -> FunctionTypeId {
    if let Some(index) = function_types
        .borrow()
        .iter()
        .position(|existing| existing == &candidate)
    {
        return FunctionTypeId(index);
    }
    let mut function_types = function_types.borrow_mut();
    let id = FunctionTypeId(function_types.len());
    function_types.push(candidate);
    id
}

'''
s = s[:full_start] + new_full + s[qualified_start:]

# Record fields need only name-resolution context.
old = '''            if let Some(ty) = resolve_type(
                record.module,
                record.unit,
                &type_node,
                modules,
                imports,
                &[],
                true,
                diagnostics,
            ) {
'''
new = '''            let type_context = TypeNameResolutionContext {
                module: record.module,
                unit: record.unit,
                modules,
                imports,
                type_parameters: &[],
            };
            if let Some(ty) = resolve_type(&type_node, &type_context, true, diagnostics) {
'''
once(old, new, 'record type call')

# Within each function header, construct one full context after type parameters resolve.
anchor = '''            .unwrap_or_default();

        let parameter_list = direct_child(&function.node, SyntaxKind::ParameterList);
'''
replacement = '''            .unwrap_or_default();

        let type_context = FullTypeResolutionContext {
            names: TypeNameResolutionContext {
                module: function.module,
                unit: function.unit,
                modules,
                imports,
                type_parameters: &type_parameters,
            },
            records,
            function_types,
        };

        let parameter_list = direct_child(&function.node, SyntaxKind::ParameterList);
'''
once(anchor, replacement, 'header type context insertion')

old = '''            if let Some(ty) = resolve_full_type(
                function.module,
                function.unit,
                &type_node,
                modules,
                imports,
                records,
                function_types,
                &type_parameters,
                true,
                diagnostics,
            ) {
'''
new = '''            if let Some(ty) = resolve_full_type(&type_node, &type_context, true, diagnostics) {
'''
once(old, new, 'header parameter full type')

old = '''                let ty = resolve_full_type(
                    function.module,
                    function.unit,
                    &type_node,
                    modules,
                    imports,
                    records,
                    function_types,
                    &type_parameters,
                    true,
                    diagnostics,
                )?;
'''
new = '''                let ty = resolve_full_type(&type_node, &type_context, true, diagnostics)?;
'''
once(old, new, 'header result full type')

# Local annotations get a full context built from the already-bound body environment.
needle = '''    let declared = resolve_full_type(
        header.module,
        header.unit,
        &type_node,
        context.modules,
        context.imports,
        context.records,
        context.function_types,
        &header.type_parameters,
        true,
        diagnostics,
    );
'''
replacement = '''    let type_context = FullTypeResolutionContext {
        names: TypeNameResolutionContext {
            module: header.module,
            unit: header.unit,
            modules: context.modules,
            imports: context.imports,
            type_parameters: &header.type_parameters,
        },
        records: context.records,
        function_types: context.function_types,
    };
    let declared = resolve_full_type(&type_node, &type_context, true, diagnostics);
'''
once(needle, replacement, 'local full type')

assert '#[expect(\n    clippy::too_many_arguments,\n    reason = "function-type resolution' not in s
p.write_text(s)
print('replaced new wide HIR helpers with explicit borrowed resolution contexts')
