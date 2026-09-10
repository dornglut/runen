from pathlib import Path

hir_path = Path('crates/runen-hir/src/lib.rs')
lower_path = Path('crates/runen-core-lowering/src/lib.rs')
hir = hir_path.read_text()
s = lower_path.read_text()

def once(text, old, new, label):
    n = text.count(old)
    assert n == 1, f'{label}: expected one match, found {n}'
    return text.replace(old, new, 1)

# Preserve opaque FunctionTypeId construction while allowing lowering to enumerate the canonical table.
hir = once(hir,
'''    pub fn function_type(&self, id: FunctionTypeId) -> &FunctionType {
        &self.function_types[id.0]
    }

''',
'''    pub fn function_type(&self, id: FunctionTypeId) -> &FunctionType {
        &self.function_types[id.0]
    }

    /// Enumerate canonical structural function types with their opaque per-compilation handles.
    pub fn function_type_entries(&self) -> impl Iterator<Item = (FunctionTypeId, &FunctionType)> {
        self.function_types
            .iter()
            .enumerate()
            .map(|(index, function_type)| (FunctionTypeId(index), function_type))
    }

''', 'function type entry iterator')

# Specialization discovery only follows direct calls. Indirect calls deliberately do not enumerate targets.
s = once(s,
'''            hir::Statement::Call {
                function,
                type_arguments,
                arguments,
                ..
            } => {
                specializations.push(specialization_key_for_call(
                    compilation,
                    current,
                    *function,
                    type_arguments,
                )?);
                for argument in arguments {
                    collect_value_specializations(compilation, current, argument, specializations)?;
                }
            }
''',
'''            hir::Statement::Call {
                target, arguments, ..
            } => {
                if let hir::CallTarget::Direct {
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
                for argument in arguments {
                    collect_value_specializations(compilation, current, argument, specializations)?;
                }
            }
''', 'statement specialization discovery')

s = once(s,
'''        hir::ValueKind::DirectCall {
            function,
            type_arguments,
            arguments,
        } => {
            specializations.push(specialization_key_for_call(
                compilation,
                current,
                *function,
                type_arguments,
            )?);
            for argument in arguments {
                collect_value_specializations(compilation, current, argument, specializations)?;
            }
        }
''',
'''        hir::ValueKind::Call { target, arguments } => {
            if let hir::CallTarget::Direct {
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
            for argument in arguments {
                collect_value_specializations(compilation, current, argument, specializations)?;
            }
        }
''', 'value specialization discovery')

s = once(s,
'''        | hir::ValueKind::RawMove { .. }
        | hir::ValueKind::BindingUse { .. } => {}
''',
'''        | hir::ValueKind::RawMove { .. }
        | hir::ValueKind::BindingUse { .. }
        | hir::ValueKind::FunctionValue { .. } => {}
''', 'function value has no specialization discovery')

# Type-map capacity and canonical callable mapping.
s = once(s,
'''            .and_then(|count| count.checked_add(raw_pointer_types.len()))
            .ok_or(LoweringError::RepresentationLimit("Core type identity"))?;
''',
'''            .and_then(|count| count.checked_add(raw_pointer_types.len()))
            .and_then(|count| count.checked_add(compilation.function_types.len()))
            .ok_or(LoweringError::RepresentationLimit("Core type identity"))?;
''', 'callable type capacity')

s = once(s,
'''        for pointee in raw_pointer_types {
            map.lower_raw_pointer(pointee)?;
        }
        Ok(map)
''',
'''        for pointee in raw_pointer_types {
            map.lower_raw_pointer(pointee)?;
        }
        let mut visiting_function_types = BTreeSet::new();
        for (id, _) in compilation.function_type_entries() {
            map.lower_function_type(compilation, id, &mut visiting_function_types)?;
        }
        Ok(map)
''', 'canonical callable type mapping')

s = once(s,
'''                hir::Type::RawPointer(_) => {
                    return Err(LoweringError::InvalidHirInvariant(
                        "HIR record field contains a raw-pointer type",
                    ));
                }
''',
'''                hir::Type::RawPointer(_) => {
                    return Err(LoweringError::InvalidHirInvariant(
                        "HIR record field contains a raw-pointer type",
                    ));
                }
                hir::Type::Function(_) => {
                    return Err(LoweringError::InvalidHirInvariant(
                        "HIR record field contains a function-value type",
                    ));
                }
''', 'function field exclusion invariant')

insert = '''    fn lower_function_type(
        &mut self,
        compilation: &hir::TypedCompilation,
        id: hir::FunctionTypeId,
        visiting: &mut BTreeSet<hir::FunctionTypeId>,
    ) -> Result<core::TypeId, LoweringError> {
        let ty = hir::Type::Function(id);
        if let Some(mapped) = self.mapped.get(&ty) {
            return Ok(*mapped);
        }
        if !visiting.insert(id) {
            return Err(LoweringError::InvalidHirInvariant(
                "HIR function-type graph is cyclic",
            ));
        }

        let interface = compilation.function_type(id).clone();
        let mut parameters = Vec::with_capacity(interface.parameters.len());
        for parameter in interface.parameters {
            parameters.push(self.lower_function_type_component(compilation, parameter, visiting)?);
        }
        let result = interface
            .result
            .map(|result| self.lower_function_type_component(compilation, result, visiting))
            .transpose()?;
        visiting.remove(&id);

        let contract = lower_safe_reference_result_contract(interface.safe_reference_result_contract);
        let name = format!("fn-type-{}", self.mapped.len());
        let mapped = self.types.push(core::TypeDef::callable(
            name,
            core::CallableInterface::new(parameters, result, contract),
        ));
        if self.mapped.insert(ty, mapped).is_some() {
            return Err(LoweringError::InvalidHirInvariant(
                "duplicate HIR function-type mapping",
            ));
        }
        Ok(mapped)
    }

    fn lower_function_type_component(
        &mut self,
        compilation: &hir::TypedCompilation,
        ty: hir::Type,
        visiting: &mut BTreeSet<hir::FunctionTypeId>,
    ) -> Result<core::TypeId, LoweringError> {
        match ty {
            hir::Type::Function(id) => self.lower_function_type(compilation, id, visiting),
            hir::Type::Parameter(_) => Err(LoweringError::InvalidHirInvariant(
                "HIR function type contains an abstract type parameter",
            )),
            hir::Type::RawPointer(_) => Err(LoweringError::InvalidHirInvariant(
                "HIR function type contains a raw-pointer component",
            )),
            concrete => self.get(concrete),
        }
    }

'''
s = once(s, '    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {\n', insert + '    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {\n', 'function type lowering helpers')

# Function-type safe references may only occur nested inside the canonical table; collect them up front.
s = once(s,
'''    for function in &compilation.functions {
''',
'''    for function_type in &compilation.function_types {
        for ty in function_type.parameters.iter().chain(function_type.result.iter()) {
            if let hir::Type::SafeReference {
                referent,
                permission,
            } = ty
            {
                references.insert((*referent, *permission));
            }
        }
    }
    for function in &compilation.functions {
''', 'nested callable safe-reference collection')

# Statement call lowering uses the shared target relation.
s = once(s,
'''                hir::Statement::Call {
                    function,
                    type_arguments,
                    arguments,
                    ..
                } => {
                    self.lower_call(*function, type_arguments, arguments, None)?;
                }
''',
'''                hir::Statement::Call {
                    target, arguments, ..
                } => {
                    let result = self.lower_call(target, arguments, None)?;
                    if result.is_some() {
                        return Err(LoweringError::InvalidHirInvariant(
                            "no-result HIR call statement produced a result temporary",
                        ));
                    }
                }
''', 'statement call lowering')

# Neutral producer categories.
remaining_direct = s.count('hir::ValueKind::DirectCall { .. }')
assert remaining_direct == 2, f'producer DirectCall checks: {remaining_direct}'
s = s.replace('hir::ValueKind::DirectCall { .. }', 'hir::ValueKind::Call { .. }')

# Value function formation and calls.
s = once(s,
'''            hir::ValueKind::DirectCall {
                function,
                type_arguments,
                arguments,
            } => {
                let arguments = self.lower_arguments(arguments)?;
                let result = self.push_temporary(value.ty)?;
                self.emit_call(
                    *function,
                    type_arguments,
                    arguments,
                    Some(core::Place::local(result)),
                )?;
                Ok(result)
            }
''',
'''            hir::ValueKind::FunctionValue { function } => {
                if !matches!(value.ty, hir::Type::Function(_)) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "HIR function value does not have a function-value type",
                    ));
                }
                let target = SpecializationKey {
                    function: *function,
                    type_arguments: Vec::new(),
                };
                let target_function = self.functions.get(&target).copied().ok_or(
                    LoweringError::InvalidHirInvariant(
                        "HIR function value does not name a non-generic root Core function",
                    ),
                )?;
                let result = self.push_temporary(value.ty)?;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(result),
                    src: core::Operand::FunctionValue(target_function),
                });
                Ok(result)
            }
            hir::ValueKind::Call { target, arguments } => self
                .lower_call(target, arguments, Some(value.ty))?
                .ok_or(LoweringError::InvalidHirInvariant(
                    "result-bearing HIR call did not produce a result temporary",
                )),
''', 'function value and neutral value call lowering')

# Shared call lowering: direct path unchanged; indirect path snapshots callee before arguments.
old_call = '''    fn lower_call(
        &mut self,
        function: hir::FunctionId,
        type_arguments: &[hir::Type],
        arguments: &[hir::Value],
        destination: Option<core::Place>,
    ) -> Result<(), LoweringError> {
        let arguments = self.lower_arguments(arguments)?;
        self.emit_call(function, type_arguments, arguments, destination)
    }
'''
new_call = '''    fn lower_call(
        &mut self,
        target: &hir::CallTarget,
        arguments: &[hir::Value],
        result: Option<hir::Type>,
    ) -> Result<Option<core::LocalId>, LoweringError> {
        match target {
            hir::CallTarget::Direct {
                function,
                type_arguments,
            } => {
                let arguments = self.lower_arguments(arguments)?;
                let result = result.map(|ty| self.push_temporary(ty)).transpose()?;
                self.emit_direct_call(
                    *function,
                    type_arguments,
                    arguments,
                    result.map(core::Place::local),
                )?;
                Ok(result)
            }
            hir::CallTarget::Indirect {
                binding,
                function_type,
            } => {
                let source = self.binding(*binding)?;
                let callable = self.types.get(hir::Type::Function(*function_type))?;
                if self.local_type(source)? != callable {
                    return Err(LoweringError::InvalidHirInvariant(
                        "indirect HIR call binding type does not match its function type",
                    ));
                }

                // Source semantics require the callee value to be evaluated and held before
                // any ordinary argument producer. Snapshot it now; nested argument lowering
                // may create blocks and observable effects before the final call terminator.
                let held_callee = self.push_core_temporary(callable)?;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(held_callee),
                    src: core::Operand::Copy(core::Place::local(source).into()),
                });

                let arguments = self.lower_arguments(arguments)?;
                let result = result.map(|ty| self.push_temporary(ty)).transpose()?;
                let continuation = self.new_block()?;
                self.terminate_current(core::Terminator::IndirectCall {
                    callable,
                    callee: core::Operand::Move(core::Place::local(held_callee).into()),
                    arguments,
                    destination: result.map(core::Place::local),
                    target: continuation,
                })?;
                self.current = continuation.0 as usize;
                Ok(result)
            }
        }
    }
'''
s = once(s, old_call, new_call, 'shared direct/indirect lower_call')
s = once(s, '    fn emit_call(\n', '    fn emit_direct_call(\n', 'direct call emitter name')

# Type exhaustiveness: function values are never reference dereference results.
s = once(s,
'''                    hir::Type::RawPointer(_) => {
                        return Err(LoweringError::InvalidHirInvariant(
                            "reference-dereference HIR value has a raw-pointer result type",
                        ));
                    }
''',
'''                    hir::Type::RawPointer(_) => {
                        return Err(LoweringError::InvalidHirInvariant(
                            "reference-dereference HIR value has a raw-pointer result type",
                        ));
                    }
                    hir::Type::Function(_) => {
                        return Err(LoweringError::InvalidHirInvariant(
                            "reference-dereference HIR value has a function-value result type",
                        ));
                    }
''', 'reference dereference function exclusion')

# One exact mapping helper for the already-accepted source/Core result contract.
anchor = '''fn lower_reference_permission(permission: hir::ReferencePermission) -> core::ReferencePermission {
'''
helper = '''fn lower_safe_reference_result_contract(
    contract: hir::SafeReferenceResultContract,
) -> core::SafeReferenceResultContract {
    match contract {
        hir::SafeReferenceResultContract::None => core::SafeReferenceResultContract::None,
        hir::SafeReferenceResultContract::SharedIdentity { origin } => {
            core::SafeReferenceResultContract::SharedIdentity { origin }
        }
        hir::SafeReferenceResultContract::SharedDirectChild { origin } => {
            core::SafeReferenceResultContract::SharedDirectChild { origin }
        }
    }
}

'''
s = once(s, anchor, helper + anchor, 'safe reference result contract mapper')

assert 'hir::ValueKind::DirectCall' not in s
hir_path.write_text(hir)
lower_path.write_text(s)
print('implemented canonical callable type/function value/direct-indirect call lowering')
