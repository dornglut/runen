use std::collections::{BTreeMap, BTreeSet};

use runen_syntax::{SyntaxKind, SyntaxNode, SyntaxToken, identifier_key};

use crate::{
    BindingId, Body, Diagnostic, DiagnosticKind, Field, Function, FunctionId, IntrinsicType,
    Module, ModuleId, OwnedUse, Parameter, Record, RecordId, Return, SourceLocation, SourceUnit,
    Statement, Type, TypedCompilation, Value, ValueKind,
};

#[derive(Debug, Clone, Copy)]
enum EntityId {
    Record(RecordId),
    Function(FunctionId),
}

#[derive(Debug, Default)]
struct ModuleBuild {
    namespace: BTreeMap<String, EntityId>,
    records: Vec<RecordId>,
    functions: Vec<FunctionId>,
}

#[derive(Debug, Clone)]
struct RecordSyntax {
    id: RecordId,
    module: ModuleId,
    unit: usize,
    name: String,
    node: SyntaxNode,
    location: SourceLocation,
}

#[derive(Debug, Clone)]
struct FunctionSyntax {
    id: FunctionId,
    module: ModuleId,
    unit: usize,
    name: String,
    node: SyntaxNode,
    location: SourceLocation,
}

#[derive(Debug, Clone)]
struct FunctionHeader {
    id: FunctionId,
    module: ModuleId,
    unit: usize,
    name: String,
    parameters: Vec<Parameter>,
    result: Option<Type>,
    body: SyntaxNode,
    location: SourceLocation,
}

#[derive(Debug, Clone, Copy)]
struct BindingState {
    id: BindingId,
    ty: Type,
    available: bool,
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

    let records = resolve_records(&record_syntax, &modules, &mut diagnostics);
    let mut next_binding = 0_usize;
    let headers = resolve_function_headers(
        &function_syntax,
        &modules,
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
            &headers,
            &mut next_binding,
            &mut diagnostics,
        );
        functions.push(Function {
            id: header.id,
            module: header.module,
            name: header.name.clone(),
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
                SyntaxKind::RecordDefinition => {
                    let id = RecordId(records.len());
                    let name_token = direct_token(&item, SyntaxKind::Ident);
                    let name = key(&name_token);
                    let location = location(unit_index, &item);
                    if insert_entity(&mut modules, unit.module, &name, EntityId::Record(id)) {
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
                        node: item,
                        location,
                    });
                }
                SyntaxKind::FunctionDefinition => {
                    let id = FunctionId(functions.len());
                    let name_token = direct_token(&item, SyntaxKind::Ident);
                    let name = key(&name_token);
                    let location = location(unit_index, &item);
                    if insert_entity(&mut modules, unit.module, &name, EntityId::Function(id)) {
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
                        node: item,
                        location,
                    });
                }
                _ => unreachable!("syntax-clean source unit contains only represented items"),
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
) -> bool {
    let namespace = &mut modules.entry(module).or_default().namespace;
    if namespace.contains_key(name) {
        false
    } else {
        namespace.insert(name.to_owned(), entity);
        true
    }
}

fn resolve_records(
    syntax: &[RecordSyntax],
    modules: &BTreeMap<ModuleId, ModuleBuild>,
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
            if let Some(ty) =
                resolve_type(record.module, record.unit, &type_node, modules, diagnostics)
            {
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
            fields,
            location: record.location,
        });
    }
    records
}

fn resolve_function_headers(
    syntax: &[FunctionSyntax],
    modules: &BTreeMap<ModuleId, ModuleBuild>,
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
                diagnostics,
            ) {
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
                resolve_type(
                    function.module,
                    function.unit,
                    &type_node,
                    modules,
                    diagnostics,
                )
            });
        let body = direct_child(&function.node, SyntaxKind::Body);
        headers.push(FunctionHeader {
            id: function.id,
            module: function.module,
            unit: function.unit,
            name: function.name.clone(),
            parameters,
            result,
            body,
            location: function.location,
        });
    }
    headers
}

fn resolve_type(
    module: ModuleId,
    unit: usize,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
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
                available: true,
            },
        );
    }

    let mut statements = Vec::new();
    let mut terminal_return = None;
    for node in header.body.children() {
        match node.kind() {
            SyntaxKind::LocalDeclaration => {
                if let Some(statement) = validate_local(
                    header,
                    &node,
                    modules,
                    headers,
                    &mut bindings,
                    next_binding,
                    diagnostics,
                ) {
                    statements.push(statement);
                }
            }
            SyntaxKind::CallStatement => {
                if let Some(statement) = validate_call_statement(
                    header,
                    &node,
                    modules,
                    headers,
                    &mut bindings,
                    diagnostics,
                ) {
                    statements.push(statement);
                }
            }
            SyntaxKind::ReturnStatement => {
                terminal_return = Some(validate_return(
                    header,
                    &node,
                    modules,
                    headers,
                    &mut bindings,
                    diagnostics,
                ));
            }
            _ => unreachable!("syntax-clean body contains only represented body nodes"),
        }
    }

    match (header.result, terminal_return.as_ref()) {
        (Some(_), None) => diagnostics.push(Diagnostic {
            kind: DiagnosticKind::MissingResultReturn,
            location: header.location,
        }),
        (
            Some(_),
            Some(Return {
                value: None,
                location,
            }),
        ) => diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ExpectedResultValue,
            location: *location,
        }),
        (
            None,
            Some(Return {
                value: Some(_),
                location,
            }),
        ) => diagnostics.push(Diagnostic {
            kind: DiagnosticKind::UnexpectedResultValue,
            location: *location,
        }),
        _ => {}
    }

    Body {
        statements,
        terminal_return,
    }
}

fn validate_local(
    header: &FunctionHeader,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    headers: &[FunctionHeader],
    bindings: &mut BTreeMap<String, BindingState>,
    next_binding: &mut usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let name_token = direct_token(node, SyntaxKind::Ident);
    let name = key(&name_token);
    let local_location = location(header.unit, node);
    let type_node = direct_child(node, SyntaxKind::TypeRef);
    let declared = resolve_type(header.module, header.unit, &type_node, modules, diagnostics);
    let value_node = value_child(node);
    let initializer = validate_value(header, &value_node, modules, headers, bindings, diagnostics);

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
    if initializer.ty != ty {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::TypeMismatch {
                expected: ty,
                found: initializer.ty,
            },
            location: initializer.location,
        });
        return None;
    }
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
            available: true,
        },
    );
    Some(Statement::Local {
        binding,
        name,
        ty,
        initializer,
        location: local_location,
    })
}

fn validate_call_statement(
    header: &FunctionHeader,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    headers: &[FunctionHeader],
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Statement> {
    let call = direct_child(node, SyntaxKind::DirectCall);
    let (function, arguments, result) =
        validate_call(header, &call, modules, headers, bindings, diagnostics)?;
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

fn validate_return(
    header: &FunctionHeader,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    headers: &[FunctionHeader],
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Return {
    let return_location = location(header.unit, node);
    let value = node.children().find(|child| {
        matches!(
            child.kind(),
            SyntaxKind::IdentifierUse | SyntaxKind::DirectCall
        )
    });
    let value = value.and_then(|value_node| {
        validate_value(header, &value_node, modules, headers, bindings, diagnostics)
    });

    if let (Some(expected), Some(value)) = (header.result, value.as_ref())
        && value.ty != expected
    {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::TypeMismatch {
                expected,
                found: value.ty,
            },
            location: value.location,
        });
    }

    Return {
        value,
        location: return_location,
    }
}

fn validate_value(
    header: &FunctionHeader,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    headers: &[FunctionHeader],
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Value> {
    match node.kind() {
        SyntaxKind::IdentifierUse => {
            let token = direct_token(node, SyntaxKind::Ident);
            let name = key(&token);
            let value_location = location(header.unit, node);
            if let Some(binding) = bindings.get_mut(&name) {
                if !binding.available {
                    diagnostics.push(Diagnostic {
                        kind: DiagnosticKind::UnavailableBinding,
                        location: value_location,
                    });
                    return None;
                }
                let ownership = if binding.ty.is_duplicable() {
                    OwnedUse::Duplicate
                } else {
                    binding.available = false;
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

            let entity = modules
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
            let call_location = location(header.unit, node);
            let (function, arguments, result) =
                validate_call(header, node, modules, headers, bindings, diagnostics)?;
            let Some(ty) = result else {
                diagnostics.push(Diagnostic {
                    kind: DiagnosticKind::NoResultCallUsedAsValue,
                    location: call_location,
                });
                return None;
            };
            Some(Value {
                ty,
                kind: ValueKind::DirectCall {
                    function,
                    arguments,
                },
                location: call_location,
            })
        }
        _ => unreachable!("syntax-clean value has represented value kind"),
    }
}

fn validate_call(
    header: &FunctionHeader,
    node: &SyntaxNode,
    modules: &BTreeMap<ModuleId, ModuleBuild>,
    headers: &[FunctionHeader],
    bindings: &mut BTreeMap<String, BindingState>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(FunctionId, Vec<Value>, Option<Type>)> {
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

    let function = match modules
        .get(&header.module)
        .and_then(|module| module.namespace.get(&name))
        .copied()
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
    };
    let target = &headers[function.0];
    let argument_list = direct_child(node, SyntaxKind::ArgumentList);
    let argument_nodes = argument_list
        .children()
        .filter(|child| {
            matches!(
                child.kind(),
                SyntaxKind::IdentifierUse | SyntaxKind::DirectCall
            )
        })
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
            modules,
            headers,
            bindings,
            diagnostics,
        )?;
        if argument.ty != parameter.ty {
            diagnostics.push(Diagnostic {
                kind: DiagnosticKind::TypeMismatch {
                    expected: parameter.ty,
                    found: argument.ty,
                },
                location: argument.location,
            });
            return None;
        }
        arguments.push(argument);
    }

    Some((function, arguments, target.result))
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
        .find(|child| {
            matches!(
                child.kind(),
                SyntaxKind::IdentifierUse | SyntaxKind::DirectCall
            )
        })
        .expect("syntax-clean value-producing construct contains a value")
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
