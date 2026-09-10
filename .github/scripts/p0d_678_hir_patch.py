from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    assert count == 1, f"{label}: expected one match, found {count}"
    return text.replace(old, new, 1)


lib_path = Path("crates/runen-hir/src/lib.rs")
lib = lib_path.read_text()

lib = replace_once(
    lib,
    """/// Opaque per-compilation function handle.\n#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct FunctionId(pub(crate) usize);\n\n""",
    """/// Opaque per-compilation function handle.\n#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct FunctionId(pub(crate) usize);\n\n/// Opaque per-compilation canonical structural function-type handle.\n#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct FunctionTypeId(pub(crate) usize);\n\n""",
    "FunctionTypeId",
)

lib = replace_once(
    lib,
    """pub enum SafeReferenceResultContract {\n    None,\n    SharedIdentity { origin: usize },\n    SharedDirectChild { origin: usize },\n}\n\n""",
    """pub enum SafeReferenceResultContract {\n    None,\n    SharedIdentity { origin: usize },\n    SharedDirectChild { origin: usize },\n}\n\n/// Canonical structural source function-value type retained for one compilation.\n#[derive(Debug, Clone, PartialEq, Eq)]\npub struct FunctionType {\n    pub parameters: Vec<Type>,\n    pub result: Option<Type>,\n    pub safe_reference_result_contract: SafeReferenceResultContract,\n}\n\n""",
    "FunctionType",
)

lib = replace_once(
    lib,
    """    RawPointer(RawPointerPointee),\n}\n""",
    """    RawPointer(RawPointerPointee),\n    /// Canonical structural captureless function-value type.\n    Function(FunctionTypeId),\n}\n""",
    "Type::Function",
)

lib = replace_once(
    lib,
    """        Type::Intrinsic(_) | Type::RawPointer(_) => true,\n        Type::Parameter(_) => false,\n""",
    """        Type::Intrinsic(_) | Type::RawPointer(_) | Type::Function(_) => true,\n        Type::Parameter(_) => false,\n""",
    "function duplicability",
)

lib = replace_once(
    lib,
    """/// Resolved producer for one typed HIR value.\n#[derive(Debug, Clone, PartialEq, Eq)]\npub enum ValueKind {\n""",
    """/// Resolved source call-target classification shared by value and statement calls.\n#[derive(Debug, Clone, PartialEq, Eq)]\npub enum CallTarget {\n    Direct {\n        function: FunctionId,\n        type_arguments: Vec<Type>,\n    },\n    Indirect {\n        binding: BindingId,\n        function_type: FunctionTypeId,\n    },\n}\n\n/// Resolved producer for one typed HIR value.\n#[derive(Debug, Clone, PartialEq, Eq)]\npub enum ValueKind {\n""",
    "CallTarget",
)

lib = replace_once(
    lib,
    """    DirectCall {\n        function: FunctionId,\n        type_arguments: Vec<Type>,\n        arguments: Vec<Value>,\n    },\n""",
    """    FunctionValue {\n        function: FunctionId,\n    },\n    Call {\n        target: CallTarget,\n        arguments: Vec<Value>,\n    },\n""",
    "ValueKind call/value",
)

lib = replace_once(
    lib,
    """    Call {\n        function: FunctionId,\n        type_arguments: Vec<Type>,\n        arguments: Vec<Value>,\n        location: SourceLocation,\n    },\n""",
    """    Call {\n        target: CallTarget,\n        arguments: Vec<Value>,\n        location: SourceLocation,\n    },\n""",
    "Statement::Call",
)

lib = replace_once(
    lib,
    """pub struct TypedCompilation {\n    pub modules: Vec<Module>,\n    pub records: Vec<Record>,\n    pub functions: Vec<Function>,\n    pub marker_traits: Vec<MarkerTrait>,\n""",
    """pub struct TypedCompilation {\n    pub modules: Vec<Module>,\n    pub records: Vec<Record>,\n    pub functions: Vec<Function>,\n    pub function_types: Vec<FunctionType>,\n    pub marker_traits: Vec<MarkerTrait>,\n""",
    "TypedCompilation function types",
)

lib = replace_once(
    lib,
    """    pub fn function(&self, id: FunctionId) -> &Function {\n        &self.functions[id.0]\n    }\n\n""",
    """    pub fn function(&self, id: FunctionId) -> &Function {\n        &self.functions[id.0]\n    }\n\n    #[must_use]\n    pub fn function_type(&self, id: FunctionTypeId) -> &FunctionType {\n        &self.function_types[id.0]\n    }\n\n""",
    "function_type accessor",
)

lib = replace_once(
    lib,
    """    RawPointerField,\n    RawPointerParameter,\n""",
    """    RawPointerField,\n    FunctionTypeField,\n    RawPointerParameter,\n""",
    "FunctionTypeField diagnostic",
)
lib = replace_once(
    lib,
    """    ExpectedFunction,\n    ArgumentCount {\n""",
    """    ExpectedFunction,\n    GenericFunctionValue,\n    ArgumentCount {\n""",
    "GenericFunctionValue diagnostic",
)

lib_path.write_text(lib)

build_path = Path("crates/runen-hir/src/build.rs")
build = build_path.read_text()

build = replace_once(
    build,
    """use std::{\n    cmp::Ordering,\n    collections::{BTreeMap, BTreeSet},\n};\n""",
    """use std::{\n    cell::RefCell,\n    cmp::Ordering,\n    collections::{BTreeMap, BTreeSet},\n};\n""",
    "RefCell import",
)

build = replace_once(
    build,
    """    Accessibility, AssignmentMutability, BinaryFloatSign, BinaryFloatValue, BindingId, Block, Body,\n    BooleanEqualityRelation, CleanupPath, Diagnostic, DiagnosticKind, Duplicability, Field,\n    FieldReceiverTransientCleanup, FieldValueReceiver, Function, FunctionId, IntrinsicType,\n""",
    """    Accessibility, AssignmentMutability, BinaryFloatSign, BinaryFloatValue, BindingId, Block, Body,\n    BooleanEqualityRelation, CallTarget, CleanupPath, Diagnostic, DiagnosticKind, Duplicability, Field,\n    FieldReceiverTransientCleanup, FieldValueReceiver, Function, FunctionId, FunctionType,\n    FunctionTypeId, IntrinsicType,\n""",
    "HIR imports",
)

build = replace_once(
    build,
    """    safe_reference_result_contract: SafeReferenceResultContract,\n    body: SyntaxNode,\n""",
    """    safe_reference_result_contract: SafeReferenceResultContract,\n    function_type: Option<FunctionTypeId>,\n    body: SyntaxNode,\n""",
    "FunctionHeader function type",
)

build = replace_once(
    build,
    """    headers: &'a [FunctionHeader],\n    marker_implementations: &'a MarkerImplementationRelation,\n""",
    """    headers: &'a [FunctionHeader],\n    function_types: &'a RefCell<Vec<FunctionType>>,\n    marker_implementations: &'a MarkerImplementationRelation,\n""",
    "BodyResolutionContext function types",
)

build = replace_once(
    build,
    """    let constants = resolve_constants(&constant_syntax, &mut diagnostics);\n    let records = resolve_records(&record_syntax, &modules, &imports, &mut diagnostics);\n""",
    """    let constants = resolve_constants(&constant_syntax, &mut diagnostics);\n    let function_types = RefCell::new(Vec::new());\n    let records = resolve_records(&record_syntax, &modules, &imports, &mut diagnostics);\n""",
    "build interner creation",
)

build = replace_once(
    build,
    """        &records,\n        &marker_traits,\n        &mut next_binding,\n""",
    """        &records,\n        &marker_traits,\n        &function_types,\n        &mut next_binding,\n""",
    "resolve_function_headers interner",
)

build = replace_once(
    build,
    """            &records,\n            &headers,\n            &marker_implementation_relation,\n""",
    """            &records,\n            &headers,\n            &function_types,\n            &marker_implementation_relation,\n""",
    "validate_body interner",
)

build = replace_once(
    build,
    """    Ok(TypedCompilation {\n        modules,\n        records,\n        functions,\n        marker_traits,\n""",
    """    Ok(TypedCompilation {\n        modules,\n        records,\n        functions,\n        function_types: function_types.into_inner(),\n        marker_traits,\n""",
    "TypedCompilation interner output",
)

# Function-valued record fields are semantically excluded even though TypeRef syntax is general.
build = replace_once(
    build,
    """            let type_node = direct_child(&field_node, SyntaxKind::TypeRef);\n            if type_ref_reference_permission(&type_node).is_some() {\n""",
    """            let type_node = direct_child(&field_node, SyntaxKind::TypeRef);\n            if type_ref_is_function(&type_node) {\n                diagnostics.push(Diagnostic {\n                    kind: DiagnosticKind::FunctionTypeField,\n                    location: location(record.unit, &type_node),\n                });\n                continue;\n            }\n            if type_ref_reference_permission(&type_node).is_some() {\n""",
    "record function field exclusion",
)

# Existing non-function resolver gains an explicit abstract-parameter admission bit.
build = replace_once(
    build,
    """fn resolve_type(\n    module: ModuleId,\n    unit: usize,\n    node: &SyntaxNode,\n    modules: &BTreeMap<ModuleId, ModuleBuild>,\n    imports: &[UnitImports],\n    type_parameters: &[TypeParameter],\n    diagnostics: &mut Vec<Diagnostic>,\n) -> Option<Type> {\n""",
    """fn resolve_type(\n    module: ModuleId,\n    unit: usize,\n    node: &SyntaxNode,\n    modules: &BTreeMap<ModuleId, ModuleBuild>,\n    imports: &[UnitImports],\n    type_parameters: &[TypeParameter],\n    allow_type_parameters: bool,\n    diagnostics: &mut Vec<Diagnostic>,\n) -> Option<Type> {\n""",
    "resolve_type admission",
)

build = replace_once(
    build,
    """                if reference_permission.is_some() || raw {\n""",
    """                if reference_permission.is_some() || raw || !allow_type_parameters {\n""",
    "abstract function-type exclusion",
)

# All pre-existing non-function calls explicitly retain their old abstract admission.
build = build.replace(
    """                &[],\n                diagnostics,\n            )""",
    """                &[],\n                true,\n                diagnostics,\n            )""",
)

# Header resolution now uses the full concrete type resolver and canonical interner.
build = replace_once(
    build,
    """    records: &[Record],\n    marker_traits: &[MarkerTrait],\n    next_binding: &mut usize,\n""",
    """    records: &[Record],\n    marker_traits: &[MarkerTrait],\n    function_types: &RefCell<Vec<FunctionType>>,\n    next_binding: &mut usize,\n""",
    "header interner parameter",
)

old_header_call = """            if let Some(ty) = resolve_type(\n                function.module,\n                function.unit,\n                &type_node,\n                modules,\n                imports,\n                &type_parameters,\n                diagnostics,\n            ) {\n"""
new_header_call = """            if let Some(ty) = resolve_full_type(\n                function.module,\n                function.unit,\n                &type_node,\n                modules,\n                imports,\n                records,\n                function_types,\n                &type_parameters,\n                true,\n                diagnostics,\n            ) {\n"""
assert build.count(old_header_call) == 1
build = build.replace(old_header_call, new_header_call, 1)

old_result_call = """                let ty = resolve_type(\n                    function.module,\n                    function.unit,\n                    &type_node,\n                    modules,\n                    imports,\n                    &type_parameters,\n                    diagnostics,\n                )?;\n"""
new_result_call = """                let ty = resolve_full_type(\n                    function.module,\n                    function.unit,\n                    &type_node,\n                    modules,\n                    imports,\n                    records,\n                    function_types,\n                    &type_parameters,\n                    true,\n                    diagnostics,\n                )?;\n"""
assert build.count(old_result_call) == 1
build = build.replace(old_result_call, new_result_call, 1)

# Exported signature checks recurse through function-type components.
build = build.replace(
    """                    records,\n                    function.unit,\n                    &type_node,\n                    diagnostics,\n                );""",
    """                    records,\n                    function_types,\n                    function.unit,\n                    &type_node,\n                    diagnostics,\n                );""",
)

build = replace_once(
    build,
    """        let body = direct_child(&function.node, SyntaxKind::Body);\n        headers.push(FunctionHeader {\n""",
    """        let function_type = if type_parameters.is_empty() {\n            Some(intern_function_type(\n                function_types,\n                FunctionType {\n                    parameters: parameters.iter().map(|parameter| parameter.ty).collect(),\n                    result,\n                    safe_reference_result_contract,\n                },\n            ))\n        } else {\n            None\n        };\n        let body = direct_child(&function.node, SyntaxKind::Body);\n        headers.push(FunctionHeader {\n""",
    "header function type derivation",
)

build = replace_once(
    build,
    """            result,\n            safe_reference_result_contract,\n            body,\n""",
    """            result,\n            safe_reference_result_contract,\n            function_type,\n            body,\n""",
    "header function type store",
)

old_export_fn = """fn validate_exported_signature_type(\n    accessibility: Accessibility,\n    ty: Type,\n    records: &[Record],\n    unit: usize,\n    type_node: &SyntaxNode,\n    diagnostics: &mut Vec<Diagnostic>,\n) {\n    if accessibility != Accessibility::Exported {\n        return;\n    }\n    let record = match ty {\n        Type::Record(record)\n        | Type::SafeReference {\n            referent: ReferenceReferent::Record(record),\n            ..\n        }\n        | Type::RawPointer(RawPointerPointee::Record(record)) => record,\n        Type::Intrinsic(_)\n        | Type::Parameter(_)\n        | Type::SafeReference {\n            referent: ReferenceReferent::Intrinsic(_),\n            ..\n        }\n        | Type::RawPointer(RawPointerPointee::Intrinsic(_)) => return,\n    };\n    if records[record.0].accessibility == Accessibility::ModulePrivate {\n        diagnostics.push(Diagnostic {\n            kind: DiagnosticKind::PrivateTypeInExportedSignature,\n            location: location(unit, type_node),\n        });\n    }\n}\n\n"""
new_export_fn = """fn validate_exported_signature_type(\n    accessibility: Accessibility,\n    ty: Type,\n    records: &[Record],\n    function_types: &RefCell<Vec<FunctionType>>,\n    unit: usize,\n    type_node: &SyntaxNode,\n    diagnostics: &mut Vec<Diagnostic>,\n) {\n    if accessibility != Accessibility::Exported {\n        return;\n    }\n    if exported_type_exposes_private_record(ty, records, function_types) {\n        diagnostics.push(Diagnostic {\n            kind: DiagnosticKind::PrivateTypeInExportedSignature,\n            location: location(unit, type_node),\n        });\n    }\n}\n\nfn exported_type_exposes_private_record(\n    ty: Type,\n    records: &[Record],\n    function_types: &RefCell<Vec<FunctionType>>,\n) -> bool {\n    match ty {\n        Type::Record(record)\n        | Type::SafeReference {\n            referent: ReferenceReferent::Record(record),\n            ..\n        }\n        | Type::RawPointer(RawPointerPointee::Record(record)) => {\n            records[record.0].accessibility == Accessibility::ModulePrivate\n        }\n        Type::Function(function_type) => {\n            let function_type = function_types.borrow()[function_type.0].clone();\n            function_type.parameters.into_iter().any(|parameter| {\n                exported_type_exposes_private_record(parameter, records, function_types)\n            }) || function_type.result.is_some_and(|result| {\n                exported_type_exposes_private_record(result, records, function_types)\n            })\n        }\n        Type::Intrinsic(_)\n        | Type::Parameter(_)\n        | Type::SafeReference {\n            referent: ReferenceReferent::Intrinsic(_),\n            ..\n        }\n        | Type::RawPointer(RawPointerPointee::Intrinsic(_)) => false,\n    }\n}\n\n"""
build = replace_once(build, old_export_fn, new_export_fn, "recursive exported callable accessibility")

build = replace_once(
    build,
    """        Type::Intrinsic(_) | Type::Parameter(_) => false,\n""",
    """        Type::Intrinsic(_) | Type::Parameter(_) | Type::Function(_) => false,\n""",
    "function type is scalar leaf for pointer scan",
)

build = replace_once(
    build,
    """        Type::Parameter(_) | Type::SafeReference { .. } | Type::RawPointer(_) => false,\n""",
    """        Type::Parameter(_)\n        | Type::SafeReference { .. }\n        | Type::RawPointer(_)\n        | Type::Function(_) => false,\n""",
    "raw pointee function exclusion",
)

build = replace_once(
    build,
    """        Type::Parameter(_) | Type::SafeReference { .. } | Type::RawPointer(_) => false,\n    });\n""",
    """        Type::Parameter(_) | Type::SafeReference { .. } | Type::RawPointer(_) => false,\n        Type::Function(_) => true,\n    });\n""",
    "record duplicability exhaustiveness",
)

# Full function-type resolution is a thin retained-handle layer over existing source type rules.
insert_before = """fn resolve_qualified_entity(\n"""
full_type_helpers = r'''fn type_ref_is_function(node: &SyntaxNode) -> bool {
    node.children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::KwFn)
}

#[expect(
    clippy::too_many_arguments,
    reason = "function-type resolution reuses the existing explicit source-resolution inputs"
)]
fn resolve_full_type(
    module: ModuleId,
    unit: usize,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    records: &[Record],
    function_types: &RefCell<Vec<FunctionType>>,
    type_parameters: &[TypeParameter],
    allow_type_parameters: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if type_ref_is_function(node) {
        return resolve_function_type(
            module,
            unit,
            node,
            modules,
            imports,
            records,
            function_types,
            type_parameters,
            diagnostics,
        );
    }
    resolve_type(
        module,
        unit,
        node,
        modules,
        imports,
        type_parameters,
        allow_type_parameters,
        diagnostics,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "function-type resolution reuses the existing explicit source-resolution inputs"
)]
fn resolve_function_type(
    module: ModuleId,
    unit: usize,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    records: &[Record],
    function_types: &RefCell<Vec<FunctionType>>,
    type_parameters: &[TypeParameter],
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
        let parameter = resolve_full_type(
            module,
            unit,
            &parameter_node,
            modules,
            imports,
            records,
            function_types,
            type_parameters,
            false,
            diagnostics,
        )?;
        if matches!(parameter, Type::RawPointer(_)) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::RawPointerParameter,
                location: location(unit, &parameter_node),
            });
            return None;
        }
        if !validate_safe_reference_referent(
            parameter,
            records,
            location(unit, &parameter_node),
            diagnostics,
        ) {
            return None;
        }
        parameters.push(parameter);
    }

    let result = if let Some(result_node) = result_node {
        let result = resolve_full_type(
            module,
            unit,
            &result_node,
            modules,
            imports,
            records,
            function_types,
            type_parameters,
            false,
            diagnostics,
        )?;
        if matches!(result, Type::RawPointer(_)) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::RawPointerResult,
                location: location(unit, &result_node),
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
                location: location(unit, &result_node),
            });
            return None;
        }
        if !validate_safe_reference_referent(
            result,
            records,
            location(unit, &result_node),
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
                    location: location(unit, node),
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
                            location: location(unit, node),
                        });
                        return None;
                    }
                    (Some(_), Some(_)) => {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::AmbiguousSharedReferenceResultOrigin,
                            location: location(unit, node),
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
        function_types,
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
assert build.count(insert_before) == 1
build = build.replace(insert_before, full_type_helpers + insert_before, 1)

# Local annotations use the same canonical concrete function-type resolver.
old_local = """    let declared = resolve_type(\n        header.module,\n        header.unit,\n        &type_node,\n        context.modules,\n        context.imports,\n        &header.type_parameters,\n        diagnostics,\n    )?;\n"""
new_local = """    let declared = resolve_full_type(\n        header.module,\n        header.unit,\n        &type_node,\n        context.modules,\n        context.imports,\n        context.records,\n        context.function_types,\n        &header.type_parameters,\n        true,\n        diagnostics,\n    )?;\n"""
build = replace_once(build, old_local, new_local, "local full type resolution")

# Body construction receives the canonical interner.
build = replace_once(
    build,
    """    records: &[Record],\n    headers: &[FunctionHeader],\n    marker_implementations: &MarkerImplementationRelation,\n""",
    """    records: &[Record],\n    headers: &[FunctionHeader],\n    function_types: &RefCell<Vec<FunctionType>>,\n    marker_implementations: &MarkerImplementationRelation,\n""",
    "validate_body signature",
)
build = replace_once(
    build,
    """        headers,\n        marker_implementations,\n        safe_reference_result_origin_authority,\n""",
    """        headers,\n        function_types,\n        marker_implementations,\n        safe_reference_result_origin_authority,\n""",
    "BodyResolutionContext construction",
)

# Non-generic declaration signatures expose one canonical value type.
function_value_helpers = r'''fn validate_function_value(
    function: FunctionId,
    required: Type,
    value_location: SourceLocation,
    context: &BodyResolutionContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let target = &context.headers[function.0];
    let Some(function_type) = target.function_type else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::GenericFunctionValue,
            location: value_location,
        });
        return None;
    };
    let found = Type::Function(function_type);
    if found != required {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::TypeMismatch {
                expected: required,
                found,
            },
            location: value_location,
        });
        return None;
    }
    Some(ProducedValue::ordinary(Value {
        ty: found,
        kind: ValueKind::FunctionValue { function },
        location: value_location,
    }))
}

'''
marker = """fn validate_identifier_use(\n"""
assert build.count(marker) == 1
build = build.replace(marker, function_value_helpers + marker, 1)

old_identifier = r'''fn validate_identifier_use(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    if node
        .children()
        .any(|child| child.kind() == SyntaxKind::QualifiedModuleMember)
    {
        let constant = resolve_constant_reference(header, node, context, diagnostics)?;
        return validate_constant_value(constant, required, value_location, diagnostics);
    }

    let token = direct_token(node, SyntaxKind::Ident);
    let name = key(&token);
    let Some(binding) = state.bindings.get(&name).cloned() else {
        let constant = resolve_constant_reference(header, node, context, diagnostics)?;
        return validate_constant_value(constant, required, value_location, diagnostics);
    };
    if binding.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnavailableBinding,
            location: value_location,
        });
        return None;
    }
    if binding.ty != required {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::TypeMismatch {
                expected: required,
                found: binding.ty,
            },
            location: value_location,
        });
        return None;
    }

    let duplicable = context.type_is_duplicable(binding.ty);
    let target = ReferenceTarget::local_root(binding.id);
    let compatible = if duplicable {
        state.target_satisfies_shared_requirement(&target)
    } else {
        state.target_satisfies_exclusive_requirement(&target)
    };
    if !compatible {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ReferencePermissionUnavailable,
            location: value_location,
        });
        return None;
    }

    let ownership = if duplicable {
        OwnedUse::Duplicate
    } else {
        state
            .bindings
            .get_mut(&name)
            .expect("resolved value binding remains active")
            .ownership
            .consume_path(&[]);
        OwnedUse::Consume
    };

    let reference_authority = if matches!(binding.ty, Type::SafeReference { .. }) {
        let authority = binding
            .reference_authority
            .expect("live safe-reference binding retains authority");
        if duplicable {
            state.add_reference_carrier(authority);
        } else {
            state
                .bindings
                .get_mut(&name)
                .expect("resolved replacement-reference binding remains active")
                .reference_authority = None;
        }
        Some(authority)
    } else {
        None
    };

    Some(ProducedValue {
        value: Value {
            ty: binding.ty,
            kind: ValueKind::BindingUse {
                binding: binding.id,
                ownership,
            },
            location: value_location,
        },
        reference_authority,
    })
}
'''
new_identifier = r'''fn validate_identifier_use(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    if let Some(qualified) = node
        .children()
        .find(|child| child.kind() == SyntaxKind::QualifiedModuleMember)
    {
        return match resolve_qualified_entity(
            header.unit,
            &qualified,
            context.modules,
            context.imports,
            diagnostics,
        )? {
            EntityId::Constant(id) => validate_constant_value(
                context.constants[id.0],
                required,
                value_location,
                diagnostics,
            ),
            EntityId::Function(function) if matches!(required, Type::Function(_)) => {
                validate_function_value(function, required, value_location, context, diagnostics)
            }
            EntityId::Record(_) | EntityId::Function(_) | EntityId::MarkerTrait(_) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedValueBinding,
                    location: value_location,
                });
                None
            }
        };
    }

    let token = direct_token(node, SyntaxKind::Ident);
    let name = key(&token);
    if let Some(binding) = state.bindings.get(&name).cloned() {
        if binding.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnavailableBinding,
                location: value_location,
            });
            return None;
        }
        if binding.ty != required {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::TypeMismatch {
                    expected: required,
                    found: binding.ty,
                },
                location: value_location,
            });
            return None;
        }

        let duplicable = context.type_is_duplicable(binding.ty);
        let target = ReferenceTarget::local_root(binding.id);
        let compatible = if duplicable {
            state.target_satisfies_shared_requirement(&target)
        } else {
            state.target_satisfies_exclusive_requirement(&target)
        };
        if !compatible {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ReferencePermissionUnavailable,
                location: value_location,
            });
            return None;
        }

        let ownership = if duplicable {
            OwnedUse::Duplicate
        } else {
            state
                .bindings
                .get_mut(&name)
                .expect("resolved value binding remains active")
                .ownership
                .consume_path(&[]);
            OwnedUse::Consume
        };

        let reference_authority = if matches!(binding.ty, Type::SafeReference { .. }) {
            let authority = binding
                .reference_authority
                .expect("live safe-reference binding retains authority");
            if duplicable {
                state.add_reference_carrier(authority);
            } else {
                state
                    .bindings
                    .get_mut(&name)
                    .expect("resolved replacement-reference binding remains active")
                    .reference_authority = None;
            }
            Some(authority)
        } else {
            None
        };

        return Some(ProducedValue {
            value: Value {
                ty: binding.ty,
                kind: ValueKind::BindingUse {
                    binding: binding.id,
                    ownership,
                },
                location: value_location,
            },
            reference_authority,
        });
    }

    let entity = context
        .modules
        .get(&header.module)
        .and_then(|module| module.namespace.get(&name))
        .copied()
        .map(|entity| entity.entity);
    match entity {
        Some(EntityId::Constant(id)) => validate_constant_value(
            context.constants[id.0],
            required,
            value_location,
            diagnostics,
        ),
        Some(EntityId::Function(function)) if matches!(required, Type::Function(_)) => {
            validate_function_value(function, required, value_location, context, diagnostics)
        }
        Some(EntityId::Record(_) | EntityId::Function(_) | EntityId::MarkerTrait(_)) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ExpectedValueBinding,
                location: value_location,
            });
            None
        }
        None => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnresolvedName,
                location: value_location,
            });
            None
        }
    }
}
'''
build = replace_once(build, old_identifier, new_identifier, "context-typed function-value formation")

# Direct/indirect call classification and one shared static interface.
old_resolve_call_target_start = build.index("fn resolve_call_target(")
old_resolve_call_target_end = build.index("fn resolve_generic_type_argument(", old_resolve_call_target_start)
old_resolve_call_target = build[old_resolve_call_target_start:old_resolve_call_target_end]
new_resolve_call_target = r'''enum ResolvedCallTarget {
    Direct(FunctionId),
    Indirect {
        binding: BindingId,
        function_type: FunctionTypeId,
    },
}

fn resolve_call_target(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ResolvedCallTarget> {
    if let Some(qualified) = node
        .children()
        .find(|child| child.kind() == SyntaxKind::QualifiedModuleMember)
    {
        return match resolve_qualified_entity(
            header.unit,
            &qualified,
            context.modules,
            context.imports,
            diagnostics,
        )? {
            EntityId::Function(id) => Some(ResolvedCallTarget::Direct(id)),
            EntityId::Record(_) | EntityId::MarkerTrait(_) | EntityId::Constant(_) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedFunction,
                    location: location(header.unit, &qualified),
                });
                None
            }
        };
    }

    let name_token = direct_token(node, SyntaxKind::Ident);
    let name = key(&name_token);
    let name_location = SourceLocation {
        unit: header.unit,
        range: name_token.text_range(),
    };

    if let Some(binding) = bindings.get(&name) {
        return match binding.ty {
            Type::Function(function_type) => Some(ResolvedCallTarget::Indirect {
                binding: binding.id,
                function_type,
            }),
            _ => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedFunction,
                    location: name_location,
                });
                None
            }
        };
    }

    match context
        .modules
        .get(&header.module)
        .and_then(|module| module.namespace.get(&name))
        .copied()
        .map(|entity| entity.entity)
    {
        Some(EntityId::Function(id)) => Some(ResolvedCallTarget::Direct(id)),
        Some(EntityId::Record(_) | EntityId::MarkerTrait(_) | EntityId::Constant(_)) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ExpectedFunction,
                location: name_location,
            });
            None
        }
        None => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnresolvedName,
                location: name_location,
            });
            None
        }
    }
}

'''
build = build[:old_resolve_call_target_start] + new_resolve_call_target + build[old_resolve_call_target_end:]

old_resolved_app = r'''struct ResolvedCallApplication {
    function: FunctionId,
    type_arguments: Vec<Type>,
    parameter_types: Vec<Type>,
    result: Option<Type>,
}
'''
new_resolved_app = r'''struct ResolvedCallApplication {
    target: CallTarget,
    parameter_types: Vec<Type>,
    result: Option<Type>,
    safe_reference_result_contract: SafeReferenceResultContract,
}
'''
build = replace_once(build, old_resolved_app, new_resolved_app, "ResolvedCallApplication")

start = build.index("fn resolve_call_application(")
end = build.index("struct ValidatedCall {", start)
old_app_fn = build[start:end]
new_app_fn = r'''fn resolve_call_application(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ResolvedCallApplication> {
    let resolved_target = resolve_call_target(header, node, context, bindings, diagnostics)?;
    let type_argument_list = node
        .children()
        .find(|child| child.kind() == SyntaxKind::GenericTypeArgumentList);

    if let ResolvedCallTarget::Indirect {
        binding,
        function_type,
    } = resolved_target
    {
        if let Some(list) = type_argument_list {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnexpectedGenericTypeArguments,
                location: location(header.unit, &list),
            });
            return None;
        }
        let interface = context.function_types.borrow()[function_type.0].clone();
        return Some(ResolvedCallApplication {
            target: CallTarget::Indirect {
                binding,
                function_type,
            },
            parameter_types: interface.parameters,
            result: interface.result,
            safe_reference_result_contract: interface.safe_reference_result_contract,
        });
    }

    let ResolvedCallTarget::Direct(function) = resolved_target else {
        unreachable!("indirect target returned above")
    };
    let target = &context.headers[function.0];
    let type_arguments = match (target.type_parameters.is_empty(), type_argument_list) {
        (false, None) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::MissingGenericTypeArguments,
                location: location(header.unit, node),
            });
            return None;
        }
        (true, Some(list)) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnexpectedGenericTypeArguments,
                location: location(header.unit, &list),
            });
            return None;
        }
        (true, None) => Vec::new(),
        (false, Some(list)) => {
            let arguments = list
                .children()
                .filter(|child| child.kind() == SyntaxKind::GenericTypeArgument)
                .collect::<Vec<_>>();
            if arguments.len() != target.type_parameters.len() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::GenericTypeArgumentCount {
                        expected: target.type_parameters.len(),
                        found: arguments.len(),
                    },
                    location: location(header.unit, &list),
                });
                return None;
            }
            let mut resolved = Vec::with_capacity(arguments.len());
            for argument in arguments {
                resolved.push(resolve_generic_type_argument(
                    header,
                    &argument,
                    context,
                    diagnostics,
                )?);
            }
            resolved
        }
    };

    for (parameter, argument) in target
        .type_parameters
        .iter()
        .zip(type_arguments.iter().copied())
    {
        for required in &parameter.requirements {
            if !type_argument_satisfies_marker_requirement(header, argument, *required, context) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::UnsatisfiedMarkerRequirement {
                        required: *required,
                        argument,
                    },
                    location: location(header.unit, node),
                });
                return None;
            }
        }
    }

    let parameter_types = target
        .parameters
        .iter()
        .map(|parameter| instantiate_call_type(parameter.ty, target, &type_arguments))
        .collect::<Vec<_>>();
    let result = target
        .result
        .map(|result| instantiate_call_type(result, target, &type_arguments));

    Some(ResolvedCallApplication {
        target: CallTarget::Direct {
            function,
            type_arguments,
        },
        parameter_types,
        result,
        safe_reference_result_contract: target.safe_reference_result_contract,
    })
}

'''
build = build[:start] + new_app_fn + build[end:]

build = replace_once(
    build,
    r'''struct ValidatedCall {
    function: FunctionId,
    type_arguments: Vec<Type>,
    arguments: Vec<Value>,
    result: Option<Type>,
    result_reference_authority: Option<ReferenceAuthorityId>,
}
''',
    r'''struct ValidatedCall {
    target: CallTarget,
    arguments: Vec<Value>,
    result: Option<Type>,
    result_reference_authority: Option<ReferenceAuthorityId>,
}
''',
    "ValidatedCall target",
)

# Explicit callee hold occurs after complete static shape/arity admission and before argument producers.
build = replace_once(
    build,
    """    if argument_nodes.len() != application.parameter_types.len() {\n        diagnostics.push(Diagnostic {\n            kind: DiagnosticKind::ArgumentCount {\n                expected: application.parameter_types.len(),\n                found: argument_nodes.len(),\n            },\n            location: location(header.unit, node),\n        });\n        return None;\n    }\n\n    let mut arguments = Vec::with_capacity(argument_nodes.len());\n""",
    """    if argument_nodes.len() != application.parameter_types.len() {\n        diagnostics.push(Diagnostic {\n            kind: DiagnosticKind::ArgumentCount {\n                expected: application.parameter_types.len(),\n                found: argument_nodes.len(),\n            },\n            location: location(header.unit, node),\n        });\n        return None;\n    }\n\n    if let CallTarget::Indirect { binding, .. } = &application.target {\n        let binding_state = binding_state_by_id(&state.bindings, *binding)\n            .expect(\"classified indirect target remains an active binding\");\n        if binding_state.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {\n            diagnostics.push(Diagnostic {\n                kind: DiagnosticKind::UnavailableBinding,\n                location: location(header.unit, node),\n            });\n            return None;\n        }\n        debug_assert!(context.type_is_duplicable(binding_state.ty));\n    }\n\n    let mut arguments = Vec::with_capacity(argument_nodes.len());\n""",
    "indirect callee hold",
)

build = replace_once(
    build,
    """    let result_reference_authority = match target.safe_reference_result_contract {\n""",
    """    let result_reference_authority = match application.safe_reference_result_contract {\n""",
    "call result contract source",
)
# target declaration is no longer needed.
build = build.replace("    let target = &context.headers[application.function.0];\n", "", 1)

build = replace_once(
    build,
    """    Some(ValidatedCall {\n        function: application.function,\n        type_arguments: application.type_arguments,\n        arguments,\n""",
    """    Some(ValidatedCall {\n        target: application.target,\n        arguments,\n""",
    "validated call output",
)

build = replace_once(
    build,
    """    Some(Statement::Call {\n        function: validated.function,\n        type_arguments: validated.type_arguments,\n        arguments: validated.arguments,\n""",
    """    Some(Statement::Call {\n        target: validated.target,\n        arguments: validated.arguments,\n""",
    "statement call target",
)

build = replace_once(
    build,
    """            Some(ProducedValue {\n                value: Value {\n                    ty,\n                    kind: ValueKind::DirectCall {\n                        function: validated.function,\n                        type_arguments: validated.type_arguments,\n                        arguments: validated.arguments,\n                    },\n""",
    """            Some(ProducedValue {\n                value: Value {\n                    ty,\n                    kind: ValueKind::Call {\n                        target: validated.target,\n                        arguments: validated.arguments,\n                    },\n""",
    "value call target",
)

# Generic marker requirements exclude function types just like references/pointers.
build = replace_once(
    build,
    """        Type::SafeReference { .. } | Type::RawPointer(_) => false,\n    }\n}\n\nstruct ResolvedCallApplication""",
    """        Type::SafeReference { .. } | Type::RawPointer(_) | Type::Function(_) => false,\n    }\n}\n\nstruct ResolvedCallApplication""",
    "generic marker function exclusion",
)

# Function values cannot become safe-reference/raw pointees.
build = build.replace(
    """                            Type::Parameter(_)\n                            | Type::SafeReference { .. }\n                            | Type::RawPointer(_) => {""",
    """                            Type::Parameter(_)\n                            | Type::SafeReference { .. }\n                            | Type::RawPointer(_)\n                            | Type::Function(_) => {""",
)
build = build.replace(
    """        Type::Parameter(_) | Type::SafeReference { .. } | Type::RawPointer(_) => {""",
    """        Type::Parameter(_) | Type::SafeReference { .. } | Type::RawPointer(_) | Type::Function(_) => {""",
)

build_path.write_text(build)

# Focused HIR conformance for retained representation, formation, and call classification.
test_path = Path("crates/runen-hir/tests/function_values.rs")
assert not test_path.exists()
test_path.write_text(r'''use runen_hir::{
    CallTarget, DiagnosticKind, FunctionTypeId, IntrinsicType, ModuleId, SafeReferenceResultContract,
    SourceUnit, Statement, Type, TypedCompilation, ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn build(source: &str) -> Result<TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn function<'a>(hir: &'a TypedCompilation, name: &str) -> &'a runen_hir::Function {
    hir.functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing HIR function {name}"))
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

fn function_type(ty: Type) -> FunctionTypeId {
    let Type::Function(id) = ty else {
        panic!("expected function type, found {ty:?}");
    };
    id
}

#[test]
fn equal_structural_function_types_share_one_canonical_handle_and_nested_types_resolve() {
    let hir = build(
        "fn left(value: I64) -> I64 { return value; } \
         fn right(value: I64) -> I64 { return value; } \
         fn use(a: fn(I64) -> I64, b: fn(I64) -> I64, nested: fn(fn(I64) -> I64) -> I64) {}",
    )
    .expect("concrete function types are valid");
    let use_fn = function(&hir, "use");
    let a = function_type(use_fn.parameters[0].ty);
    let b = function_type(use_fn.parameters[1].ty);
    assert_eq!(a, b);
    let nested = function_type(use_fn.parameters[2].ty);
    assert_ne!(a, nested);
    assert_eq!(hir.function_type(nested).parameters, &[Type::Function(a)]);
    assert_eq!(
        hir.function_type(a).result,
        Some(Type::Intrinsic(IntrinsicType::I64))
    );
}

#[test]
fn function_values_form_contextually_and_remain_distinct_payloads_under_equal_type() {
    let hir = build(
        "fn left(value: I64) -> I64 { return value; } \
         fn right(value: I64) -> I64 { return value; } \
         fn use() -> I64 { \
             let mut f: fn(I64) -> I64 = left; \
             let g: fn(I64) -> I64 = right; \
             f = g; \
             return f(7); \
         }",
    )
    .expect("function values use ordinary local transport");
    let use_fn = function(&hir, "use");
    let [Statement::Local { initializer: left, .. }, Statement::Local { initializer: right, .. }, Statement::Assignment { value: assigned, .. }] = use_fn.body.statements.as_slice() else {
        panic!("expected two locals and one assignment");
    };
    let ValueKind::FunctionValue { function: left_id } = left.kind else {
        panic!("left initializer must be a function value");
    };
    let ValueKind::FunctionValue { function: right_id } = right.kind else {
        panic!("right initializer must be a function value");
    };
    assert_ne!(left_id, right_id);
    assert_eq!(left.ty, right.ty);
    assert!(matches!(assigned.kind, ValueKind::BindingUse { .. }));

    let returned = use_fn
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("use returns an indirect call result");
    let ValueKind::Call { target, .. } = &returned.kind else {
        panic!("return must retain one call");
    };
    assert!(matches!(target, CallTarget::Indirect { .. }));
}

#[test]
fn indirect_no_result_call_uses_the_same_call_target_relation() {
    let hir = build(
        "fn sink(value: I64) {} fn use(f: fn(I64)) { f(1); }",
    )
    .expect("no-result indirect call is valid");
    let use_fn = function(&hir, "use");
    let [Statement::Call { target, arguments, .. }] = use_fn.body.statements.as_slice() else {
        panic!("expected one call statement");
    };
    assert!(matches!(target, CallTarget::Indirect { .. }));
    assert_eq!(arguments.len(), 1);
}

#[test]
fn direct_calls_remain_direct_and_non_callable_locals_block_module_fallback() {
    let hir = build(
        "fn target(value: I64) -> I64 { return value; } fn good() -> I64 { return target(1); }",
    )
    .expect("ordinary direct call remains valid");
    let returned = function(&hir, "good")
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("good returns a call");
    let ValueKind::Call { target, .. } = &returned.kind else {
        panic!("expected retained call");
    };
    assert!(matches!(target, CallTarget::Direct { .. }));

    let errors = build(
        "fn target(value: I64) -> I64 { return value; } fn bad(target: I64) -> I64 { return target(1); }",
    )
    .expect_err("local lookup is final for call classification");
    assert!(has_diagnostic(&errors, DiagnosticKind::ExpectedFunction));
}

#[test]
fn indirect_generic_arguments_and_generic_function_values_are_rejected() {
    let errors = build(
        "fn use(f: fn(I64) -> I64) -> I64 { return f[I64](1); }",
    )
    .expect_err("indirect calls do not accept generic arguments");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::UnexpectedGenericTypeArguments
    ));

    let errors = build(
        "fn id[T](value: T) -> T { return value; } fn bad() -> fn(I64) -> I64 { return id; }",
    )
    .expect_err("generic functions do not form function values");
    assert!(has_diagnostic(&errors, DiagnosticKind::GenericFunctionValue));
}

#[test]
fn abstract_components_and_function_valued_record_fields_remain_excluded() {
    let errors = build("fn bad[T](f: fn(T) -> I64) {}").expect_err("function types are concrete");
    assert!(has_diagnostic(
        &errors,
        DiagnosticKind::InvalidGenericTypeParameterPosition
    ));

    let errors = build("record Bad { f: fn(I64) -> I64 }")
        .expect_err("record fields cannot be function-valued");
    assert!(has_diagnostic(&errors, DiagnosticKind::FunctionTypeField));
}

#[test]
fn shared_reference_function_type_contract_is_derived_once_and_retained() {
    let hir = build("fn use(f: fn(&I64) -> &I64) {}").expect("shared result has unique origin");
    let function_type = function_type(function(&hir, "use").parameters[0].ty);
    assert_eq!(
        hir.function_type(function_type).safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
}
''')

print("staged bounded HIR function-value and indirect-call implementation")
