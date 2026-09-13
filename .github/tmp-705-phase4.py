from pathlib import Path


def replace_exact(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:140]!r}")
    p.write_text(text.replace(old, new, count))

# Public HIR: execution origin owns the category-specific parameter/body representation.
replace_exact(
    "crates/runen-hir/src/lib.rs",
    '''/// One resolved source function entity and its typed body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub id: FunctionId,
    pub module: ModuleId,
    pub name: String,
    pub accessibility: Accessibility,
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<Parameter>,
    pub result: Option<Type>,
    pub safe_reference_result_contract: SafeReferenceResultContract,
    pub body: Body,
    pub location: SourceLocation,
}
''',
    '''/// Category-specific execution origin for one resolved source function entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionExecution {
    Runen {
        parameters: Vec<Parameter>,
        body: Body,
    },
    External {
        parameters: Vec<IntrinsicType>,
    },
}

/// One resolved source function entity with exactly one execution origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub id: FunctionId,
    pub module: ModuleId,
    pub name: String,
    pub accessibility: Accessibility,
    pub type_parameters: Vec<TypeParameter>,
    pub result: Option<Type>,
    pub safe_reference_result_contract: SafeReferenceResultContract,
    pub execution: FunctionExecution,
    pub location: SourceLocation,
}

impl Function {
    #[must_use]
    pub fn parameter_types(&self) -> Vec<Type> {
        match &self.execution {
            FunctionExecution::Runen { parameters, .. } => {
                parameters.iter().map(|parameter| parameter.ty).collect()
            }
            FunctionExecution::External { parameters } => {
                parameters.iter().copied().map(Type::Intrinsic).collect()
            }
        }
    }

    #[must_use]
    pub fn runen_parameters(&self) -> Option<&[Parameter]> {
        match &self.execution {
            FunctionExecution::Runen { parameters, .. } => Some(parameters),
            FunctionExecution::External { .. } => None,
        }
    }

    #[must_use]
    pub fn runen_body(&self) -> Option<&Body> {
        match &self.execution {
            FunctionExecution::Runen { body, .. } => Some(body),
            FunctionExecution::External { .. } => None,
        }
    }

    #[must_use]
    pub const fn is_external(&self) -> bool {
        matches!(self.execution, FunctionExecution::External { .. })
    }
}
''',
)
replace_exact(
    "crates/runen-hir/src/lib.rs",
    "    GenericFunctionValue,\n",
    "    GenericFunctionValue,\n    ExternalFunctionValue,\n",
)

# Builder imports the explicit execution-origin representation.
replace_exact(
    "crates/runen-hir/src/build.rs",
    "    FieldValueReceiver, Function, FunctionId, FunctionType, FunctionTypeId, IntrinsicType,\n",
    "    FieldValueReceiver, Function, FunctionExecution, FunctionId, FunctionType, FunctionTypeId,\n    IntrinsicType,\n",
)

# Internal header carries the same invariant: only Runen headers have bindings/body/function-value type.
replace_exact(
    "crates/runen-hir/src/build.rs",
    '''#[derive(Debug, Clone)]
struct FunctionHeader {
    id: FunctionId,
    module: ModuleId,
    unit: usize,
    name: String,
    accessibility: Accessibility,
    type_parameters: Vec<TypeParameter>,
    parameters: Vec<Parameter>,
    result: Option<Type>,
    safe_reference_result_contract: SafeReferenceResultContract,
    function_type: Option<FunctionTypeId>,
    body: SyntaxNode,
    location: SourceLocation,
}
''',
    '''#[derive(Debug, Clone)]
enum FunctionHeaderExecution {
    Runen {
        parameters: Vec<Parameter>,
        function_type: Option<FunctionTypeId>,
        body: SyntaxNode,
    },
    External {
        parameters: Vec<IntrinsicType>,
    },
}

#[derive(Debug, Clone)]
struct FunctionHeader {
    id: FunctionId,
    module: ModuleId,
    unit: usize,
    name: String,
    accessibility: Accessibility,
    type_parameters: Vec<TypeParameter>,
    result: Option<Type>,
    safe_reference_result_contract: SafeReferenceResultContract,
    execution: FunctionHeaderExecution,
    location: SourceLocation,
}

impl FunctionHeader {
    fn parameter_types(&self) -> Vec<Type> {
        match &self.execution {
            FunctionHeaderExecution::Runen { parameters, .. } => {
                parameters.iter().map(|parameter| parameter.ty).collect()
            }
            FunctionHeaderExecution::External { parameters } => {
                parameters.iter().copied().map(Type::Intrinsic).collect()
            }
        }
    }

    fn runen_parameters(&self) -> Option<&[Parameter]> {
        match &self.execution {
            FunctionHeaderExecution::Runen { parameters, .. } => Some(parameters),
            FunctionHeaderExecution::External { .. } => None,
        }
    }

    fn runen_body(&self) -> Option<&SyntaxNode> {
        match &self.execution {
            FunctionHeaderExecution::Runen { body, .. } => Some(body),
            FunctionHeaderExecution::External { .. } => None,
        }
    }

    fn function_type(&self) -> Option<FunctionTypeId> {
        match self.execution {
            FunctionHeaderExecution::Runen { function_type, .. } => function_type,
            FunctionHeaderExecution::External { .. } => None,
        }
    }

    fn is_external(&self) -> bool {
        matches!(self.execution, FunctionHeaderExecution::External { .. })
    }
}
''',
)

# Collect external declarations into the existing function namespace/identity sequence.
anchor = "                SyntaxKind::FunctionDefinition => {\n"
external_arm = '''                SyntaxKind::ExternalFunctionDeclaration => {
                    let id = FunctionId(functions.len());
                    let mut identifiers = item
                        .children_with_tokens()
                        .filter_map(|element| element.into_token())
                        .filter(|token| token.kind() == SyntaxKind::Ident);
                    let introducer = identifiers
                        .next()
                        .expect("syntax-clean external declaration has contextual introducer");
                    debug_assert_eq!(key(&introducer), "external");
                    let name_token = identifiers
                        .next()
                        .expect("syntax-clean external declaration has one declaration name");
                    debug_assert!(identifiers.next().is_none());
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
'''
p = Path("crates/runen-hir/src/build.rs")
text = p.read_text()
if text.count(anchor) != 1:
    raise SystemExit("FunctionDefinition collection anchor mismatch")
p.write_text(text.replace(anchor, external_arm + anchor, 1))

# Resolve external scalar-only headers before the ordinary named-parameter/generic path.
p = Path("crates/runen-hir/src/build.rs")
text = p.read_text()
anchor = "fn resolve_function_headers(\n"
helper = '''fn resolve_external_function_header(function: &FunctionSyntax) -> FunctionHeader {
    debug_assert_eq!(function.node.kind(), SyntaxKind::ExternalFunctionDeclaration);
    let parameter_list = direct_child(&function.node, SyntaxKind::ParameterList);
    let parameters = parameter_list
        .children()
        .filter(|node| node.kind() == SyntaxKind::TypeRef)
        .map(|type_node| {
            let token = type_node
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .find(|token| !token.kind().is_trivia())
                .expect("syntax-clean external parameter has one intrinsic type token");
            intrinsic_type(token.kind())
                .expect("syntax-clean external parameter type is represented intrinsic")
        })
        .collect::<Vec<_>>();
    let result = function
        .node
        .children()
        .find(|node| node.kind() == SyntaxKind::ResultClause)
        .map(|clause| {
            let type_node = direct_child(&clause, SyntaxKind::TypeRef);
            let token = type_node
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .find(|token| !token.kind().is_trivia())
                .expect("syntax-clean external result has one intrinsic type token");
            Type::Intrinsic(
                intrinsic_type(token.kind())
                    .expect("syntax-clean external result type is represented intrinsic"),
            )
        });
    FunctionHeader {
        id: function.id,
        module: function.module,
        unit: function.unit,
        name: function.name.clone(),
        accessibility: function.accessibility,
        type_parameters: Vec::new(),
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        execution: FunctionHeaderExecution::External { parameters },
        location: function.location,
    }
}

'''
if text.count(anchor) != 1:
    raise SystemExit("resolve_function_headers anchor mismatch")
text = text.replace(anchor, helper + anchor, 1)
loop_anchor = "    for function in syntax {\n        let mut type_parameter_names = BTreeSet::new();\n"
loop_replacement = "    for function in syntax {\n        if function.node.kind() == SyntaxKind::ExternalFunctionDeclaration {\n            headers.push(resolve_external_function_header(function));\n            continue;\n        }\n        debug_assert_eq!(function.node.kind(), SyntaxKind::FunctionDefinition);\n        let mut type_parameter_names = BTreeSet::new();\n"
if text.count(loop_anchor) != 1:
    raise SystemExit("function-header loop anchor mismatch")
text = text.replace(loop_anchor, loop_replacement, 1)
ordinary_fields = '''            accessibility: function.accessibility,
            type_parameters,
            parameters,
            result,
            safe_reference_result_contract,
            function_type,
            body,
            location: function.location,
'''
ordinary_replacement = '''            accessibility: function.accessibility,
            type_parameters,
            result,
            safe_reference_result_contract,
            execution: FunctionHeaderExecution::Runen {
                parameters,
                function_type,
                body,
            },
            location: function.location,
'''
if text.count(ordinary_fields) != 1:
    raise SystemExit("ordinary FunctionHeader construction anchor mismatch")
text = text.replace(ordinary_fields, ordinary_replacement, 1)
p.write_text(text)

# Build public Function execution variants; external declarations never enter body validation.
p = Path("crates/runen-hir/src/build.rs")
text = p.read_text()
old = '''    let mut functions = Vec::with_capacity(headers.len());
    for header in &headers {
        let body = validate_body(
            header,
            &modules,
            &imports,
            &constants,
            &statics,
            &records,
            &headers,
            &function_types,
            &closures,
            &marker_implementation_relation,
            &mut next_binding,
            &mut diagnostics,
        );
        functions.push(Function {
            id: header.id,
            module: header.module,
            name: header.name.clone(),
            accessibility: header.accessibility,
            type_parameters: header.type_parameters.clone(),
            parameters: header.parameters.clone(),
            result: header.result,
            safe_reference_result_contract: header.safe_reference_result_contract,
            body,
            location: header.location,
        });
    }
'''
new = '''    let mut functions = Vec::with_capacity(headers.len());
    for header in &headers {
        let execution = match &header.execution {
            FunctionHeaderExecution::Runen { parameters, .. } => {
                let body = validate_body(
                    header,
                    &modules,
                    &imports,
                    &constants,
                    &statics,
                    &records,
                    &headers,
                    &function_types,
                    &closures,
                    &marker_implementation_relation,
                    &mut next_binding,
                    &mut diagnostics,
                );
                FunctionExecution::Runen {
                    parameters: parameters.clone(),
                    body,
                }
            }
            FunctionHeaderExecution::External { parameters } => FunctionExecution::External {
                parameters: parameters.clone(),
            },
        };
        functions.push(Function {
            id: header.id,
            module: header.module,
            name: header.name.clone(),
            accessibility: header.accessibility,
            type_parameters: header.type_parameters.clone(),
            result: header.result,
            safe_reference_result_contract: header.safe_reference_result_contract,
            execution,
            location: header.location,
        });
    }
'''
if text.count(old) != 1:
    raise SystemExit("public Function construction anchor mismatch")
text = text.replace(old, new, 1)

# Body validation is valid only for Runen-origin headers.
old = "    let mut state = SemanticState::default();\n    for (slot, parameter) in header.parameters.iter().enumerate() {\n"
new = "    let parameters = header\n        .runen_parameters()\n        .expect(\"body validation requires Runen execution origin\");\n    let body = header\n        .runen_body()\n        .expect(\"body validation requires Runen execution origin\");\n    let mut state = SemanticState::default();\n    for (slot, parameter) in parameters.iter().enumerate() {\n"
if text.count(old) != 1:
    raise SystemExit("validate_body parameter anchor mismatch")
text = text.replace(old, new, 1)
text = text.replace("header.parameters[origin].binding", "parameters[origin].binding", 1)
text = text.replace("        &header.body,\n", "        body,\n", 1)
# The normal-continuation contract check is also Runen-body-only.
old = "    for (slot, parameter) in header.parameters.iter().enumerate() {\n        if matches!(\n"
new = "    let parameters = header\n        .runen_parameters()\n        .expect(\"safe-result continuation validation requires Runen execution origin\");\n    for (slot, parameter) in parameters.iter().enumerate() {\n        if matches!(\n"
if text.count(old) != 1:
    raise SystemExit("normal-continuation parameter anchor mismatch")
text = text.replace(old, new, 1)

# Direct calls derive the signature from either execution origin.
old = '''    let parameter_types = target
        .parameters
        .iter()
        .map(|parameter| instantiate_call_type(parameter.ty, target, &type_arguments))
        .collect::<Vec<_>>();
'''
new = '''    let parameter_types = target
        .parameter_types()
        .into_iter()
        .map(|parameter| instantiate_call_type(parameter, target, &type_arguments))
        .collect::<Vec<_>>();
'''
if text.count(old) != 1:
    raise SystemExit("direct-call parameter signature anchor mismatch")
text = text.replace(old, new, 1)

# External functions are direct-call-only and do not reuse the generic-function diagnostic.
old = '''    let target = &context.headers[function.0];
    let Some(function_type) = target.function_type else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::GenericFunctionValue,
            location: value_location,
        });
        return None;
    };
'''
new = '''    let target = &context.headers[function.0];
    if target.is_external() {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::ExternalFunctionValue,
            location: value_location,
        });
        return None;
    }
    let Some(function_type) = target.function_type() else {
        diagnostics.push(Diagnostic {
            kind: DiagnosticKind::GenericFunctionValue,
            location: value_location,
        });
        return None;
    };
'''
if text.count(old) != 1:
    raise SystemExit("function-value classification anchor mismatch")
text = text.replace(old, new, 1)
p.write_text(text)

# Focused HIR conformance.
Path("crates/runen-hir/tests/external_callables.rs").write_text(r'''use runen_hir::{
    DiagnosticKind, FunctionExecution, IntrinsicType, ModuleId, SourceUnit, Statement, Type,
    TypedCompilation, ValueKind, build_typed_hir,
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

#[test]
fn external_declarations_share_function_identity_namespace_but_retain_bodyless_execution_origin() {
    let hir = build(
        "external fn sink(I64); \
         export external fn transform(Bool, F32) -> U64; \
         fn caller() -> U64 { sink(7); return transform(true, 1.0); }",
    )
    .expect("external scalar declarations and direct calls are valid");
    assert_eq!(hir.functions.len(), 3);
    let sink = &hir.functions[0];
    let transform = &hir.functions[1];
    let caller = &hir.functions[2];

    assert_eq!(sink.name, "sink");
    assert_eq!(sink.type_parameters, []);
    assert_eq!(sink.result, None);
    assert_eq!(sink.parameter_types(), vec![Type::Intrinsic(IntrinsicType::I64)]);
    assert!(sink.is_external());
    let FunctionExecution::External { parameters } = &transform.execution else {
        panic!("transform must retain external execution origin");
    };
    assert_eq!(parameters, &[IntrinsicType::Bool, IntrinsicType::F32]);
    assert_eq!(transform.result, Some(Type::Intrinsic(IntrinsicType::U64)));

    let body = caller.runen_body().expect("caller is a Runen body");
    let Statement::Call { target, .. } = &body.statements[0] else {
        panic!("first caller statement must be direct call");
    };
    let runen_hir::CallTarget::Direct { function, type_arguments } = target else {
        panic!("external sink remains a direct function target");
    };
    assert_eq!(*function, sink.id);
    assert!(type_arguments.is_empty());
    let returned = body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("caller returns transform result");
    let ValueKind::Call { target, .. } = &returned.kind else {
        panic!("return value is external call");
    };
    let runen_hir::CallTarget::Direct { function, .. } = target else {
        panic!("transform remains a direct function target");
    };
    assert_eq!(*function, transform.id);
}

#[test]
fn external_function_cannot_form_a_function_value() {
    let errors = build(
        "external fn sink(I64); fn bad() { let f: fn(I64) = sink; f(1); }",
    )
    .expect_err("external functions are direct-call-only");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::ExternalFunctionValue),
        "{errors:?}"
    );
}

#[test]
fn external_and_runen_functions_share_one_module_binding_category() {
    let errors = build("external fn same(I64); fn same(value: I64) {}")
        .expect_err("same-category duplicate binding must be rejected");
    assert!(
        errors
            .iter()
            .any(|error| error.kind == DiagnosticKind::DuplicateModuleBinding),
        "{errors:?}"
    );
}
''')

print("staged #705 typed HIR external execution origin and direct-call semantics")
