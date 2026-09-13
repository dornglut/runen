from pathlib import Path


def replace_exact(path, old, new, count=1):
    p = Path(path)
    text = p.read_text()
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count} anchors, found {found}: {old[:160]!r}")
    p.write_text(text.replace(old, new, count))

p = Path('crates/runen-core-lowering/src/lib.rs')
text = p.read_text()

# Keep external identities disjoint from Core function-specialization identities.
old = '''    persistent: BTreeMap<hir::StaticId, core::PersistentId>,
    specializations: Vec<SpecializationKey>,
    functions: BTreeMap<SpecializationKey, core::FunctionId>,
'''
new = '''    persistent: BTreeMap<hir::StaticId, core::PersistentId>,
    external_callables: BTreeMap<hir::FunctionId, core::ExternalCallableId>,
    specializations: Vec<SpecializationKey>,
    functions: BTreeMap<SpecializationKey, core::FunctionId>,
'''
if text.count(old) != 1:
    raise SystemExit('Lowerer field anchor mismatch')
text = text.replace(old, new, 1)

old = '''        let (specializations, functions) = discover_specializations(compilation)?;
        let mut closure_functions = BTreeMap::new();
'''
new = '''        let mut external_callables = BTreeMap::new();
        for function in &compilation.functions {
            if !function.is_external() {
                continue;
            }
            let id = core::ExternalCallableId(index_u32(
                external_callables.len(),
                "Core external callable identity",
            )?);
            if external_callables.insert(function.id, id).is_some() {
                return Err(LoweringError::InvalidHirInvariant(
                    "duplicate HIR external function identity",
                ));
            }
        }
        let (specializations, functions) = discover_specializations(compilation)?;
        let mut closure_functions = BTreeMap::new();
'''
if text.count(old) != 1:
    raise SystemExit('Lowerer external-map construction anchor mismatch')
text = text.replace(old, new, 1)
text = text.replace(
    '''            persistent,
            specializations,
            functions,
''',
    '''            persistent,
            external_callables,
            specializations,
            functions,
''',
    1,
)

# Share the exact external map with every Runen/closure body lowerer.
text = text.replace(
    '''            persistent: &self.persistent,
            functions: &self.functions,
''',
    '''            persistent: &self.persistent,
            external_callables: &self.external_callables,
            functions: &self.functions,
''',
    1,
)

# Materialize Core external declarations in the same filtered HIR declaration order used by the map.
old = '''        let program = core::Program {
            persistent,
            types: self.types.types,
            functions,
        };
'''
new = '''        let external_callables = self
            .compilation
            .functions
            .iter()
            .filter(|function| function.is_external())
            .map(|function| {
                let parameters = function
                    .parameter_types()
                    .into_iter()
                    .map(|ty| self.types.get(ty))
                    .collect::<Result<Vec<_>, LoweringError>>()?;
                let result = function.result.map(|ty| self.types.get(ty)).transpose()?;
                if !matches!(
                    function.safe_reference_result_contract,
                    hir::SafeReferenceResultContract::None
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "HIR external function retains a safe-reference result contract",
                    ));
                }
                Ok(core::ExternalCallableDecl::new(core::CallableInterface {
                    parameters,
                    result,
                    safe_reference_result_contract: core::SafeReferenceResultContract::None,
                }))
            })
            .collect::<Result<Vec<_>, LoweringError>>()?;
        if external_callables.len() != self.external_callables.len() {
            return Err(LoweringError::InvalidHirInvariant(
                "HIR external function map and declaration sequence disagree",
            ));
        }
        let program = core::Program {
            persistent,
            external_callables,
            types: self.types.types,
            functions,
        };
'''
if text.count(old) != 1:
    raise SystemExit('Core Program lowering anchor mismatch')
text = text.replace(old, new, 1)

# External functions never enter specialization discovery.
old = '''    for function in &compilation.functions {
        if function.type_parameters.is_empty() {
            let specialization = SpecializationKey {
'''
new = '''    for function in &compilation.functions {
        if function.is_external() {
            if !function.type_parameters.is_empty() {
                return Err(LoweringError::InvalidHirInvariant(
                    "HIR external function unexpectedly has type parameters",
                ));
            }
            continue;
        }
        if function.type_parameters.is_empty() {
            let specialization = SpecializationKey {
'''
if text.count(old) != 1:
    raise SystemExit('root specialization discovery anchor mismatch')
text = text.replace(old, new, 1)

old = '''        let function = find_function(compilation, specialization.function)?;
        let mut discovered = Vec::new();
        collect_body_specializations(
            compilation,
            &specialization,
            &function.body,
            &mut discovered,
        )?;
'''
new = '''        let function = find_function(compilation, specialization.function)?;
        let body = function.runen_body().ok_or(LoweringError::InvalidHirInvariant(
            "Core function specialization names an external HIR function",
        ))?;
        let mut discovered = Vec::new();
        collect_body_specializations(compilation, &specialization, body, &mut discovered)?;
'''
if text.count(old) != 1:
    raise SystemExit('specialization body walk anchor mismatch')
text = text.replace(old, new, 1)

# A direct external call is not a Core specialization edge. Apply to statement and value calls.
old = '''                if let hir::CallTarget::Direct {
                    function,
                    type_arguments,
                } = target
                {
                    specializations.push(specialization_key_for_call(
                        compilation,
                        current,
                        *function,
                        type_arguments,
                    )?);
                }
'''
new = '''                if let hir::CallTarget::Direct {
                    function,
                    type_arguments,
                } = target
                    && !find_function(compilation, *function)?.is_external()
                {
                    specializations.push(specialization_key_for_call(
                        compilation,
                        current,
                        *function,
                        type_arguments,
                    )?);
                }
'''
if text.count(old) != 2:
    raise SystemExit(f'direct specialization edge anchors mismatch: {text.count(old)}')
text = text.replace(old, new, 2)

# Defensive specialization invariant: an external function can never be specialized.
old = '''    let target = find_function(compilation, function)?;
    if type_arguments.len() != target.type_parameters.len() {
'''
new = '''    let target = find_function(compilation, function)?;
    if target.is_external() {
        return Err(LoweringError::InvalidHirInvariant(
            "external HIR function requested a Core function specialization",
        ));
    }
    if type_arguments.len() != target.type_parameters.len() {
'''
if text.count(old) != 1:
    raise SystemExit('specialization key target anchor mismatch')
text = text.replace(old, new, 1)

# Type discovery scans only Runen-local binding/body shapes. External signatures are scalar-only.
old = '''    for function in &compilation.functions {
        for parameter in &function.parameters {
            if let hir::Type::Function(id) = parameter.ty {
                function_types.insert(id);
            }
        }
        if let Some(hir::Type::Function(id)) = function.result {
            function_types.insert(id);
        }
        collect_statement_function_types(&function.body.statements, &mut function_types);
    }
'''
new = '''    for function in &compilation.functions {
        if let Some(parameters) = function.runen_parameters() {
            for parameter in parameters {
                if let hir::Type::Function(id) = parameter.ty {
                    function_types.insert(id);
                }
            }
        }
        if let Some(hir::Type::Function(id)) = function.result {
            function_types.insert(id);
        }
        if let Some(body) = function.runen_body() {
            collect_statement_function_types(&body.statements, &mut function_types);
        }
    }
'''
if text.count(old) != 1:
    raise SystemExit('function-type discovery anchor mismatch')
text = text.replace(old, new, 1)

old = '''    for function in &compilation.functions {
        for parameter in &function.parameters {
            if let hir::Type::SafeReference {
                referent,
                permission,
            } = parameter.ty
            {
                references.insert((referent, permission));
            }
        }
        if let Some(hir::Type::SafeReference {
            referent,
            permission,
        }) = function.result
        {
            references.insert((referent, permission));
        }
        collect_statement_safe_reference_types(&function.body.statements, &mut references);
    }
'''
new = '''    for function in &compilation.functions {
        if let Some(parameters) = function.runen_parameters() {
            for parameter in parameters {
                if let hir::Type::SafeReference {
                    referent,
                    permission,
                } = parameter.ty
                {
                    references.insert((referent, permission));
                }
            }
        }
        if let Some(hir::Type::SafeReference {
            referent,
            permission,
        }) = function.result
        {
            references.insert((referent, permission));
        }
        if let Some(body) = function.runen_body() {
            collect_statement_safe_reference_types(&body.statements, &mut references);
        }
    }
'''
if text.count(old) != 1:
    raise SystemExit('safe-reference type discovery anchor mismatch')
text = text.replace(old, new, 1)

old = '''    for function in &compilation.functions {
        for parameter in &function.parameters {
            collect_raw_pointer_type(parameter.ty, &mut pointees);
        }
        if let Some(result) = function.result {
            collect_raw_pointer_type(result, &mut pointees);
        }
        collect_statement_raw_pointer_types(&function.body.statements, &mut pointees);
    }
'''
new = '''    for function in &compilation.functions {
        if let Some(parameters) = function.runen_parameters() {
            for parameter in parameters {
                collect_raw_pointer_type(parameter.ty, &mut pointees);
            }
        }
        if let Some(result) = function.result {
            collect_raw_pointer_type(result, &mut pointees);
        }
        if let Some(body) = function.runen_body() {
            collect_statement_raw_pointer_types(&body.statements, &mut pointees);
        }
    }
'''
if text.count(old) != 1:
    raise SystemExit('raw-pointer type discovery anchor mismatch')
text = text.replace(old, new, 1)

# Function-body lowering is explicitly Runen-origin-only.
old = '''        validate_specialization_key(context.compilation, specialization)?;
        if specialization.function != function.id {
'''
new = '''        validate_specialization_key(context.compilation, specialization)?;
        let parameters = function.runen_parameters().ok_or(
            LoweringError::InvalidHirInvariant(
                "Core function lowering requires Runen HIR execution origin",
            ),
        )?;
        let body = function.runen_body().ok_or(
            LoweringError::InvalidHirInvariant(
                "Core function lowering requires Runen HIR execution origin",
            ),
        )?;
        if specialization.function != function.id {
'''
if text.count(old) != 1:
    raise SystemExit('FunctionLowerer::new origin anchor mismatch')
text = text.replace(old, new, 1)
# These three occurrences are all within FunctionLowerer::new after the type-discovery loops were rewritten.
if text.count('for parameter in &function.parameters {') != 1:
    raise SystemExit(f'FunctionLowerer parameter loop residual mismatch: {text.count("for parameter in &function.parameters {")}')
text = text.replace('for parameter in &function.parameters {', 'for parameter in parameters {', 1)
if text.count('            .parameters\n            .iter()') != 1:
    raise SystemExit('FunctionLowerer parameter iterator anchor mismatch')
text = text.replace('            .parameters\n            .iter()', '            .runen_parameters()\n            .expect("Runen execution origin established above")\n            .iter()', 1)
text = text.replace('            body: function.body.clone(),', '            body: body.clone(),', 1)
text = text.replace('        lowerer.register_source_locals(&function.body.statements)?;', '        lowerer.register_source_locals(&body.statements)?;', 1)

# Shared body-lowering context gains the external identity map.
old = '''    persistent: &'a BTreeMap<hir::StaticId, core::PersistentId>,
    functions: &'a BTreeMap<SpecializationKey, core::FunctionId>,
'''
new = '''    persistent: &'a BTreeMap<hir::StaticId, core::PersistentId>,
    external_callables: &'a BTreeMap<hir::FunctionId, core::ExternalCallableId>,
    functions: &'a BTreeMap<SpecializationKey, core::FunctionId>,
'''
if text.count(old) != 1:
    raise SystemExit('FunctionLoweringContext external map anchor mismatch')
text = text.replace(old, new, 1)

# The FunctionLowerer owns references to all three identity maps.
old = '''    persistent: &'a BTreeMap<hir::StaticId, core::PersistentId>,
    functions: &'a BTreeMap<SpecializationKey, core::FunctionId>,
    closure_functions: &'a BTreeMap<hir::ClosureId, core::FunctionId>,
    name: String,
'''
new = '''    persistent: &'a BTreeMap<hir::StaticId, core::PersistentId>,
    external_callables: &'a BTreeMap<hir::FunctionId, core::ExternalCallableId>,
    functions: &'a BTreeMap<SpecializationKey, core::FunctionId>,
    closure_functions: &'a BTreeMap<hir::ClosureId, core::FunctionId>,
    name: String,
'''
if text.count(old) != 1:
    raise SystemExit('FunctionLowerer identity-map field anchor mismatch')
text = text.replace(old, new, 1)
# Constructors copy the shared map into both ordinary and closure lowerers.
if text.count('            functions: context.functions,') != 2:
    raise SystemExit(f'FunctionLowerer constructor functions-map anchors mismatch: {text.count("            functions: context.functions,")}')
text = text.replace(
    '            functions: context.functions,',
    '            external_callables: context.external_callables,\n            functions: context.functions,',
    2,
)

# Direct calls dispatch on execution origin: external calls never enter the specialization map.
old = '''    ) -> Result<(), LoweringError> {
        let target = specialization_key_for_call(
            self.compilation,
            self.types.specialization,
            function,
            type_arguments,
        )?;
        let target_function =
            self.functions
                .get(&target)
                .copied()
                .ok_or(LoweringError::InvalidHirInvariant(
                    "reachable HIR specialization is absent from function map",
                ))?;
        let continuation = self.new_block()?;
        self.terminate_current(core::Terminator::Call {
            function: target_function,
            arguments,
            destination,
            target: continuation,
        })?;
        self.current = continuation.0 as usize;
        Ok(())
    }
'''
new = '''    ) -> Result<(), LoweringError> {
        let target_declaration = find_function(self.compilation, function)?;
        let continuation = self.new_block()?;
        if target_declaration.is_external() {
            if !type_arguments.is_empty() {
                return Err(LoweringError::InvalidHirInvariant(
                    "external HIR direct call unexpectedly has type arguments",
                ));
            }
            let external = self.external_callables.get(&function).copied().ok_or(
                LoweringError::InvalidHirInvariant(
                    "HIR external function is absent from external callable map",
                ),
            )?;
            self.terminate_current(core::Terminator::ExternalCall {
                external,
                arguments,
                destination,
                target: continuation,
            })?;
        } else {
            let target = specialization_key_for_call(
                self.compilation,
                self.types.specialization,
                function,
                type_arguments,
            )?;
            let target_function = self.functions.get(&target).copied().ok_or(
                LoweringError::InvalidHirInvariant(
                    "reachable HIR specialization is absent from function map",
                ),
            )?;
            self.terminate_current(core::Terminator::Call {
                function: target_function,
                arguments,
                destination,
                target: continuation,
            })?;
        }
        self.current = continuation.0 as usize;
        Ok(())
    }
'''
if text.count(old) != 1:
    raise SystemExit('emit_direct_call anchor mismatch')
text = text.replace(old, new, 1)

p.write_text(text)

# Focused lowering conformance.
Path('crates/runen-core-lowering/tests/external_callables.rs').write_text(r'''use runen_core_ir::{
    ExternalCallableId, ScalarType, Terminator, TypeKind, ValidatedProgram,
};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("external-callable source must produce accepted HIR");
    lower(&hir).expect("accepted external-callable HIR must lower to validated Core")
}

fn external_calls(program: &runen_core_ir::Program) -> Vec<ExternalCallableId> {
    program
        .functions
        .iter()
        .flat_map(|function| &function.body.blocks)
        .filter_map(|block| match block.terminator {
            Terminator::ExternalCall { external, .. } => Some(external),
            _ => None,
        })
        .collect()
}

#[test]
fn external_declaration_lowers_once_without_becoming_a_core_function() {
    let lowered = lower_source(
        "external fn transform(I64, Bool) -> U64; \
         fn caller() -> U64 { return transform(7, true); }",
    );
    let program = lowered.as_program();
    assert_eq!(program.external_callables.len(), 1);
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "caller");
    let interface = &program.external_callables[0].interface;
    assert_eq!(interface.parameters.len(), 2);
    assert_eq!(interface.result.is_some(), true);
    assert!(matches!(
        program.types.get(interface.parameters[0]).map(|ty| &ty.kind),
        Some(TypeKind::Scalar(ScalarType::I64))
    ));
    assert!(matches!(
        program.types.get(interface.parameters[1]).map(|ty| &ty.kind),
        Some(TypeKind::Scalar(ScalarType::Bool))
    ));
    assert_eq!(external_calls(program), vec![ExternalCallableId(0)]);
}

#[test]
fn generic_specialization_and_closure_reuse_one_external_identity() {
    let lowered = lower_source(
        "external fn ext(I64) -> I64; \
         fn generic[T](value: T) -> I64 { return ext(7); } \
         fn root() -> I64 { \
             let call = fn[](value: I64) -> I64 { return ext(value); }; \
             let first: I64 = generic[I64](1); \
             return call(first); \
         }",
    );
    let program = lowered.as_program();
    assert_eq!(program.external_callables.len(), 1);
    assert!(program.functions.iter().all(|function| function.name != "ext"));
    let calls = external_calls(program);
    assert_eq!(calls.len(), 2, "generic body and closure wrapper each call ext once");
    assert!(calls.iter().all(|id| *id == ExternalCallableId(0)));
}
''')

print('staged #705 HIR-to-Core external callable lowering')
