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
            functions.push(FunctionLowerer::new(&self.types, &self.functions, function)?.lower()?);
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

    fn is_scalar(&self, ty: core::TypeId) -> Result<bool, LoweringError> {
        let definition = self
            .types
            .get(ty)
            .ok_or(LoweringError::InvalidHirInvariant(
                "lowered Core type is absent from the type table",
            ))?;
        Ok(matches!(definition.kind, core::TypeKind::Scalar(_)))
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
}

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
        types: &'a TypeMap,
        functions: &'a BTreeMap<hir::FunctionId, core::FunctionId>,
        function: &'a hir::Function,
    ) -> Result<Self, LoweringError> {
        let mut lowerer = Self {
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
                hir::Statement::Assignment { .. } | hir::Statement::Call { .. } => {}
            }
        }
        Ok(())
    }

    fn lower(mut self) -> Result<core::Function, LoweringError> {
        let body = self.function.body.clone();
        self.lower_statements(&body.statements)?;

        let terminator = match body.terminal_return.as_ref() {
            Some(hir::Return {
                value: Some(value), ..
            }) => {
                let result = self.lower_value(value)?;
                core::Terminator::Return(Some(core::Operand::Move(
                    core::Place::local(result).into(),
                )))
            }
            Some(hir::Return { value: None, .. }) | None => core::Terminator::Return(None),
        };
        self.terminate_current(terminator)?;

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

    fn lower_statements(&mut self, statements: &[hir::Statement]) -> Result<(), LoweringError> {
        for statement in statements {
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
                    root,
                    bindings,
                    ..
                } => self.lower_record_destructure(*record, *root, bindings)?,
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
                hir::Statement::Block(block) => {
                    self.lower_statements(&block.statements)?;
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
                }
            }
        }
        Ok(())
    }

    fn lower_record_destructure(
        &mut self,
        record: hir::RecordId,
        root: hir::BindingId,
        bindings: &[hir::RecordPatternBinding],
    ) -> Result<(), LoweringError> {
        let root_local = self.binding(root)?;
        let root_ty = self.local_type(root_local)?;
        let expected_root_ty = self.types.get(hir::Type::Record(record))?;
        if root_ty != expected_root_ty {
            return Err(LoweringError::InvalidHirInvariant(
                "record destructuring root type does not match its record identity",
            ));
        }

        let mut seen_fields = BTreeSet::new();
        for binding in bindings {
            if !seen_fields.insert(binding.field) {
                return Err(LoweringError::InvalidHirInvariant(
                    "record destructuring contains duplicate resolved field identity",
                ));
            }

            let source = self.binding_place(root, &[binding.field])?;
            let projected_ty = self.types.project_type(root_ty, &source.projections)?;
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

            let scalar = self.types.is_scalar(projected_ty)?;
            let operand = match (binding.ownership, scalar) {
                (hir::OwnedUse::Duplicate, true) => core::Operand::Copy(source.into()),
                (hir::OwnedUse::Consume, false) => core::Operand::Move(source.into()),
                _ => {
                    return Err(LoweringError::InvalidHirInvariant(
                        "record destructuring ownership does not match retained field type",
                    ));
                }
            };
            self.push_statement(core::Statement::Init {
                dst: core::Place::local(destination),
                src: operand,
            });
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
                binding,
                fields,
                ownership,
            } => {
                if fields.is_empty() {
                    return Err(LoweringError::InvalidHirInvariant(
                        "field-value use has empty field path",
                    ));
                }
                let place = self.binding_place(*binding, fields)?;
                let temporary = self.push_temporary(value.ty)?;
                let operand = match ownership {
                    hir::OwnedUse::Duplicate => core::Operand::Copy(place.into()),
                    hir::OwnedUse::Consume => core::Operand::Move(place.into()),
                };
                self.push_statement(core::Statement::Init {
                    dst: core::Place::local(temporary),
                    src: operand,
                });
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
