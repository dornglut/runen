use std::collections::{BTreeMap, BTreeSet};

use runen_syntax::{SyntaxKind, SyntaxNode, SyntaxToken, identifier_key};

use crate::{
    Accessibility, AssignmentMutability, BindingId, Block, Body, CleanupPath, Diagnostic,
    DiagnosticKind, Field, Function, FunctionId, IntrinsicType, LiteralValue, Module, ModuleId,
    OwnedUse, Parameter, Record, RecordFieldValue, RecordId, RecordPatternBinding,
    RecordPatternScrutinee, RecordPatternTransientCleanup, Return, SourceLocation, SourceUnit,
    Statement, Type, TypedCompilation, Value, ValueKind,
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
    body: SyntaxNode,
    location: SourceLocation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct StructuralOwnershipState {
    consumed_paths: BTreeSet<Vec<usize>>,
}

#[derive(Debug, Clone)]
struct BindingState {
    id: BindingId,
    ty: Type,
    mutability: AssignmentMutability,
    ownership: StructuralOwnershipState,
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
            let field_location = location(record.unit, &field_node);
            if !field_names.insert(name.clone()) {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::DuplicateRecordField,
                    location: field_location,
                });
            }
            let type_node = direct_child(&field_node, SyntaxKind::TypeRef);
            if let Some(ty) = resolve_type(
                record.module,
                record.unit,
                &type_node,
                modules,
                imports,
                diagnostics,
            ) {
                fields.push(Field {
                    name,
                    ty,
                    location: field_location,
                });
            }
        }
        records.push(Record {
            id: record.id,
            module: record.module,
            name: record.name.clone(),
            accessibility: record.accessibility,
            fields,
            location: record.location,
        });
    }
    records
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
                );
                if let Some(ty) = ty {
                    validate_exported_signature_type(
                        function.accessibility,
                        ty,
                        records,
                        function.unit,
                        &type_node,
                        diagnostics,
                    );
                }
                ty
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
    let Type::Record(record) = ty else {
        return;
    };
    if records[record.0].accessibility == Accessibility::ModulePrivate {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::PrivateTypeInExportedSignature,
            location: location(unit, type_node),
        });
    }
}

fn resolve_type(
    module: ModuleId,
    unit: usize,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if let Some(qualified) = node
        .children()
        .find(|child| child.kind() == SyntaxKind::QualifiedModuleMember)
    {
        return match resolve_qualified_entity(unit, &qualified, modules, imports, diagnostics)? {
            EntityId::Record(id) => Some(Type::Record(id)),
            EntityId::Function(_) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedRecordType,
                    location: location(unit, &qualified),
                });
                None
            }
        };
    }

    let token = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .find(|token| !token.kind().is_trivia())
        .expect("syntax-clean type reference contains one type token");
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
        return Some(Type::Intrinsic(intrinsic));
    }

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
        Some(EntityId::Record(id)) => Some(Type::Record(id)),
        Some(EntityId::Function(_)) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ExpectedRecordType,
                location: token_location,
            });
            None
        }
        None => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnresolvedName,
                location: token_location,
            });
            None
        }
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

fn validate_body(
    header: &FunctionHeader,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    imports: &[UnitImports],
    records: &[Record],
    headers: &[FunctionHeader],
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Body {
    let mut bindings = BTreeMap::<String, BindingState>::new();
    for parameter in &header.parameters {
        bindings.insert(
            parameter.name.clone(),
            BindingState {
                id: parameter.binding,
                ty: parameter.ty,
                mutability: AssignmentMutability::Immutable,
                ownership: StructuralOwnershipState::default(),
            },
        );
    }

    let context = BodyResolutionContext {
        modules,
        imports,
        records,
        headers,
    };
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
                &mut bindings,
                diagnostics,
            ));
            has_normal_continuation = false;
            continue;
        }

        if let Some(statement) = validate_body_statement(
            header,
            &node,
            &context,
            &mut bindings,
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
    }

    Body {
        statements,
        terminal_return,
        has_normal_continuation,
    }
}

fn validate_body_statement(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    match node.kind() {
        SyntaxKind::LocalDeclaration => {
            validate_local(header, node, context, bindings, next_binding, diagnostics)
        }
        SyntaxKind::RecordDestructuringDeclaration => {
            validate_record_destructure(header, node, context, bindings, next_binding, diagnostics)
        }
        SyntaxKind::AssignmentStatement => {
            validate_assignment(header, node, context, bindings, diagnostics)
        }
        SyntaxKind::CallStatement => {
            validate_call_statement(header, node, context, bindings, diagnostics)
        }
        SyntaxKind::BlockStatement => Some(validate_block(
            header,
            node,
            context,
            bindings,
            next_binding,
            diagnostics,
        )),
        SyntaxKind::IfStatement => {
            validate_if(header, node, context, bindings, next_binding, diagnostics)
        }
        SyntaxKind::ReturnStatement => {
            unreachable!("terminal return is handled by its containing source sequence")
        }
        _ => unreachable!("syntax-clean sequence contains only represented body nodes"),
    }
}

fn statement_has_normal_continuation(statement: &Statement) -> bool {
    match statement {
        Statement::Block(block) => block.has_normal_continuation,
        Statement::If {
            then_block,
            else_block,
            ..
        } => else_block.as_ref().is_none_or(|else_block| {
            then_block.has_normal_continuation || else_block.has_normal_continuation
        }),
        Statement::Local { .. }
        | Statement::RecordDestructure { .. }
        | Statement::Assignment { .. }
        | Statement::Call { .. } => true,
    }
}

fn validate_block(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Statement {
    let mut statements = Vec::new();
    let mut direct_bindings = Vec::<(String, BindingId)>::new();
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
                bindings,
                diagnostics,
            ));
            has_normal_continuation = false;
            continue;
        }

        let statement =
            validate_body_statement(header, &child, context, bindings, next_binding, diagnostics);

        if let Some(statement) = statement {
            match &statement {
                Statement::Local { binding, name, .. } => {
                    direct_bindings.push((name.clone(), *binding));
                }
                Statement::RecordDestructure { bindings, .. } => {
                    direct_bindings.extend(
                        bindings
                            .iter()
                            .map(|binding| (binding.name.clone(), binding.binding)),
                    );
                }
                Statement::Assignment { .. }
                | Statement::Call { .. }
                | Statement::Block(_)
                | Statement::If { .. } => {}
            }
            has_normal_continuation = statement_has_normal_continuation(&statement);
            statements.push(statement);
        }
    }

    let mut normal_cleanup = Vec::new();
    if has_normal_continuation {
        for (name, binding) in direct_bindings.iter().rev() {
            let state = bindings
                .get(name)
                .expect("validated direct child binding remains active through block end");
            debug_assert_eq!(state.id, *binding);
            for fields in remaining_ownership_frontier(state.ty, &state.ownership, context.records)
            {
                normal_cleanup.push(CleanupPath {
                    binding: *binding,
                    fields,
                });
            }
        }
    }

    for (name, binding) in direct_bindings {
        let removed = bindings
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

fn validate_if(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let mut post_condition = bindings.clone();
    let condition_node = value_child(node);
    let condition = validate_value(
        header,
        &condition_node,
        Type::Intrinsic(IntrinsicType::Bool),
        context,
        &mut post_condition,
        diagnostics,
    )?;

    let mut blocks = node
        .children()
        .filter(|child| child.kind() == SyntaxKind::BlockStatement);
    let then_node = blocks
        .next()
        .expect("syntax-clean conditional contains a then block");
    let else_node = blocks.next();
    debug_assert!(blocks.next().is_none());

    let mut then_bindings = post_condition.clone();
    let then_diagnostics = diagnostics.len();
    let then_statement = validate_block(
        header,
        &then_node,
        context,
        &mut then_bindings,
        next_binding,
        diagnostics,
    );
    let then_valid = diagnostics.len() == then_diagnostics;
    let Statement::Block(then_block) = then_statement else {
        unreachable!("block validation returns one block statement");
    };

    let mut else_bindings = post_condition.clone();
    let (else_block, else_valid) = if let Some(else_node) = else_node {
        let else_diagnostics = diagnostics.len();
        let else_statement = validate_block(
            header,
            &else_node,
            context,
            &mut else_bindings,
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
            let equal = post_condition.values().all(|enclosing| {
                let then_state = then_bindings
                    .values()
                    .find(|state| state.id == enclosing.id)
                    .expect("then outcome retains every enclosing binding identity");
                let else_state = else_bindings
                    .values()
                    .find(|state| state.id == enclosing.id)
                    .expect("false outcome retains every enclosing binding identity");
                debug_assert_eq!(then_state.ty, enclosing.ty);
                debug_assert_eq!(else_state.ty, enclosing.ty);
                debug_assert_eq!(then_state.mutability, enclosing.mutability);
                debug_assert_eq!(else_state.mutability, enclosing.mutability);
                then_state.ownership == else_state.ownership
            });

            if !equal {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ConditionalOwnershipMismatch,
                    location: location(header.unit, node),
                });
                return None;
            }
            *bindings = then_bindings;
        }
        (true, false) => *bindings = then_bindings,
        (false, true) => *bindings = else_bindings,
        (false, false) => {}
    }

    Some(Statement::If {
        condition,
        then_block,
        else_block: else_block.map(Box::new),
        location: location(header.unit, node),
    })
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
    bindings: &mut BTreeMap<String, BindingState>,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
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
    );
    let initializer = declared.and_then(|required| {
        let value_node = value_child(node);
        validate_value(
            header,
            &value_node,
            required,
            context,
            bindings,
            diagnostics,
        )
    });

    let shadows = bindings.contains_key(&name);
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

    let binding = BindingId(*next_binding);
    *next_binding += 1;
    bindings.insert(
        name.clone(),
        BindingState {
            id: binding,
            ty,
            mutability,
            ownership: StructuralOwnershipState::default(),
        },
    );
    Some(Statement::Local {
        binding,
        name,
        ty,
        mutability,
        initializer,
        location: local_location,
    })
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

        if record_decl.module != header.module {
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

    if seen_fields.len() != record_decl.fields.len() {
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
    bindings: &mut BTreeMap<String, BindingState>,
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

    let mut validation = PatternValidation::new(bindings);
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
        match bindings.get(root_name).cloned() {
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
        let mut producer_bindings = bindings.clone();
        let value = validate_value(
            header,
            &producer_node,
            Type::Record(record),
            context,
            &mut producer_bindings,
            diagnostics,
        )?;
        *bindings = producer_bindings;

        let mut transient = StructuralOwnershipState::default();
        for leaf in &validation.bindings {
            if !leaf.ty.is_duplicable() {
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
        let ownership = if resolved.ty.is_duplicable() {
            OwnedUse::Duplicate
        } else {
            if let Some((root_name, _)) = &direct_root {
                bindings
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
                ownership: StructuralOwnershipState::default(),
            },
        ));
    }

    for (name, state) in new_states {
        let previous = bindings.insert(name, state);
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
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let target_token = direct_token(node, SyntaxKind::Ident);
    let name = key(&target_token);
    let target_location = SourceLocation {
        unit: header.unit,
        range: target_token.text_range(),
    };

    let (target, target_ty) = match bindings.get(&name) {
        Some(binding) => {
            if !binding.mutability.is_mutable() {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ImmutableAssignmentTarget,
                    location: target_location,
                });
                return None;
            }
            (binding.id, binding.ty)
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
        bindings,
        diagnostics,
    )?;

    bindings
        .get_mut(&name)
        .expect("resolved assignment target remains in the active binding scope")
        .ownership = StructuralOwnershipState::default();

    Some(Statement::Assignment {
        target,
        value,
        location: location(header.unit, node),
    })
}

fn validate_call_statement(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let call = direct_child(node, SyntaxKind::DirectCall);
    let (function, arguments, result) =
        validate_call(header, &call, context, bindings, diagnostics)?;
    if result.is_some() {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ResultCallUsedAsStatement,
            location: location(header.unit, &call),
        });
        return None;
    }
    Some(Statement::Call {
        function,
        arguments,
        location: location(header.unit, node),
    })
}

fn validate_terminal_return(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Return {
    let returned = validate_return(header, node, context, bindings, diagnostics);
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
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Return {
    let return_location = location(header.unit, node);
    let value_node = node.children().find(|child| is_value_node(child.kind()));
    let value = match (header.result, value_node) {
        (Some(required), Some(value_node)) => validate_value(
            header,
            &value_node,
            required,
            context,
            bindings,
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

    Return {
        value,
        location: return_location,
    }
}

fn validate_value(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Value> {
    let value_location = location(header.unit, node);
    match node.kind() {
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
            Some(Value {
                ty: found,
                kind: ValueKind::Literal(LiteralValue::Bool(token.kind() == SyntaxKind::KwTrue)),
                location: value_location,
            })
        }
        SyntaxKind::DecimalIntegerLiteral => {
            let literal = materialize_integer_literal(node, required, value_location, diagnostics)?;
            Some(Value {
                ty: required,
                kind: ValueKind::Literal(literal),
                location: value_location,
            })
        }
        SyntaxKind::IdentifierUse => {
            let token = direct_token(node, SyntaxKind::Ident);
            let name = key(&token);
            if let Some(binding) = bindings.get_mut(&name) {
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
                let ownership = if binding.ty.is_duplicable() {
                    OwnedUse::Duplicate
                } else {
                    binding.ownership.consume_path(&[]);
                    OwnedUse::Consume
                };
                return Some(Value {
                    ty: binding.ty,
                    kind: ValueKind::BindingUse {
                        binding: binding.id,
                        ownership,
                    },
                    location: value_location,
                });
            }

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
            None
        }
        SyntaxKind::DirectCall => {
            let (function, arguments, result) =
                validate_call(header, node, context, bindings, diagnostics)?;
            let Some(ty) = result else {
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
            Some(Value {
                ty,
                kind: ValueKind::DirectCall {
                    function,
                    arguments,
                },
                location: value_location,
            })
        }
        SyntaxKind::RecordConstruction => {
            validate_record_construction(header, node, required, context, bindings, diagnostics)
        }
        SyntaxKind::FieldValueUse => {
            validate_field_value_use(header, node, required, context, bindings, diagnostics)
        }
        _ => unreachable!("syntax-clean value has represented value kind"),
    }
}

fn validate_field_value_use(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Value> {
    let value_location = location(header.unit, node);
    let identifiers = node
        .children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Ident)
        .collect::<Vec<_>>();
    let (root_token, selectors) = identifiers
        .split_first()
        .expect("syntax-clean field-value use contains a root identifier");
    debug_assert!(!selectors.is_empty());

    let root_name = key(root_token);
    let root_location = SourceLocation {
        unit: header.unit,
        range: root_token.text_range(),
    };
    let (binding_id, binding_ty) = match bindings.get(&root_name) {
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

    let mut current = binding_ty;
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
        if record_decl.module != header.module {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::InaccessibleRecordField,
                location: selector_location,
            });
            return None;
        }

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
        fields.push(field);
        current = record_decl.fields[field].ty;
    }

    let binding = bindings
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

    if current != required {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::TypeMismatch {
                expected: required,
                found: current,
            },
            location: value_location,
        });
        return None;
    }

    let ownership = if current.is_duplicable() {
        OwnedUse::Duplicate
    } else {
        bindings
            .get_mut(&root_name)
            .expect("resolved field-value root remains in active binding scope")
            .ownership
            .consume_path(&fields);
        OwnedUse::Consume
    };

    Some(Value {
        ty: current,
        kind: ValueKind::FieldValueUse {
            binding: binding_id,
            fields,
            ownership,
        },
        location: value_location,
    })
}

fn validate_record_construction(
    header: &FunctionHeader,
    node: &SyntaxNode,
    required: Type,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Value> {
    let construction_location = location(header.unit, node);
    let target_token = direct_token(node, SyntaxKind::Ident);
    let target_name = key(&target_token);
    let target_location = SourceLocation {
        unit: header.unit,
        range: target_token.text_range(),
    };

    let record = match context
        .modules
        .get(&header.module)
        .and_then(|module| module.namespace.get(&target_name))
        .copied()
        .map(|entity| entity.entity)
    {
        Some(EntityId::Record(record)) => record,
        Some(EntityId::Function(_)) => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::ExpectedRecordType,
                location: target_location,
            });
            return None;
        }
        None => {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::UnresolvedName,
                location: target_location,
            });
            return None;
        }
    };

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
            bindings,
            diagnostics,
        )?;
        fields.push(RecordFieldValue { field, value });
    }

    Some(Value {
        ty: record_ty,
        kind: ValueKind::RecordConstruction { record, fields },
        location: construction_location,
    })
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

fn validate_call(
    header: &FunctionHeader,
    node: &SyntaxNode,
    context: &BodyResolutionContext<'_>,
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(FunctionId, Vec<Value>, Option<Type>)> {
    let function = if let Some(qualified) = node
        .children()
        .find(|child| child.kind() == SyntaxKind::QualifiedModuleMember)
    {
        match resolve_qualified_entity(
            header.unit,
            &qualified,
            context.modules,
            context.imports,
            diagnostics,
        )? {
            EntityId::Function(id) => id,
            EntityId::Record(_) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedFunction,
                    location: location(header.unit, &qualified),
                });
                return None;
            }
        }
    } else {
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
            Some(EntityId::Function(id)) => id,
            Some(EntityId::Record(_)) => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::ExpectedFunction,
                    location: name_location,
                });
                return None;
            }
            None => {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::UnresolvedName,
                    location: name_location,
                });
                return None;
            }
        }
    };

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
    for (argument_node, parameter) in argument_nodes.into_iter().zip(&target.parameters) {
        let argument = validate_value(
            header,
            &argument_node,
            parameter.ty,
            context,
            bindings,
            diagnostics,
        )?;
        arguments.push(argument);
    }

    Some((function, arguments, target.result))
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
        SyntaxKind::BooleanLiteral
            | SyntaxKind::DecimalIntegerLiteral
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
