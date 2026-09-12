from pathlib import Path
import re


def replace_exact(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:100]!r}")
    p.write_text(text.replace(old, new, count))

# Syntax: append node 137 and parse contextual external declarations.
replace_exact(
    "crates/runen-syntax/src/lib.rs",
    "    StaticDeclaration,\n}",
    "    StaticDeclaration,\n    ExternalFunctionDeclaration,\n}",
)
replace_exact(
    "crates/runen-syntax/src/lib.rs",
    "            136 => SyntaxKind::StaticDeclaration,\n            other => panic!(\"unknown Runen syntax kind {other}\"),",
    "            136 => SyntaxKind::StaticDeclaration,\n            137 => SyntaxKind::ExternalFunctionDeclaration,\n            other => panic!(\"unknown Runen syntax kind {other}\"),",
)
replace_exact(
    "crates/runen-syntax/src/parser.rs",
    "                Some(SyntaxKind::KwExport) if self.at_contextual_ident(1, \"static\") => {\n                    self.parse_static_declaration(true);\n                }\n                Some(SyntaxKind::KwExport) => match self.peek_nontrivia(1) {",
    "                Some(SyntaxKind::KwExport) if self.at_contextual_ident(1, \"static\") => {\n                    self.parse_static_declaration(true);\n                }\n                Some(SyntaxKind::KwExport) if self.at_contextual_ident(1, \"external\") => {\n                    self.parse_external_function_declaration(true);\n                }\n                Some(SyntaxKind::KwExport) => match self.peek_nontrivia(1) {",
)
replace_exact(
    "crates/runen-syntax/src/parser.rs",
    "                Some(SyntaxKind::Ident) if self.at_contextual_ident(0, \"static\") => {\n                    self.parse_static_declaration(false);\n                }\n                Some(_) => {",
    "                Some(SyntaxKind::Ident) if self.at_contextual_ident(0, \"static\") => {\n                    self.parse_static_declaration(false);\n                }\n                Some(SyntaxKind::Ident) if self.at_contextual_ident(0, \"external\") => {\n                    self.parse_external_function_declaration(false);\n                }\n                Some(_) => {",
)
anchor = "    fn parse_function_definition(&mut self, exported: bool) {\n"
insert = '''    fn parse_external_function_declaration(&mut self, exported: bool) {
        debug_assert!(self.at_contextual_ident(usize::from(exported), "external"));
        self.builder
            .start_node(SyntaxKind::ExternalFunctionDeclaration.into());
        if exported {
            self.expect(SyntaxKind::KwExport, ExpectedSyntax::Item);
        }
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Item);
        self.expect(SyntaxKind::KwFn, ExpectedSyntax::Item);
        self.expect(SyntaxKind::Ident, ExpectedSyntax::Identifier);
        self.builder.start_node(SyntaxKind::ParameterList.into());
        if self.expect(SyntaxKind::LParen, ExpectedSyntax::LeftParen) {
            self.bump_trivia();
            while !self.at(SyntaxKind::RParen) && self.current().is_some() {
                if self.at(SyntaxKind::Arrow)
                    || self.at(SyntaxKind::LBrace)
                    || self.at(SyntaxKind::LBracket)
                    || self.at_any(TOP_LEVEL_STARTERS)
                {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::RightParen));
                    break;
                }
                self.parse_external_intrinsic_type();
                if self.eat(SyntaxKind::Comma) {
                    self.bump_trivia();
                    continue;
                }
                if !self.at(SyntaxKind::RParen) {
                    self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::CommaOrRightParen));
                    self.recover_until(&[
                        SyntaxKind::Comma,
                        SyntaxKind::RParen,
                        SyntaxKind::Arrow,
                        SyntaxKind::Semicolon,
                        SyntaxKind::LBrace,
                        SyntaxKind::LBracket,
                        SyntaxKind::KwImport,
                        SyntaxKind::KwExport,
                        SyntaxKind::KwFn,
                        SyntaxKind::KwRecord,
                    ]);
                    self.eat(SyntaxKind::Comma);
                }
                self.bump_trivia();
            }
            self.expect(SyntaxKind::RParen, ExpectedSyntax::RightParen);
        }
        self.builder.finish_node();

        if self.at(SyntaxKind::Arrow) {
            self.builder.start_node(SyntaxKind::ResultClause.into());
            self.bump();
            self.parse_external_intrinsic_type();
            self.builder.finish_node();
        }
        self.expect(SyntaxKind::Semicolon, ExpectedSyntax::Semicolon);
        self.builder.finish_node();
    }

    fn parse_external_intrinsic_type(&mut self) {
        self.builder.start_node(SyntaxKind::TypeRef.into());
        if self.current().is_some_and(is_intrinsic_type_start) {
            self.bump();
        } else {
            self.error_here(SyntaxErrorKind::Expected(ExpectedSyntax::Type));
            if self.current().is_some()
                && !self.at(SyntaxKind::Comma)
                && !self.at(SyntaxKind::RParen)
                && !self.at(SyntaxKind::Arrow)
                && !self.at(SyntaxKind::Semicolon)
            {
                self.recover_one();
            }
        }
        self.builder.finish_node();
    }

'''
p = Path("crates/runen-syntax/src/parser.rs")
text = p.read_text()
if text.count(anchor) != 1:
    raise SystemExit("parser function-definition anchor mismatch")
p.write_text(text.replace(anchor, insert + anchor, 1))

Path("crates/runen-syntax/tests/external_callables.rs").write_text(r'''use runen_syntax::{SyntaxKind, parse};

fn count(parse: &runen_syntax::Parse, kind: SyntaxKind) -> usize {
    parse.syntax().descendants().filter(|node| node.kind() == kind).count()
}

#[test]
fn parses_private_and_exported_scalar_external_declarations() {
    let parsed = parse("external fn sink(); export external fn transform(I64, Bool,) -> U64;");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::ExternalFunctionDeclaration), 2);
    assert_eq!(count(&parsed, SyntaxKind::ParameterList), 2);
    assert_eq!(count(&parsed, SyntaxKind::ResultClause), 1);
    assert_eq!(count(&parsed, SyntaxKind::TypeRef), 3);
}

#[test]
fn external_remains_contextual_identifier_outside_item_introducer() {
    let parsed = parse("fn external(external: I64) -> I64 { return external; }");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    assert_eq!(count(&parsed, SyntaxKind::ExternalFunctionDeclaration), 0);
    assert_eq!(count(&parsed, SyntaxKind::FunctionDefinition), 1);
}

#[test]
fn external_declaration_rejects_named_nonintrinsic_generic_and_body_shapes() {
    for source in [
        "external fn bad(value: I64);",
        "external fn bad(Name);",
        "external fn bad[T](I64);",
        "external fn bad(I64) { return; }",
        "external fn bad(I64) -> Name;",
    ] {
        assert!(!parse(source).errors().is_empty(), "unexpectedly valid: {source}");
    }
}
''')

# Core identity/data model.
replace_exact(
    "crates/runen-core-ir/src/common.rs",
    "pub struct FunctionId(pub u32);\n\n/// Stable-in-one-program identity for one execution-persistent storage declaration.",
    "pub struct FunctionId(pub u32);\n\n/// Stable-in-one-program identity for one declaration-only external callable requirement.\n#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct ExternalCallableId(pub u32);\n\n/// Stable-in-one-program identity for one execution-persistent storage declaration.",
)
replace_exact(
    "crates/runen-core-ir/src/interprocedural.rs",
    "    BasicBlockId, CallableInterface, Fault, FunctionId, LoanDecl, LocalDecl, LocalId, Operand,\n    PersistentDecl, PersistentId, Place, SafeReferenceResultContract, Statement, TypeId, TypeTable,\n",
    "    BasicBlockId, CallableInterface, ExternalCallableId, Fault, FunctionId, LoanDecl, LocalDecl,\n    LocalId, Operand, PersistentDecl, PersistentId, Place, SafeReferenceResultContract, Statement,\n    TypeId, TypeTable,\n",
)
replace_exact(
    "crates/runen-core-ir/src/interprocedural.rs",
    "    Call {\n        function: FunctionId,\n        arguments: Vec<Operand>,\n        destination: Option<Place>,\n        target: BasicBlockId,\n    },\n",
    "    Call {\n        function: FunctionId,\n        arguments: Vec<Operand>,\n        destination: Option<Place>,\n        target: BasicBlockId,\n    },\n    ExternalCall {\n        external: ExternalCallableId,\n        arguments: Vec<Operand>,\n        destination: Option<Place>,\n        target: BasicBlockId,\n    },\n",
)
insert_before_program = '''/// One declaration-only external callable requirement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalCallableDecl {
    pub interface: CallableInterface,
}

impl ExternalCallableDecl {
    #[must_use]
    pub const fn new(interface: CallableInterface) -> Self {
        Self { interface }
    }
}

'''
p = Path("crates/runen-core-ir/src/interprocedural.rs")
text = p.read_text()
anchor = "/// One finite Core program with one shared type-identity domain.\n"
if text.count(anchor) != 1:
    raise SystemExit("Core program anchor mismatch")
text = text.replace(anchor, insert_before_program + anchor, 1)
text = text.replace(
    "pub struct Program {\n    pub types: TypeTable,\n    pub persistent: Vec<PersistentDecl>,\n    pub functions: Vec<Function>,\n}",
    "pub struct Program {\n    pub types: TypeTable,\n    pub persistent: Vec<PersistentDecl>,\n    pub external_callables: Vec<ExternalCallableDecl>,\n    pub functions: Vec<Function>,\n}",
    1,
)
text = text.replace(
    "    pub fn function(&self, id: FunctionId) -> Option<&Function> {\n        self.functions.get(id.0 as usize)\n    }",
    "    pub fn external_callable(&self, id: ExternalCallableId) -> Option<&ExternalCallableDecl> {\n        self.external_callables.get(id.0 as usize)\n    }\n\n    #[must_use]\n    pub fn function(&self, id: FunctionId) -> Option<&Function> {\n        self.functions.get(id.0 as usize)\n    }",
    1,
)
p.write_text(text)
replace_exact(
    "crates/runen-core-ir/src/lib.rs",
    "pub use interprocedural::{BasicBlock, Body, Function, Program, Terminator};",
    "pub use interprocedural::{BasicBlock, Body, ExternalCallableDecl, Function, Program, Terminator};",
)

# Core validation.
p = Path("crates/runen-core-ir/src/interprocedural_validation.rs")
text = p.read_text()
text = text.replace(
    "    BasicBlockId, BorrowKind, CallableInterface, FunctionId, LoanDecl, LoanId, LocalId, Operand,\n    PersistentId, Place, PlaceAccess, Projection, ReferenceAccess, ReferencePermission,\n",
    "    BasicBlockId, BorrowKind, CallableInterface, ExternalCallableId, FunctionId, LoanDecl, LoanId,\n    LocalId, Operand, PersistentId, Place, PlaceAccess, Projection, ReferenceAccess, ReferencePermission,\n",
    1,
)
text = text.replace(
    "    InvalidFunction(FunctionId),\n    InvalidPersistent(PersistentId),",
    "    InvalidFunction(FunctionId),\n    InvalidExternalCallable(ExternalCallableId),\n    InvalidExternalCallableType {\n        external: ExternalCallableId,\n        ty: TypeId,\n    },\n    ExternalCallableRequiresNoReferenceResultContract(ExternalCallableId),\n    InvalidPersistent(PersistentId),",
    1,
)
text = text.replace(
    "    validate_type_table(&program.types)?;\n    validate_persistent_declarations(&program)?;\n",
    "    validate_type_table(&program.types)?;\n    validate_persistent_declarations(&program)?;\n    validate_external_callable_declarations(&program)?;\n",
    1,
)
anchor = "fn validate_persistent_declarations(program: &Program) -> Result<(), MirValidationError> {\n"
external_validation = '''fn validate_external_callable_declarations(program: &Program) -> Result<(), MirValidationError> {
    for (index, declaration) in program.external_callables.iter().enumerate() {
        let external = ExternalCallableId(
            u32::try_from(index).expect("external callable declaration index exceeds u32::MAX"),
        );
        if !matches!(
            declaration.interface.safe_reference_result_contract,
            SafeReferenceResultContract::None
        ) {
            return Err(program_error(
                MirValidationErrorKind::ExternalCallableRequiresNoReferenceResultContract(external),
            ));
        }
        for ty in declaration
            .interface
            .parameters
            .iter()
            .copied()
            .chain(declaration.interface.result)
        {
            if program.types.get(ty).is_none() {
                return Err(program_error(MirValidationErrorKind::UnknownType(ty)));
            }
            if !is_external_scalar_type(&program.types, ty) {
                return Err(program_error(
                    MirValidationErrorKind::InvalidExternalCallableType { external, ty },
                ));
            }
        }
    }
    Ok(())
}

fn is_external_scalar_type(types: &TypeTable, ty: TypeId) -> bool {
    matches!(
        types.get(ty).map(|definition| &definition.kind),
        Some(TypeKind::Scalar(
            ScalarType::Bool
                | ScalarType::I8
                | ScalarType::I16
                | ScalarType::I32
                | ScalarType::I64
                | ScalarType::U8
                | ScalarType::U16
                | ScalarType::U32
                | ScalarType::U64
                | ScalarType::F16
                | ScalarType::F32
                | ScalarType::F64
        ))
    )
}

'''
if text.count(anchor) != 1:
    raise SystemExit("persistent validation anchor mismatch")
text = text.replace(anchor, external_validation + anchor, 1)
call_arm = '''        Terminator::Call {
            function: target_id,
            arguments,
            destination,
            target,
        } => {
            require_target(body, *target, point)?;
            let callee = program.function(*target_id).ok_or_else(|| {
                point_error(point, MirValidationErrorKind::InvalidFunction(*target_id))
            })?;
            let interface = callee
                .callable_interface()
                .expect("declaration validation establishes the direct-call interface");
            validate_static_call_destination(&program.types, body, &interface, destination, point)?;
            validate_static_call_arguments(program, body, &interface, arguments, point)
        }
'''
external_arm = call_arm + '''        Terminator::ExternalCall {
            external,
            arguments,
            destination,
            target,
        } => {
            require_target(body, *target, point)?;
            let declaration = program.external_callable(*external).ok_or_else(|| {
                point_error(point, MirValidationErrorKind::InvalidExternalCallable(*external))
            })?;
            validate_static_call_destination(
                &program.types,
                body,
                &declaration.interface,
                destination,
                point,
            )?;
            validate_static_call_arguments(
                program,
                body,
                &declaration.interface,
                arguments,
                point,
            )
        }
'''
if text.count(call_arm) != 1:
    raise SystemExit("static Call arm mismatch")
text = text.replace(call_arm, external_arm, 1)
# Add may_fault to state validation so external calls do not inherit callee fault cleanup.
text = text.replace(
    "    destination: &'a Option<Place>,\n}",
    "    destination: &'a Option<Place>,\n    may_fault: bool,\n}",
    1,
)
text = text.replace(
    "        destination,\n    } = call;",
    "        destination,\n        may_fault,\n    } = call;",
    1,
)
text = text.replace(
    "                            destination,\n                        },",
    "                            destination,\n                            may_fault: true,\n                        },",
    2,
)
text = text.replace(
    "    let mut fault_state = state.clone();\n    destroy_transient_values(types, &held, &mut fault_state);\n    cleanup_function(types, body, &mut fault_state, point)?;\n",
    "    if may_fault {\n        let mut fault_state = state.clone();\n        destroy_transient_values(types, &held, &mut fault_state);\n        cleanup_function(types, body, &mut fault_state, point)?;\n    }\n",
    1,
)
path_call_arm = '''            Terminator::Call {
                function: target_function,
                arguments,
                destination,
                target,
            } => {
                let callee = program
                    .function(*target_function)
                    .expect("static validation establishes call target");
                let interface = callee
                    .callable_interface()
                    .expect("declaration validation establishes direct-call interface");
                if matches!(
                    validate_call_state(
                        types,
                        body,
                        &mut state,
                        CallStateInput {
                            interface: &interface,
                            callee: None,
                            arguments,
                            destination,
                            may_fault: true,
                        },
                        &point,
                    )?,
                    DefinedStep::NoDefinedContinuation
                ) {
                    continue 'worklist;
                }
                state.current = *target;
                worklist.push_back(state);
            }
'''
path_external_arm = path_call_arm + '''            Terminator::ExternalCall {
                external,
                arguments,
                destination,
                target,
            } => {
                let declaration = program
                    .external_callable(*external)
                    .expect("static validation establishes external call target");
                if matches!(
                    validate_call_state(
                        types,
                        body,
                        &mut state,
                        CallStateInput {
                            interface: &declaration.interface,
                            callee: None,
                            arguments,
                            destination,
                            may_fault: false,
                        },
                        &point,
                    )?,
                    DefinedStep::NoDefinedContinuation
                ) {
                    continue 'worklist;
                }
                state.current = *target;
                worklist.push_back(state);
            }
'''
if text.count(path_call_arm) != 1:
    raise SystemExit("path Call arm mismatch after may_fault edits")
text = text.replace(path_call_arm, path_external_arm, 1)
p.write_text(text)

# Mechanical Program constructor migration across Rust files. This is a real public field.
for path in Path(".").rglob("*.rs"):
    if any(part in {"target", ".git"} for part in path.parts):
        continue
    if path.as_posix() == "crates/runen-core-ir/src/interprocedural.rs":
        continue
    text = path.read_text()
    pattern = re.compile(r"(?<!struct )\b(?:core::)?Program\s*\{")
    matches = list(pattern.finditer(text))
    if not matches:
        continue
    offset = 0
    chars = text
    for m in matches:
        start = m.end() + offset
        # Do not duplicate an already migrated constructor nearby.
        if "external_callables:" in chars[start:start + 300]:
            continue
        line_start = chars.rfind("\n", 0, m.start() + offset) + 1
        base_indent = re.match(r"\s*", chars[line_start:m.start() + offset]).group(0)
        insertion = "\n" + base_indent + "    external_callables: Vec::new(),"
        chars = chars[:start] + insertion + chars[start:]
        offset += len(insertion)
    path.write_text(chars)

Path("crates/runen-core-ir/tests/external_callables.rs").write_text(r'''use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, ExternalCallableDecl, ExternalCallableId,
    Function, LocalDecl, LocalId, MirValidationErrorKind, Operand, Place, Program,
    SafeReferenceResultContract, ScalarType, Terminator, TypeDef, TypeId, TypeTable, validate_program,
};

fn body(locals: Vec<LocalDecl>, terminator: Terminator) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(Vec::new(), terminator)],
    }
}

fn root(result: Option<TypeId>, body: Body) -> Function {
    Function {
        name: "root".into(),
        parameters: Vec::new(),
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body,
    }
}

#[test]
fn external_identity_and_scalar_interface_are_distinct_from_functions() {
    let first = ExternalCallableId(0);
    let second = ExternalCallableId(1);
    assert_ne!(first, second);

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let interface = CallableInterface {
        parameters: vec![i64_ty],
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
    };
    let function = root(
        Some(i64_ty),
        body(
            vec![LocalDecl::new("result", i64_ty, false)],
            Terminator::ExternalCall {
                external: first,
                arguments: vec![Operand::Constant(runen_core_ir::Value::I64(4))],
                destination: Some(Place::local(LocalId(0))),
                target: BasicBlockId(1),
            },
        ),
    );
    let mut function = function;
    function.body.blocks.push(BasicBlock::new(
        Vec::new(),
        Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
    ));
    validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(interface.clone()), ExternalCallableDecl::new(interface)],
        types,
        persistent: Vec::new(),
        functions: vec![function],
    })
    .expect("scalar external call is valid");
}

#[test]
fn external_declarations_reject_non_scalar_and_reference_contract_interfaces() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let record_ty = types.push(TypeDef::structure("Record", vec![]));
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: vec![record_ty],
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
        })],
        types,
        persistent: Vec::new(),
        functions: Vec::new(),
    })
    .expect_err("aggregate external interface is invalid");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidExternalCallableType {
            external: ExternalCallableId(0),
            ty: record_ty,
        }
    );

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(CallableInterface {
            parameters: vec![i64_ty],
            result: Some(i64_ty),
            safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity { origin: 0 },
        })],
        types,
        persistent: Vec::new(),
        functions: Vec::new(),
    })
    .expect_err("external interface contract must be None");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::ExternalCallableRequiresNoReferenceResultContract(ExternalCallableId(0))
    );
}

#[test]
fn external_call_checks_target_arity_and_destination_shape() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let interface = CallableInterface {
        parameters: vec![i64_ty],
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
    };
    let error = validate_program(Program {
        external_callables: vec![ExternalCallableDecl::new(interface)],
        types,
        persistent: Vec::new(),
        functions: vec![root(
            None,
            body(
                Vec::new(),
                Terminator::ExternalCall {
                    external: ExternalCallableId(0),
                    arguments: Vec::new(),
                    destination: None,
                    target: BasicBlockId(0),
                },
            ),
        )],
    })
    .expect_err("result-bearing external call requires destination");
    assert_eq!(error.kind, MirValidationErrorKind::MissingResultDestination);
}
''')

print("staged #705 phase-1 syntax and Core external-call substrate")
