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
        let maximum_types = compilation
            .records
            .len()
            .checked_add(INTRINSICS.len())
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

    fn get(&self, ty: hir::Type) -> Result<core::TypeId, LoweringError> {
        self.mapped
            .get(&ty)
            .copied()
            .ok_or(LoweringError::InvalidHirInvariant(
                "HIR type is absent from the lowering type map",
            ))
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
}

impl<'a> FunctionLowerer<'a> {
    fn new(
        compilation: &'a hir::TypedCompilation,
        types: &'a TypeMap,
        functions: &'a BTreeMap<hir::FunctionId, core::FunctionId>,
        function: &'a hir::Function,
    ) -> Result<Self, LoweringError> {
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
                | hir::Statement::Call { .. }
                | hir::Statement::Fault { .. } => {}
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

        Ok(core::Function {
            name: self.function.name.clone(),
            parameters: self.parameter_locals,
            result,
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
                hir::Statement::Assignment { target, value, .. } => {
                    let destination = self.binding(*target)?;
                    let value = self.lower_value(value)?;
                    self.push_statement(core::Statement::Assign {
                        dst: core::Place::local(destination).into(),
                        src: core::Operand::Move(core::Place::local(value).into()),
                    });
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
            hir::Statement::Fault { .. } => false,
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
            self.lower_normal_cleanup(block)?;
        }
        Ok(())
    }

    fn lower_normal_cleanup(&mut self, block: &hir::Block) -> Result<(), LoweringError> {
        for cleanup in &block.normal_cleanup {
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
        self.lower_block(body)?;
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
            hir::ValueKind::BindingUse { binding, ownership } => {
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
    }
}

fn index_u32(index: usize, what: &'static str) -> Result<u32, LoweringError> {
    u32::try_from(index).map_err(|_| LoweringError::RepresentationLimit(what))
}
