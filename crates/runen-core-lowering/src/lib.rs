#![forbid(unsafe_code)]
//! Implementation-only lowering from accepted typed source HIR to validated Core MIR.

use std::collections::{BTreeMap, BTreeSet};

use runen_core_ir as core;
use runen_hir as hir;

/// Failure to refine an accepted typed HIR compilation into the represented Core form.
///
/// These are compiler-boundary failures, not source-language diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoweringError {
    RepresentationLimit(&'static str),
    InvalidHirInvariant(&'static str),
    CoreValidation(core::MirValidationError),
}

/// Lower one accepted typed HIR compilation into a Core program that has passed
/// the canonical Core validator.
pub fn lower(compilation: &hir::TypedCompilation) -> Result<core::ValidatedProgram, LoweringError> {
    Lowerer::new(compilation)?.lower()
}

struct Lowerer<'a> {
    compilation: &'a hir::TypedCompilation,
    types: TypeMap,
    functions: BTreeMap<hir::FunctionId, core::FunctionId>,
}

impl<'a> Lowerer<'a> {
    fn new(compilation: &'a hir::TypedCompilation) -> Result<Self, LoweringError> {
        let types = TypeMap::new(compilation)?;
        let mut functions = BTreeMap::new();
        for (index, function) in compilation.functions.iter().enumerate() {
            let id = core::FunctionId(index_u32(index, "Core function identity")?);
            if functions.insert(function.id, id).is_some() {
                return Err(LoweringError::InvalidHirInvariant(
                    "duplicate HIR function identity",
                ));
            }
        }
        Ok(Self {
            compilation,
            types,
            functions,
        })
    }

    fn lower(self) -> Result<core::ValidatedProgram, LoweringError> {
        let mut functions = Vec::with_capacity(self.compilation.functions.len());
        for function in &self.compilation.functions {
            functions.push(
                FunctionLowerer::new(self.compilation, &self.types, &self.functions, function)?
                    .lower()?,
            );
        }

        let program = core::Program {
            types: self.types.types,
            functions,
        };
        core::validate_program(program).map_err(LoweringError::CoreValidation)
    }
}

struct TypeMap {
    types: core::TypeTable,
    mapped: BTreeMap<hir::Type, core::TypeId>,
}

impl TypeMap {
    fn new(compilation: &hir::TypedCompilation) -> Result<Self, LoweringError> {
        let reference_types = collect_used_safe_reference_types(compilation);
        let raw_pointer_types = collect_used_raw_pointer_types(compilation);
        let maximum_types = compilation
            .records
            .len()
            .checked_add(INTRINSICS.len())
            .and_then(|count| count.checked_add(reference_types.len()))
            .and_then(|count| count.checked_add(raw_pointer_types.len()))
            .ok_or(LoweringError::RepresentationLimit("Core type identity"))?;
        if index_u32(maximum_types - 1, "Core type identity").is_err() {
            return Err(LoweringError::RepresentationLimit("Core type identity"));
        }

        let mut map = Self {
            types: core::TypeTable::new(),
            mapped: BTreeMap::new(),
        };
        for (intrinsic, scalar, name) in INTRINSICS {
            let id = map.types.push(core::TypeDef::scalar(name, scalar));
            map.mapped.insert(hir::Type::Intrinsic(intrinsic), id);
        }

        let mut visiting = BTreeSet::new();
        for record in &compilation.records {
            map.lower_record(compilation, record.id, &mut visiting)?;
        }
        for (referent, permission) in reference_types {
            map.lower_safe_reference(referent, permission)?;
        }
        for pointee in raw_pointer_types {
            map.lower_raw_pointer(pointee)?;
        }
        Ok(map)
    }

    fn lower_record(
        &mut self,
        compilation: &hir::TypedCompilation,
        id: hir::RecordId,
        visiting: &mut BTreeSet<hir::RecordId>,
    ) -> Result<core::TypeId, LoweringError> {
        let ty = hir::Type::Record(id);
        if let Some(mapped) = self.mapped.get(&ty) {
            return Ok(*mapped);
        }
        if !visiting.insert(id) {
            return Err(LoweringError::InvalidHirInvariant(
                "HIR record containment graph is cyclic",
            ));
        }

        let record = compilation.record(id).clone();
        let mut fields = Vec::with_capacity(record.fields.len());
        for field in record.fields {
            let field_ty = match field.ty {
                hir::Type::Intrinsic(intrinsic) => self
                    .mapped
                    .get(&hir::Type::Intrinsic(intrinsic))
                    .copied()
                    .ok_or(LoweringError::InvalidHirInvariant(
                        "represented intrinsic type is not mapped",
                    ))?,
                hir::Type::Record(record) => self.lower_record(compilation, record, visiting)?,
                hir::Type::SafeReference { .. } => {
                    return Err(LoweringError::InvalidHirInvariant(
                        "HIR record field contains a safe-reference type",
                    ));
                }
                hir::Type::RawPointer(_) => {
                    return Err(LoweringError::InvalidHirInvariant(
                        "HIR record field contains a raw-pointer type",
                    ));
                }
            };
            fields.push(core::Field::new(field.name, field_ty));
        }
        visiting.remove(&id);

        let mapped = self
            .types
            .push(core::TypeDef::structure(record.name, fields));
        self.mapped.insert(ty, mapped);
        Ok(mapped)
    }

    fn lower_safe_reference(
        &mut self,
        referent: hir::ReferenceReferent,
        permission: hir::ReferencePermission,
    ) -> Result<core::TypeId, LoweringError> {
        let ty = hir::Type::SafeReference {
            referent,
            permission,
        };
        if let Some(mapped) = self.mapped.get(&ty) {
            return Ok(*mapped);
        }

        let referent_ty = self.get(referent.ty())?;
        let referent_name = self
            .types
            .get(referent_ty)
            .ok_or(LoweringError::InvalidHirInvariant(
                "safe-reference referent is absent from the Core type table",
            ))?
            .name
            .clone();
        let name = match permission {
            hir::ReferencePermission::Shared => format!("&{referent_name}"),
            hir::ReferencePermission::ExclusiveReplace => format!("&mut {referent_name}"),
        };
        let mapped = self.types.push(core::TypeDef::reference(
            name,
            referent_ty,
            lower_reference_permission(permission),
        ));
        if self.mapped.insert(ty, mapped).is_some() {
            return Err(LoweringError::InvalidHirInvariant(
                "duplicate HIR safe-reference type mapping",
            ));
        }
        Ok(mapped)
    }

    fn lower_raw_pointer(
        &mut self,
        pointee: hir::RawPointerPointee,
    ) -> Result<core::TypeId, LoweringError> {
        let ty = hir::Type::RawPointer(pointee);
        if let Some(mapped) = self.mapped.get(&ty) {
            return Ok(*mapped);
        }

        let pointee_ty = self.get(pointee.ty())?;
        let pointee_name = self
            .types
            .get(pointee_ty)
            .ok_or(LoweringError::InvalidHirInvariant(
                "raw-pointer pointee is absent from the Core type table",
            ))?
            .name
            .clone();
        let mapped = self.types.push(core::TypeDef::raw_pointer(
            format!("raw {pointee_name}"),
            pointee_ty,
        ));
        if self.mapped.insert(ty, mapped).is_some() {
            return Err(LoweringError::InvalidHirInvariant(
                "duplicate HIR raw-pointer type mapping",
            ));
        }
        Ok(mapped)
    }

    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {
        self.mapped
            .get(&ty)
            .copied()
            .ok_or(LoweringError::InvalidHirInvariant(
                "HIR type is absent from the lowering type map",
            ))
    }

    fn raw_pointer_pointee(&self, ty: core::TypeId) -> Result<core::TypeId, LoweringError> {
        let definition = self
            .types
            .get(ty)
            .ok_or(LoweringError::InvalidHirInvariant(
                "lowered Core type is absent from the type table",
            ))?;
        match &definition.kind {
            core::TypeKind::Scalar(core::ScalarType::RawPointer(pointee)) => Ok(*pointee),
            _ => Err(LoweringError::InvalidHirInvariant(
                "HIR raw-pointer binding does not lower to a Core raw-pointer type",
            )),
        }
    }

    fn project_type(
        &self,
        root: core::TypeId,
        projections: &[core::Projection],
    ) -> Result<core::TypeId, LoweringError> {
        self.types
            .project_type(root, projections)
            .ok_or(LoweringError::InvalidHirInvariant(
                "HIR structural path does not match lowered Core type shape",
            ))
    }

    fn has_scalar_leaf(&self, ty: core::TypeId) -> Result<bool, LoweringError> {
        let definition = self
            .types
            .get(ty)
            .ok_or(LoweringError::InvalidHirInvariant(
                "lowered Core type is absent from the type table",
            ))?;
        match &definition.kind {
            core::TypeKind::Scalar(_) => Ok(true),
            core::TypeKind::Struct(fields) => {
                for field in fields {
                    if self.has_scalar_leaf(field.ty)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
        }
    }

    fn remaining_frontier_after_consumed_path(
        &self,
        root: core::TypeId,
        consumed: &[usize],
    ) -> Result<Vec<Vec<usize>>, LoweringError> {
        if consumed.is_empty() {
            return Err(LoweringError::InvalidHirInvariant(
                "field-value use has empty field path",
            ));
        }

        let mut frontier = Vec::new();
        let mut path = Vec::new();
        self.append_remaining_frontier_after_consumed_path(
            root,
            consumed,
            0,
            &mut path,
            &mut frontier,
        )?;
        Ok(frontier)
    }

    fn append_remaining_frontier_after_consumed_path(
        &self,
        ty: core::TypeId,
        consumed: &[usize],
        depth: usize,
        path: &mut Vec<usize>,
        frontier: &mut Vec<Vec<usize>>,
    ) -> Result<(), LoweringError> {
        if depth == consumed.len() {
            return Ok(());
        }

        let definition = self
            .types
            .get(ty)
            .ok_or(LoweringError::InvalidHirInvariant(
                "lowered Core type is absent from the type table",
            ))?;
        let core::TypeKind::Struct(fields) = &definition.kind else {
            return Err(LoweringError::InvalidHirInvariant(
                "HIR structural path does not match lowered Core type shape",
            ));
        };
        let selected = consumed[depth];
        if selected >= fields.len() {
            return Err(LoweringError::InvalidHirInvariant(
                "HIR structural path does not match lowered Core type shape",
            ));
        }

        for (field_index, field) in fields.iter().enumerate().rev() {
            path.push(field_index);
            if field_index == selected {
                self.append_remaining_frontier_after_consumed_path(
                    field.ty,
                    consumed,
                    depth + 1,
                    path,
                    frontier,
                )?;
            } else {
                frontier.push(path.clone());
            }
            path.pop();
        }
        Ok(())
    }
}

fn collect_used_safe_reference_types(
    compilation: &hir::TypedCompilation,
) -> BTreeSet<(hir::ReferenceReferent, hir::ReferencePermission)> {
    let mut references = BTreeSet::new();
    for function in &compilation.functions {
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
    references
}

fn collect_statement_safe_reference_types(
    statements: &[hir::Statement],
    references: &mut BTreeSet<(hir::ReferenceReferent, hir::ReferencePermission)>,
) {
    for statement in statements {
        match statement {
            hir::Statement::Local { ty, .. } => {
                if let hir::Type::SafeReference {
                    referent,
                    permission,
                } = ty
                {
                    references.insert((*referent, *permission));
                }
            }
            hir::Statement::Block(block) => {
                collect_statement_safe_reference_types(&block.statements, references);
            }
            hir::Statement::If {
                then_block,
                else_block,
                ..
            } => {
                collect_statement_safe_reference_types(&then_block.statements, references);
                if let Some(else_block) = else_block {
                    collect_statement_safe_reference_types(&else_block.statements, references);
                }
            }
            hir::Statement::While { body, .. } => {
                collect_statement_safe_reference_types(&body.statements, references);
            }
            hir::Statement::RecordDestructure { .. }
            | hir::Statement::Assignment { .. }
            | hir::Statement::ReferenceAssign { .. }
            | hir::Statement::RawAssign { .. }
            | hir::Statement::Call { .. }
            | hir::Statement::Fault { .. }
            | hir::Statement::Break { .. }
            | hir::Statement::Continue { .. } => {}
        }
    }
}

fn collect_used_raw_pointer_types(
    compilation: &hir::TypedCompilation,
) -> BTreeSet<hir::RawPointerPointee> {
    let mut pointees = BTreeSet::new();
    for function in &compilation.functions {
        collect_statement_raw_pointer_types(&function.body.statements, &mut pointees);
    }
    pointees
}

fn collect_statement_raw_pointer_types(
    statements: &[hir::Statement],
    pointees: &mut BTreeSet<hir::RawPointerPointee>,
) {
    for statement in statements {
        match statement {
            hir::Statement::Local { ty, .. } => {
                if let hir::Type::RawPointer(pointee) = ty {
                    pointees.insert(*pointee);
                }
            }
            hir::Statement::Block(block) => {
                collect_statement_raw_pointer_types(&block.statements, pointees);
            }
            hir::Statement::If {
                then_block,
                else_block,
                ..
            } => {
                collect_statement_raw_pointer_types(&then_block.statements, pointees);
                if let Some(else_block) = else_block {
                    collect_statement_raw_pointer_types(&else_block.statements, pointees);
                }
            }
            hir::Statement::While { body, .. } => {
                collect_statement_raw_pointer_types(&body.statements, pointees);
            }
            hir::Statement::RecordDestructure { .. }
            | hir::Statement::Assignment { .. }
            | hir::Statement::ReferenceAssign { .. }
            | hir::Statement::RawAssign { .. }
            | hir::Statement::Call { .. }
            | hir::Statement::Fault { .. }
            | hir::Statement::Break { .. }
            | hir::Statement::Continue { .. } => {}
        }
    }
}

const EXPLICIT_FAULT_CODE: &str = "source.explicit";

const INTRINSICS: [(hir::IntrinsicType, core::ScalarType, &str); 12] = [
    (hir::IntrinsicType::Bool, core::ScalarType::Bool, "Bool"),
    (hir::IntrinsicType::I8, core::ScalarType::I8, "I8"),
    (hir::IntrinsicType::I16, core::ScalarType::I16, "I16"),
    (hir::IntrinsicType::I32, core::ScalarType::I32, "I32"),
    (hir::IntrinsicType::I64, core::ScalarType::I64, "I64"),
    (hir::IntrinsicType::U8, core::ScalarType::U8, "U8"),
    (hir::IntrinsicType::U16, core::ScalarType::U16, "U16"),
    (hir::IntrinsicType::U32, core::ScalarType::U32, "U32"),
    (hir::IntrinsicType::U64, core::ScalarType::U64, "U64"),
    (hir::IntrinsicType::F16, core::ScalarType::F16, "F16"),
    (hir::IntrinsicType::F32, core::ScalarType::F32, "F32"),
    (hir::IntrinsicType::F64, core::ScalarType::F64, "F64"),
];

#[derive(Default)]
struct BlockDraft {
    statements: Vec<core::Statement>,
    terminator: Option<core::Terminator>,
}

#[derive(Clone, Copy)]
struct LoopLoweringTarget {
    header: core::BasicBlockId,
    continuation: core::BasicBlockId,
}

struct FunctionLowerer<'a> {
    compilation: &'a hir::TypedCompilation,
    types: &'a TypeMap,
    functions: &'a BTreeMap<hir::FunctionId, core::FunctionId>,
    function: &'a hir::Function,
    locals: Vec<core::LocalDecl>,
    bindings: BTreeMap<hir::BindingId, core::LocalId>,
    parameter_locals: Vec<core::LocalId>,
    blocks: Vec<BlockDraft>,
    current: usize,
    next_temp: usize,
    loops: Vec<LoopLoweringTarget>,
}

impl<'a> FunctionLowerer<'a> {
    fn new(
        compilation: &'a hir::TypedCompilation,
        types: &'a TypeMap,
        functions: &'a BTreeMap<hir::FunctionId, core::FunctionId>,
        function: &'a hir::Function,
    ) -> Result<Self, LoweringError> {
        if function
            .parameters
            .iter()
            .any(|parameter| matches!(parameter.ty, hir::Type::RawPointer(_)))
        {
            return Err(LoweringError::InvalidHirInvariant(
                "HIR function parameter contains a raw-pointer type",
            ));
        }
        if function
            .result
            .is_some_and(|result| matches!(result, hir::Type::RawPointer(_)))
        {
            return Err(LoweringError::InvalidHirInvariant(
                "HIR function result contains a raw-pointer type",
            ));
        }

        let mut lowerer = Self {
            compilation,
            types,
            functions,
            function,
            locals: Vec::new(),
            bindings: BTreeMap::new(),
            parameter_locals: Vec::new(),
            blocks: vec![BlockDraft::default()],
            current: 0,
            next_temp: 0,
            loops: Vec::new(),
        };

        for parameter in &function.parameters {
            let local = lowerer.push_source_local(parameter.name.clone(), parameter.ty, false)?;
            if lowerer.bindings.insert(parameter.binding, local).is_some() {
                return Err(LoweringError::InvalidHirInvariant(
                    "duplicate HIR binding identity",
                ));
            }
            lowerer.parameter_locals.push(local);
        }

        lowerer.register_source_locals(&function.body.statements)?;

        Ok(lowerer)
    }

    fn register_source_locals(
        &mut self,
        statements: &[hir::Statement],
    ) -> Result<(), LoweringError> {
        for statement in statements {
            match statement {
                hir::Statement::Local {
                    binding,
                    name,
                    ty,
                    mutability,
                    ..
                } => {
                    let local = self.push_source_local(
                        name.clone(),
                        *ty,
                        matches!(mutability, hir::AssignmentMutability::Mutable),
                    )?;
                    if self.bindings.insert(*binding, local).is_some() {
                        return Err(LoweringError::InvalidHirInvariant(
                            "duplicate HIR binding identity",
                        ));
                    }
                }
                hir::Statement::RecordDestructure { bindings, .. } => {
                    for binding in bindings {
                        let local =
                            self.push_source_local(binding.name.clone(), binding.ty, false)?;
                        if self.bindings.insert(binding.binding, local).is_some() {
                            return Err(LoweringError::InvalidHirInvariant(
                                "duplicate HIR binding identity",
                            ));
                        }
                    }
                }
                hir::Statement::Block(block) => self.register_source_locals(&block.statements)?,
                hir::Statement::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.register_source_locals(&then_block.statements)?;
                    if let Some(else_block) = else_block {
                        self.register_source_locals(&else_block.statements)?;
                    }
                }
                hir::Statement::While { body, .. } => {
                    self.register_source_locals(&body.statements)?;
                }
                hir::Statement::Assignment { .. }
                | hir::Statement::ReferenceAssign { .. }
                | hir::Statement::RawAssign { .. }
                | hir::Statement::Call { .. }
                | hir::Statement::Fault { .. }
                | hir::Statement::Break { .. }
                | hir::Statement::Continue { .. } => {}
            }
        }
        Ok(())
    }

    fn lower(mut self) -> Result<core::Function, LoweringError> {
        let body = self.function.body.clone();
        let sequence_normal = self.lower_statements(&body.statements)?;
        Self::validate_sequence_completion(
            sequence_normal,
            body.terminal_return.as_ref(),
            body.has_normal_continuation,
        )?;

        if self.function.result.is_some() && body.has_normal_continuation {
            return Err(LoweringError::InvalidHirInvariant(
                "result-bearing HIR body has normal continuation",
            ));
        }

        if let Some(returned) = body.terminal_return.as_ref() {
            self.lower_return(returned)?;
        } else if body.has_normal_continuation {
            self.terminate_current(core::Terminator::Return(None))?;
        }

        if !self.loops.is_empty() {
            return Err(LoweringError::InvalidHirInvariant(
                "lowering loop target stack is unbalanced",
            ));
        }

        let result = self
            .function
            .result
            .map(|ty| self.types.get(ty))
            .transpose()?;

        let blocks = self
            .blocks
            .into_iter()
            .map(|block| {
                let terminator = block.terminator.ok_or(LoweringError::InvalidHirInvariant(
                    "lowered Core block has no terminator",
                ))?;
                Ok(core::BasicBlock::new(block.statements, terminator))
            })
            .collect::<Result<Vec<_>, LoweringError>>()?;

        let safe_reference_result_contract = match self.function.safe_reference_result_contract {
            hir::SafeReferenceResultContract::None => core::SafeReferenceResultContract::None,
            hir::SafeReferenceResultContract::SharedIdentity { origin } => {
                core::SafeReferenceResultContract::SharedIdentity { origin }
            }
            hir::SafeReferenceResultContract::SharedDirectChild { origin } => {
                core::SafeReferenceResultContract::SharedDirectChild { origin }
            }
        };

        Ok(core::Function {
            name: self.function.name.clone(),
            parameters: self.parameter_locals,
            result,
            safe_reference_result_contract,
            body: core::Body {
                locals: self.locals,
                loans: Vec::new(),
                entry: core::BasicBlockId(0),
                blocks,
            },
        })
    }

    fn lower_statements(&mut self, statements: &[hir::Statement]) -> Result<bool, LoweringError> {
        let mut has_normal_continuation = true;
        for statement in statements {
            if !has_normal_continuation {
                return Err(LoweringError::InvalidHirInvariant(
                    "HIR statement follows a no-normal-continuation statement",
                ));
            }

            match statement {
                hir::Statement::Local {
                    binding,
                    initializer,
                    ..
                } => {
                    let destination = self.binding(*binding)?;
                    let value = self.lower_value(initializer)?;
                    self.push_statement(core::Statement::Init {
                        dst: core::Place::local(destination),
                        src: core::Operand::Move(core::Place::local(value).into()),
                    });
                }
                hir::Statement::RecordDestructure {
                    record,
                    scrutinee,
                    bindings,
                    ..
                } => self.lower_record_destructure(*record, scrutinee, bindings)?,
                hir::Statement::Assignment {
                    target,
                    fields,
                    value,
                    ..
                } => {
                    let destination = self.binding_place(*target, fields)?;
                    let root_ty = self.local_type(destination.local)?;
                    let projected_ty = self.types.project_type(root_ty, &destination.projections)?;
                    let retained_ty = self.types.get(value.ty)?;
                    if projected_ty != retained_ty {
                        return Err(LoweringError::InvalidHirInvariant(
                            "assignment target type does not match retained RHS type",
                        ));
                    }
                    let value_local = self.lower_value(value)?;
                    if self.local_type(value_local)? != retained_ty {
                        return Err(LoweringError::InvalidHirInvariant(
                            "assignment RHS temporary type does not match retained HIR type",
                        ));
                    }
                    self.push_statement(core::Statement::Assign {
                        dst: destination.into(),
                        src: core::Operand::Move(core::Place::local(value_local).into()),
                    });
                }
                hir::Statement::ReferenceAssign {
                    reference, value, ..
                } => {
                    self.lower_reference_assign(*reference, value)?;
                }
                hir::Statement::RawAssign { pointer, value, .. } => {
                    self.lower_raw_assign(*pointer, value)?;
                }
                hir::Statement::Call {
                    function,
                    arguments,
                    ..
                } => {
                    self.lower_call(*function, arguments, None)?;
                }
                hir::Statement::Fault { .. } => {
                    self.terminate_current(core::Terminator::Fault(core::Fault::new(
                        EXPLICIT_FAULT_CODE,
                    )))?;
                }
                hir::Statement::Break { cleanup, .. } => {
                    self.lower_loop_transfer(cleanup, true)?;
                }
                hir::Statement::Continue { cleanup, .. } => {
                    self.lower_loop_transfer(cleanup, false)?;
                }
                hir::Statement::Block(block) => self.lower_block(block)?,
                hir::Statement::If {
                    condition,
                    then_block,
                    else_block,
                    ..
                } => self.lower_if(condition, then_block, else_block.as_deref())?,
                hir::Statement::While {
                    condition, body, ..
                } => self.lower_while(condition, body)?,
            }

            has_normal_continuation = Self::statement_has_normal_continuation(statement);
        }
        Ok(has_normal_continuation)
    }

    fn statement_has_normal_continuation(statement: &hir::Statement) -> bool {
        match statement {
            hir::Statement::Fault { .. }
            | hir::Statement::Break { .. }
            | hir::Statement::Continue { .. } => false,
            hir::Statement::Block(block) => block.has_normal_continuation,
            hir::Statement::If {
                then_block,
                else_block,
                ..
            } => else_block.as_ref().is_none_or(|else_block| {
                then_block.has_normal_continuation || else_block.has_normal_continuation
            }),
            hir::Statement::While { .. }
            | hir::Statement::Local { .. }
            | hir::Statement::RecordDestructure { .. }
            | hir::Statement::Assignment { .. }
            | hir::Statement::ReferenceAssign { .. }
            | hir::Statement::RawAssign { .. }
            | hir::Statement::Call { .. } => true,
        }
    }

    fn validate_sequence_completion(
        sequence_normal: bool,
        terminal_return: Option<&hir::Return>,
        retained_normal: bool,
    ) -> Result<(), LoweringError> {
        match terminal_return {
            Some(_) if !sequence_normal => Err(LoweringError::InvalidHirInvariant(
                "HIR terminal return follows a no-normal-continuation statement",
            )),
            Some(_) if retained_normal => Err(LoweringError::InvalidHirInvariant(
                "HIR terminal return retains normal continuation",
            )),
            Some(_) => Ok(()),
            None if sequence_normal != retained_normal => Err(LoweringError::InvalidHirInvariant(
                "HIR retained continuation disagrees with its statement sequence",
            )),
            None => Ok(()),
        }
    }

    fn lower_return(&mut self, returned: &hir::Return) -> Result<(), LoweringError> {
        let terminator = match returned.value.as_ref() {
            Some(value) => {
                let result = self.lower_value(value)?;
                core::Terminator::Return(Some(core::Operand::Move(
                    core::Place::local(result).into(),
                )))
            }
            None => core::Terminator::Return(None),
        };
        self.terminate_current(terminator)
    }

    fn lower_block(&mut self, block: &hir::Block) -> Result<(), LoweringError> {
        if !block.has_normal_continuation && !block.normal_cleanup.is_empty() {
            return Err(LoweringError::InvalidHirInvariant(
                "no-normal HIR block retains normal cleanup",
            ));
        }

        let sequence_normal = self.lower_statements(&block.statements)?;
        Self::validate_sequence_completion(
            sequence_normal,
            block.terminal_return.as_ref(),
            block.has_normal_continuation,
        )?;

        if let Some(returned) = block.terminal_return.as_ref() {
            self.lower_return(returned)?;
        } else if block.has_normal_continuation {
            self.lower_cleanup_paths(&block.normal_cleanup)?;
        }
        Ok(())
    }

    fn lower_cleanup_paths(&mut self, cleanup: &[hir::CleanupPath]) -> Result<(), LoweringError> {
        for cleanup in cleanup {
            let place = self.binding_place(cleanup.binding, &cleanup.fields)?;
            let root_ty = self.local_type(place.local)?;
            let ty = self.types.project_type(root_ty, &place.projections)?;
            if self.types.has_scalar_leaf(ty)? {
                self.push_statement(core::Statement::Drop {
                    place: place.into(),
                });
            }
        }
        Ok(())
    }

    fn lower_reference_assign(
        &mut self,
        reference: hir::BindingId,
        value: &hir::Value,
    ) -> Result<(), LoweringError> {
        let reference_local = self.binding(reference)?;
        let reference_ty = self.local_type(reference_local)?;
        let Some((referent_ty, permission)) = self.types.types.reference(reference_ty) else {
            return Err(LoweringError::InvalidHirInvariant(
                "reference assignment destination is not a safe-reference binding",
            ));
        };
        if permission != core::ReferencePermission::ExclusiveReplace {
            return Err(LoweringError::InvalidHirInvariant(
                "reference assignment destination is not replacement-capable",
            ));
        }

        let value_local = self.lower_value(value)?;
        if self.local_type(value_local)? != referent_ty {
            return Err(LoweringError::InvalidHirInvariant(
                "reference assignment RHS type does not match reference referent",
            ));
        }
        self.push_statement(core::Statement::ReferenceAssign {
            dst: core::ReferenceAccess::new(core::Place::local(reference_local)),
            src: core::Operand::Move(core::Place::local(value_local).into()),
        });
        Ok(())
    }

    fn lower_raw_assign(
        &mut self,
        pointer: hir::BindingId,
        value: &hir::Value,
    ) -> Result<(), LoweringError> {
        let pointer_local = self.binding(pointer)?;
        let pointer_ty = self.local_type(pointer_local)?;
        let pointee_ty = self.types.raw_pointer_pointee(pointer_ty)?;

        let snapshot = self.push_core_temporary(pointer_ty)?;
        self.push_statement(core::Statement::Init {
            dst: core::Place::local(snapshot),
            src: core::Operand::Copy(core::Place::local(pointer_local).into()),
        });

        let value_local = self.lower_value(value)?;
        if self.local_type(value_local)? != pointee_ty {
            return Err(LoweringError::InvalidHirInvariant(
                "raw assignment RHS type does not match pointer pointee",
            ));
        }
        self.push_statement(core::Statement::RawAssign {
            pointer: core::Place::local(snapshot).into(),
            src: core::Operand::Move(core::Place::local(value_local).into()),
        });
        self.push_statement(core::Statement::Drop {
            place: core::Place::local(snapshot).into(),
        });
        Ok(())
    }

    fn lower_loop_transfer(
        &mut self,
        cleanup: &[hir::CleanupPath],
        is_break: bool,
    ) -> Result<(), LoweringError> {
        let target = self
            .loops
            .last()
            .copied()
            .ok_or(LoweringError::InvalidHirInvariant(
                "loop transfer has no enclosing lowering target",
            ))?;
        self.lower_cleanup_paths(cleanup)?;
        self.terminate_current(core::Terminator::Goto(if is_break {
            target.continuation
        } else {
            target.header
        }))
    }

    fn lower_if(
        &mut self,
        condition: &hir::Value,
        then_block: &hir::Block,
        else_block: Option<&hir::Block>,
    ) -> Result<(), LoweringError> {
        let bool_ty = hir::Type::Intrinsic(hir::IntrinsicType::Bool);
        if condition.ty != bool_ty {
            return Err(LoweringError::InvalidHirInvariant(
                "conditional condition type is not Bool",
            ));
        }

        let condition_local = self.lower_value(condition)?;
        if self.local_type(condition_local)? != self.types.get(bool_ty)? {
            return Err(LoweringError::InvalidHirInvariant(
                "lowered conditional condition temporary is not Bool",
            ));
        }

        let then_normal = then_block.has_normal_continuation;
        let else_normal = else_block.is_none_or(|block| block.has_normal_continuation);
        let then_target = self.new_block()?;
        let else_target = else_block.map(|_| self.new_block()).transpose()?;

        match (then_normal, else_normal) {
            (true, true) => {
                let join_target = self.new_block()?;
                self.terminate_current(core::Terminator::Branch {
                    condition: core::Operand::Move(core::Place::local(condition_local).into()),
                    true_target: then_target,
                    false_target: else_target.unwrap_or(join_target),
                })?;

                self.current = then_target.0 as usize;
                self.lower_block(then_block)?;
                self.terminate_current(core::Terminator::Goto(join_target))?;

                if let (Some(else_target), Some(else_block)) = (else_target, else_block) {
                    self.current = else_target.0 as usize;
                    self.lower_block(else_block)?;
                    self.terminate_current(core::Terminator::Goto(join_target))?;
                }

                self.current = join_target.0 as usize;
            }
            (true, false) | (false, true) => {
                let normal_target = if else_block.is_none() {
                    if then_normal {
                        return Err(LoweringError::InvalidHirInvariant(
                            "omitted else cannot remove the false normal outcome",
                        ));
                    }
                    Some(self.new_block()?)
                } else {
                    None
                };
                let false_target =
                    else_target
                        .or(normal_target)
                        .ok_or(LoweringError::InvalidHirInvariant(
                            "conditional has no false target for its retained outcome",
                        ))?;
                self.terminate_current(core::Terminator::Branch {
                    condition: core::Operand::Move(core::Place::local(condition_local).into()),
                    true_target: then_target,
                    false_target,
                })?;

                let mut continuation = normal_target;

                self.current = then_target.0 as usize;
                self.lower_block(then_block)?;
                if then_normal {
                    continuation = Some(core::BasicBlockId(index_u32(
                        self.current,
                        "Core basic block identity",
                    )?));
                }

                if let (Some(else_target), Some(else_block)) = (else_target, else_block) {
                    self.current = else_target.0 as usize;
                    self.lower_block(else_block)?;
                    if else_normal {
                        continuation = Some(core::BasicBlockId(index_u32(
                            self.current,
                            "Core basic block identity",
                        )?));
                    }
                }

                self.current = continuation
                    .ok_or(LoweringError::InvalidHirInvariant(
                        "one-normal conditional lost its sole normal continuation",
                    ))?
                    .0 as usize;
            }
            (false, false) => {
                let else_target = else_target.ok_or(LoweringError::InvalidHirInvariant(
                    "zero-normal conditional cannot omit else",
                ))?;
                let else_block = else_block.ok_or(LoweringError::InvalidHirInvariant(
                    "zero-normal conditional cannot omit else",
                ))?;
                self.terminate_current(core::Terminator::Branch {
                    condition: core::Operand::Move(core::Place::local(condition_local).into()),
                    true_target: then_target,
                    false_target: else_target,
                })?;

                self.current = then_target.0 as usize;
                self.lower_block(then_block)?;

                self.current = else_target.0 as usize;
                self.lower_block(else_block)?;
            }
        }

        Ok(())
    }

    fn lower_while(
        &mut self,
        condition: &hir::Value,
        body: &hir::Block,
    ) -> Result<(), LoweringError> {
        let bool_ty = hir::Type::Intrinsic(hir::IntrinsicType::Bool);
        if condition.ty != bool_ty {
            return Err(LoweringError::InvalidHirInvariant(
                "while condition type is not Bool",
            ));
        }

        let header = self.new_block()?;
        self.terminate_current(core::Terminator::Goto(header))?;
        self.current = header.0 as usize;

        let condition_local = self.lower_value(condition)?;
        if self.local_type(condition_local)? != self.types.get(bool_ty)? {
            return Err(LoweringError::InvalidHirInvariant(
                "lowered while condition temporary is not Bool",
            ));
        }

        let body_target = self.new_block()?;
        let continuation = self.new_block()?;
        self.terminate_current(core::Terminator::Branch {
            condition: core::Operand::Move(core::Place::local(condition_local).into()),
            true_target: body_target,
            false_target: continuation,
        })?;

        self.current = body_target.0 as usize;
        self.loops.push(LoopLoweringTarget {
            header,
            continuation,
        });
        let body_result = self.lower_block(body);
        let popped = self
            .loops
            .pop()
            .expect("lowering loop target stack is balanced by construction");
        debug_assert_eq!(popped.header, header);
        debug_assert_eq!(popped.continuation, continuation);
        body_result?;
        if body.has_normal_continuation {
            self.terminate_current(core::Terminator::Goto(header))?;
        }

        self.current = continuation.0 as usize;
        Ok(())
    }

    fn lower_record_destructure(
        &mut self,
        record: hir::RecordId,
        scrutinee: &hir::RecordPatternScrutinee,
        bindings: &[hir::RecordPatternBinding],
    ) -> Result<(), LoweringError> {
        let expected_ty = self.types.get(hir::Type::Record(record))?;
        let source_local = match scrutinee {
            hir::RecordPatternScrutinee::DirectRoot(root) => {
                let root_local = self.binding(*root)?;
                if self.local_type(root_local)? != expected_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "record destructuring root type does not match its record identity",
                    ));
                }
                root_local
            }
            hir::RecordPatternScrutinee::Producer { value, .. } => {
                if value.ty != hir::Type::Record(record) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "producer-backed record destructuring value type does not match its record identity",
                    ));
                }
                let temporary = self.lower_value(value)?;
                if self.local_type(temporary)? != expected_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "producer-backed record destructuring temporary type does not match its record identity",
                    ));
                }
                temporary
            }
        };

        let mut seen_paths = Vec::<Vec<usize>>::new();
        for binding in bindings {
            if binding.fields.is_empty() {
                return Err(LoweringError::InvalidHirInvariant(
                    "record destructuring binding has empty structural path",
                ));
            }
            if seen_paths
                .iter()
                .any(|seen| binding.fields.starts_with(seen) || seen.starts_with(&binding.fields))
            {
                return Err(LoweringError::InvalidHirInvariant(
                    "record destructuring binding paths are not structurally disjoint",
                ));
            }
            seen_paths.push(binding.fields.clone());

            let source = self.local_place(source_local, &binding.fields)?;
            let projected_ty = self.types.project_type(expected_ty, &source.projections)?;
            let retained_ty = self.types.get(binding.ty)?;
            if projected_ty != retained_ty {
                return Err(LoweringError::InvalidHirInvariant(
                    "record destructuring retained binding type does not match projected field type",
                ));
            }

            let destination = self.binding(binding.binding)?;
            if self.local_type(destination)? != retained_ty {
                return Err(LoweringError::InvalidHirInvariant(
                    "record destructuring destination local type does not match retained binding type",
                ));
            }

            let operand = match binding.ownership {
                hir::OwnedUse::Duplicate => core::Operand::Copy(source.into()),
                hir::OwnedUse::Consume => core::Operand::Move(source.into()),
            };
            self.push_statement(core::Statement::Init {
                dst: core::Place::local(destination),
                src: operand,
            });
        }

        if let hir::RecordPatternScrutinee::Producer { cleanup, .. } = scrutinee {
            self.lower_record_pattern_transient_cleanup(source_local, expected_ty, cleanup)?;
        }
        Ok(())
    }

    fn lower_record_pattern_transient_cleanup(
        &mut self,
        source_local: core::LocalId,
        source_ty: core::TypeId,
        cleanup: &hir::RecordPatternTransientCleanup,
    ) -> Result<(), LoweringError> {
        self.lower_transient_cleanup_paths(
            source_local,
            source_ty,
            &cleanup.paths,
            "record pattern transient cleanup paths are not structurally disjoint",
        )
    }

    fn lower_field_receiver_transient_cleanup(
        &mut self,
        source_local: core::LocalId,
        source_ty: core::TypeId,
        cleanup: &hir::FieldReceiverTransientCleanup,
    ) -> Result<(), LoweringError> {
        self.lower_transient_cleanup_paths(
            source_local,
            source_ty,
            &cleanup.paths,
            "field receiver transient cleanup paths are not structurally disjoint",
        )
    }

    fn lower_transient_cleanup_paths(
        &mut self,
        source_local: core::LocalId,
        source_ty: core::TypeId,
        paths: &[Vec<usize>],
        disjoint_error: &'static str,
    ) -> Result<(), LoweringError> {
        let mut seen_paths = Vec::<Vec<usize>>::new();
        for path in paths {
            if seen_paths
                .iter()
                .any(|seen| path.starts_with(seen) || seen.starts_with(path))
            {
                return Err(LoweringError::InvalidHirInvariant(disjoint_error));
            }
            seen_paths.push(path.clone());

            let place = self.local_place(source_local, path)?;
            let ty = self.types.project_type(source_ty, &place.projections)?;
            if self.types.has_scalar_leaf(ty)? {
                self.push_statement(core::Statement::Drop {
                    place: place.into(),
                });
            }
        }
        Ok(())
    }

    fn lower_value(&mut self, value: &hir::Value) -> Result<core::LocalId, LoweringError> {
        match &value.kind {
            hir::ValueKind::Literal(literal) => {
                let temporary = self.push_temporary(value.ty)?;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: core::Operand::Constant(lower_literal(*literal)),
                });
                Ok(temporary)
            }
            hir::ValueKind::IntegerNeg { operand } => {
                let integer_ty = value.ty;
                let zero = match integer_ty {
                    hir::Type::Intrinsic(hir::IntrinsicType::I8) => core::Value::I8(0),
                    hir::Type::Intrinsic(hir::IntrinsicType::I16) => core::Value::I16(0),
                    hir::Type::Intrinsic(hir::IntrinsicType::I32) => core::Value::I32(0),
                    hir::Type::Intrinsic(hir::IntrinsicType::I64) => core::Value::I64(0),
                    hir::Type::Intrinsic(hir::IntrinsicType::U8) => core::Value::U8(0),
                    hir::Type::Intrinsic(hir::IntrinsicType::U16) => core::Value::U16(0),
                    hir::Type::Intrinsic(hir::IntrinsicType::U32) => core::Value::U32(0),
                    hir::Type::Intrinsic(hir::IntrinsicType::U64) => core::Value::U64(0),
                    _ => {
                        return Err(LoweringError::InvalidHirInvariant(
                            "Integer-neg result type is not a fixed-width integer",
                        ));
                    }
                };
                if operand.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-neg operand type does not match result type",
                    ));
                }

                let core_integer_ty = self.types.get(integer_ty)?;
                let operand_local = self.lower_value(operand)?;
                if self.local_type(operand_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-neg operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(integer_ty)?;
                self.push_statement(core::Statement::IntegerSub {
                    dst: core::Place::local(result),
                    left: core::Operand::Constant(zero),
                    right: core::Operand::Move(core::Place::local(operand_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::IntegerComplement { operand } => {
                let integer_ty = value.ty;
                let minus_one_mod_width = match integer_ty {
                    hir::Type::Intrinsic(hir::IntrinsicType::I8) => core::Value::I8(-1),
                    hir::Type::Intrinsic(hir::IntrinsicType::I16) => core::Value::I16(-1),
                    hir::Type::Intrinsic(hir::IntrinsicType::I32) => core::Value::I32(-1),
                    hir::Type::Intrinsic(hir::IntrinsicType::I64) => core::Value::I64(-1),
                    hir::Type::Intrinsic(hir::IntrinsicType::U8) => core::Value::U8(255),
                    hir::Type::Intrinsic(hir::IntrinsicType::U16) => core::Value::U16(65_535),
                    hir::Type::Intrinsic(hir::IntrinsicType::U32) => {
                        core::Value::U32(4_294_967_295)
                    }
                    hir::Type::Intrinsic(hir::IntrinsicType::U64) => {
                        core::Value::U64(18_446_744_073_709_551_615)
                    }
                    _ => {
                        return Err(LoweringError::InvalidHirInvariant(
                            "Integer-complement result type is not a fixed-width integer",
                        ));
                    }
                };
                if operand.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-complement operand type does not match result type",
                    ));
                }

                let core_integer_ty = self.types.get(integer_ty)?;
                let operand_local = self.lower_value(operand)?;
                if self.local_type(operand_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-complement operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(integer_ty)?;
                self.push_statement(core::Statement::IntegerSub {
                    dst: core::Place::local(result),
                    left: core::Operand::Constant(minus_one_mod_width),
                    right: core::Operand::Move(core::Place::local(operand_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::IntegerAdd { left, right } => {
                let integer_ty = value.ty;
                if !matches!(
                    integer_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::I8
                            | hir::IntrinsicType::I16
                            | hir::IntrinsicType::I32
                            | hir::IntrinsicType::I64
                            | hir::IntrinsicType::U8
                            | hir::IntrinsicType::U16
                            | hir::IntrinsicType::U32
                            | hir::IntrinsicType::U64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-add result type is not a fixed-width integer",
                    ));
                }
                if left.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-add left operand type does not match result type",
                    ));
                }
                if right.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-add right operand type does not match result type",
                    ));
                }

                let core_integer_ty = self.types.get(integer_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-add left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-add right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(integer_ty)?;
                self.push_statement(core::Statement::IntegerAdd {
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::FloatAdd {
                contract,
                left,
                right,
            } => {
                let float_ty = value.ty;
                if !matches!(
                    float_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::F16 | hir::IntrinsicType::F32 | hir::IntrinsicType::F64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-add result type is not a represented floating type",
                    ));
                }
                if left.ty != float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-add left operand type does not match result type",
                    ));
                }
                if right.ty != float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-add right operand type does not match result type",
                    ));
                }

                let core_float_ty = self.types.get(float_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Float-add left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Float-add right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(float_ty)?;
                self.push_statement(core::Statement::FloatAdd {
                    contract: lower_numeric_contract(*contract),
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::FloatSub {
                contract,
                left,
                right,
            } => {
                let float_ty = value.ty;
                if !matches!(
                    float_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::F16 | hir::IntrinsicType::F32 | hir::IntrinsicType::F64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-sub result type is not a represented floating type",
                    ));
                }
                if left.ty != float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-sub left operand type does not match result type",
                    ));
                }
                if right.ty != float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-sub right operand type does not match result type",
                    ));
                }

                let core_float_ty = self.types.get(float_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Float-sub left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Float-sub right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(float_ty)?;
                self.push_statement(core::Statement::FloatSub {
                    contract: lower_numeric_contract(*contract),
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::FloatMul {
                contract,
                left,
                right,
            } => {
                let float_ty = value.ty;
                if !matches!(
                    float_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::F16 | hir::IntrinsicType::F32 | hir::IntrinsicType::F64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-mul result type is not a represented floating type",
                    ));
                }
                if left.ty != float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-mul left operand type does not match result type",
                    ));
                }
                if right.ty != float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-mul right operand type does not match result type",
                    ));
                }

                let core_float_ty = self.types.get(float_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Float-mul left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Float-mul right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(float_ty)?;
                self.push_statement(core::Statement::FloatMul {
                    contract: lower_numeric_contract(*contract),
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::FloatDiv {
                contract,
                left,
                right,
            } => {
                let float_ty = value.ty;
                if !matches!(
                    float_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::F16 | hir::IntrinsicType::F32 | hir::IntrinsicType::F64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-div result type is not a represented floating type",
                    ));
                }
                if left.ty != float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-div left operand type does not match result type",
                    ));
                }
                if right.ty != float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Float-div right operand type does not match result type",
                    ));
                }

                let core_float_ty = self.types.get(float_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Float-div left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_float_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Float-div right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(float_ty)?;
                self.push_statement(core::Statement::FloatDiv {
                    contract: lower_numeric_contract(*contract),
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::IntegerSub { left, right } => {
                let integer_ty = value.ty;
                if !matches!(
                    integer_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::I8
                            | hir::IntrinsicType::I16
                            | hir::IntrinsicType::I32
                            | hir::IntrinsicType::I64
                            | hir::IntrinsicType::U8
                            | hir::IntrinsicType::U16
                            | hir::IntrinsicType::U32
                            | hir::IntrinsicType::U64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-sub result type is not a fixed-width integer",
                    ));
                }
                if left.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-sub left operand type does not match result type",
                    ));
                }
                if right.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-sub right operand type does not match result type",
                    ));
                }

                let core_integer_ty = self.types.get(integer_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-sub left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-sub right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(integer_ty)?;
                self.push_statement(core::Statement::IntegerSub {
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::IntegerMul { left, right } => {
                let integer_ty = value.ty;
                if !matches!(
                    integer_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::I8
                            | hir::IntrinsicType::I16
                            | hir::IntrinsicType::I32
                            | hir::IntrinsicType::I64
                            | hir::IntrinsicType::U8
                            | hir::IntrinsicType::U16
                            | hir::IntrinsicType::U32
                            | hir::IntrinsicType::U64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-mul result type is not a fixed-width integer",
                    ));
                }
                if left.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-mul left operand type does not match result type",
                    ));
                }
                if right.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-mul right operand type does not match result type",
                    ));
                }

                let core_integer_ty = self.types.get(integer_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-mul left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-mul right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(integer_ty)?;
                self.push_statement(core::Statement::IntegerMul {
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::IntegerXor { left, right } => {
                let integer_ty = value.ty;
                if !matches!(
                    integer_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::I8
                            | hir::IntrinsicType::I16
                            | hir::IntrinsicType::I32
                            | hir::IntrinsicType::I64
                            | hir::IntrinsicType::U8
                            | hir::IntrinsicType::U16
                            | hir::IntrinsicType::U32
                            | hir::IntrinsicType::U64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-XOR result type is not a fixed-width integer",
                    ));
                }
                if left.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-XOR left operand type does not match result type",
                    ));
                }
                if right.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-XOR right operand type does not match result type",
                    ));
                }

                let core_integer_ty = self.types.get(integer_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-XOR left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-XOR right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(integer_ty)?;
                self.push_statement(core::Statement::IntegerXor {
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::IntegerOr { left, right } => {
                let integer_ty = value.ty;
                if !matches!(
                    integer_ty,
                    hir::Type::Intrinsic(
                        hir::IntrinsicType::I8
                            | hir::IntrinsicType::I16
                            | hir::IntrinsicType::I32
                            | hir::IntrinsicType::I64
                            | hir::IntrinsicType::U8
                            | hir::IntrinsicType::U16
                            | hir::IntrinsicType::U32
                            | hir::IntrinsicType::U64
                    )
                ) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-OR result type is not a fixed-width integer",
                    ));
                }
                if left.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-OR left operand type does not match result type",
                    ));
                }
                if right.ty != integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Integer-OR right operand type does not match result type",
                    ));
                }

                let core_integer_ty = self.types.get(integer_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-OR left operand temporary type does not match result type",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_integer_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Integer-OR right operand temporary type does not match result type",
                    ));
                }

                let result = self.push_temporary(integer_ty)?;
                self.push_statement(core::Statement::IntegerOr {
                    dst: core::Place::local(result),
                    left: core::Operand::Move(core::Place::local(left_local).into()),
                    right: core::Operand::Move(core::Place::local(right_local).into()),
                });
                Ok(result)
            }
            hir::ValueKind::BooleanNot { operand } => {
                let bool_ty = hir::Type::Intrinsic(hir::IntrinsicType::Bool);
                if value.ty != bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Boolean-not result type is not Bool",
                    ));
                }
                if operand.ty != bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Boolean-not operand type is not Bool",
                    ));
                }

                let operand_local = self.lower_value(operand)?;
                let core_bool_ty = self.types.get(bool_ty)?;
                if self.local_type(operand_local)? != core_bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Boolean-not operand temporary is not Bool",
                    ));
                }

                let result = self.push_temporary(bool_ty)?;
                let true_target = self.new_block()?;
                let false_target = self.new_block()?;
                let join_target = self.new_block()?;
                self.terminate_current(core::Terminator::Branch {
                    condition: core::Operand::Move(core::Place::local(operand_local).into()),
                    true_target,
                    false_target,
                })?;

                self.current = true_target.0 as usize;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(result),
                    src: core::Operand::Constant(core::Value::Bool(false)),
                });
                self.terminate_current(core::Terminator::Goto(join_target))?;

                self.current = false_target.0 as usize;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(result),
                    src: core::Operand::Constant(core::Value::Bool(true)),
                });
                self.terminate_current(core::Terminator::Goto(join_target))?;

                self.current = join_target.0 as usize;
                Ok(result)
            }
            hir::ValueKind::BooleanAnd { left, right } => {
                let bool_ty = hir::Type::Intrinsic(hir::IntrinsicType::Bool);
                if value.ty != bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Boolean-conjunction result type is not Bool",
                    ));
                }
                if left.ty != bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Boolean-conjunction left operand type is not Bool",
                    ));
                }
                if right.ty != bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Boolean-conjunction right operand type is not Bool",
                    ));
                }

                let core_bool_ty = self.types.get(bool_ty)?;
                let left_local = self.lower_value(left)?;
                if self.local_type(left_local)? != core_bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Boolean-conjunction left operand temporary is not Bool",
                    ));
                }

                let result = self.push_temporary(bool_ty)?;
                let true_target = self.new_block()?;
                let false_target = self.new_block()?;
                let join_target = self.new_block()?;
                self.terminate_current(core::Terminator::Branch {
                    condition: core::Operand::Move(core::Place::local(left_local).into()),
                    true_target,
                    false_target,
                })?;

                self.current = false_target.0 as usize;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(result),
                    src: core::Operand::Constant(core::Value::Bool(false)),
                });
                self.terminate_current(core::Terminator::Goto(join_target))?;

                self.current = true_target.0 as usize;
                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Boolean-conjunction right operand temporary is not Bool",
                    ));
                }
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(result),
                    src: core::Operand::Move(core::Place::local(right_local).into()),
                });
                self.terminate_current(core::Terminator::Goto(join_target))?;

                self.current = join_target.0 as usize;
                Ok(result)
            }
            hir::ValueKind::BooleanEquality {
                relation,
                left,
                right,
            } => {
                let bool_ty = hir::Type::Intrinsic(hir::IntrinsicType::Bool);
                if value.ty != bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Boolean-equality result type is not Bool",
                    ));
                }
                if left.ty != bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Boolean-equality left operand type is not Bool",
                    ));
                }
                if right.ty != bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "Boolean-equality right operand type is not Bool",
                    ));
                }

                let left_local = self.lower_value(left)?;
                let core_bool_ty = self.types.get(bool_ty)?;
                if self.local_type(left_local)? != core_bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Boolean-equality left operand temporary is not Bool",
                    ));
                }

                let right_local = self.lower_value(right)?;
                if self.local_type(right_local)? != core_bool_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "lowered Boolean-equality right operand temporary is not Bool",
                    ));
                }

                let result = self.push_temporary(bool_ty)?;
                let left_true = self.new_block()?;
                let left_false = self.new_block()?;
                let true_true = self.new_block()?;
                let true_false = self.new_block()?;
                let false_true = self.new_block()?;
                let false_false = self.new_block()?;
                let join_target = self.new_block()?;
                let (ff, ft, tf, tt) = match *relation {
                    hir::BooleanEqualityRelation::Equal => (true, false, false, true),
                    hir::BooleanEqualityRelation::NotEqual => (false, true, true, false),
                };

                self.terminate_current(core::Terminator::Branch {
                    condition: core::Operand::Move(core::Place::local(left_local).into()),
                    true_target: left_true,
                    false_target: left_false,
                })?;

                self.current = left_true.0 as usize;
                self.terminate_current(core::Terminator::Branch {
                    condition: core::Operand::Move(core::Place::local(right_local).into()),
                    true_target: true_true,
                    false_target: true_false,
                })?;

                self.current = left_false.0 as usize;
                self.terminate_current(core::Terminator::Branch {
                    condition: core::Operand::Move(core::Place::local(right_local).into()),
                    true_target: false_true,
                    false_target: false_false,
                })?;

                for (target, constant) in [
                    (true_true, tt),
                    (true_false, tf),
                    (false_true, ft),
                    (false_false, ff),
                ] {
                    self.current = target.0 as usize;
                    self.push_statement(core::Statement::Init {
                        dst: core::Place::local(result),
                        src: core::Operand::Constant(core::Value::Bool(constant)),
                    });
                    self.terminate_current(core::Terminator::Goto(join_target))?;
                }

                self.current = join_target.0 as usize;
                Ok(result)
            }
            hir::ValueKind::ReferenceRoot {
                target,
                fields,
                permission,
            } => {
                let hir::Type::SafeReference {
                    referent,
                    permission: value_permission,
                } = value.ty
                else {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-root HIR value does not have a safe-reference type",
                    ));
                };
                if *permission != value_permission {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-root permission does not match its safe-reference type",
                    ));
                }
                let place = self.binding_place(*target, fields)?;
                let root_ty = self.local_type(place.local)?;
                let projected_ty = self.types.project_type(root_ty, &place.projections)?;
                let referent_ty = self.types.get(referent.ty())?;
                if projected_ty != referent_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-root target type does not match its reference referent",
                    ));
                }

                let temporary = self.push_temporary(value.ty)?;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: core::Operand::ReferenceRoot {
                        permission: lower_reference_permission(*permission),
                        place,
                    },
                });
                Ok(temporary)
            }
            hir::ValueKind::ReferenceReborrow {
                reference,
                fields,
                permission,
            } => {
                let hir::Type::SafeReference {
                    referent,
                    permission: value_permission,
                } = value.ty
                else {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-reborrow HIR value does not have a safe-reference type",
                    ));
                };
                if *permission != value_permission {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-reborrow permission does not match its safe-reference type",
                    ));
                }
                if !fields.is_empty() && *permission != hir::ReferencePermission::Shared {
                    return Err(LoweringError::InvalidHirInvariant(
                        "projected reference-reborrow is not Shared",
                    ));
                }

                let source = self.binding(*reference)?;
                let source_ty = self.local_type(source)?;
                let Some((parent_referent, parent_permission)) =
                    self.types.types.reference(source_ty)
                else {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-reborrow source is not a safe-reference binding",
                    ));
                };
                let mut access = core::ReferenceAccess::new(core::Place::local(source));
                for field in fields {
                    access = access.field(index_u32(*field, "Core field projection")?);
                }
                let projected_referent = self
                    .types
                    .project_type(parent_referent, &access.projections)?;
                let referent_ty = self.types.get(referent.ty())?;
                if projected_referent != referent_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-reborrow projected parent referent does not match child referent",
                    ));
                }
                let requested = lower_reference_permission(*permission);
                if !parent_permission.permits(requested) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-reborrow permission exceeds parent permission",
                    ));
                }

                let temporary = self.push_temporary(value.ty)?;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: core::Operand::ReferenceReborrow {
                        permission: requested,
                        src: access,
                    },
                });
                Ok(temporary)
            }
            hir::ValueKind::ReferenceDereference {
                reference,
                ownership,
            } => {
                let referent = match value.ty {
                    hir::Type::Intrinsic(intrinsic) => hir::ReferenceReferent::Intrinsic(intrinsic),
                    hir::Type::Record(record) => hir::ReferenceReferent::Record(record),
                    hir::Type::SafeReference { .. } => {
                        return Err(LoweringError::InvalidHirInvariant(
                            "reference-dereference HIR value has a nested-reference result type",
                        ));
                    }
                    hir::Type::RawPointer(_) => {
                        return Err(LoweringError::InvalidHirInvariant(
                            "reference-dereference HIR value has a raw-pointer result type",
                        ));
                    }
                };

                let source = self.binding(*reference)?;
                let source_ty = self.local_type(source)?;
                let Some((source_referent, source_permission)) =
                    self.types.types.reference(source_ty)
                else {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-dereference source is not a safe-reference binding",
                    ));
                };
                let referent_ty = self.types.get(referent.ty())?;
                if source_referent != referent_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "reference-dereference binding type does not match its result referent",
                    ));
                }

                let duplicable = self.compilation.type_is_duplicable(value.ty);
                let access = core::ReferenceAccess::new(core::Place::local(source));
                let operand = match ownership {
                    hir::OwnedUse::Duplicate => {
                        if !duplicable {
                            return Err(LoweringError::InvalidHirInvariant(
                                "reference-dereference duplication has a non-duplicable referent",
                            ));
                        }
                        core::Operand::ReferenceCopy(access)
                    }
                    hir::OwnedUse::Consume => {
                        if duplicable {
                            return Err(LoweringError::InvalidHirInvariant(
                                "reference-dereference move has a duplicable referent",
                            ));
                        }
                        if source_permission != core::ReferencePermission::ExclusiveReplace {
                            return Err(LoweringError::InvalidHirInvariant(
                                "reference-dereference move is not replacement-capable",
                            ));
                        }
                        core::Operand::ReferenceMove(access)
                    }
                };

                let temporary = self.push_temporary(value.ty)?;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: operand,
                });
                Ok(temporary)
            }
            hir::ValueKind::RawAddressRoot { target } => {
                let hir::Type::RawPointer(pointee) = value.ty else {
                    return Err(LoweringError::InvalidHirInvariant(
                        "raw-address HIR value does not have a raw-pointer type",
                    ));
                };
                let target_local = self.binding(*target)?;
                let pointee_ty = self.types.get(pointee.ty())?;
                if self.local_type(target_local)? != pointee_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "raw-address target type does not match its pointer pointee",
                    ));
                }

                let temporary = self.push_temporary(value.ty)?;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: core::Operand::AddressOf(core::Place::local(target_local).into()),
                });
                Ok(temporary)
            }
            hir::ValueKind::RawMove { pointer } => {
                let pointer_local = self.binding(*pointer)?;
                let pointer_ty = self.local_type(pointer_local)?;
                let pointee_ty = self.types.raw_pointer_pointee(pointer_ty)?;
                if self.types.get(value.ty)? != pointee_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "raw-move HIR result type does not match pointer pointee",
                    ));
                }

                let temporary = self.push_temporary(value.ty)?;
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: core::Operand::RawMove(core::Place::local(pointer_local).into()),
                });
                Ok(temporary)
            }
            hir::ValueKind::BindingUse { binding, ownership } => {
                if let hir::Type::SafeReference { permission, .. } = value.ty {
                    match (permission, *ownership) {
                        (hir::ReferencePermission::Shared, hir::OwnedUse::Duplicate)
                        | (hir::ReferencePermission::ExclusiveReplace, hir::OwnedUse::Consume) => {}
                        (hir::ReferencePermission::Shared, _) => {
                            return Err(LoweringError::InvalidHirInvariant(
                                "Shared-reference binding use is not a source duplication",
                            ));
                        }
                        (hir::ReferencePermission::ExclusiveReplace, _) => {
                            return Err(LoweringError::InvalidHirInvariant(
                                "replacement-reference binding use is not a source move",
                            ));
                        }
                    }
                }
                if matches!(value.ty, hir::Type::RawPointer(_))
                    && *ownership != hir::OwnedUse::Duplicate
                {
                    return Err(LoweringError::InvalidHirInvariant(
                        "raw-pointer binding use is not a source duplication",
                    ));
                }
                let source = self.binding(*binding)?;
                let temporary = self.push_temporary(value.ty)?;
                let operand = match ownership {
                    hir::OwnedUse::Duplicate => {
                        core::Operand::Copy(core::Place::local(source).into())
                    }
                    hir::OwnedUse::Consume => {
                        core::Operand::Move(core::Place::local(source).into())
                    }
                };
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: operand,
                });
                Ok(temporary)
            }
            hir::ValueKind::DirectCall {
                function,
                arguments,
            } => {
                let arguments = self.lower_arguments(arguments)?;
                let result = self.push_temporary(value.ty)?;
                self.emit_call(*function, arguments, Some(core::Place::local(result)))?;
                Ok(result)
            }
            hir::ValueKind::RecordConstruction { record, fields } => {
                if value.ty != hir::Type::Record(*record) {
                    return Err(LoweringError::InvalidHirInvariant(
                        "record construction result type does not match its record identity",
                    ));
                }

                let mut lowered_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let temporary = self.lower_value(&field.value)?;
                    lowered_fields.push((field.field, temporary));
                }

                let result = self.push_temporary(value.ty)?;
                if lowered_fields.is_empty() {
                    self.push_statement(core::Statement::Init {
                        dst: core::Place::local(result),
                        src: core::Operand::Constant(core::Value::Struct(Vec::new())),
                    });
                } else {
                    for (field, temporary) in lowered_fields {
                        let field = index_u32(field, "Core field projection")?;
                        self.push_statement(core::Statement::Init {
                            dst: core::Place::local(result).field(field),
                            src: core::Operand::Move(core::Place::local(temporary).into()),
                        });
                    }
                }
                Ok(result)
            }
            hir::ValueKind::FieldValueUse {
                receiver,
                fields,
                ownership,
            } => {
                if fields.is_empty() {
                    return Err(LoweringError::InvalidHirInvariant(
                        "field-value use has empty field path",
                    ));
                }

                let expected_ownership = if self.compilation.type_is_duplicable(value.ty) {
                    hir::OwnedUse::Duplicate
                } else {
                    hir::OwnedUse::Consume
                };
                if *ownership != expected_ownership {
                    return Err(LoweringError::InvalidHirInvariant(
                        "field-value ownership does not match retained result duplicability",
                    ));
                }

                let retained_result_ty = self.types.get(value.ty)?;
                let (source_local, source_ty, producer_cleanup) = match receiver {
                    hir::FieldValueReceiver::Binding { binding, ty } => {
                        let source_local = self.binding(*binding)?;
                        let source_ty = self.types.get(*ty)?;
                        if self.local_type(source_local)? != source_ty {
                            return Err(LoweringError::InvalidHirInvariant(
                                "field-value binding receiver type does not match mapped binding local",
                            ));
                        }
                        (source_local, source_ty, None)
                    }
                    hir::FieldValueReceiver::Producer {
                        value: producer,
                        cleanup,
                    } => {
                        if !matches!(
                            &producer.kind,
                            hir::ValueKind::DirectCall { .. }
                                | hir::ValueKind::RecordConstruction { .. }
                        ) {
                            return Err(LoweringError::InvalidHirInvariant(
                                "field-value producer receiver has unrepresented producer category",
                            ));
                        }
                        let source_ty = self.types.get(producer.ty)?;
                        let source_local = self.lower_value(producer)?;
                        if self.local_type(source_local)? != source_ty {
                            return Err(LoweringError::InvalidHirInvariant(
                                "field-value producer receiver type does not match lowered producer temporary",
                            ));
                        }
                        (source_local, source_ty, Some(cleanup))
                    }
                };

                let place = self.local_place(source_local, fields)?;
                let projected_ty = self.types.project_type(source_ty, &place.projections)?;
                if projected_ty != retained_result_ty {
                    return Err(LoweringError::InvalidHirInvariant(
                        "field-value retained result type does not match projected receiver field type",
                    ));
                }

                if let Some(cleanup) = producer_cleanup {
                    match ownership {
                        hir::OwnedUse::Duplicate => {
                            if cleanup.paths.as_slice() != [Vec::<usize>::new()] {
                                return Err(LoweringError::InvalidHirInvariant(
                                    "duplicating field receiver does not retain complete receiver cleanup",
                                ));
                            }
                        }
                        hir::OwnedUse::Consume => {
                            if cleanup.paths.iter().any(|path| {
                                path.starts_with(fields.as_slice())
                                    || fields.as_slice().starts_with(path.as_slice())
                            }) {
                                return Err(LoweringError::InvalidHirInvariant(
                                    "consumed field receiver path overlaps retained cleanup frontier",
                                ));
                            }
                            let expected = self
                                .types
                                .remaining_frontier_after_consumed_path(source_ty, fields)?;
                            if cleanup.paths.as_slice() != expected.as_slice() {
                                return Err(LoweringError::InvalidHirInvariant(
                                    "consuming field receiver cleanup does not match canonical remaining frontier",
                                ));
                            }
                        }
                    }
                }

                let temporary = self.push_temporary(value.ty)?;
                let operand = match ownership {
                    hir::OwnedUse::Duplicate => core::Operand::Copy(place.into()),
                    hir::OwnedUse::Consume => core::Operand::Move(place.into()),
                };
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: operand,
                });

                if let Some(cleanup) = producer_cleanup {
                    self.lower_field_receiver_transient_cleanup(source_local, source_ty, cleanup)?;
                }
                Ok(temporary)
            }
        }
    }

    fn lower_call(
        &mut self,
        function: hir::FunctionId,
        arguments: &[hir::Value],
        destination: Option<core::Place>,
    ) -> Result<(), LoweringError> {
        let arguments = self.lower_arguments(arguments)?;
        self.emit_call(function, arguments, destination)
    }

    fn lower_arguments(
        &mut self,
        arguments: &[hir::Value],
    ) -> Result<Vec<core::Operand>, LoweringError> {
        let mut lowered = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let temporary = self.lower_value(argument)?;
            lowered.push(core::Operand::Move(core::Place::local(temporary).into()));
        }
        Ok(lowered)
    }

    fn emit_call(
        &mut self,
        function: hir::FunctionId,
        arguments: Vec<core::Operand>,
        destination: Option<core::Place>,
    ) -> Result<(), LoweringError> {
        let target_function =
            self.functions
                .get(&function)
                .copied()
                .ok_or(LoweringError::InvalidHirInvariant(
                    "HIR call target is absent from function map",
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

    fn push_source_local(
        &mut self,
        name: String,
        ty: hir::Type,
        mutable: bool,
    ) -> Result<core::LocalId, LoweringError> {
        let ty = self.types.get(ty)?;
        let id = self.next_local_id()?;
        self.locals.push(core::LocalDecl::new(name, ty, mutable));
        Ok(id)
    }

    fn push_temporary(&mut self, ty: hir::Type) -> Result<core::LocalId, LoweringError> {
        let ty = self.types.get(ty)?;
        self.push_core_temporary(ty)
    }

    fn push_core_temporary(&mut self, ty: core::TypeId) -> Result<core::LocalId, LoweringError> {
        let id = self.next_local_id()?;
        let name = format!("$tmp{}", self.next_temp);
        self.next_temp =
            self.next_temp
                .checked_add(1)
                .ok_or(LoweringError::RepresentationLimit(
                    "compiler temporary identity",
                ))?;
        self.locals.push(core::LocalDecl::new(name, ty, false));
        Ok(id)
    }

    fn next_local_id(&self) -> Result<core::LocalId, LoweringError> {
        Ok(core::LocalId(index_u32(
            self.locals.len(),
            "Core local identity",
        )?))
    }

    fn binding(&self, binding: hir::BindingId) -> Result<core::LocalId, LoweringError> {
        self.bindings
            .get(&binding)
            .copied()
            .ok_or(LoweringError::InvalidHirInvariant(
                "HIR binding use is absent from local map",
            ))
    }

    fn binding_place(
        &self,
        binding: hir::BindingId,
        fields: &[usize],
    ) -> Result<core::Place, LoweringError> {
        let local = self.binding(binding)?;
        self.local_place(local, fields)
    }

    fn local_place(
        &self,
        local: core::LocalId,
        fields: &[usize],
    ) -> Result<core::Place, LoweringError> {
        let mut place = core::Place::local(local);
        for field in fields {
            place = place.field(index_u32(*field, "Core field projection")?);
        }
        Ok(place)
    }

    fn local_type(&self, local: core::LocalId) -> Result<core::TypeId, LoweringError> {
        self.locals
            .get(local.0 as usize)
            .map(|local| local.ty)
            .ok_or(LoweringError::InvalidHirInvariant(
                "bound Core local is absent from local declarations",
            ))
    }

    fn push_statement(&mut self, statement: core::Statement) {
        self.blocks[self.current].statements.push(statement);
    }

    fn new_block(&mut self) -> Result<core::BasicBlockId, LoweringError> {
        let id = core::BasicBlockId(index_u32(self.blocks.len(), "Core basic block identity")?);
        self.blocks.push(BlockDraft::default());
        Ok(id)
    }

    fn terminate_current(&mut self, terminator: core::Terminator) -> Result<(), LoweringError> {
        let slot = &mut self.blocks[self.current].terminator;
        if slot.replace(terminator).is_some() {
            return Err(LoweringError::InvalidHirInvariant(
                "lowering attempted to terminate one Core block twice",
            ));
        }
        Ok(())
    }
}

fn lower_literal(value: hir::LiteralValue) -> core::Value {
    match value {
        hir::LiteralValue::Bool(value) => core::Value::Bool(value),
        hir::LiteralValue::I8(value) => core::Value::I8(value),
        hir::LiteralValue::I16(value) => core::Value::I16(value),
        hir::LiteralValue::I32(value) => core::Value::I32(value),
        hir::LiteralValue::I64(value) => core::Value::I64(value),
        hir::LiteralValue::U8(value) => core::Value::U8(value),
        hir::LiteralValue::U16(value) => core::Value::U16(value),
        hir::LiteralValue::U32(value) => core::Value::U32(value),
        hir::LiteralValue::U64(value) => core::Value::U64(value),
        hir::LiteralValue::F16(value) => core::Value::F16(lower_binary_float(value)),
        hir::LiteralValue::F32(value) => core::Value::F32(lower_binary_float(value)),
        hir::LiteralValue::F64(value) => core::Value::F64(lower_binary_float(value)),
    }
}

fn lower_binary_float(value: hir::BinaryFloatValue) -> core::BinaryFloatValue {
    match value {
        hir::BinaryFloatValue::Zero(sign) => core::BinaryFloatValue::Zero(lower_float_sign(sign)),
        hir::BinaryFloatValue::Subnormal { sign, significand } => {
            core::BinaryFloatValue::Subnormal {
                sign: lower_float_sign(sign),
                significand,
            }
        }
        hir::BinaryFloatValue::Normal {
            sign,
            significand,
            exponent,
        } => core::BinaryFloatValue::Normal {
            sign: lower_float_sign(sign),
            significand,
            exponent,
        },
        hir::BinaryFloatValue::Infinity(sign) => {
            core::BinaryFloatValue::Infinity(lower_float_sign(sign))
        }
    }
}

fn lower_float_sign(sign: hir::BinaryFloatSign) -> core::BinaryFloatSign {
    match sign {
        hir::BinaryFloatSign::Positive => core::BinaryFloatSign::Positive,
        hir::BinaryFloatSign::Negative => core::BinaryFloatSign::Negative,
    }
}

fn lower_numeric_contract(contract: hir::NumericContract) -> core::NumericContract {
    match contract {
        hir::NumericContract::Standard => core::NumericContract::Standard,
        hir::NumericContract::Reproducible => core::NumericContract::Reproducible,
        hir::NumericContract::Fast => core::NumericContract::Fast,
    }
}

fn lower_reference_permission(permission: hir::ReferencePermission) -> core::ReferencePermission {
    match permission {
        hir::ReferencePermission::Shared => core::ReferencePermission::Shared,
        hir::ReferencePermission::ExclusiveReplace => core::ReferencePermission::ExclusiveReplace,
    }
}

fn index_u32(index: usize, what: &'static str) -> Result<u32, LoweringError> {
    u32::try_from(index).map_err(|_| LoweringError::RepresentationLimit(what))
}
