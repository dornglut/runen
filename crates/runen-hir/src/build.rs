use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use runen_syntax::{SyntaxKind, SyntaxNode, SyntaxToken, identifier_key};

use crate::{
    Accessibility, AssignmentMutability, BinaryFloatSign, BinaryFloatValue, BindingId, Block, Body,
    BooleanEqualityRelation, CleanupPath, Diagnostic, DiagnosticKind, Duplicability, Field,
    FieldReceiverTransientCleanup, FieldValueReceiver, Function, FunctionId, IntrinsicType,
    LiteralValue, Module, ModuleId, NumericContract, OwnedUse, Parameter, RawPointerPointee,
    Record, RecordFieldValue, RecordId, RecordPatternBinding, RecordPatternScrutinee,
    RecordPatternTransientCleanup, ReferencePermission, ReferenceReferent, Return,
    SafeReferenceResultContract, SourceLocation, SourceUnit, Statement, Type, TypedCompilation,
    Value, ValueKind, type_is_duplicable_in_records,
};

#[derive(Debug, Clone, Copy)]
enum EntityId {
    Record(RecordId),
    Function(FunctionId),
}

#[derive(Debug, Clone, Copy)]
struct ModuleEntity {
    entity: EntityId,
    accessibility: Accessibility,
}

#[derive(Debug, Default)]
struct ModuleBuild {
    namespace: BTreeMap<String, ModuleEntity>,
    records: Vec<RecordId>,
    functions: Vec<FunctionId>,
}

#[derive(Debug, Clone)]
struct RecordSyntax {
    id: RecordId,
    module: ModuleId,
    unit: usize,
    name: String,
    accessibility: Accessibility,
    duplicability_selection: Option<SourceLocation>,
    node: SyntaxNode,
    location: SourceLocation,
}

#[derive(Debug, Clone)]
struct FunctionSyntax {
    id: FunctionId,
    module: ModuleId,
    unit: usize,
    name: String,
    accessibility: Accessibility,
    node: SyntaxNode,
    location: SourceLocation,
}

#[derive(Debug, Clone)]
struct FunctionHeader {
    id: FunctionId,
    module: ModuleId,
    unit: usize,
    name: String,
    accessibility: Accessibility,
    parameters: Vec<Parameter>,
    result: Option<Type>,
    safe_reference_result_contract: SafeReferenceResultContract,
    body: SyntaxNode,
    location: SourceLocation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct StructuralOwnershipState {
    consumed_paths: BTreeSet<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingSource {
    Parameter,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ReferenceAuthorityId(usize);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ReferenceTarget {
    Local {
        binding: BindingId,
        fields: Vec<usize>,
    },
    External(usize),
}

impl ReferenceTarget {
    fn local(binding: BindingId, fields: &[usize]) -> Self {
        Self::Local {
            binding,
            fields: fields.to_vec(),
        }
    }

    fn local_root(binding: BindingId) -> Self {
        Self::local(binding, &[])
    }

    fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Local {
                    binding: left_binding,
                    fields: left_fields,
                },
                Self::Local {
                    binding: right_binding,
                    fields: right_fields,
                },
            ) => {
                left_binding == right_binding
                    && (left_fields.starts_with(right_fields)
                        || right_fields.starts_with(left_fields))
            }
            (Self::External(left), Self::External(right)) => left == right,
            (Self::Local { .. }, Self::External(_)) | (Self::External(_), Self::Local { .. }) => {
                false
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceAuthorityState {
    target: ReferenceTarget,
    permission: ReferencePermission,
    parent: Option<ReferenceAuthorityId>,
    carriers: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BindingState {
    id: BindingId,
    ty: Type,
    mutability: AssignmentMutability,
    source: BindingSource,
    ownership: StructuralOwnershipState,
    reference_authority: Option<ReferenceAuthorityId>,
    pointer_origin: Option<BindingId>,
    raw_pointer_target_domain: Option<BTreeSet<BindingId>>,
}

#[derive(Debug, Clone, Default)]
struct SemanticState {
    bindings: BTreeMap<String, BindingState>,
    external_referents: BTreeMap<usize, StructuralOwnershipState>,
    reference_authorities: BTreeMap<ReferenceAuthorityId, ReferenceAuthorityState>,
    next_reference_authority: usize,
}

impl SemanticState {
    fn create_reference_authority(
        &mut self,
        target: ReferenceTarget,
        permission: ReferencePermission,
        parent: Option<ReferenceAuthorityId>,
    ) -> ReferenceAuthorityId {
        let authority = ReferenceAuthorityId(self.next_reference_authority);
        self.next_reference_authority += 1;
        let previous = self.reference_authorities.insert(
            authority,
            ReferenceAuthorityState {
                target,
                permission,
                parent,
                carriers: 1,
            },
        );
        debug_assert!(previous.is_none());
        authority
    }

    fn add_reference_carrier(&mut self, authority: ReferenceAuthorityId) {
        self.reference_authorities
            .get_mut(&authority)
            .expect("live reference carrier names one live authority")
            .carriers += 1;
    }

    fn release_reference_carrier(&mut self, authority: ReferenceAuthorityId) {
        let state = self
            .reference_authorities
            .get_mut(&authority)
            .expect("released reference carrier names one live authority");
        debug_assert!(state.carriers > 0);
        state.carriers -= 1;
        self.end_carrierless_authority_branch(authority);
    }

    fn end_carrierless_authority_branch(&mut self, mut authority: ReferenceAuthorityId) {
        loop {
            let should_end = self
                .reference_authorities
                .get(&authority)
                .is_some_and(|state| {
                    state.carriers == 0
                        && !self
                            .reference_authorities
                            .values()
                            .any(|candidate| candidate.parent == Some(authority))
                });
            if !should_end {
                return;
            }
            let parent = self
                .reference_authorities
                .remove(&authority)
                .expect("carrierless authority remains present until branch termination")
                .parent;
            let Some(parent) = parent else {
                return;
            };
            authority = parent;
        }
    }

    fn reference_target(&self, authority: ReferenceAuthorityId) -> ReferenceTarget {
        self.reference_authorities
            .get(&authority)
            .expect("live authority is present")
            .target
            .clone()
    }

    fn reference_capability(&self, authority: ReferenceAuthorityId) -> Option<ReferencePermission> {
        let state = self
            .reference_authorities
            .get(&authority)
            .expect("live authority is present");
        let mut has_shared_child = false;
        for child in self
            .reference_authorities
            .values()
            .filter(|candidate| candidate.parent == Some(authority))
        {
            match child.permission {
                ReferencePermission::ExclusiveReplace => return None,
                ReferencePermission::Shared => has_shared_child = true,
            }
        }
        if has_shared_child {
            Some(ReferencePermission::Shared)
        } else {
            Some(state.permission)
        }
    }

    fn authority_satisfies(
        &self,
        authority: ReferenceAuthorityId,
        required: ReferencePermission,
    ) -> bool {
        matches!(
            (self.reference_capability(authority), required),
            (Some(ReferencePermission::ExclusiveReplace), _)
                | (
                    Some(ReferencePermission::Shared),
                    ReferencePermission::Shared
                )
        )
    }

    fn target_is_fully_available(&self, target: &ReferenceTarget) -> bool {
        match target {
            ReferenceTarget::Local { binding, fields } => {
                binding_state_by_id(&self.bindings, *binding).is_some_and(|state| {
                    state.ownership.path_availability(fields) == PathAvailability::FullyAvailable
                })
            }
            ReferenceTarget::External(slot) => {
                self.external_referents.get(slot).is_none_or(|state| {
                    state.path_availability(&[]) == PathAvailability::FullyAvailable
                })
            }
        }
    }

    fn consume_reference_target(&mut self, target: &ReferenceTarget) {
        match target {
            ReferenceTarget::Local { binding, fields } => binding_state_by_id_mut(
                &mut self.bindings,
                *binding,
            )
            .expect("reference target names active local binding")
            .ownership
            .consume_path(fields),
            ReferenceTarget::External(slot) => self
                .external_referents
                .get_mut(slot)
                .expect("consuming reference target has external structural root")
                .consume_path(&[]),
        }
    }

    fn restore_reference_target(&mut self, target: &ReferenceTarget) {
        match target {
            ReferenceTarget::Local { binding, fields } => {
                debug_assert!(
                    fields.is_empty(),
                    "replacement-capable local reference targets remain complete roots"
                );
                binding_state_by_id_mut(&mut self.bindings, *binding)
                    .expect("reference target names active local binding")
                    .ownership = StructuralOwnershipState::default();
            }
            ReferenceTarget::External(slot) => {
                *self
                    .external_referents
                    .get_mut(slot)
                    .expect("replacement target has external structural root") =
                    StructuralOwnershipState::default();
            }
        }
    }

    fn target_satisfies_shared_requirement(&self, target: &ReferenceTarget) -> bool {
        self.reference_authorities.values().all(|authority| {
            !authority.target.overlaps(target)
                || authority.permission == ReferencePermission::Shared
        })
    }

    fn target_satisfies_exclusive_requirement(&self, target: &ReferenceTarget) -> bool {
        self.reference_authorities
            .values()
            .all(|authority| !authority.target.overlaps(target))
    }

    fn release_binding_reference_carrier(&mut self, binding: BindingId) {
        let authority = binding_state_by_id(&self.bindings, binding)
            .and_then(|state| state.reference_authority);
        if let Some(authority) = authority {
            self.release_reference_carrier(authority);
        }
    }
}

#[derive(Debug, Clone)]
struct ProducedValue {
    value: Value,
    reference_authority: Option<ReferenceAuthorityId>,
}

impl ProducedValue {
    fn ordinary(value: Value) -> Self {
        Self {
            value,
            reference_authority: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathAvailability {
    FullyAvailable,
    PartiallyAvailable,
    Unavailable,
}

impl StructuralOwnershipState {
    fn path_availability(&self, path: &[usize]) -> PathAvailability {
        if self
            .consumed_paths
            .iter()
            .any(|consumed| path.starts_with(consumed))
        {
            return PathAvailability::Unavailable;
        }
        if self
            .consumed_paths
            .iter()
            .any(|consumed| consumed.len() > path.len() && consumed.starts_with(path))
        {
            return PathAvailability::PartiallyAvailable;
        }
        PathAvailability::FullyAvailable
    }

    fn consume_path(&mut self, path: &[usize]) {
        debug_assert_eq!(
            self.path_availability(path),
            PathAvailability::FullyAvailable
        );
        let inserted = self.consumed_paths.insert(path.to_vec());
        debug_assert!(inserted);
        debug_assert!(self.consumed_paths.iter().all(|left| {
            self.consumed_paths
                .iter()
                .all(|right| left == right || !(left.starts_with(right) || right.starts_with(left)))
        }));
    }
}

type UnitImports = BTreeMap<String, ModuleId>;

struct BodyResolutionContext<'a> {
    modules: &'a BTreeMap<ModuleId, ModuleBuild>,
    imports: &'a [UnitImports],
    records: &'a [Record],
    headers: &'a [FunctionHeader],
    safe_reference_result_origin_authority: Option<ReferenceAuthorityId>,
}

impl BodyResolutionContext<'_> {
    fn type_is_duplicable(&self, ty: Type) -> bool {
        type_is_duplicable_in_records(ty, self.records)
    }
}

#[derive(Debug, Default)]
struct ControlValidationContext {
    scopes: Vec<ScopeFrame>,
    loops: Vec<LoopTarget>,
}

#[derive(Debug, Clone, Default)]
struct ValueValidationContext {
    unsafe_active: bool,
}

#[derive(Debug)]
struct ScopeFrame {
    direct_bindings: Vec<BindingId>,
}

#[derive(Debug)]
struct LoopTarget {
    body_scope: usize,
    head: SemanticState,
    continuation: SemanticState,
}

pub(crate) fn build(units: &[SourceUnit<'_>]) -> Result<TypedCompilation, Vec<Diagnostic>> {
    let mut diagnostics = syntax_admission_diagnostics(units);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let (mut modules, record_syntax, function_syntax) =
        collect_declarations(units, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let imports = collect_imports(units, &modules, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let records = resolve_records(&record_syntax, &modules, &imports, &mut diagnostics);
    let mut next_binding = 0_usize;
    let headers = resolve_function_headers(
        &function_syntax,
        &modules,
        &imports,
        &records,
        &mut next_binding,
        &mut diagnostics,
    );
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    validate_record_acyclicity(&records, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    validate_record_duplicability(&record_syntax, &records, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut functions = Vec::with_capacity(headers.len());
    for header in &headers {
        let body = validate_body(
            header,
            &modules,
            &imports,
            &records,
            &headers,
            &mut next_binding,
            &mut diagnostics,
        );
        functions.push(Function {
            id: header.id,
            module: header.module,
            name: header.name.clone(),
            accessibility: header.accessibility,
            parameters: header.parameters.clone(),
            result: header.result,
            safe_reference_result_contract: header.safe_reference_result_contract,
            body,
            location: header.location,
        });
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let modules = modules
        .iter_mut()
        .map(|(id, module)| Module {
            id: *id,
            records: std::mem::take(&mut module.records),
            functions: std::mem::take(&mut module.functions),
        })
        .collect();

    Ok(TypedCompilation {
        modules,
        records,
        functions,
    })
}

fn syntax_admission_diagnostics(units: &[SourceUnit<'_>]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (unit_index, unit) in units.iter().enumerate() {
        for error in unit.parse.errors() {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::SyntaxError(error.kind()),
                location: SourceLocation {
                    unit: unit_index,
                    range: error.range(),
                },
            });
        }
    }
    diagnostics
}

fn collect_declarations(
    units: &[SourceUnit<'_>],
    diagnostics: &mut Vec<Diagnostic>,
) -> (
    BTreeMap<ModuleId, ModuleBuild>,
    Vec<RecordSyntax>,
    Vec<FunctionSyntax>,
) {
    let mut modules = BTreeMap::<ModuleId, ModuleBuild>::new();
    let mut records = Vec::new();
    let mut functions = Vec::new();

    for (unit_index, unit) in units.iter().enumerate() {
        modules.entry(unit.module).or_default();
        let root = unit.parse.syntax();
        for item in root.children() {
            match item.kind() {
                SyntaxKind::ImportDeclaration => {}
                SyntaxKind::RecordDefinition => {
                    let id = RecordId(records.len());
                    let name_token = direct_token(&item, SyntaxKind::Ident);
                    let name = key(&name_token);
                    let accessibility = declaration_accessibility(&item);
                    let duplicability_selection = item
                        .children_with_tokens()
                        .filter_map(|element| element.into_token())
                        .find(|token| token.kind() == SyntaxKind::KwCopy)
                        .map(|token| SourceLocation {
                            unit: unit_index,
                            range: token.text_range(),
                        });
                    let location = location(unit_index, &item);
                    if insert_entity(
                        &mut modules,
                        unit.module,
                        &name,
                        EntityId::Record(id),
                        accessibility,
                    ) {
                        modules
                            .get_mut(&unit.module)
                            .expect("module inserted")
                            .records
                            .push(id);
                    } else {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::DuplicateModuleBinding,
                            location,
                        });
                    }
                    records.push(RecordSyntax {
                        id,
                        module: unit.module,
                        unit: unit_index,
                        name,
                        accessibility,
                        duplicability_selection,
                        node: item,
                        location,
                    });
                }
                SyntaxKind::FunctionDefinition => {
                    let id = FunctionId(functions.len());
                    let name_token = direct_token(&item, SyntaxKind::Ident);
                    let name = key(&name_token);
                    let accessibility = declaration_accessibility(&item);
                    let location = location(unit_index, &item);
                    if insert_entity(
                        &mut modules,
                        unit.module,
                        &name,
                        EntityId::Function(id),
                        accessibility,
                    ) {
                        modules
                            .get_mut(&unit.module)
                            .expect("module inserted")
                            .functions
                            .push(id);
                    } else {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::DuplicateModuleBinding,
                            location,
                        });
                    }
                    functions.push(FunctionSyntax {
                        id,
                        module: unit.module,
                        unit: unit_index,
                        name,
                        accessibility,
                        node: item,
                        location,
                    });
                }
                _ => unreachable!(
                    "syntax-clean source unit contains only imports and represented items"
                ),
            }
        }
    }

    (modules, records, functions)
}

fn insert_entity(
    modules: &mut BTreeMap<ModuleId, ModuleBuild>,
    module: ModuleId,
    name: &str,
    entity: EntityId,
    accessibility: Accessibility,
) -> bool {
    let namespace = &mut modules.entry(module).or_default().namespace;
    if namespace.contains_key(name) {
        false
    } else {
        namespace.insert(
            name.to_owned(),
            ModuleEntity {
                entity,
                accessibility,
            },
        );
        true
    }
}

fn collect_imports(
    units: &[SourceUnit<'_>],
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<UnitImports> {
    let mut result = Vec::with_capacity(units.len());

    for (unit_index, unit) in units.iter().enumerate() {
        let mut seen_aliases = BTreeSet::<String>::new();
        let mut aliases = BTreeMap::<String, ModuleId>::new();
        let root = unit.parse.syntax();
        for import in root
            .children()
            .filter(|node| node.kind() == SyntaxKind::ImportDeclaration)
        {
            let alias_token = direct_token(&import, SyntaxKind::Ident);
            let alias = key(&alias_token);
            let import_location = location(unit_index, &import);
            let mut valid = true;

            if !seen_aliases.insert(alias.clone()) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::DuplicateImportAlias,
                    location: import_location,
                });
                valid = false;
            }

            if modules
                .get(&unit.module)
                .is_some_and(|module| module.namespace.contains_key(&alias))
            {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ImportDeclarationConflict,
                    location: import_location,
                });
                valid = false;
            }

            let mut targets = unit.imports.iter().filter(|target| target.alias() == alias);
            let target = targets.next();
            match (target, targets.next()) {
                (None, _) => {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::MissingImportTarget,
                        location: import_location,
                    });
                }
                (Some(_), Some(_)) => {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::DuplicateImportTarget,
                        location: import_location,
                    });
                }
                (Some(target), None) => {
                    if target.module == unit.module {
                        diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::SelfImport,
                            location: import_location,
                        });
                        valid = false;
                    }
                    if valid {
                        aliases.insert(alias, target.module);
                    }
                }
            }
        }
        result.push(aliases);
    }

    result
}

fn resolve_records(
    syntax: &[RecordSyntax],
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Record> {
    let mut records = Vec::with_capacity(syntax.len());
    for record in syntax {
        let mut field_names = BTreeSet::new();
        let mut fields = Vec::new();
        for field_node in record
            .node
            .children()
            .filter(|node| node.kind() == SyntaxKind::RecordField)
        {
            let name_token = direct_token(&field_node, SyntaxKind::Ident);
            let name = key(&name_token);
            let accessibility = declaration_accessibility(&field_node);
            let field_location = location(record.unit, &field_node);
            if !field_names.insert(name.clone()) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::DuplicateRecordField,
                    location: field_location,
                });
            }
            let type_node = direct_child(&field_node, SyntaxKind::TypeRef);
            if type_ref_reference_permission(&type_node).is_some() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::SafeReferenceField,
                    location: location(record.unit, &type_node),
                });
                continue;
            }
            if type_ref_is_raw_pointer(&type_node) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::RawPointerField,
                    location: location(record.unit, &type_node),
                });
                continue;
            }
            if let Some(ty) = resolve_type(
                record.module,
                record.unit,
                &type_node,
                modules,
                imports,
                diagnostics,
            ) {
                validate_exported_field_type(
                    record,
                    accessibility,
                    ty,
                    syntax,
                    &type_node,
                    diagnostics,
                );
                fields.push(Field {
                    name,
                    ty,
                    accessibility,
                    location: field_location,
                });
            }
        }
        records.push(Record {
            id: record.id,
            module: record.module,
            name: record.name.clone(),
            accessibility: record.accessibility,
            duplicability: if record.duplicability_selection.is_some() {
                Duplicability::Duplicable
            } else {
                Duplicability::NonDuplicable
            },
            fields,
            location: record.location,
        });
    }
    records
}

fn validate_exported_field_type(
    containing: &RecordSyntax,
    field_accessibility: Accessibility,
    ty: Type,
    records: &[RecordSyntax],
    type_node: &SyntaxNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if containing.accessibility != Accessibility::Exported
        || field_accessibility != Accessibility::Exported
    {
        return;
    }
    let Type::Record(record) = ty else {
        return;
    };
    let target = &records[record.0];
    if target.module == containing.module && target.accessibility == Accessibility::ModulePrivate {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::PrivateTypeInExportedField,
            location: location(containing.unit, type_node),
        });
    }
}

fn resolve_function_headers(
    syntax: &[FunctionSyntax],
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    records: &[Record],
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<FunctionHeader> {
    let mut headers = Vec::with_capacity(syntax.len());
    for function in syntax {
        let parameter_list = direct_child(&function.node, SyntaxKind::ParameterList);
        let mut parameter_names = BTreeSet::new();
        let mut parameters = Vec::new();
        for parameter_node in parameter_list
            .children()
            .filter(|node| node.kind() == SyntaxKind::Parameter)
        {
            let name_token = direct_token(&parameter_node, SyntaxKind::Ident);
            let name = key(&name_token);
            let parameter_location = location(function.unit, &parameter_node);
            if !parameter_names.insert(name.clone()) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::DuplicateParameter,
                    location: parameter_location,
                });
            }
            let type_node = direct_child(&parameter_node, SyntaxKind::TypeRef);
            if let Some(ty) = resolve_type(
                function.module,
                function.unit,
                &type_node,
                modules,
                imports,
                diagnostics,
            ) {
                if matches!(ty, Type::RawPointer(_)) {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::RawPointerParameter,
                        location: location(function.unit, &type_node),
                    });
                    continue;
                }
                if !validate_safe_reference_referent(
                    ty,
                    records,
                    location(function.unit, &type_node),
                    diagnostics,
                ) {
                    continue;
                }
                validate_exported_signature_type(
                    function.accessibility,
                    ty,
                    records,
                    function.unit,
                    &type_node,
                    diagnostics,
                );
                let binding = BindingId(*next_binding);
                *next_binding += 1;
                parameters.push(Parameter {
                    binding,
                    name,
                    ty,
                    location: parameter_location,
                });
            }
        }

        let mut safe_reference_result_contract = SafeReferenceResultContract::None;
        let result = function
            .node
            .children()
            .find(|node| node.kind() == SyntaxKind::ResultClause)
            .and_then(|result_clause| {
                let type_node = direct_child(&result_clause, SyntaxKind::TypeRef);
                let ty = resolve_type(
                    function.module,
                    function.unit,
                    &type_node,
                    modules,
                    imports,
                    diagnostics,
                )?;
                if matches!(ty, Type::RawPointer(_)) {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::RawPointerResult,
                        location: location(function.unit, &type_node),
                    });
                    return None;
                }
                if matches!(
                    ty,
                    Type::SafeReference {
                        permission: ReferencePermission::ExclusiveReplace,
                        ..
                    }
                ) {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::ReplacementReferenceResult,
                        location: location(function.unit, &type_node),
                    });
                    return None;
                }
                if !validate_safe_reference_referent(
                    ty,
                    records,
                    location(function.unit, &type_node),
                    diagnostics,
                ) {
                    return None;
                }
                validate_exported_signature_type(
                    function.accessibility,
                    ty,
                    records,
                    function.unit,
                    &type_node,
                    diagnostics,
                );
                if let Type::SafeReference {
                    referent,
                    permission: ReferencePermission::Shared,
                } = ty
                {
                    let mut matching_shared = parameters
                        .iter()
                        .enumerate()
                        .filter(|(_, parameter)| parameter.ty == ty);
                    match (matching_shared.next(), matching_shared.next()) {
                        (Some((slot, _)), None) => {
                            safe_reference_result_contract =
                                SafeReferenceResultContract::SharedIdentity { origin: slot };
                        }
                        (Some(_), Some(_)) => diagnostics.push(Diagnostic {
                            kind: DiagnosticKind::AmbiguousSharedReferenceResultOrigin,
                            location: location(function.unit, &type_node),
                        }),
                        (None, _) => {
                            let replacement_ty = Type::SafeReference {
                                referent,
                                permission: ReferencePermission::ExclusiveReplace,
                            };
                            let mut matching_replacement = parameters
                                .iter()
                                .enumerate()
                                .filter(|(_, parameter)| parameter.ty == replacement_ty);
                            match (matching_replacement.next(), matching_replacement.next()) {
                                (None, _) => diagnostics.push(Diagnostic {
                                    kind: DiagnosticKind::MissingSharedReferenceResultOrigin,
                                    location: location(function.unit, &type_node),
                                }),
                                (Some((slot, _)), None) => {
                                    safe_reference_result_contract =
                                        SafeReferenceResultContract::SharedDirectChild {
                                            origin: slot,
                                        };
                                }
                                (Some(_), Some(_)) => diagnostics.push(Diagnostic {
                                    kind: DiagnosticKind::AmbiguousSharedReferenceResultOrigin,
                                    location: location(function.unit, &type_node),
                                }),
                            }
                        }
                    }
                }
                Some(ty)
            });
        let body = direct_child(&function.node, SyntaxKind::Body);
        headers.push(FunctionHeader {
            id: function.id,
            module: function.module,
            unit: function.unit,
            name: function.name.clone(),
            accessibility: function.accessibility,
            parameters,
            result,
            safe_reference_result_contract,
            body,
            location: function.location,
        });
    }
    headers
}

fn validate_exported_signature_type(
    accessibility: Accessibility,
    ty: Type,
    records: &[Record],
    unit: usize,
    type_node: &SyntaxNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if accessibility != Accessibility::Exported {
        return;
    }
    let record = match ty {
        Type::Record(record)
        | Type::SafeReference {
            referent: ReferenceReferent::Record(record),
            ..
        }
        | Type::RawPointer(RawPointerPointee::Record(record)) => record,
        Type::Intrinsic(_)
        | Type::SafeReference {
            referent: ReferenceReferent::Intrinsic(_),
            ..
        }
        | Type::RawPointer(RawPointerPointee::Intrinsic(_)) => return,
    };
    if records[record.0].accessibility == Accessibility::ModulePrivate {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::PrivateTypeInExportedSignature,
            location: location(unit, type_node),
        });
    }
}

fn type_contains_reference_or_pointer(ty: Type, records: &[Record]) -> bool {
    type_contains_reference_or_pointer_inner(ty, records, &mut BTreeSet::new())
}

fn type_contains_reference_or_pointer_inner(
    ty: Type,
    records: &[Record],
    visiting: &mut BTreeSet<RecordId>,
) -> bool {
    match ty {
        Type::Intrinsic(_) => false,
        Type::Record(record) => {
            if !visiting.insert(record) {
                return false;
            }
            let contains = records[record.0]
                .fields
                .iter()
                .any(|field| type_contains_reference_or_pointer_inner(field.ty, records, visiting));
            visiting.remove(&record);
            contains
        }
        Type::SafeReference { .. } | Type::RawPointer(_) => true,
    }
}

fn validate_safe_reference_referent(
    ty: Type,
    records: &[Record],
    type_location: SourceLocation,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Type::SafeReference {
        referent,
        permission,
    } = ty
    else {
        return true;
    };
    let referent = referent.ty();
    let valid_shape = !type_contains_reference_or_pointer(referent, records);
    let valid = valid_shape
        && (permission == ReferencePermission::ExclusiveReplace
            || type_is_duplicable_in_records(referent, records));
    if valid {
        true
    } else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::InvalidSafeReferenceReferent {
                referent,
                permission,
            },
            location: type_location,
        });
        false
    }
}

fn raw_pointer_pointee_type_is_valid(ty: Type, records: &[Record]) -> bool {
    match ty {
        Type::Intrinsic(_) => true,
        Type::Record(record) => records[record.0]
            .fields
            .iter()
            .all(|field| raw_pointer_pointee_type_is_valid(field.ty, records)),
        Type::SafeReference { .. } | Type::RawPointer(_) => false,
    }
}

fn validate_raw_pointer_pointee(
    ty: Type,
    records: &[Record],
    type_location: SourceLocation,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Type::RawPointer(pointee) = ty else {
        return true;
    };
    let pointee = pointee.ty();
    if raw_pointer_pointee_type_is_valid(pointee, records) {
        true
    } else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::InvalidRawPointerPointee { pointee },
            location: type_location,
        });
        false
    }
}

fn type_ref_reference_permission(node: &SyntaxNode) -> Option<ReferencePermission> {
    let mut has_amp = false;
    let mut has_mut = false;
    for token in node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
    {
        has_amp |= token.kind() == SyntaxKind::Amp;
        has_mut |= token.kind() == SyntaxKind::KwMut;
    }
    has_amp.then_some(if has_mut {
        ReferencePermission::ExclusiveReplace
    } else {
        ReferencePermission::Shared
    })
}

fn type_ref_is_raw_pointer(node: &SyntaxNode) -> bool {
    node.children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::KwRaw)
}

fn resolve_type(
    module: ModuleId,
    unit: usize,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let reference_permission = type_ref_reference_permission(node);
    let raw = type_ref_is_raw_pointer(node);
    debug_assert!(!(reference_permission.is_some() && raw));
    let ordinary = if let Some(qualified) = node
        .children()
        .find(|child| child.kind() == SyntaxKind::QualifiedModuleMember)
    {
        match resolve_qualified_entity(unit, &qualified, modules, imports, diagnostics)? {
            EntityId::Record(id) => Type::Record(id),
            EntityId::Function(_) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedRecordType,
                    location: location(unit, &qualified),
                });
                return None;
            }
        }
    } else {
        let token = node
            .children_with_tokens()
            .filter_map(|element| element.into_token())
            .find(|token| {
                !token.kind().is_trivia()
                    && !matches!(
                        token.kind(),
                        SyntaxKind::Amp | SyntaxKind::KwMut | SyntaxKind::KwRaw
                    )
            })
            .expect("syntax-clean type reference contains one referent token");
        let intrinsic = match token.kind() {
            SyntaxKind::TyBool => Some(IntrinsicType::Bool),
            SyntaxKind::TyI8 => Some(IntrinsicType::I8),
            SyntaxKind::TyI16 => Some(IntrinsicType::I16),
            SyntaxKind::TyI32 => Some(IntrinsicType::I32),
            SyntaxKind::TyI64 => Some(IntrinsicType::I64),
            SyntaxKind::TyU8 => Some(IntrinsicType::U8),
            SyntaxKind::TyU16 => Some(IntrinsicType::U16),
            SyntaxKind::TyU32 => Some(IntrinsicType::U32),
            SyntaxKind::TyU64 => Some(IntrinsicType::U64),
            SyntaxKind::TyF16 => Some(IntrinsicType::F16),
            SyntaxKind::TyF32 => Some(IntrinsicType::F32),
            SyntaxKind::TyF64 => Some(IntrinsicType::F64),
            _ => None,
        };
        if let Some(intrinsic) = intrinsic {
            Type::Intrinsic(intrinsic)
        } else {
            debug_assert_eq!(token.kind(), SyntaxKind::Ident);
            let name = key(&token);
            let token_location = SourceLocation {
                unit,
                range: token.text_range(),
            };
            match modules
                .get(&module)
                .and_then(|module| module.namespace.get(&name))
                .copied()
                .map(|entity| entity.entity)
            {
                Some(EntityId::Record(id)) => Type::Record(id),
                Some(EntityId::Function(_)) => {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::ExpectedRecordType,
                        location: token_location,
                    });
                    return None;
                }
                None => {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::UnresolvedName,
                        location: token_location,
                    });
                    return None;
                }
            }
        }
    };

    if let Some(permission) = reference_permission {
        let referent = match ordinary {
            Type::Intrinsic(intrinsic) => ReferenceReferent::Intrinsic(intrinsic),
            Type::Record(record) => ReferenceReferent::Record(record),
            Type::SafeReference { .. } | Type::RawPointer(_) => {
                unreachable!("source reference type syntax is non-recursive")
            }
        };
        Some(Type::SafeReference {
            referent,
            permission,
        })
    } else if raw {
        let pointee = match ordinary {
            Type::Intrinsic(intrinsic) => RawPointerPointee::Intrinsic(intrinsic),
            Type::Record(record) => RawPointerPointee::Record(record),
            Type::SafeReference { .. } | Type::RawPointer(_) => {
                unreachable!("source raw-pointer type syntax is non-recursive")
            }
        };
        Some(Type::RawPointer(pointee))
    } else {
        Some(ordinary)
    }
}

fn resolve_qualified_entity(
    unit: usize,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<EntityId> {
    let identifiers = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Ident)
        .collect::<Vec<_>>();
    let [alias_token, member_token] = identifiers.as_slice() else {
        unreachable!("syntax-clean qualified module member has two identifiers");
    };
    let alias = key(alias_token);
    let member = key(member_token);

    let Some(target_module) = imports[unit].get(&alias).copied() else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnresolvedName,
            location: SourceLocation {
                unit,
                range: alias_token.text_range(),
            },
        });
        return None;
    };

    let Some(target) = modules
        .get(&target_module)
        .and_then(|module| module.namespace.get(&member))
        .copied()
    else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnresolvedName,
            location: SourceLocation {
                unit,
                range: member_token.text_range(),
            },
        });
        return None;
    };

    if target.accessibility != Accessibility::Exported {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::InaccessibleBinding,
            location: SourceLocation {
                unit,
                range: member_token.text_range(),
            },
        });
        return None;
    }

    Some(target.entity)
}

fn validate_record_acyclicity(records: &[Record], diagnostics: &mut Vec<Diagnostic>) {
    let mut state = vec![0_u8; records.len()];
    for record in records {
        visit_record(record.id, records, &mut state, diagnostics);
    }
}

fn visit_record(
    id: RecordId,
    records: &[Record],
    state: &mut [u8],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if state[id.0] == 2 {
        return;
    }
    if state[id.0] == 1 {
        return;
    }
    state[id.0] = 1;
    for field in &records[id.0].fields {
        let Type::Record(target) = field.ty else {
            continue;
        };
        if state[target.0] == 1 {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::RecordContainmentCycle,
                location: field.location,
            });
        } else if state[target.0] == 0 {
            visit_record(target, records, state, diagnostics);
        }
    }
    state[id.0] = 2;
}

fn validate_record_duplicability(
    syntax: &[RecordSyntax],
    records: &[Record],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut resolved = vec![None; records.len()];
    for record in records {
        if record.duplicability != Duplicability::Duplicable {
            resolved[record.id.0] = Some(false);
            continue;
        }
        if !record_duplicability_is_valid(record.id, records, &mut resolved) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::InvalidRecordDuplicabilitySelection,
                location: syntax[record.id.0]
                    .duplicability_selection
                    .expect("selected record retains its copy-token location"),
            });
        }
    }
}

fn record_duplicability_is_valid(
    id: RecordId,
    records: &[Record],
    resolved: &mut [Option<bool>],
) -> bool {
    if let Some(duplicable) = resolved[id.0] {
        return duplicable;
    }
    if records[id.0].duplicability != Duplicability::Duplicable {
        resolved[id.0] = Some(false);
        return false;
    }

    let duplicable = records[id.0].fields.iter().all(|field| match field.ty {
        Type::Intrinsic(_) => true,
        Type::Record(target) => record_duplicability_is_valid(target, records, resolved),
        Type::SafeReference { .. } | Type::RawPointer(_) => false,
    });
    resolved[id.0] = Some(duplicable);
    duplicable
}

fn validate_body(
    header: &FunctionHeader,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    records: &[Record],
    headers: &[FunctionHeader],
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Body {
    let mut state = SemanticState::default();
    for (slot, parameter) in header.parameters.iter().enumerate() {
        let reference_authority = match parameter.ty {
            Type::SafeReference { permission, .. } => {
                if permission == ReferencePermission::ExclusiveReplace {
                    state
                        .external_referents
                        .insert(slot, StructuralOwnershipState::default());
                }
                Some(state.create_reference_authority(
                    ReferenceTarget::External(slot),
                    permission,
                    None,
                ))
            }
            _ => None,
        };
        state.bindings.insert(
            parameter.name.clone(),
            BindingState {
                id: parameter.binding,
                ty: parameter.ty,
                mutability: AssignmentMutability::Immutable,
                source: BindingSource::Parameter,
                ownership: StructuralOwnershipState::default(),
                reference_authority,
                pointer_origin: None,
                raw_pointer_target_domain: None,
            },
        );
    }

    let safe_reference_result_origin_authority = match header.safe_reference_result_contract {
        SafeReferenceResultContract::None => None,
        SafeReferenceResultContract::SharedIdentity { origin }
        | SafeReferenceResultContract::SharedDirectChild { origin } => Some(
            state
                .bindings
                .values()
                .find(|binding| binding.id == header.parameters[origin].binding)
                .and_then(|binding| binding.reference_authority)
                .expect("safe-reference result origin parameter has activation authority"),
        ),
    };

    let context = BodyResolutionContext {
        modules,
        imports,
        records,
        headers,
        safe_reference_result_origin_authority,
    };
    let mut control = ControlValidationContext::default();
    let value_context = ValueValidationContext::default();
    let mut statements = Vec::new();
    let mut terminal_return = None;
    let mut has_normal_continuation = true;

    for node in header.body.children() {
        if !has_normal_continuation {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnreachableStatement,
                location: location(header.unit, &node),
            });
            continue;
        }

        if node.kind() == SyntaxKind::ReturnStatement {
            terminal_return = Some(validate_terminal_return(
                header,
                &node,
                &context,
                &value_context,
                &mut state,
                diagnostics,
            ));
            has_normal_continuation = false;
            continue;
        }

        if let Some(statement) = validate_body_statement(
            header,
            &node,
            &context,
            &value_context,
            &mut state,
            &mut control,
            next_binding,
            diagnostics,
        ) {
            has_normal_continuation = statement_has_normal_continuation(&statement);
            statements.push(statement);
        }
    }

    if header.result.is_some() && has_normal_continuation {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::MissingResultReturn,
            location: header.location,
        });
    } else if header.result.is_none() && has_normal_continuation {
        validate_external_referent_restoration(header, &state, header.location, diagnostics);
    }

    debug_assert!(control.scopes.is_empty());
    debug_assert!(control.loops.is_empty());

    Body {
        statements,
        terminal_return,
        has_normal_continuation,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "source validation threads independent resolution and semantic state explicitly"
)]
fn validate_body_statement(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    control: &mut ControlValidationContext,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    match node.kind() {
        SyntaxKind::LocalDeclaration => validate_local(
            header,
            node,
            context,
            value_context,
            state,
            next_binding,
            diagnostics,
        ),
        SyntaxKind::RecordDestructuringDeclaration => validate_record_destructure(
            header,
            node,
            context,
            value_context,
            state,
            next_binding,
            diagnostics,
        ),
        SyntaxKind::AssignmentStatement => {
            validate_assignment(header, node, context, value_context, state, diagnostics)
        }
        SyntaxKind::ReferenceAssignStatement => {
            validate_reference_assign(header, node, context, value_context, state, diagnostics)
        }
        SyntaxKind::RawAssignStatement => {
            validate_raw_assign(header, node, context, value_context, state, diagnostics)
        }
        SyntaxKind::CallStatement => {
            validate_call_statement(header, node, context, value_context, state, diagnostics)
        }
        SyntaxKind::FaultStatement => Some(Statement::Fault {
            location: location(header.unit, node),
        }),
        SyntaxKind::BreakStatement => {
            validate_loop_transfer(header, node, context, state, control, true, diagnostics)
        }
        SyntaxKind::ContinueStatement => {
            validate_loop_transfer(header, node, context, state, control, false, diagnostics)
        }
        SyntaxKind::BlockStatement => Some(validate_block(
            header,
            node,
            context,
            value_context,
            state,
            control,
            next_binding,
            diagnostics,
        )),
        SyntaxKind::UnsafeBlockStatement => {
            let block = direct_child(node, SyntaxKind::BlockStatement);
            let mut unsafe_context = value_context.clone();
            unsafe_context.unsafe_active = true;
            Some(validate_block(
                header,
                &block,
                context,
                &unsafe_context,
                state,
                control,
                next_binding,
                diagnostics,
            ))
        }
        SyntaxKind::IfStatement => validate_if(
            header,
            node,
            context,
            value_context,
            state,
            control,
            next_binding,
            diagnostics,
        ),
        SyntaxKind::WhileStatement => validate_while(
            header,
            node,
            context,
            value_context,
            state,
            control,
            next_binding,
            diagnostics,
        ),
        SyntaxKind::ReturnStatement => {
            unreachable!("terminal return is handled by its containing source sequence")
        }
        _ => unreachable!("syntax-clean sequence contains only represented body nodes"),
    }
}

fn statement_has_normal_continuation(statement: &Statement) -> bool {
    match statement {
        Statement::Fault { .. } | Statement::Break { .. } | Statement::Continue { .. } => false,
        Statement::Block(block) => block.has_normal_continuation,
        Statement::If {
            then_block,
            else_block,
            ..
        } => else_block.as_ref().is_none_or(|else_block| {
            then_block.has_normal_continuation || else_block.has_normal_continuation
        }),
        Statement::While { .. }
        | Statement::Local { .. }
        | Statement::RecordDestructure { .. }
        | Statement::Assignment { .. }
        | Statement::ReferenceAssign { .. }
        | Statement::RawAssign { .. }
        | Statement::Call { .. } => true,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "source validation threads independent resolution and semantic state explicitly"
)]
fn validate_block(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    control: &mut ControlValidationContext,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Statement {
    control.scopes.push(ScopeFrame {
        direct_bindings: Vec::new(),
    });
    let mut statements = Vec::new();
    let mut terminal_return = None;
    let mut has_normal_continuation = true;

    for child in node.children() {
        if !has_normal_continuation {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnreachableStatement,
                location: location(header.unit, &child),
            });
            continue;
        }

        if child.kind() == SyntaxKind::ReturnStatement {
            terminal_return = Some(validate_terminal_return(
                header,
                &child,
                context,
                value_context,
                state,
                diagnostics,
            ));
            has_normal_continuation = false;
            continue;
        }

        let statement = validate_body_statement(
            header,
            &child,
            context,
            value_context,
            state,
            control,
            next_binding,
            diagnostics,
        );

        if let Some(statement) = statement {
            match &statement {
                Statement::Local { binding, .. } => {
                    control
                        .scopes
                        .last_mut()
                        .expect("validated block retains its lexical scope")
                        .direct_bindings
                        .push(*binding);
                }
                Statement::RecordDestructure { bindings, .. } => {
                    control
                        .scopes
                        .last_mut()
                        .expect("validated block retains its lexical scope")
                        .direct_bindings
                        .extend(bindings.iter().map(|binding| binding.binding));
                }
                Statement::Assignment { .. }
                | Statement::ReferenceAssign { .. }
                | Statement::RawAssign { .. }
                | Statement::Call { .. }
                | Statement::Fault { .. }
                | Statement::Break { .. }
                | Statement::Continue { .. }
                | Statement::Block(_)
                | Statement::If { .. }
                | Statement::While { .. } => {}
            }
            has_normal_continuation = statement_has_normal_continuation(&statement);
            statements.push(statement);
        }
    }

    let scope = control
        .scopes
        .pop()
        .expect("validated block lexical scope is balanced");
    let mut normal_cleanup = Vec::new();
    if has_normal_continuation {
        for binding in scope.direct_bindings.iter().rev() {
            let binding_state = binding_state_by_id(&state.bindings, *binding)
                .expect("validated direct child binding remains active through block end");
            for fields in remaining_ownership_frontier(
                binding_state.ty,
                &binding_state.ownership,
                context.records,
            ) {
                normal_cleanup.push(CleanupPath {
                    binding: *binding,
                    fields,
                });
            }
        }
    }

    for binding in scope.direct_bindings {
        state.release_binding_reference_carrier(binding);
        let name = state
            .bindings
            .iter()
            .find_map(|(name, binding_state)| (binding_state.id == binding).then(|| name.clone()))
            .expect("validated direct child binding is active at block end");
        let removed = state
            .bindings
            .remove(&name)
            .expect("validated direct child binding is removed at block end");
        debug_assert_eq!(removed.id, binding);
    }

    Statement::Block(Block {
        statements,
        terminal_return,
        normal_cleanup,
        has_normal_continuation,
        location: location(header.unit, node),
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "source validation threads independent resolution and semantic state explicitly"
)]
fn validate_if(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    control: &mut ControlValidationContext,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let mut post_condition = state.clone();
    let condition_node = value_child(node);
    let condition = validate_value(
        header,
        &condition_node,
        Type::Intrinsic(IntrinsicType::Bool),
        context,
        value_context,
        &mut post_condition,
        diagnostics,
    )?
    .value;

    let mut blocks = node
        .children()
        .filter(|child| child.kind() == SyntaxKind::BlockStatement);
    let then_node = blocks
        .next()
        .expect("syntax-clean conditional contains a then block");
    let else_node = blocks.next();
    debug_assert!(blocks.next().is_none());

    let mut then_state = post_condition.clone();
    let then_diagnostics = diagnostics.len();
    let then_statement = validate_block(
        header,
        &then_node,
        context,
        value_context,
        &mut then_state,
        control,
        next_binding,
        diagnostics,
    );
    let then_valid = diagnostics.len() == then_diagnostics;
    let Statement::Block(then_block) = then_statement else {
        unreachable!("block validation returns one block statement");
    };

    let mut else_state = post_condition.clone();
    let (else_block, else_valid) = if let Some(else_node) = else_node {
        let else_diagnostics = diagnostics.len();
        let else_statement = validate_block(
            header,
            &else_node,
            context,
            value_context,
            &mut else_state,
            control,
            next_binding,
            diagnostics,
        );
        let else_valid = diagnostics.len() == else_diagnostics;
        let Statement::Block(else_block) = else_statement else {
            unreachable!("block validation returns one block statement");
        };
        (Some(else_block), else_valid)
    } else {
        (None, true)
    };

    if !then_valid || !else_valid {
        return None;
    }

    let then_normal = then_block.has_normal_continuation;
    let else_normal = else_block
        .as_ref()
        .is_none_or(|else_block| else_block.has_normal_continuation);

    match (then_normal, else_normal) {
        (true, true) => {
            let ownership_equal = binding_ownership_matches_target(&then_state, &else_state);
            let pointer_equal = pointer_origins_match_target(&then_state, &else_state);
            let reference_equal = reference_state_matches_target(&then_state, &else_state);

            if !ownership_equal {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ConditionalOwnershipMismatch,
                    location: location(header.unit, node),
                });
            }
            if !pointer_equal {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ConditionalPointerOriginMismatch,
                    location: location(header.unit, node),
                });
            }
            if !reference_equal {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ConditionalReferenceStateMismatch,
                    location: location(header.unit, node),
                });
            }
            if !ownership_equal || !pointer_equal || !reference_equal {
                return None;
            }
            *state = then_state;
        }
        (true, false) => *state = then_state,
        (false, true) => *state = else_state,
        (false, false) => {}
    }

    Some(Statement::If {
        condition,
        then_block,
        else_block: else_block.map(Box::new),
        location: location(header.unit, node),
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "source validation threads independent resolution and semantic state explicitly"
)]
fn validate_while(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    control: &mut ControlValidationContext,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let loop_head = state.clone();
    let mut post_condition = loop_head.clone();
    let condition_node = value_child(node);
    let condition = validate_value(
        header,
        &condition_node,
        Type::Intrinsic(IntrinsicType::Bool),
        context,
        value_context,
        &mut post_condition,
        diagnostics,
    )?
    .value;

    let body_node = node
        .children()
        .find(|child| child.kind() == SyntaxKind::BlockStatement)
        .expect("syntax-clean while contains one body block");
    let mut body_state = post_condition.clone();
    let body_scope = control.scopes.len();
    control.loops.push(LoopTarget {
        body_scope,
        head: loop_head.clone(),
        continuation: post_condition.clone(),
    });
    let body_diagnostics = diagnostics.len();
    let body_statement = validate_block(
        header,
        &body_node,
        context,
        value_context,
        &mut body_state,
        control,
        next_binding,
        diagnostics,
    );
    let popped = control
        .loops
        .pop()
        .expect("validated while target stack is balanced");
    debug_assert_eq!(popped.body_scope, body_scope);
    let body_valid = diagnostics.len() == body_diagnostics;
    let Statement::Block(body) = body_statement else {
        unreachable!("block validation returns one block statement");
    };

    if !body_valid {
        return None;
    }

    if body.has_normal_continuation {
        let ownership_equal = binding_ownership_matches_target(&body_state, &loop_head);
        let pointer_equal = pointer_origins_match_target(&body_state, &loop_head);
        let reference_equal = reference_state_matches_target(&body_state, &loop_head);

        if !ownership_equal {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::LoopOwnershipMismatch,
                location: location(header.unit, node),
            });
        }
        if !pointer_equal {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::LoopPointerOriginMismatch,
                location: location(header.unit, node),
            });
        }
        if !reference_equal {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::LoopReferenceStateMismatch,
                location: location(header.unit, node),
            });
        }
        if !ownership_equal || !pointer_equal || !reference_equal {
            return None;
        }
    }

    *state = post_condition;
    Some(Statement::While {
        condition,
        body,
        location: location(header.unit, node),
    })
}

fn validate_loop_transfer(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    state: &SemanticState,
    control: &ControlValidationContext,
    is_break: bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let transfer_location = location(header.unit, node);
    let Some(target) = control.loops.last() else {
        diagnostics.push(Diagnostic {
            kind: if is_break {
                DiagnosticKind::BreakOutsideLoop
            } else {
                DiagnosticKind::ContinueOutsideLoop
            },
            location: transfer_location,
        });
        return None;
    };

    let required = if is_break {
        &target.continuation
    } else {
        &target.head
    };
    let mut transferred = state.clone();
    release_transfer_reference_carriers(&mut transferred, control, target.body_scope);
    let ownership_equal = binding_ownership_matches_target(&transferred, required);
    let pointer_equal = pointer_origins_match_target(&transferred, required);
    let reference_equal = reference_state_matches_target(&transferred, required);
    if !ownership_equal {
        diagnostics.push(Diagnostic {
            kind: if is_break {
                DiagnosticKind::BreakOwnershipMismatch
            } else {
                DiagnosticKind::ContinueOwnershipMismatch
            },
            location: transfer_location,
        });
    }
    if !pointer_equal {
        diagnostics.push(Diagnostic {
            kind: if is_break {
                DiagnosticKind::BreakPointerOriginMismatch
            } else {
                DiagnosticKind::ContinuePointerOriginMismatch
            },
            location: transfer_location,
        });
    }
    if !reference_equal {
        diagnostics.push(Diagnostic {
            kind: if is_break {
                DiagnosticKind::BreakReferenceStateMismatch
            } else {
                DiagnosticKind::ContinueReferenceStateMismatch
            },
            location: transfer_location,
        });
    }
    if !ownership_equal || !pointer_equal || !reference_equal {
        return None;
    }

    let cleanup = transfer_cleanup(&state.bindings, context.records, control, target.body_scope);
    Some(if is_break {
        Statement::Break {
            cleanup,
            location: transfer_location,
        }
    } else {
        Statement::Continue {
            cleanup,
            location: transfer_location,
        }
    })
}

fn release_transfer_reference_carriers(
    state: &mut SemanticState,
    control: &ControlValidationContext,
    body_scope: usize,
) {
    debug_assert!(body_scope < control.scopes.len());
    let bindings = control.scopes[body_scope..]
        .iter()
        .rev()
        .flat_map(|scope| scope.direct_bindings.iter().rev().copied())
        .collect::<Vec<_>>();
    for binding in bindings {
        state.release_binding_reference_carrier(binding);
    }
}

fn binding_ownership_matches_target(actual: &SemanticState, required: &SemanticState) -> bool {
    required.bindings.values().all(|expected| {
        binding_state_by_id(&actual.bindings, expected.id)
            .is_some_and(|candidate| candidate.ownership == expected.ownership)
    })
}

fn pointer_origins_match_target(actual: &SemanticState, required: &SemanticState) -> bool {
    required.bindings.values().all(|expected| {
        binding_state_by_id(&actual.bindings, expected.id).is_some_and(|candidate| {
            debug_assert_eq!(candidate.ty, expected.ty);
            debug_assert_eq!(
                candidate.raw_pointer_target_domain,
                expected.raw_pointer_target_domain
            );
            candidate.pointer_origin == expected.pointer_origin
        })
    })
}

fn reference_state_matches_target(actual: &SemanticState, required: &SemanticState) -> bool {
    if actual.external_referents != required.external_referents
        || actual.reference_authorities != required.reference_authorities
    {
        return false;
    }
    required.bindings.values().all(|expected| {
        binding_state_by_id(&actual.bindings, expected.id)
            .is_some_and(|candidate| candidate.reference_authority == expected.reference_authority)
    })
}

fn transfer_cleanup(
    bindings: &BTreeMap<String, BindingState>,
    records: &[Record],
    control: &ControlValidationContext,
    body_scope: usize,
) -> Vec<CleanupPath> {
    debug_assert!(body_scope < control.scopes.len());
    let mut cleanup = Vec::new();
    for scope in control.scopes[body_scope..].iter().rev() {
        for binding in scope.direct_bindings.iter().rev() {
            let state = binding_state_by_id(bindings, *binding)
                .expect("active transfer scope retains every direct binding");
            for fields in remaining_ownership_frontier(state.ty, &state.ownership, records) {
                cleanup.push(CleanupPath {
                    binding: *binding,
                    fields,
                });
            }
        }
    }
    cleanup
}

fn binding_state_by_id(
    bindings: &BTreeMap<String, BindingState>,
    binding: BindingId,
) -> Option<&BindingState> {
    bindings.values().find(|state| state.id == binding)
}

fn binding_state_by_id_mut(
    bindings: &mut BTreeMap<String, BindingState>,
    binding: BindingId,
) -> Option<&mut BindingState> {
    bindings.values_mut().find(|state| state.id == binding)
}

fn remaining_ownership_frontier(
    ty: Type,
    state: &StructuralOwnershipState,
    records: &[Record],
) -> Vec<Vec<usize>> {
    let mut frontier = Vec::new();
    let mut path = Vec::new();
    append_remaining_ownership_frontier(state, ty, records, &mut path, &mut frontier);
    frontier
}

fn append_remaining_ownership_frontier(
    state: &StructuralOwnershipState,
    ty: Type,
    records: &[Record],
    path: &mut Vec<usize>,
    frontier: &mut Vec<Vec<usize>>,
) {
    match state.path_availability(path) {
        PathAvailability::Unavailable => {}
        PathAvailability::FullyAvailable => frontier.push(path.clone()),
        PathAvailability::PartiallyAvailable => {
            let Type::Record(record) = ty else {
                unreachable!("only record paths can contain consumed descendants");
            };
            for (field_index, field) in records[record.0].fields.iter().enumerate().rev() {
                path.push(field_index);
                append_remaining_ownership_frontier(state, field.ty, records, path, frontier);
                path.pop();
            }
        }
    }
}

fn validate_local(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let mut candidate = state.clone();
    let name_token = direct_token(node, SyntaxKind::Ident);
    let name = key(&name_token);
    let mutability = if node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::KwMut)
    {
        AssignmentMutability::Mutable
    } else {
        AssignmentMutability::Immutable
    };
    let local_location = location(header.unit, node);
    let type_node = direct_child(node, SyntaxKind::TypeRef);
    let declared = resolve_type(
        header.module,
        header.unit,
        &type_node,
        context.modules,
        context.imports,
        diagnostics,
    )
    .and_then(|ty| {
        if matches!(ty, Type::SafeReference { .. }) {
            if mutability.is_mutable() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::MutableSafeReferenceLocal,
                    location: location(header.unit, &type_node),
                });
                return None;
            }
            if !validate_safe_reference_referent(
                ty,
                context.records,
                location(header.unit, &type_node),
                diagnostics,
            ) {
                return None;
            }
        }
        if !validate_raw_pointer_pointee(
            ty,
            context.records,
            location(header.unit, &type_node),
            diagnostics,
        ) {
            return None;
        }
        Some(ty)
    });
    let raw_pointer_target_domain = declared
        .is_some_and(|ty| matches!(ty, Type::RawPointer(_)))
        .then(|| {
            candidate
                .bindings
                .values()
                .map(|binding| binding.id)
                .collect::<BTreeSet<_>>()
        });
    let initializer = declared.and_then(|required| {
        let value_node = value_child(node);
        validate_value(
            header,
            &value_node,
            required,
            context,
            value_context,
            &mut candidate,
            diagnostics,
        )
    });

    let shadows = candidate.bindings.contains_key(&name);
    if shadows {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::LocalShadowing,
            location: SourceLocation {
                unit: header.unit,
                range: name_token.text_range(),
            },
        });
    }

    let (Some(ty), Some(initializer)) = (declared, initializer) else {
        return None;
    };
    if shadows {
        return None;
    }

    let pointer_origin = pointer_origin_for_value(&initializer.value, &candidate.bindings);
    debug_assert_eq!(pointer_origin.is_some(), matches!(ty, Type::RawPointer(_)));
    if let (Some(origin), Some(domain)) = (pointer_origin, raw_pointer_target_domain.as_ref())
        && !domain.contains(&origin)
    {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::RawPointerTargetExtentMismatch,
            location: initializer.value.location,
        });
        return None;
    }

    let reference_authority = initializer.reference_authority;
    debug_assert_eq!(
        reference_authority.is_some(),
        matches!(ty, Type::SafeReference { .. })
    );
    let binding = BindingId(*next_binding);
    *next_binding += 1;
    candidate.bindings.insert(
        name.clone(),
        BindingState {
            id: binding,
            ty,
            mutability,
            source: BindingSource::Local,
            ownership: StructuralOwnershipState::default(),
            reference_authority,
            pointer_origin,
            raw_pointer_target_domain,
        },
    );
    let statement = Statement::Local {
        binding,
        name,
        ty,
        mutability,
        initializer: initializer.value,
        location: local_location,
    };
    *state = candidate;
    Some(statement)
}

fn pointer_origin_for_value(
    value: &Value,
    bindings: &BTreeMap<String, BindingState>,
) -> Option<BindingId> {
    match &value.kind {
        ValueKind::RawAddressRoot { target } => Some(*target),
        ValueKind::BindingUse { binding, .. } if matches!(value.ty, Type::RawPointer(_)) => {
            binding_state_by_id(bindings, *binding)
                .expect("validated raw-pointer binding remains active")
                .pointer_origin
        }
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct ResolvedPatternBinding {
    fields: Vec<usize>,
    name: String,
    ty: Type,
    location: SourceLocation,
}

#[derive(Debug)]
struct PatternValidation {
    valid: bool,
    bindings: Vec<ResolvedPatternBinding>,
    seen_binding_names: BTreeSet<String>,
    active_binding_names: BTreeSet<String>,
}

impl PatternValidation {
    fn new(active_bindings: &BTreeMap<String, BindingState>) -> Self {
        Self {
            valid: true,
            bindings: Vec::new(),
            seen_binding_names: BTreeSet::new(),
            active_binding_names: active_bindings.keys().cloned().collect(),
        }
    }
}

fn validate_record_pattern_node(
    header: &FunctionHeader,
    node: &SyntaxNode,
    expected: Option<Type>,
    context: &BodyResolutionContext<'_>,
    path: &mut Vec<usize>,
    validation: &mut PatternValidation,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<RecordId> {
    debug_assert_eq!(node.kind(), SyntaxKind::RecordPattern);

    let (record, head_location) = if let Some(qualified) = node
        .children()
        .find(|child| child.kind() == SyntaxKind::QualifiedModuleMember)
    {
        let head_location = location(header.unit, &qualified);
        let Some(entity) = resolve_qualified_entity(
            header.unit,
            &qualified,
            context.modules,
            context.imports,
            diagnostics,
        ) else {
            validation.valid = false;
            return None;
        };
        let record = match entity {
            EntityId::Record(record) => record,
            EntityId::Function(_) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedRecordType,
                    location: head_location,
                });
                validation.valid = false;
                return None;
            }
        };
        (record, head_location)
    } else {
        let head_token = direct_token(node, SyntaxKind::Ident);
        let head_name = key(&head_token);
        let head_location = SourceLocation {
            unit: header.unit,
            range: head_token.text_range(),
        };
        let record = match context
            .modules
            .get(&header.module)
            .and_then(|module| module.namespace.get(&head_name))
            .copied()
            .map(|entity| entity.entity)
        {
            Some(EntityId::Record(record)) => record,
            Some(EntityId::Function(_)) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedRecordType,
                    location: head_location,
                });
                validation.valid = false;
                return None;
            }
            None => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::UnresolvedName,
                    location: head_location,
                });
                validation.valid = false;
                return None;
            }
        };
        (record, head_location)
    };

    let record_ty = Type::Record(record);
    if let Some(expected) = expected
        && expected != record_ty
    {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::TypeMismatch {
                expected,
                found: record_ty,
            },
            location: head_location,
        });
        validation.valid = false;
    }

    let record_decl = &context.records[record.0];
    let has_rest = node
        .children()
        .any(|child| child.kind() == SyntaxKind::RecordPatternRest);
    let mut seen_fields = BTreeSet::<usize>::new();
    for pattern_field in node
        .children()
        .filter(|child| child.kind() == SyntaxKind::RecordPatternField)
    {
        let field_token = direct_token(&pattern_field, SyntaxKind::Ident);
        let field_name = key(&field_token);
        let field_location = SourceLocation {
            unit: header.unit,
            range: field_token.text_range(),
        };

        let Some(field) = record_decl
            .fields
            .iter()
            .position(|field| field.name == field_name)
        else {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnknownRecordField,
                location: field_location,
            });
            validation.valid = false;
            continue;
        };

        if !seen_fields.insert(field) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::DuplicateRecordPatternField,
                location: field_location,
            });
            validation.valid = false;
        }

        if !record_field_is_accessible(header.module, record_decl, field) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::InaccessibleRecordField,
                location: field_location,
            });
            validation.valid = false;
        }

        path.push(field);
        if let Some(nested) = pattern_field
            .children()
            .find(|child| child.kind() == SyntaxKind::RecordPattern)
        {
            validate_record_pattern_node(
                header,
                &nested,
                Some(record_decl.fields[field].ty),
                context,
                path,
                validation,
                diagnostics,
            );
        } else {
            let identifiers = pattern_field
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .filter(|token| token.kind() == SyntaxKind::Ident)
                .collect::<Vec<_>>();
            let [_, binding_token] = identifiers.as_slice() else {
                unreachable!("syntax-clean binding leaf has field and binding identifiers");
            };
            let binding_name = key(binding_token);
            let binding_location = SourceLocation {
                unit: header.unit,
                range: binding_token.text_range(),
            };
            if !validation.seen_binding_names.insert(binding_name.clone()) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::DuplicatePatternBinding,
                    location: binding_location,
                });
                validation.valid = false;
            }
            if validation.active_binding_names.contains(&binding_name) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::LocalShadowing,
                    location: binding_location,
                });
                validation.valid = false;
            }
            validation.bindings.push(ResolvedPatternBinding {
                fields: path.clone(),
                name: binding_name,
                ty: record_decl.fields[field].ty,
                location: field_location,
            });
        }
        path.pop();
    }

    if !has_rest && seen_fields.len() != record_decl.fields.len() {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::MissingRecordPatternField,
            location: location(header.unit, node),
        });
        validation.valid = false;
    }

    Some(record)
}

fn validate_record_destructure(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let pattern_node = direct_child(node, SyntaxKind::RecordPattern);
    let producer_node = node.children().find(|child| {
        matches!(
            child.kind(),
            SyntaxKind::DirectCall | SyntaxKind::RecordConstruction | SyntaxKind::FieldValueUse
        )
    });
    let direct_root_token = producer_node
        .is_none()
        .then(|| direct_token(node, SyntaxKind::Ident));

    let mut validation = PatternValidation::new(&state.bindings);
    let mut path = Vec::new();
    let record = validate_record_pattern_node(
        header,
        &pattern_node,
        None,
        context,
        &mut path,
        &mut validation,
        diagnostics,
    )?;

    let direct_root = direct_root_token.map(|root_token| {
        let root_name = key(&root_token);
        let root_location = SourceLocation {
            unit: header.unit,
            range: root_token.text_range(),
        };
        (root_name, root_location)
    });
    let root_state = if let Some((root_name, root_location)) = &direct_root {
        match state.bindings.get(root_name).cloned() {
            Some(binding) => Some(binding),
            None => {
                let entity = context
                    .modules
                    .get(&header.module)
                    .and_then(|module| module.namespace.get(root_name));
                diagnostics.push(Diagnostic {
                    kind: if entity.is_some() {
                        DiagnosticKind::ExpectedValueBinding
                    } else {
                        DiagnosticKind::UnresolvedName
                    },
                    location: *root_location,
                });
                return None;
            }
        }
    } else {
        None
    };

    if let (Some(root_state), Some((_, root_location))) = (&root_state, &direct_root) {
        if root_state.ty != Type::Record(record) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::TypeMismatch {
                    expected: Type::Record(record),
                    found: root_state.ty,
                },
                location: *root_location,
            });
            validation.valid = false;
        } else {
            for leaf in &validation.bindings {
                if root_state.ownership.path_availability(&leaf.fields)
                    != PathAvailability::FullyAvailable
                {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::UnavailableFieldValue,
                        location: leaf.location,
                    });
                    validation.valid = false;
                    continue;
                }
                let target = ReferenceTarget::local(root_state.id, &leaf.fields);
                let compatible = if context.type_is_duplicable(leaf.ty) {
                    state.target_satisfies_shared_requirement(&target)
                } else {
                    state.target_satisfies_exclusive_requirement(&target)
                };
                if !compatible {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::ReferencePermissionUnavailable,
                        location: leaf.location,
                    });
                    validation.valid = false;
                }
            }
        }
    }

    if !validation.valid {
        return None;
    }

    let scrutinee = if direct_root.is_some() {
        RecordPatternScrutinee::DirectRoot(
            root_state
                .as_ref()
                .expect("validated direct-root pattern has root state")
                .id,
        )
    } else {
        let producer_node =
            producer_node.expect("syntax-clean producer-backed pattern has producer");
        let value = validate_value(
            header,
            &producer_node,
            Type::Record(record),
            context,
            value_context,
            state,
            diagnostics,
        )?
        .value;

        let mut transient = StructuralOwnershipState::default();
        for leaf in &validation.bindings {
            if !context.type_is_duplicable(leaf.ty) {
                transient.consume_path(&leaf.fields);
            }
        }
        let cleanup = RecordPatternTransientCleanup {
            paths: remaining_ownership_frontier(Type::Record(record), &transient, context.records),
        };
        RecordPatternScrutinee::Producer { value, cleanup }
    };

    let mut pattern_bindings = Vec::with_capacity(validation.bindings.len());
    let mut new_states = Vec::with_capacity(validation.bindings.len());
    for resolved in validation.bindings {
        let ownership = if context.type_is_duplicable(resolved.ty) {
            OwnedUse::Duplicate
        } else {
            if let Some((root_name, _)) = &direct_root {
                state
                    .bindings
                    .get_mut(root_name)
                    .expect("validated pattern root remains in active binding scope")
                    .ownership
                    .consume_path(&resolved.fields);
            }
            OwnedUse::Consume
        };
        let binding = BindingId(*next_binding);
        *next_binding += 1;
        pattern_bindings.push(RecordPatternBinding {
            fields: resolved.fields,
            binding,
            name: resolved.name.clone(),
            ty: resolved.ty,
            ownership,
        });
        new_states.push((
            resolved.name,
            BindingState {
                id: binding,
                ty: resolved.ty,
                mutability: AssignmentMutability::Immutable,
                source: BindingSource::Local,
                ownership: StructuralOwnershipState::default(),
                reference_authority: None,
                pointer_origin: None,
                raw_pointer_target_domain: None,
            },
        ));
    }

    for (name, binding_state) in new_states {
        let previous = state.bindings.insert(name, binding_state);
        debug_assert!(previous.is_none());
    }

    Some(Statement::RecordDestructure {
        record,
        scrutinee,
        bindings: pattern_bindings,
        location: location(header.unit, node),
    })
}

fn validate_assignment(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let target_token = direct_token(node, SyntaxKind::Ident);
    let name = key(&target_token);
    let target_location = SourceLocation {
        unit: header.unit,
        range: target_token.text_range(),
    };

    let (target, target_ty, target_domain) = match state.bindings.get(&name) {
        Some(binding) => {
            if !binding.mutability.is_mutable() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ImmutableAssignmentTarget,
                    location: target_location,
                });
                return None;
            }
            (
                binding.id,
                binding.ty,
                binding.raw_pointer_target_domain.clone(),
            )
        }
        None => {
            let entity = context
                .modules
                .get(&header.module)
                .and_then(|module| module.namespace.get(&name));
            diagnostics.push(Diagnostic {
                kind: if entity.is_some() {
                    DiagnosticKind::ExpectedValueBinding
                } else {
                    DiagnosticKind::UnresolvedName
                },
                location: target_location,
            });
            return None;
        }
    };

    let value_node = value_child(node);
    let value = validate_value(
        header,
        &value_node,
        target_ty,
        context,
        value_context,
        state,
        diagnostics,
    )?;

    let reference_target = ReferenceTarget::local_root(target);
    if !state.target_satisfies_exclusive_requirement(&reference_target) {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::BorrowedAssignmentTarget,
            location: target_location,
        });
        return None;
    }

    let incoming_pointer_origin = if matches!(target_ty, Type::RawPointer(_)) {
        let origin = pointer_origin_for_value(&value.value, &state.bindings)
            .expect("validated raw-pointer value retains exact source origin");
        let domain = target_domain
            .as_ref()
            .expect("validated raw-pointer local retains lexical target domain");
        if !domain.contains(&origin) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::RawPointerTargetExtentMismatch,
                location: value.value.location,
            });
            return None;
        }
        Some(origin)
    } else {
        None
    };

    let target_state = state
        .bindings
        .get_mut(&name)
        .expect("resolved assignment target remains in the active binding scope");
    target_state.ownership = StructuralOwnershipState::default();
    if matches!(target_ty, Type::RawPointer(_)) {
        target_state.pointer_origin = incoming_pointer_origin;
    }

    Some(Statement::Assignment {
        target,
        value: value.value,
        location: location(header.unit, node),
    })
}

fn validate_reference_assign(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let statement_location = location(header.unit, node);
    let token = direct_token(node, SyntaxKind::Ident);
    let name = key(&token);
    let reference_location = SourceLocation {
        unit: header.unit,
        range: token.text_range(),
    };
    let (binding, referent, original_authority) = match state.bindings.get(&name) {
        Some(binding) => {
            if binding.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ReferenceReplacementUnavailable,
                    location: reference_location,
                });
                return None;
            }
            let Type::SafeReference {
                referent,
                permission: ReferencePermission::ExclusiveReplace,
            } = binding.ty
            else {
                diagnostics.push(Diagnostic {
                    kind: if matches!(binding.ty, Type::SafeReference { .. }) {
                        DiagnosticKind::ReferenceReplacementUnavailable
                    } else {
                        DiagnosticKind::ExpectedSafeReference
                    },
                    location: reference_location,
                });
                return None;
            };
            (
                binding.id,
                referent,
                binding
                    .reference_authority
                    .expect("live replacement reference binding has authority"),
            )
        }
        None => {
            let entity = context
                .modules
                .get(&header.module)
                .and_then(|module| module.namespace.get(&name));
            diagnostics.push(Diagnostic {
                kind: if entity.is_some() {
                    DiagnosticKind::ExpectedValueBinding
                } else {
                    DiagnosticKind::UnresolvedName
                },
                location: reference_location,
            });
            return None;
        }
    };
    let target = state.reference_target(original_authority);

    let value_node = value_child(node);
    let value = validate_value(
        header,
        &value_node,
        referent.ty(),
        context,
        value_context,
        state,
        diagnostics,
    )?;

    let destination_usable = state.bindings.get(&name).is_some_and(|candidate| {
        candidate.id == binding
            && candidate.ownership.path_availability(&[]) == PathAvailability::FullyAvailable
            && candidate.reference_authority == Some(original_authority)
    });
    if !destination_usable
        || !state.authority_satisfies(original_authority, ReferencePermission::ExclusiveReplace)
    {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ReferenceReplacementUnavailable,
            location: statement_location,
        });
        return None;
    }

    state.restore_reference_target(&target);
    Some(Statement::ReferenceAssign {
        reference: binding,
        value: value.value,
        location: statement_location,
    })
}

fn validate_raw_assign(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let statement_location = location(header.unit, node);
    if !value_context.unsafe_active {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnsafeOperationOutsideUnsafeBlock,
            location: statement_location,
        });
        return None;
    }

    let identifiers = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Ident)
        .collect::<Vec<_>>();
    let [_, pointer_token] = identifiers.as_slice() else {
        unreachable!("syntax-clean raw assignment has contextual assign and pointer identifiers");
    };
    let pointer_name = key(pointer_token);
    let pointer_location = SourceLocation {
        unit: header.unit,
        range: pointer_token.text_range(),
    };
    let (pointer, pointee, target) = match state.bindings.get(&pointer_name) {
        Some(binding) => {
            if binding.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::UnavailableBinding,
                    location: pointer_location,
                });
                return None;
            }
            let Type::RawPointer(pointee) = binding.ty else {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedRawPointer,
                    location: pointer_location,
                });
                return None;
            };
            (
                binding.id,
                pointee,
                binding
                    .pointer_origin
                    .expect("validated raw-pointer binding retains exact source origin"),
            )
        }
        None => {
            let entity = context
                .modules
                .get(&header.module)
                .and_then(|module| module.namespace.get(&pointer_name));
            diagnostics.push(Diagnostic {
                kind: if entity.is_some() {
                    DiagnosticKind::ExpectedValueBinding
                } else {
                    DiagnosticKind::UnresolvedName
                },
                location: pointer_location,
            });
            return None;
        }
    };

    let value_node = value_child(node);
    let value = validate_value(
        header,
        &value_node,
        pointee.ty(),
        context,
        value_context,
        state,
        diagnostics,
    )?;

    let reference_target = ReferenceTarget::local_root(target);
    if !state.target_satisfies_exclusive_requirement(&reference_target) {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::RawTargetSafeAuthorityConflict,
            location: statement_location,
        });
        return None;
    }

    binding_state_by_id_mut(&mut state.bindings, target)
        .expect("validated raw-pointer origin names an active target binding")
        .ownership = StructuralOwnershipState::default();

    Some(Statement::RawAssign {
        pointer,
        value: value.value,
        location: statement_location,
    })
}

fn validate_call_statement(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let mut candidate = state.clone();
    let call = direct_child(node, SyntaxKind::DirectCall);
    let validated = validate_call(
        header,
        &call,
        context,
        value_context,
        &mut candidate,
        diagnostics,
    )?;
    if validated.result.is_some() {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ResultCallUsedAsStatement,
            location: location(header.unit, &call),
        });
        return None;
    }
    *state = candidate;
    Some(Statement::Call {
        function: validated.function,
        arguments: validated.arguments,
        location: location(header.unit, node),
    })
}

fn validate_terminal_return(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Return {
    let returned = validate_return(header, node, context, value_context, state, diagnostics);
    if header.result.is_some() && returned.value.is_none() {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ExpectedResultValue,
            location: returned.location,
        });
    }
    returned
}

fn validate_return(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Return {
    let return_location = location(header.unit, node);
    let value_node = node.children().find(|child| is_value_node(child.kind()));
    let produced = match (header.result, value_node) {
        (Some(required), Some(value_node)) => validate_value(
            header,
            &value_node,
            required,
            context,
            value_context,
            state,
            diagnostics,
        ),
        (None, Some(_)) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnexpectedResultValue,
                location: return_location,
            });
            None
        }
        (_, None) => None,
    };

    if let Some(produced) = produced.as_ref() {
        let valid_reference_result = match header.safe_reference_result_contract {
            SafeReferenceResultContract::None => true,
            SafeReferenceResultContract::SharedIdentity { .. } => {
                produced.reference_authority == context.safe_reference_result_origin_authority
            }
            SafeReferenceResultContract::SharedDirectChild { .. } => {
                let origin = context
                    .safe_reference_result_origin_authority
                    .expect("direct-child result contract has activation origin authority");
                match (
                    produced.reference_authority,
                    state.reference_authorities.get(&origin),
                ) {
                    (Some(authority), Some(origin_state)) => state
                        .reference_authorities
                        .get(&authority)
                        .is_some_and(|result_state| {
                            result_state.permission == ReferencePermission::Shared
                                && result_state.parent == Some(origin)
                                && result_state.target == origin_state.target
                                && state.authority_satisfies(authority, ReferencePermission::Shared)
                        }),
                    _ => false,
                }
            }
        };
        if !valid_reference_result {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::SharedReferenceResultOriginMismatch,
                location: produced.value.location,
            });
        }
    }

    validate_external_referent_restoration(header, state, return_location, diagnostics);

    Return {
        value: produced.map(|produced| produced.value),
        location: return_location,
    }
}

fn validate_external_referent_restoration(
    header: &FunctionHeader,
    state: &SemanticState,
    location: SourceLocation,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (slot, parameter) in header.parameters.iter().enumerate() {
        if matches!(
            parameter.ty,
            Type::SafeReference {
                permission: ReferencePermission::ExclusiveReplace,
                ..
            }
        ) && state
            .external_referents
            .get(&slot)
            .is_some_and(|root| root.path_availability(&[]) != PathAvailability::FullyAvailable)
        {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ReferenceRestorationRequired,
                location,
            });
        }
    }
}

fn validate_value(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let mut candidate = state.clone();
    let value = validate_value_inner(
        header,
        node,
        required,
        context,
        value_context,
        &mut candidate,
        diagnostics,
    )?;
    *state = candidate;
    Some(value)
}

fn validate_value_inner(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    match node.kind() {
        SyntaxKind::GroupedValue => {
            let inner = value_child(node);
            validate_value(
                header,
                &inner,
                required,
                context,
                value_context,
                state,
                diagnostics,
            )
        }
        SyntaxKind::SafeReferenceValue => {
            validate_safe_reference_value(header, node, required, context, state, diagnostics)
        }
        SyntaxKind::ReferenceDereferenceValue => {
            validate_reference_dereference(header, node, required, context, state, diagnostics)
        }
        SyntaxKind::RawAddressOfValue => {
            validate_raw_address(header, node, required, context, state, diagnostics)
        }
        SyntaxKind::RawMoveValue => validate_raw_move(
            header,
            node,
            required,
            context,
            value_context,
            state,
            diagnostics,
        ),
        SyntaxKind::NumericContractSelectedValue => validate_numeric_contract_selected_value(
            header,
            node,
            required,
            context,
            value_context,
            state,
            diagnostics,
        ),
        SyntaxKind::IntegerNegValue => {
            if !matches!(
                required,
                Type::Intrinsic(
                    IntrinsicType::I8
                        | IntrinsicType::I16
                        | IntrinsicType::I32
                        | IntrinsicType::I64
                        | IntrinsicType::U8
                        | IntrinsicType::U16
                        | IntrinsicType::U32
                        | IntrinsicType::U64
                )
            ) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::IntegerNegationRequiresInteger { required },
                    location: value_location,
                });
                return None;
            }

            let operand_node = value_child(node);
            let operand = validate_value(
                header,
                &operand_node,
                required,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            Some(ProducedValue::ordinary(Value {
                ty: required,
                kind: ValueKind::IntegerNeg {
                    operand: Box::new(operand),
                },
                location: value_location,
            }))
        }
        SyntaxKind::IntegerComplementValue => {
            if !matches!(
                required,
                Type::Intrinsic(
                    IntrinsicType::I8
                        | IntrinsicType::I16
                        | IntrinsicType::I32
                        | IntrinsicType::I64
                        | IntrinsicType::U8
                        | IntrinsicType::U16
                        | IntrinsicType::U32
                        | IntrinsicType::U64
                )
            ) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::IntegerComplementRequiresInteger { required },
                    location: value_location,
                });
                return None;
            }

            let operand_node = value_child(node);
            let operand = validate_value(
                header,
                &operand_node,
                required,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            Some(ProducedValue::ordinary(Value {
                ty: required,
                kind: ValueKind::IntegerComplement {
                    operand: Box::new(operand),
                },
                location: value_location,
            }))
        }
        SyntaxKind::AddValue => match required {
            Type::Intrinsic(IntrinsicType::F16 | IntrinsicType::F32 | IntrinsicType::F64) => {
                validate_float_add(
                    header,
                    node,
                    required,
                    NumericContract::Standard,
                    context,
                    value_context,
                    state,
                    diagnostics,
                )
            }
            Type::Intrinsic(
                IntrinsicType::I8
                | IntrinsicType::I16
                | IntrinsicType::I32
                | IntrinsicType::I64
                | IntrinsicType::U8
                | IntrinsicType::U16
                | IntrinsicType::U32
                | IntrinsicType::U64,
            ) => {
                let mut operands = node.children().filter(|child| is_value_node(child.kind()));
                let left_node = operands
                    .next()
                    .expect("syntax-clean addition contains a left operand");
                let right_node = operands
                    .next()
                    .expect("syntax-clean addition contains a right operand");
                debug_assert!(operands.next().is_none());
                let left = validate_value(
                    header,
                    &left_node,
                    required,
                    context,
                    value_context,
                    state,
                    diagnostics,
                )?
                .value;
                let right = validate_value(
                    header,
                    &right_node,
                    required,
                    context,
                    value_context,
                    state,
                    diagnostics,
                )?
                .value;
                Some(ProducedValue::ordinary(Value {
                    ty: required,
                    kind: ValueKind::IntegerAdd {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    location: value_location,
                }))
            }
            _ => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::AdditionRequiresIntegerOrFloating { required },
                    location: value_location,
                });
                None
            }
        },
        SyntaxKind::SubValue => match required {
            Type::Intrinsic(IntrinsicType::F16 | IntrinsicType::F32 | IntrinsicType::F64) => {
                validate_float_sub(
                    header,
                    node,
                    required,
                    NumericContract::Standard,
                    context,
                    value_context,
                    state,
                    diagnostics,
                )
            }
            Type::Intrinsic(
                IntrinsicType::I8
                | IntrinsicType::I16
                | IntrinsicType::I32
                | IntrinsicType::I64
                | IntrinsicType::U8
                | IntrinsicType::U16
                | IntrinsicType::U32
                | IntrinsicType::U64,
            ) => {
                let mut operands = node.children().filter(|child| is_value_node(child.kind()));
                let left_node = operands
                    .next()
                    .expect("syntax-clean subtraction contains a left operand");
                let right_node = operands
                    .next()
                    .expect("syntax-clean subtraction contains a right operand");
                debug_assert!(operands.next().is_none());
                let left = validate_value(
                    header,
                    &left_node,
                    required,
                    context,
                    value_context,
                    state,
                    diagnostics,
                )?
                .value;
                let right = validate_value(
                    header,
                    &right_node,
                    required,
                    context,
                    value_context,
                    state,
                    diagnostics,
                )?
                .value;
                Some(ProducedValue::ordinary(Value {
                    ty: required,
                    kind: ValueKind::IntegerSub {
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    location: value_location,
                }))
            }
            _ => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::SubtractionRequiresIntegerOrFloating { required },
                    location: value_location,
                });
                None
            }
        },
        SyntaxKind::MulValue => {
            let operator = node
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .find(|token| matches!(token.kind(), SyntaxKind::Star | SyntaxKind::Slash))
                .expect("syntax-clean multiplicative value contains one operator token")
                .kind();
            match (operator, required) {
                (
                    SyntaxKind::Slash,
                    Type::Intrinsic(IntrinsicType::F16 | IntrinsicType::F32 | IntrinsicType::F64),
                ) => validate_float_div(
                    header,
                    node,
                    required,
                    NumericContract::Standard,
                    context,
                    value_context,
                    state,
                    diagnostics,
                ),
                (SyntaxKind::Slash, _) => {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::DivisionRequiresFloating { required },
                        location: value_location,
                    });
                    None
                }
                (
                    SyntaxKind::Star,
                    Type::Intrinsic(IntrinsicType::F16 | IntrinsicType::F32 | IntrinsicType::F64),
                ) => validate_float_mul(
                    header,
                    node,
                    required,
                    NumericContract::Standard,
                    context,
                    value_context,
                    state,
                    diagnostics,
                ),
                (
                    SyntaxKind::Star,
                    Type::Intrinsic(
                        IntrinsicType::I8
                        | IntrinsicType::I16
                        | IntrinsicType::I32
                        | IntrinsicType::I64
                        | IntrinsicType::U8
                        | IntrinsicType::U16
                        | IntrinsicType::U32
                        | IntrinsicType::U64,
                    ),
                ) => {
                    let mut operands = node.children().filter(|child| is_value_node(child.kind()));
                    let left_node = operands
                        .next()
                        .expect("syntax-clean integer multiplication contains a left operand");
                    let right_node = operands
                        .next()
                        .expect("syntax-clean integer multiplication contains a right operand");
                    debug_assert!(operands.next().is_none());
                    let left = validate_value(
                        header,
                        &left_node,
                        required,
                        context,
                        value_context,
                        state,
                        diagnostics,
                    )?
                    .value;
                    let right = validate_value(
                        header,
                        &right_node,
                        required,
                        context,
                        value_context,
                        state,
                        diagnostics,
                    )?
                    .value;
                    Some(ProducedValue::ordinary(Value {
                        ty: required,
                        kind: ValueKind::IntegerMul {
                            left: Box::new(left),
                            right: Box::new(right),
                        },
                        location: value_location,
                    }))
                }
                (SyntaxKind::Star, _) => {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::MultiplicationRequiresIntegerOrFloating { required },
                        location: value_location,
                    });
                    None
                }
                _ => unreachable!("multiplicative operator has accepted token kind"),
            }
        }
        SyntaxKind::IntegerXorValue => {
            if !matches!(
                required,
                Type::Intrinsic(
                    IntrinsicType::I8
                        | IntrinsicType::I16
                        | IntrinsicType::I32
                        | IntrinsicType::I64
                        | IntrinsicType::U8
                        | IntrinsicType::U16
                        | IntrinsicType::U32
                        | IntrinsicType::U64
                )
            ) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::IntegerXorRequiresInteger { required },
                    location: value_location,
                });
                return None;
            }
            let mut operands = node.children().filter(|child| is_value_node(child.kind()));
            let left_node = operands
                .next()
                .expect("syntax-clean integer XOR contains a left operand");
            let right_node = operands
                .next()
                .expect("syntax-clean integer XOR contains a right operand");
            debug_assert!(operands.next().is_none());
            let left = validate_value(
                header,
                &left_node,
                required,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            let right = validate_value(
                header,
                &right_node,
                required,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            Some(ProducedValue::ordinary(Value {
                ty: required,
                kind: ValueKind::IntegerXor {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                location: value_location,
            }))
        }
        SyntaxKind::IntegerOrValue => {
            if !matches!(
                required,
                Type::Intrinsic(
                    IntrinsicType::I8
                        | IntrinsicType::I16
                        | IntrinsicType::I32
                        | IntrinsicType::I64
                        | IntrinsicType::U8
                        | IntrinsicType::U16
                        | IntrinsicType::U32
                        | IntrinsicType::U64
                )
            ) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::IntegerOrRequiresInteger { required },
                    location: value_location,
                });
                return None;
            }
            let mut operands = node.children().filter(|child| is_value_node(child.kind()));
            let left_node = operands
                .next()
                .expect("syntax-clean integer OR contains a left operand");
            let right_node = operands
                .next()
                .expect("syntax-clean integer OR contains a right operand");
            debug_assert!(operands.next().is_none());
            let left = validate_value(
                header,
                &left_node,
                required,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            let right = validate_value(
                header,
                &right_node,
                required,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            Some(ProducedValue::ordinary(Value {
                ty: required,
                kind: ValueKind::IntegerOr {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                location: value_location,
            }))
        }
        SyntaxKind::BooleanAndValue => {
            let found = Type::Intrinsic(IntrinsicType::Bool);
            if required != found {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::TypeMismatch {
                        expected: required,
                        found,
                    },
                    location: value_location,
                });
                return None;
            }
            let mut operands = node.children().filter(|child| is_value_node(child.kind()));
            let left_node = operands
                .next()
                .expect("syntax-clean Boolean conjunction contains a left operand");
            let right_node = operands
                .next()
                .expect("syntax-clean Boolean conjunction contains a right operand");
            debug_assert!(operands.next().is_none());

            let mut left_state = state.clone();
            let left = validate_value(
                header,
                &left_node,
                found,
                context,
                value_context,
                &mut left_state,
                diagnostics,
            )?
            .value;
            let mut right_state = left_state.clone();
            let right = validate_value(
                header,
                &right_node,
                found,
                context,
                value_context,
                &mut right_state,
                diagnostics,
            )?
            .value;

            if !complete_semantic_state_matches(&right_state, &left_state) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::BooleanConjunctionOwnershipMismatch,
                    location: value_location,
                });
                return None;
            }

            *state = left_state;
            Some(ProducedValue::ordinary(Value {
                ty: found,
                kind: ValueKind::BooleanAnd {
                    left: Box::new(left),
                    right: Box::new(right),
                },
                location: value_location,
            }))
        }
        SyntaxKind::BooleanEqualityValue => {
            let found = Type::Intrinsic(IntrinsicType::Bool);
            if required != found {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::TypeMismatch {
                        expected: required,
                        found,
                    },
                    location: value_location,
                });
                return None;
            }
            let operator = node
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .find(|token| matches!(token.kind(), SyntaxKind::EqEq | SyntaxKind::BangEq))
                .expect("syntax-clean Boolean equality contains one operator token");
            let relation = match operator.kind() {
                SyntaxKind::EqEq => BooleanEqualityRelation::Equal,
                SyntaxKind::BangEq => BooleanEqualityRelation::NotEqual,
                _ => unreachable!("equality operator token has accepted equality kind"),
            };
            let mut operands = node.children().filter(|child| is_value_node(child.kind()));
            let left_node = operands
                .next()
                .expect("syntax-clean Boolean equality contains a left operand");
            let right_node = operands
                .next()
                .expect("syntax-clean Boolean equality contains a right operand");
            debug_assert!(operands.next().is_none());
            let left = validate_value(
                header,
                &left_node,
                found,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            let right = validate_value(
                header,
                &right_node,
                found,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            Some(ProducedValue::ordinary(Value {
                ty: found,
                kind: ValueKind::BooleanEquality {
                    relation,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                location: value_location,
            }))
        }
        SyntaxKind::BooleanNotValue => {
            let found = Type::Intrinsic(IntrinsicType::Bool);
            if required != found {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::TypeMismatch {
                        expected: required,
                        found,
                    },
                    location: value_location,
                });
                return None;
            }
            let operand_node = value_child(node);
            let operand = validate_value(
                header,
                &operand_node,
                found,
                context,
                value_context,
                state,
                diagnostics,
            )?
            .value;
            Some(ProducedValue::ordinary(Value {
                ty: found,
                kind: ValueKind::BooleanNot {
                    operand: Box::new(operand),
                },
                location: value_location,
            }))
        }
        SyntaxKind::BooleanLiteral => {
            let found = Type::Intrinsic(IntrinsicType::Bool);
            if required != found {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::TypeMismatch {
                        expected: required,
                        found,
                    },
                    location: value_location,
                });
                return None;
            }
            let token = node
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .find(|token| matches!(token.kind(), SyntaxKind::KwTrue | SyntaxKind::KwFalse))
                .expect("syntax-clean boolean literal contains one boolean token");
            Some(ProducedValue::ordinary(Value {
                ty: found,
                kind: ValueKind::Literal(LiteralValue::Bool(token.kind() == SyntaxKind::KwTrue)),
                location: value_location,
            }))
        }
        SyntaxKind::DecimalIntegerLiteral => {
            let literal = materialize_integer_literal(node, required, value_location, diagnostics)?;
            Some(ProducedValue::ordinary(Value {
                ty: required,
                kind: ValueKind::Literal(literal),
                location: value_location,
            }))
        }
        SyntaxKind::DecimalFloatingLiteral => {
            let literal =
                materialize_floating_literal(node, required, value_location, diagnostics)?;
            Some(ProducedValue::ordinary(Value {
                ty: required,
                kind: ValueKind::Literal(literal),
                location: value_location,
            }))
        }
        SyntaxKind::IdentifierUse => {
            validate_identifier_use(header, node, required, context, state, diagnostics)
        }
        SyntaxKind::DirectCall => {
            let validated =
                validate_call(header, node, context, value_context, state, diagnostics)?;
            let Some(ty) = validated.result else {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::NoResultCallUsedAsValue,
                    location: value_location,
                });
                return None;
            };
            if ty != required {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::TypeMismatch {
                        expected: required,
                        found: ty,
                    },
                    location: value_location,
                });
                return None;
            }
            Some(ProducedValue {
                value: Value {
                    ty,
                    kind: ValueKind::DirectCall {
                        function: validated.function,
                        arguments: validated.arguments,
                    },
                    location: value_location,
                },
                reference_authority: validated.result_reference_authority,
            })
        }
        SyntaxKind::RecordConstruction => validate_record_construction(
            header,
            node,
            required,
            context,
            value_context,
            state,
            diagnostics,
        ),
        SyntaxKind::FieldValueUse => validate_field_value_use(
            header,
            node,
            required,
            context,
            value_context,
            state,
            diagnostics,
        ),
        _ => unreachable!("syntax-clean value has represented value kind"),
    }
}

fn complete_semantic_state_matches(actual: &SemanticState, required: &SemanticState) -> bool {
    binding_ownership_matches_target(actual, required)
        && pointer_origins_match_target(actual, required)
        && reference_state_matches_target(actual, required)
}

fn validate_identifier_use(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    let token = direct_token(node, SyntaxKind::Ident);
    let name = key(&token);
    let Some(binding) = state.bindings.get(&name).cloned() else {
        let entity = context
            .modules
            .get(&header.module)
            .and_then(|module| module.namespace.get(&name));
        diagnostics.push(Diagnostic {
            kind: if entity.is_some() {
                DiagnosticKind::ExpectedValueBinding
            } else {
                DiagnosticKind::UnresolvedName
            },
            location: SourceLocation {
                unit: header.unit,
                range: token.text_range(),
            },
        });
        return None;
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

fn requested_reference_permission(node: &SyntaxNode) -> ReferencePermission {
    if node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::KwMut)
    {
        ReferencePermission::ExclusiveReplace
    } else {
        ReferencePermission::Shared
    }
}

fn validate_safe_reference_value(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let reborrow = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::Star);
    if reborrow {
        validate_reference_reborrow(header, node, required, context, state, diagnostics)
    } else {
        validate_reference_root(header, node, required, context, state, diagnostics)
    }
}

fn validate_reference_root(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    let permission = requested_reference_permission(node);
    let identifiers = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Ident)
        .collect::<Vec<_>>();
    let (root_token, selectors) = identifiers
        .split_first()
        .expect("syntax-clean reference root has one root identifier");
    debug_assert!(permission == ReferencePermission::Shared || selectors.is_empty());
    let name = key(root_token);
    let Some(binding) = state.bindings.get(&name).cloned() else {
        let entity = context
            .modules
            .get(&header.module)
            .and_then(|module| module.namespace.get(&name));
        diagnostics.push(Diagnostic {
            kind: if entity.is_some() {
                DiagnosticKind::ExpectedValueBinding
            } else {
                DiagnosticKind::UnresolvedName
            },
            location: value_location,
        });
        return None;
    };

    let (fields, selected_ty) = if selectors.is_empty() {
        (Vec::new(), binding.ty)
    } else {
        resolve_field_path(header, selectors, binding.ty, context, diagnostics)?
    };
    let referent = match selected_ty {
        Type::Intrinsic(intrinsic) => ReferenceReferent::Intrinsic(intrinsic),
        Type::Record(record) => ReferenceReferent::Record(record),
        Type::SafeReference { .. } | Type::RawPointer(_) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::InvalidSafeReferenceReferent {
                    referent: selected_ty,
                    permission,
                },
                location: value_location,
            });
            return None;
        }
    };
    let found = Type::SafeReference {
        referent,
        permission,
    };
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
    if !validate_safe_reference_referent(found, context.records, value_location, diagnostics) {
        return None;
    }

    let target = ReferenceTarget::local(binding.id, &fields);
    match permission {
        ReferencePermission::Shared => {
            if binding.ownership.path_availability(&fields) != PathAvailability::FullyAvailable {
                let (kind, unavailable_location) = if let Some(selector) = selectors.last() {
                    (
                        DiagnosticKind::UnavailableFieldValue,
                        SourceLocation {
                            unit: header.unit,
                            range: selector.text_range(),
                        },
                    )
                } else {
                    (DiagnosticKind::UnavailableBinding, value_location)
                };
                diagnostics.push(Diagnostic {
                    kind,
                    location: unavailable_location,
                });
                return None;
            }
            if !state.target_satisfies_shared_requirement(&target) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ReferencePermissionUnavailable,
                    location: value_location,
                });
                return None;
            }
        }
        ReferencePermission::ExclusiveReplace => {
            debug_assert!(fields.is_empty());
            if binding.source != BindingSource::Local
                || !binding.mutability.is_mutable()
                || binding.ownership.path_availability(&[]) != PathAvailability::FullyAvailable
                || !state.target_satisfies_exclusive_requirement(&target)
            {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::InvalidReplacementReferenceTarget,
                    location: value_location,
                });
                return None;
            }
        }
    }

    let authority = state.create_reference_authority(target, permission, None);
    Some(ProducedValue {
        value: Value {
            ty: found,
            kind: ValueKind::ReferenceRoot {
                target: binding.id,
                fields,
                permission,
            },
            location: value_location,
        },
        reference_authority: Some(authority),
    })
}

fn validate_reference_reborrow(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    let permission = requested_reference_permission(node);
    let token = direct_token(node, SyntaxKind::Ident);
    let name = key(&token);
    let Some(binding) = state.bindings.get(&name).cloned() else {
        let entity = context
            .modules
            .get(&header.module)
            .and_then(|module| module.namespace.get(&name));
        diagnostics.push(Diagnostic {
            kind: if entity.is_some() {
                DiagnosticKind::ExpectedValueBinding
            } else {
                DiagnosticKind::UnresolvedName
            },
            location: value_location,
        });
        return None;
    };
    if binding.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnavailableBinding,
            location: value_location,
        });
        return None;
    }
    let Type::SafeReference { referent, .. } = binding.ty else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ExpectedSafeReference,
            location: value_location,
        });
        return None;
    };
    let found = Type::SafeReference {
        referent,
        permission,
    };
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
    if !validate_safe_reference_referent(found, context.records, value_location, diagnostics) {
        return None;
    }
    let parent = binding
        .reference_authority
        .expect("live safe-reference binding retains authority");
    let target = state.reference_target(parent);
    if !state.target_is_fully_available(&target)
        || !state.authority_satisfies(parent, permission)
    {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ReferencePermissionUnavailable,
            location: value_location,
        });
        return None;
    }
    let authority = state.create_reference_authority(target, permission, Some(parent));
    Some(ProducedValue {
        value: Value {
            ty: found,
            kind: ValueKind::ReferenceReborrow {
                reference: binding.id,
                permission,
            },
            location: value_location,
        },
        reference_authority: Some(authority),
    })
}

fn validate_reference_dereference(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    let token = direct_token(node, SyntaxKind::Ident);
    let name = key(&token);
    let Some(binding) = state.bindings.get(&name).cloned() else {
        let entity = context
            .modules
            .get(&header.module)
            .and_then(|module| module.namespace.get(&name));
        diagnostics.push(Diagnostic {
            kind: if entity.is_some() {
                DiagnosticKind::ExpectedValueBinding
            } else {
                DiagnosticKind::UnresolvedName
            },
            location: value_location,
        });
        return None;
    };
    if binding.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnavailableBinding,
            location: value_location,
        });
        return None;
    }
    let Type::SafeReference {
        referent,
        permission,
    } = binding.ty
    else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ExpectedSafeReference,
            location: value_location,
        });
        return None;
    };
    let found = referent.ty();
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
    let authority = binding
        .reference_authority
        .expect("live safe-reference binding retains authority");
    let target = state.reference_target(authority);
    if !state.target_is_fully_available(&target) {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnavailableBinding,
            location: value_location,
        });
        return None;
    }
    let ownership =
        if permission == ReferencePermission::Shared || context.type_is_duplicable(found) {
            if !state.authority_satisfies(authority, ReferencePermission::Shared) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ReferencePermissionUnavailable,
                    location: value_location,
                });
                return None;
            }
            OwnedUse::Duplicate
        } else {
            if !state.authority_satisfies(authority, ReferencePermission::ExclusiveReplace) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ReferencePermissionUnavailable,
                    location: value_location,
                });
                return None;
            }
            state.consume_reference_target(&target);
            OwnedUse::Consume
        };
    Some(ProducedValue::ordinary(Value {
        ty: found,
        kind: ValueKind::ReferenceDereference {
            reference: binding.id,
            ownership,
        },
        location: value_location,
    }))
}

fn validate_raw_address(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    state: &SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    let token = direct_token(node, SyntaxKind::Ident);
    let name = key(&token);
    let Some(binding) = state.bindings.get(&name) else {
        let entity = context
            .modules
            .get(&header.module)
            .and_then(|module| module.namespace.get(&name));
        diagnostics.push(Diagnostic {
            kind: if entity.is_some() {
                DiagnosticKind::ExpectedValueBinding
            } else {
                DiagnosticKind::UnresolvedName
            },
            location: SourceLocation {
                unit: header.unit,
                range: token.text_range(),
            },
        });
        return None;
    };

    let pointee = match binding.ty {
        Type::Intrinsic(intrinsic) => RawPointerPointee::Intrinsic(intrinsic),
        Type::Record(record) => RawPointerPointee::Record(record),
        Type::SafeReference { .. } | Type::RawPointer(_) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::InvalidRawPointerPointee {
                    pointee: binding.ty,
                },
                location: value_location,
            });
            return None;
        }
    };
    let found = Type::RawPointer(pointee);
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
    let target = ReferenceTarget::local_root(binding.id);
    if !state.target_satisfies_shared_requirement(&target) {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::RawTargetSafeAuthorityConflict,
            location: value_location,
        });
        return None;
    }

    Some(ProducedValue::ordinary(Value {
        ty: found,
        kind: ValueKind::RawAddressRoot { target: binding.id },
        location: value_location,
    }))
}

fn validate_raw_move(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    if !value_context.unsafe_active {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnsafeOperationOutsideUnsafeBlock,
            location: value_location,
        });
        return None;
    }

    let identifiers = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Ident)
        .collect::<Vec<_>>();
    let [_, pointer_token] = identifiers.as_slice() else {
        unreachable!("syntax-clean raw move has contextual move and pointer identifiers");
    };
    let pointer_name = key(pointer_token);
    let pointer_location = SourceLocation {
        unit: header.unit,
        range: pointer_token.text_range(),
    };
    let (pointer, pointee, target) = match state.bindings.get(&pointer_name) {
        Some(binding) => {
            if binding.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::UnavailableBinding,
                    location: pointer_location,
                });
                return None;
            }
            let Type::RawPointer(pointee) = binding.ty else {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedRawPointer,
                    location: pointer_location,
                });
                return None;
            };
            (
                binding.id,
                pointee,
                binding
                    .pointer_origin
                    .expect("validated raw-pointer binding retains exact source origin"),
            )
        }
        None => {
            let entity = context
                .modules
                .get(&header.module)
                .and_then(|module| module.namespace.get(&pointer_name));
            diagnostics.push(Diagnostic {
                kind: if entity.is_some() {
                    DiagnosticKind::ExpectedValueBinding
                } else {
                    DiagnosticKind::UnresolvedName
                },
                location: pointer_location,
            });
            return None;
        }
    };

    let target_state = binding_state_by_id(&state.bindings, target)
        .expect("validated raw-pointer origin names an active target binding");
    if target_state.ownership.path_availability(&[]) != PathAvailability::FullyAvailable {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::RawMoveTargetUnavailable,
            location: value_location,
        });
        return None;
    }
    let reference_target = ReferenceTarget::local_root(target);
    if !state.target_satisfies_exclusive_requirement(&reference_target) {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::RawTargetSafeAuthorityConflict,
            location: value_location,
        });
        return None;
    }

    let found = pointee.ty();
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

    binding_state_by_id_mut(&mut state.bindings, target)
        .expect("validated raw-pointer origin names an active target binding")
        .ownership
        .consume_path(&[]);

    Some(ProducedValue::ordinary(Value {
        ty: found,
        kind: ValueKind::RawMove { pointer },
        location: value_location,
    }))
}

fn validate_numeric_contract_selected_value(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let selection_location = location(header.unit, node);
    let mut root = value_child(node);
    while root.kind() == SyntaxKind::GroupedValue {
        root = value_child(&root);
    }

    if !matches!(
        required,
        Type::Intrinsic(IntrinsicType::F16 | IntrinsicType::F32 | IntrinsicType::F64)
    ) {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::NumericContractSelectionRequiresGovernedFloatingOperation {
                required,
            },
            location: selection_location,
        });
        return None;
    }

    match root.kind() {
        SyntaxKind::MulValue => {
            let operator = root
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .find(|token| matches!(token.kind(), SyntaxKind::Star | SyntaxKind::Slash))
                .expect("syntax-clean multiplicative value contains one operator token")
                .kind();
            match operator {
                SyntaxKind::Star => validate_float_mul(
                    header,
                    &root,
                    required,
                    NumericContract::Fast,
                    context,
                    value_context,
                    state,
                    diagnostics,
                ),
                SyntaxKind::Slash => validate_float_div(
                    header,
                    &root,
                    required,
                    NumericContract::Fast,
                    context,
                    value_context,
                    state,
                    diagnostics,
                ),
                _ => unreachable!("multiplicative operator has accepted token kind"),
            }
        }
        SyntaxKind::AddValue => validate_float_add(
            header,
            &root,
            required,
            NumericContract::Fast,
            context,
            value_context,
            state,
            diagnostics,
        ),
        SyntaxKind::SubValue => validate_float_sub(
            header,
            &root,
            required,
            NumericContract::Fast,
            context,
            value_context,
            state,
            diagnostics,
        ),
        _ => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::NumericContractSelectionRequiresGovernedFloatingOperation {
                    required,
                },
                location: selection_location,
            });
            None
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "source validation threads independent resolution and semantic state explicitly"
)]
fn validate_float_add(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    contract: NumericContract,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    debug_assert_eq!(node.kind(), SyntaxKind::AddValue);
    let mut operands = node.children().filter(|child| is_value_node(child.kind()));
    let left_node = operands
        .next()
        .expect("syntax-clean floating addition contains a left operand");
    let right_node = operands
        .next()
        .expect("syntax-clean floating addition contains a right operand");
    debug_assert!(operands.next().is_none());
    let left = validate_value(
        header,
        &left_node,
        required,
        context,
        value_context,
        state,
        diagnostics,
    )?
    .value;
    let right = validate_value(
        header,
        &right_node,
        required,
        context,
        value_context,
        state,
        diagnostics,
    )?
    .value;
    Some(ProducedValue::ordinary(Value {
        ty: required,
        kind: ValueKind::FloatAdd {
            contract,
            left: Box::new(left),
            right: Box::new(right),
        },
        location: location(header.unit, node),
    }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "source validation threads independent resolution and semantic state explicitly"
)]
fn validate_float_sub(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    contract: NumericContract,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    debug_assert_eq!(node.kind(), SyntaxKind::SubValue);
    let mut operands = node.children().filter(|child| is_value_node(child.kind()));
    let left_node = operands
        .next()
        .expect("syntax-clean floating subtraction contains a left operand");
    let right_node = operands
        .next()
        .expect("syntax-clean floating subtraction contains a right operand");
    debug_assert!(operands.next().is_none());
    let left = validate_value(
        header,
        &left_node,
        required,
        context,
        value_context,
        state,
        diagnostics,
    )?
    .value;
    let right = validate_value(
        header,
        &right_node,
        required,
        context,
        value_context,
        state,
        diagnostics,
    )?
    .value;
    Some(ProducedValue::ordinary(Value {
        ty: required,
        kind: ValueKind::FloatSub {
            contract,
            left: Box::new(left),
            right: Box::new(right),
        },
        location: location(header.unit, node),
    }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "source validation threads independent resolution and semantic state explicitly"
)]
fn validate_float_mul(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    contract: NumericContract,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    debug_assert_eq!(node.kind(), SyntaxKind::MulValue);
    let mut operands = node.children().filter(|child| is_value_node(child.kind()));
    let left_node = operands
        .next()
        .expect("syntax-clean floating multiplication contains a left operand");
    let right_node = operands
        .next()
        .expect("syntax-clean floating multiplication contains a right operand");
    debug_assert!(operands.next().is_none());
    let left = validate_value(
        header,
        &left_node,
        required,
        context,
        value_context,
        state,
        diagnostics,
    )?
    .value;
    let right = validate_value(
        header,
        &right_node,
        required,
        context,
        value_context,
        state,
        diagnostics,
    )?
    .value;
    Some(ProducedValue::ordinary(Value {
        ty: required,
        kind: ValueKind::FloatMul {
            contract,
            left: Box::new(left),
            right: Box::new(right),
        },
        location: location(header.unit, node),
    }))
}

#[expect(
    clippy::too_many_arguments,
    reason = "source validation threads independent resolution and semantic state explicitly"
)]
fn validate_float_div(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    contract: NumericContract,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    debug_assert_eq!(node.kind(), SyntaxKind::MulValue);
    let mut operands = node.children().filter(|child| is_value_node(child.kind()));
    let left_node = operands
        .next()
        .expect("syntax-clean floating division contains a left operand");
    let right_node = operands
        .next()
        .expect("syntax-clean floating division contains a right operand");
    debug_assert!(operands.next().is_none());
    let left = validate_value(
        header,
        &left_node,
        required,
        context,
        value_context,
        state,
        diagnostics,
    )?
    .value;
    let right = validate_value(
        header,
        &right_node,
        required,
        context,
        value_context,
        state,
        diagnostics,
    )?
    .value;
    Some(ProducedValue::ordinary(Value {
        ty: required,
        kind: ValueKind::FloatDiv {
            contract,
            left: Box::new(left),
            right: Box::new(right),
        },
        location: location(header.unit, node),
    }))
}

fn validate_field_value_use(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let value_location = location(header.unit, node);
    let producer_node = node.children().find(|child| {
        matches!(
            child.kind(),
            SyntaxKind::DirectCall | SyntaxKind::RecordConstruction
        )
    });
    let identifiers = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Ident)
        .collect::<Vec<_>>();

    if let Some(producer_node) = producer_node {
        debug_assert!(!identifiers.is_empty());
        let receiver_ty = match producer_node.kind() {
            SyntaxKind::DirectCall => {
                let function = resolve_call_target(
                    header,
                    &producer_node,
                    context,
                    &state.bindings,
                    diagnostics,
                )?;
                let Some(result) = context.headers[function.0].result else {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::NoResultCallUsedAsValue,
                        location: location(header.unit, &producer_node),
                    });
                    return None;
                };
                result
            }
            SyntaxKind::RecordConstruction => Type::Record(resolve_record_construction_target(
                header,
                &producer_node,
                context,
                diagnostics,
            )?),
            _ => unreachable!("producer-backed field receiver has accepted producer kind"),
        };

        let (fields, final_ty) =
            resolve_field_path(header, &identifiers, receiver_ty, context, diagnostics)?;
        if final_ty != required {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::TypeMismatch {
                    expected: required,
                    found: final_ty,
                },
                location: value_location,
            });
            return None;
        }
        let ownership = if context.type_is_duplicable(final_ty) {
            OwnedUse::Duplicate
        } else {
            OwnedUse::Consume
        };

        let producer = validate_value(
            header,
            &producer_node,
            receiver_ty,
            context,
            value_context,
            state,
            diagnostics,
        )?
        .value;

        let mut transient = StructuralOwnershipState::default();
        if ownership == OwnedUse::Consume {
            transient.consume_path(&fields);
        }
        let cleanup = FieldReceiverTransientCleanup {
            paths: remaining_ownership_frontier(receiver_ty, &transient, context.records),
        };

        return Some(ProducedValue::ordinary(Value {
            ty: final_ty,
            kind: ValueKind::FieldValueUse {
                receiver: FieldValueReceiver::Producer {
                    value: Box::new(producer),
                    cleanup,
                },
                fields,
                ownership,
            },
            location: value_location,
        }));
    }

    let (root_token, selectors) = identifiers
        .split_first()
        .expect("syntax-clean binding-root field-value use contains a root identifier");
    debug_assert!(!selectors.is_empty());

    let root_name = key(root_token);
    let root_location = SourceLocation {
        unit: header.unit,
        range: root_token.text_range(),
    };
    let (binding_id, binding_ty) = match state.bindings.get(&root_name) {
        Some(binding) => (binding.id, binding.ty),
        None => {
            let entity = context
                .modules
                .get(&header.module)
                .and_then(|module| module.namespace.get(&root_name));
            diagnostics.push(Diagnostic {
                kind: if entity.is_some() {
                    DiagnosticKind::ExpectedValueBinding
                } else {
                    DiagnosticKind::UnresolvedName
                },
                location: root_location,
            });
            return None;
        }
    };

    let (fields, final_ty) =
        resolve_field_path(header, selectors, binding_ty, context, diagnostics)?;
    let binding = state
        .bindings
        .get(&root_name)
        .expect("resolved field-value root remains in active binding scope");
    if binding.ownership.path_availability(&fields) != PathAvailability::FullyAvailable {
        let selector = selectors
            .last()
            .expect("syntax-clean field-value use has at least one selector");
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnavailableFieldValue,
            location: SourceLocation {
                unit: header.unit,
                range: selector.text_range(),
            },
        });
        return None;
    }

    if final_ty != required {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::TypeMismatch {
                expected: required,
                found: final_ty,
            },
            location: value_location,
        });
        return None;
    }

    let ownership = if context.type_is_duplicable(final_ty) {
        OwnedUse::Duplicate
    } else {
        OwnedUse::Consume
    };
    let target = ReferenceTarget::local(binding_id, &fields);
    let compatible = if ownership == OwnedUse::Duplicate {
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
    if ownership == OwnedUse::Consume {
        state
            .bindings
            .get_mut(&root_name)
            .expect("resolved field-value root remains in active binding scope")
            .ownership
            .consume_path(&fields);
    }

    Some(ProducedValue::ordinary(Value {
        ty: final_ty,
        kind: ValueKind::FieldValueUse {
            receiver: FieldValueReceiver::Binding {
                binding: binding_id,
                ty: binding_ty,
            },
            fields,
            ownership,
        },
        location: value_location,
    }))
}

fn record_field_is_accessible(module: ModuleId, record: &Record, field: usize) -> bool {
    record.module == module
        || (record.accessibility == Accessibility::Exported
            && record.fields[field].accessibility == Accessibility::Exported)
}

fn resolve_field_path(
    header: &FunctionHeader,
    selectors: &[SyntaxToken],
    receiver_ty: Type,
    context: &BodyResolutionContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(Vec<usize>, Type)> {
    debug_assert!(!selectors.is_empty());
    let mut current = receiver_ty;
    let mut fields = Vec::with_capacity(selectors.len());
    for selector in selectors {
        let selector_location = SourceLocation {
            unit: header.unit,
            range: selector.text_range(),
        };
        let Type::Record(record) = current else {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ExpectedRecordForFieldAccess,
                location: selector_location,
            });
            return None;
        };
        let record_decl = &context.records[record.0];
        let selector_name = key(selector);
        let Some(field) = record_decl
            .fields
            .iter()
            .position(|field| field.name == selector_name)
        else {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnknownRecordField,
                location: selector_location,
            });
            return None;
        };

        if !record_field_is_accessible(header.module, record_decl, field) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::InaccessibleRecordField,
                location: selector_location,
            });
            return None;
        }

        fields.push(field);
        current = record_decl.fields[field].ty;
    }
    Some((fields, current))
}

fn resolve_record_construction_target(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<RecordId> {
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
            EntityId::Record(record) => Some(record),
            EntityId::Function(_) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedRecordType,
                    location: location(header.unit, &qualified),
                });
                None
            }
        };
    }

    let target_token = direct_token(node, SyntaxKind::Ident);
    let target_name = key(&target_token);
    let target_location = SourceLocation {
        unit: header.unit,
        range: target_token.text_range(),
    };

    match context
        .modules
        .get(&header.module)
        .and_then(|module| module.namespace.get(&target_name))
        .copied()
        .map(|entity| entity.entity)
    {
        Some(EntityId::Record(record)) => Some(record),
        Some(EntityId::Function(_)) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ExpectedRecordType,
                location: target_location,
            });
            None
        }
        None => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnresolvedName,
                location: target_location,
            });
            None
        }
    }
}

fn validate_record_construction(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ProducedValue> {
    let construction_location = location(header.unit, node);
    let record = resolve_record_construction_target(header, node, context, diagnostics)?;
    let record_ty = Type::Record(record);
    let record_decl = &context.records[record.0];
    let mut seen = BTreeSet::<usize>::new();
    let mut resolved = Vec::<(usize, SyntaxNode)>::new();
    let mut structurally_valid = true;

    for initializer in node
        .children()
        .filter(|child| child.kind() == SyntaxKind::RecordInitializer)
    {
        let name_token = direct_token(&initializer, SyntaxKind::Ident);
        let name = key(&name_token);
        let initializer_location = location(header.unit, &initializer);
        let Some(field) = record_decl
            .fields
            .iter()
            .position(|field| field.name == name)
        else {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnknownRecordField,
                location: initializer_location,
            });
            structurally_valid = false;
            continue;
        };

        if !record_field_is_accessible(header.module, record_decl, field) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::InaccessibleRecordField,
                location: initializer_location,
            });
            structurally_valid = false;
        }

        if !seen.insert(field) {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::DuplicateRecordInitializer,
                location: initializer_location,
            });
            structurally_valid = false;
            continue;
        }
        resolved.push((field, initializer));
    }

    if seen.len() != record_decl.fields.len() {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::MissingRecordInitializer,
            location: construction_location,
        });
        structurally_valid = false;
    }

    if record_ty != required {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::TypeMismatch {
                expected: required,
                found: record_ty,
            },
            location: construction_location,
        });
        structurally_valid = false;
    }

    if !structurally_valid {
        return None;
    }

    let mut fields = Vec::with_capacity(resolved.len());
    for (field, initializer) in resolved {
        let value_node = value_child(&initializer);
        let value = validate_value(
            header,
            &value_node,
            record_decl.fields[field].ty,
            context,
            value_context,
            state,
            diagnostics,
        )?
        .value;
        fields.push(RecordFieldValue { field, value });
    }

    Some(ProducedValue::ordinary(Value {
        ty: record_ty,
        kind: ValueKind::RecordConstruction { record, fields },
        location: construction_location,
    }))
}

fn materialize_integer_literal(
    node: &SyntaxNode,
    required: Type,
    value_location: SourceLocation,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<LiteralValue> {
    let Type::Intrinsic(intrinsic) = required else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::IntegerLiteralRequiresInteger { required },
            location: value_location,
        });
        return None;
    };

    let negative = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::Minus);
    let magnitude = direct_token(node, SyntaxKind::DecimalMagnitude);
    let text = magnitude.text();

    let literal = match intrinsic {
        IntrinsicType::I8 => materialize_signed(text, negative, 127, 128)
            .and_then(|value| i8::try_from(value).ok())
            .map(LiteralValue::I8),
        IntrinsicType::I16 => materialize_signed(text, negative, 32_767, 32_768)
            .and_then(|value| i16::try_from(value).ok())
            .map(LiteralValue::I16),
        IntrinsicType::I32 => materialize_signed(text, negative, 2_147_483_647, 2_147_483_648)
            .and_then(|value| i32::try_from(value).ok())
            .map(LiteralValue::I32),
        IntrinsicType::I64 => materialize_signed(
            text,
            negative,
            9_223_372_036_854_775_807,
            9_223_372_036_854_775_808,
        )
        .and_then(|value| i64::try_from(value).ok())
        .map(LiteralValue::I64),
        IntrinsicType::U8 => materialize_unsigned(text, negative, u64::from(u8::MAX))
            .and_then(|value| u8::try_from(value).ok())
            .map(LiteralValue::U8),
        IntrinsicType::U16 => materialize_unsigned(text, negative, u64::from(u16::MAX))
            .and_then(|value| u16::try_from(value).ok())
            .map(LiteralValue::U16),
        IntrinsicType::U32 => materialize_unsigned(text, negative, u64::from(u32::MAX))
            .and_then(|value| u32::try_from(value).ok())
            .map(LiteralValue::U32),
        IntrinsicType::U64 => materialize_unsigned(text, negative, u64::MAX).map(LiteralValue::U64),
        IntrinsicType::Bool | IntrinsicType::F16 | IntrinsicType::F32 | IntrinsicType::F64 => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::IntegerLiteralRequiresInteger { required },
                location: value_location,
            });
            return None;
        }
    };

    match literal {
        Some(literal) => Some(literal),
        None => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::IntegerLiteralOutOfRange { required },
                location: value_location,
            });
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BinaryFloatFormat {
    precision: u32,
    emin: i32,
    emax: i32,
}

#[derive(Debug)]
struct DecimalDatum {
    digits: String,
    scale: usize,
}

#[derive(Debug)]
struct SmallDecimalInt {
    limbs: Vec<u32>,
}

impl SmallDecimalInt {
    const BASE: u64 = 1_000_000_000;

    fn from_u64(mut value: u64) -> Self {
        let mut limbs = Vec::new();
        while value != 0 {
            limbs.push((value % Self::BASE) as u32);
            value /= Self::BASE;
        }
        if limbs.is_empty() {
            limbs.push(0);
        }
        Self { limbs }
    }

    fn mul_small(&mut self, factor: u32) {
        let mut carry = 0_u64;
        for limb in &mut self.limbs {
            let product = u64::from(*limb) * u64::from(factor) + carry;
            *limb = (product % Self::BASE) as u32;
            carry = product / Self::BASE;
        }
        while carry != 0 {
            self.limbs.push((carry % Self::BASE) as u32);
            carry /= Self::BASE;
        }
    }

    fn to_decimal_string(&self) -> String {
        let mut limbs = self.limbs.iter().rev();
        let first = limbs.next().expect("decimal integer has one limb");
        let mut text = first.to_string();
        for limb in limbs {
            text.push_str(&format!("{limb:09}"));
        }
        text
    }
}

fn materialize_floating_literal(
    node: &SyntaxNode,
    required: Type,
    value_location: SourceLocation,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<LiteralValue> {
    let Type::Intrinsic(intrinsic) = required else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::FloatingLiteralRequiresFloating { required },
            location: value_location,
        });
        return None;
    };

    let (format, wrap): (BinaryFloatFormat, fn(BinaryFloatValue) -> LiteralValue) = match intrinsic
    {
        IntrinsicType::F16 => (
            BinaryFloatFormat {
                precision: 11,
                emin: -14,
                emax: 15,
            },
            LiteralValue::F16,
        ),
        IntrinsicType::F32 => (
            BinaryFloatFormat {
                precision: 24,
                emin: -126,
                emax: 127,
            },
            LiteralValue::F32,
        ),
        IntrinsicType::F64 => (
            BinaryFloatFormat {
                precision: 53,
                emin: -1022,
                emax: 1023,
            },
            LiteralValue::F64,
        ),
        IntrinsicType::Bool
        | IntrinsicType::I8
        | IntrinsicType::I16
        | IntrinsicType::I32
        | IntrinsicType::I64
        | IntrinsicType::U8
        | IntrinsicType::U16
        | IntrinsicType::U32
        | IntrinsicType::U64 => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::FloatingLiteralRequiresFloating { required },
                location: value_location,
            });
            return None;
        }
    };

    let sign = if node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::Minus)
    {
        BinaryFloatSign::Negative
    } else {
        BinaryFloatSign::Positive
    };
    let magnitude = direct_token(node, SyntaxKind::DecimalFloatingMagnitude);
    let datum = decimal_floating_datum(magnitude.text());
    let value = if datum.digits.is_empty() {
        BinaryFloatValue::Zero(sign)
    } else {
        round_decimal_binary(&datum, sign, format)
    };
    Some(wrap(value))
}

fn decimal_floating_datum(text: &str) -> DecimalDatum {
    let dot = text
        .find('.')
        .expect("syntax floating magnitude contains one decimal point");
    let scale = text.len() - dot - 1;
    let mut digits = String::with_capacity(text.len() - 1);
    digits.push_str(&text[..dot]);
    digits.push_str(&text[dot + 1..]);
    let leading = digits.bytes().take_while(|byte| *byte == b'0').count();
    digits.drain(..leading);
    DecimalDatum { digits, scale }
}

fn round_decimal_binary(
    datum: &DecimalDatum,
    sign: BinaryFloatSign,
    format: BinaryFloatFormat,
) -> BinaryFloatValue {
    debug_assert!(!datum.digits.is_empty());
    let normal_min = 1_u64 << (format.precision - 1);
    let significand_limit = 1_u64 << format.precision;
    let max_significand = significand_limit - 1;
    let quantum_exponent = format.emin - (format.precision as i32 - 1);

    if compare_decimal_to_dyadic(datum, 1, format.emin) == Ordering::Less {
        let lower = greatest_significand_not_above(datum, 0, normal_min - 1, quantum_exponent);
        if lower != 0
            && compare_decimal_to_dyadic(datum, lower, quantum_exponent) == Ordering::Equal
        {
            return BinaryFloatValue::Subnormal {
                sign,
                significand: lower,
            };
        }
        let midpoint = compare_decimal_to_dyadic(
            datum,
            lower
                .checked_mul(2)
                .and_then(|value| value.checked_add(1))
                .expect("bounded floating midpoint significand"),
            quantum_exponent - 1,
        );
        let selected = match midpoint {
            Ordering::Less => lower,
            Ordering::Greater => lower + 1,
            Ordering::Equal if lower.is_multiple_of(2) => lower,
            Ordering::Equal => lower + 1,
        };
        return if selected == 0 {
            BinaryFloatValue::Zero(sign)
        } else if selected == normal_min {
            BinaryFloatValue::Normal {
                sign,
                significand: normal_min,
                exponent: format.emin as i16,
            }
        } else {
            BinaryFloatValue::Subnormal {
                sign,
                significand: selected,
            }
        };
    }

    let exponent = greatest_exponent_not_above(datum, format.emin, format.emax);
    let grid_exponent = exponent - (format.precision as i32 - 1);
    let lower = greatest_significand_not_above(datum, normal_min, max_significand, grid_exponent);
    if compare_decimal_to_dyadic(datum, lower, grid_exponent) == Ordering::Equal {
        return BinaryFloatValue::Normal {
            sign,
            significand: lower,
            exponent: exponent as i16,
        };
    }

    let midpoint = compare_decimal_to_dyadic(
        datum,
        lower
            .checked_mul(2)
            .and_then(|value| value.checked_add(1))
            .expect("bounded floating midpoint significand"),
        grid_exponent - 1,
    );
    let selected = match midpoint {
        Ordering::Less => lower,
        Ordering::Greater => lower + 1,
        Ordering::Equal if lower.is_multiple_of(2) => lower,
        Ordering::Equal => lower + 1,
    };
    if selected < significand_limit {
        return BinaryFloatValue::Normal {
            sign,
            significand: selected,
            exponent: exponent as i16,
        };
    }
    if exponent == format.emax {
        BinaryFloatValue::Infinity(sign)
    } else {
        BinaryFloatValue::Normal {
            sign,
            significand: normal_min,
            exponent: (exponent + 1) as i16,
        }
    }
}

fn greatest_exponent_not_above(datum: &DecimalDatum, mut low: i32, mut high: i32) -> i32 {
    while low < high {
        let mid = low + (high - low + 1) / 2;
        if compare_decimal_to_dyadic(datum, 1, mid) != Ordering::Less {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

fn greatest_significand_not_above(
    datum: &DecimalDatum,
    mut low: u64,
    mut high: u64,
    exponent: i32,
) -> u64 {
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if compare_decimal_to_dyadic(datum, mid, exponent) != Ordering::Less {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

fn compare_decimal_to_dyadic(datum: &DecimalDatum, significand: u64, exponent: i32) -> Ordering {
    if significand == 0 {
        return Ordering::Greater;
    }
    let (candidate, candidate_scale) = dyadic_decimal(significand, exponent);
    compare_scaled_decimal(&datum.digits, datum.scale, &candidate, candidate_scale)
}

fn dyadic_decimal(significand: u64, exponent: i32) -> (String, usize) {
    debug_assert!(significand != 0);
    let mut integer = SmallDecimalInt::from_u64(significand);
    if exponent >= 0 {
        for _ in 0..exponent {
            integer.mul_small(2);
        }
        (integer.to_decimal_string(), 0)
    } else {
        let scale = (-exponent) as usize;
        for _ in 0..scale {
            integer.mul_small(5);
        }
        (integer.to_decimal_string(), scale)
    }
}

fn compare_scaled_decimal(
    source_digits: &str,
    source_scale: usize,
    candidate_digits: &str,
    candidate_scale: usize,
) -> Ordering {
    let source_cross_len = source_digits.len() + candidate_scale;
    let candidate_cross_len = candidate_digits.len() + source_scale;
    match source_cross_len.cmp(&candidate_cross_len) {
        Ordering::Equal => {}
        other => return other,
    }

    let source = source_digits.as_bytes();
    let candidate = candidate_digits.as_bytes();
    for index in 0..source_cross_len {
        let left = source.get(index).copied().unwrap_or(b'0');
        let right = candidate.get(index).copied().unwrap_or(b'0');
        match left.cmp(&right) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}

fn materialize_signed(
    text: &str,
    negative: bool,
    positive_limit: u64,
    negative_limit: u64,
) -> Option<i128> {
    let limit = if negative {
        negative_limit
    } else {
        positive_limit
    };
    let magnitude = parse_decimal_magnitude(text, limit)?;
    let magnitude = i128::from(magnitude);
    Some(if negative { -magnitude } else { magnitude })
}

fn materialize_unsigned(text: &str, negative: bool, limit: u64) -> Option<u64> {
    let magnitude = parse_decimal_magnitude(text, limit)?;
    if negative && magnitude != 0 {
        None
    } else {
        Some(magnitude)
    }
}

fn parse_decimal_magnitude(text: &str, limit: u64) -> Option<u64> {
    let mut value = 0_u64;
    for byte in text.bytes() {
        debug_assert!(byte.is_ascii_digit());
        let digit = u64::from(byte - b'0');
        let remaining = limit.checked_sub(digit)?;
        if value > remaining / 10 {
            return None;
        }
        value = value * 10 + digit;
    }
    Some(value)
}

fn resolve_call_target(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<FunctionId> {
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
            EntityId::Function(id) => Some(id),
            EntityId::Record(_) => {
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

    if bindings.contains_key(&name) {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ExpectedFunction,
            location: name_location,
        });
        return None;
    }

    match context
        .modules
        .get(&header.module)
        .and_then(|module| module.namespace.get(&name))
        .copied()
        .map(|entity| entity.entity)
    {
        Some(EntityId::Function(id)) => Some(id),
        Some(EntityId::Record(_)) => {
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

struct ValidatedCall {
    function: FunctionId,
    arguments: Vec<Value>,
    result: Option<Type>,
    result_reference_authority: Option<ReferenceAuthorityId>,
}

fn validate_call(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ValidatedCall> {
    let mut candidate = state.clone();
    let result = validate_call_inner(
        header,
        node,
        context,
        value_context,
        &mut candidate,
        diagnostics,
    )?;
    *state = candidate;
    Some(result)
}

fn validate_call_inner(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    value_context: &ValueValidationContext,
    state: &mut SemanticState,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ValidatedCall> {
    let function = resolve_call_target(header, node, context, &state.bindings, diagnostics)?;
    let target = &context.headers[function.0];
    let argument_list = direct_child(node, SyntaxKind::ArgumentList);
    let argument_nodes = argument_list
        .children()
        .filter(|child| is_value_node(child.kind()))
        .collect::<Vec<_>>();

    if argument_nodes.len() != target.parameters.len() {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ArgumentCount {
                expected: target.parameters.len(),
                found: argument_nodes.len(),
            },
            location: location(header.unit, node),
        });
        return None;
    }

    let mut arguments = Vec::with_capacity(argument_nodes.len());
    let mut authorities = Vec::with_capacity(argument_nodes.len());
    for (argument_node, parameter) in argument_nodes.into_iter().zip(&target.parameters) {
        let argument = validate_value(
            header,
            &argument_node,
            parameter.ty,
            context,
            value_context,
            state,
            diagnostics,
        )?;
        authorities.push(argument.reference_authority);
        arguments.push(argument.value);
    }

    for (parameter, authority) in target.parameters.iter().zip(&authorities) {
        let Type::SafeReference { permission, .. } = parameter.ty else {
            debug_assert!(authority.is_none());
            continue;
        };
        let authority = authority.expect("validated safe-reference argument has one carrier");
        let reference_target = state.reference_target(authority);
        if !state.target_is_fully_available(&reference_target)
            || !state.authority_satisfies(authority, permission)
        {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ReferencePermissionUnavailable,
                location: location(header.unit, node),
            });
            return None;
        }
    }

    let result_reference_authority = match target.safe_reference_result_contract {
        SafeReferenceResultContract::None => None,
        SafeReferenceResultContract::SharedIdentity { origin } => {
            let authority = authorities[origin]
                .expect("Shared identity result origin has safe-reference argument authority");
            state.add_reference_carrier(authority);
            Some(authority)
        }
        SafeReferenceResultContract::SharedDirectChild { origin } => {
            let parent = authorities[origin]
                .expect("Shared direct-child result origin has safe-reference argument authority");
            let reference_target = state.reference_target(parent);
            Some(state.create_reference_authority(
                reference_target,
                ReferencePermission::Shared,
                Some(parent),
            ))
        }
    };
    for authority in authorities.into_iter().flatten() {
        state.release_reference_carrier(authority);
    }

    Some(ValidatedCall {
        function,
        arguments,
        result: target.result,
        result_reference_authority,
    })
}

fn declaration_accessibility(node: &SyntaxNode) -> Accessibility {
    if node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .any(|token| token.kind() == SyntaxKind::KwExport)
    {
        Accessibility::Exported
    } else {
        Accessibility::ModulePrivate
    }
}

fn direct_child(node: &SyntaxNode, kind: SyntaxKind) -> SyntaxNode {
    node.children()
        .find(|child| child.kind() == kind)
        .expect("syntax-clean tree contains required child node")
}

fn direct_token(node: &SyntaxNode, kind: SyntaxKind) -> SyntaxToken {
    node.children_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| token.kind() == kind)
        .expect("syntax-clean tree contains required token")
}

fn value_child(node: &SyntaxNode) -> SyntaxNode {
    node.children()
        .find(|child| is_value_node(child.kind()))
        .expect("syntax-clean value-producing construct contains a value")
}

fn is_value_node(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::GroupedValue
            | SyntaxKind::SafeReferenceValue
            | SyntaxKind::ReferenceDereferenceValue
            | SyntaxKind::RawAddressOfValue
            | SyntaxKind::RawMoveValue
            | SyntaxKind::NumericContractSelectedValue
            | SyntaxKind::IntegerNegValue
            | SyntaxKind::IntegerComplementValue
            | SyntaxKind::AddValue
            | SyntaxKind::SubValue
            | SyntaxKind::MulValue
            | SyntaxKind::IntegerXorValue
            | SyntaxKind::IntegerOrValue
            | SyntaxKind::BooleanAndValue
            | SyntaxKind::BooleanEqualityValue
            | SyntaxKind::BooleanNotValue
            | SyntaxKind::BooleanLiteral
            | SyntaxKind::DecimalIntegerLiteral
            | SyntaxKind::DecimalFloatingLiteral
            | SyntaxKind::IdentifierUse
            | SyntaxKind::DirectCall
            | SyntaxKind::RecordConstruction
            | SyntaxKind::FieldValueUse
    )
}

fn key(token: &SyntaxToken) -> String {
    identifier_key(token.text()).expect("syntax identifier token has accepted identifier form")
}

fn location(unit: usize, node: &SyntaxNode) -> SourceLocation {
    SourceLocation {
        unit,
        range: node.text_range(),
    }
}
