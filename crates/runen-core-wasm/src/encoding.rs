use std::borrow::Cow;
use std::collections::BTreeMap;

use runen_core_ir::{
    BasicBlockId, Fault, Function, FunctionId, LocalId, Operand, Place, PlaceAccess, ScalarType,
    Statement, Terminator, TypeId, TypeKind, TypeTable, ValidatedProgram,
};
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, ElementSection, Elements, EntityType, ExportKind,
    ExportSection, Function as WasmFunction, FunctionSection, GlobalSection, GlobalType,
    ImportSection, Instruction, Module, RefType, TableSection, TableType, TypeSection, ValType,
};

use crate::RealizationError;
use crate::layout::{
    constant_carriers, projected_span, result_carrier_count, storage_carrier_count,
};
use crate::scalar::{FloatingScalarValue, ScalarKind, constant_residue, mask};

#[path = "f16_encoding.rs"]
mod f16_encoding;
#[path = "reference_encoding.rs"]
mod reference_encoding;

pub(crate) struct EncodedProgram {
    pub(crate) bytes: Vec<u8>,
    pub(crate) faults: Vec<Fault>,
    pub(crate) entries: Vec<EntryInfo>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EntryInfo {
    pub(crate) parameter_count: usize,
    pub(crate) result: Option<TypeId>,
    pub(crate) result_carrier_count: usize,
}

pub(crate) fn entry_export_name(function: FunctionId) -> String {
    format!("__runen_core_entry_{}", function.0)
}

pub(crate) fn encode(program: &ValidatedProgram) -> Result<EncodedProgram, RealizationError> {
    let program = program.as_program();
    let external_count = u32::try_from(program.external_callables.len())
        .map_err(|_| invariant("Core external callable count exceeds u32::MAX"))?;
    let mut module = Module::new();
    let mut types = TypeSection::new();
    let mut imports = ImportSection::new();
    let mut functions = FunctionSection::new();
    let mut tables = TableSection::new();
    let mut globals = GlobalSection::new();
    let mut exports = ExportSection::new();
    let mut elements = ElementSection::new();
    let mut entries = Vec::with_capacity(program.functions.len());

    for (index, function) in program.functions.iter().enumerate() {
        let function_id = checked_function_id(index)?;
        let parameter_carriers = function_parameter_carrier_count(&program.types, function)?;
        let semantic_result_carriers = function
            .result
            .map(|ty| result_carrier_count(&program.types, ty))
            .transpose()?
            .unwrap_or(0);
        let payload_carriers = semantic_result_carriers.max(1);
        let params = vec![ValType::I64; parameter_carriers];
        let mut results = Vec::with_capacity(payload_carriers + 1);
        results.push(ValType::I32);
        results.extend(std::iter::repeat_n(ValType::I64, payload_carriers));
        types.ty().function(params, results);
        functions.function(function_id.0);
        if function.parameters.is_empty() {
            exports.export(
                &entry_export_name(function_id),
                ExportKind::Func,
                defined_function_index(external_count, function_id)?,
            );
        }
        entries.push(EntryInfo {
            parameter_count: function.parameters.len(),
            result: function.result,
            result_carrier_count: semantic_result_carriers,
        });
    }

    let external_type_indices = encode_external_types(&mut types, program)?;
    for (index, type_index) in external_type_indices.iter().copied().enumerate() {
        imports.import(
            "__runen_external",
            &format!("external_{index}"),
            EntityType::Function(type_index),
        );
    }

    let callable_type_indices = encode_callable_types(&mut types, program)?;
    if !callable_type_indices.is_empty() {
        let function_count = u64::try_from(program.functions.len())
            .map_err(|_| invariant("Core function count exceeds u64::MAX"))?;
        tables.table(TableType {
            element_type: RefType::FUNCREF,
            table64: false,
            minimum: function_count,
            maximum: Some(function_count),
            shared: false,
        });
        let function_indices = (0..program.functions.len())
            .map(|index| {
                let function = checked_function_id(index)?;
                defined_function_index(external_count, function)
            })
            .collect::<Result<Vec<_>, _>>()?;
        elements.active(
            None,
            &ConstExpr::i32_const(0),
            Elements::Functions(Cow::Owned(function_indices)),
        );
    }

    for persistent in &program.persistent {
        let _kind = supported_kind(&program.types, persistent.ty)?;
        let residue = constant_residue(&persistent.initial).ok_or_else(|| {
            invariant("coverage admission allowed an unsupported persistent initializer")
        })?;
        globals.global(
            persistent_global_type(),
            &ConstExpr::i64_const(residue_as_i64(residue)),
        );
    }

    module.section(&types);
    if !external_type_indices.is_empty() {
        module.section(&imports);
    }
    module.section(&functions);
    if !callable_type_indices.is_empty() {
        module.section(&tables);
    }
    if !program.persistent.is_empty() {
        module.section(&globals);
    }
    module.section(&exports);
    if !callable_type_indices.is_empty() {
        module.section(&elements);
    }

    let mut faults = Vec::new();
    let mut code = CodeSection::new();
    for function in &program.functions {
        let encoded = encode_function(
            &program.types,
            function,
            &callable_type_indices,
            external_count,
            &mut faults,
        )?;
        code.function(&encoded);
    }
    module.section(&code);

    Ok(EncodedProgram {
        bytes: module.finish(),
        faults,
        entries,
    })
}

fn function_parameter_carrier_count(
    types: &TypeTable,
    function: &Function,
) -> Result<usize, RealizationError> {
    function
        .parameters
        .iter()
        .try_fold(0_usize, |count, local| {
            let ty = function
                .body
                .local(*local)
                .ok_or_else(|| invariant("validated parameter local is missing"))?
                .ty;
            count
                .checked_add(storage_carrier_count(types, ty)?)
                .ok_or_else(|| invariant("Wasm parameter carrier count overflow"))
        })
}

fn encode_external_types(
    types: &mut TypeSection,
    program: &runen_core_ir::Program,
) -> Result<Vec<u32>, RealizationError> {
    program
        .external_callables
        .iter()
        .map(|external| {
            for ty in external.interface.parameters.iter().copied() {
                let _ = supported_kind(&program.types, ty)?;
            }
            if let Some(ty) = external.interface.result {
                let _ = supported_kind(&program.types, ty)?;
            }
            let type_index = types.len();
            let params = vec![ValType::I64; external.interface.parameters.len()];
            let results = external.interface.result.into_iter().map(|_| ValType::I64);
            types.ty().function(params, results);
            Ok(type_index)
        })
        .collect()
}

fn encode_callable_types(
    types: &mut TypeSection,
    program: &runen_core_ir::Program,
) -> Result<BTreeMap<TypeId, u32>, RealizationError> {
    let mut callable_type_indices = BTreeMap::new();
    for function in &program.functions {
        for block in &function.body.blocks {
            let Terminator::IndirectCall { callable, .. } = block.terminator else {
                continue;
            };
            if callable_type_indices.contains_key(&callable) {
                continue;
            }
            let (parameter_carriers, semantic_result_carriers) =
                callable_carrier_counts(&program.types, callable)?;
            let payload_carriers = semantic_result_carriers.max(1);
            let mut results = Vec::with_capacity(payload_carriers + 1);
            results.push(ValType::I32);
            results.extend(std::iter::repeat_n(ValType::I64, payload_carriers));
            let type_index = types.len();
            types
                .ty()
                .function(vec![ValType::I64; parameter_carriers], results);
            callable_type_indices.insert(callable, type_index);
        }
    }
    Ok(callable_type_indices)
}

fn callable_carrier_counts(
    types: &TypeTable,
    ty: TypeId,
) -> Result<(usize, usize), RealizationError> {
    let definition = types
        .get(ty)
        .ok_or_else(|| invariant("validated callable type is missing"))?;
    let TypeKind::Scalar(ScalarType::Callable(interface)) = &definition.kind else {
        return Err(invariant(
            "coverage admission allowed a non-callable indirect-call type",
        ));
    };
    let parameter_carriers = interface.parameters.iter().try_fold(0_usize, |count, ty| {
        count
            .checked_add(result_carrier_count(types, *ty)?)
            .ok_or_else(|| invariant("Wasm callable parameter carrier count overflow"))
    })?;
    let result_carriers = interface
        .result
        .map(|ty| result_carrier_count(types, ty))
        .transpose()?
        .unwrap_or(0);
    Ok((parameter_carriers, result_carriers))
}

fn encode_function(
    types: &TypeTable,
    function: &Function,
    callable_type_indices: &BTreeMap<TypeId, u32>,
    defined_function_offset: u32,
    faults: &mut Vec<Fault>,
) -> Result<WasmFunction, RealizationError> {
    let layout = FunctionLayout::new(types, function)?;
    let i64_local_count = u32::try_from(layout.non_parameter_i64_count)
        .map_err(|_| invariant("Wasm i64 local count exceeds u32::MAX"))?;
    let mut encoded = WasmFunction::new([
        (i64_local_count, ValType::I64),
        (1, ValType::F32),
        (1, ValType::F64),
        (2, ValType::I32),
    ]);

    emit_i32_const(&mut encoded, function.body.entry.0);
    encoded.instruction(&Instruction::LocalSet(layout.pc));
    encoded.instruction(&Instruction::Loop(BlockType::Empty));

    let mut context = FunctionEncoder {
        types,
        function,
        layout: &layout,
        callable_type_indices,
        defined_function_offset,
        faults,
        result_payload_count: function
            .result
            .map(|ty| result_carrier_count(types, ty))
            .transpose()?
            .unwrap_or(0)
            .max(1),
    };

    for (block_index, block) in function.body.blocks.iter().enumerate() {
        let block_id = BasicBlockId(
            u32::try_from(block_index)
                .map_err(|_| invariant("Core block index exceeds u32::MAX"))?,
        );
        encoded.instruction(&Instruction::LocalGet(layout.pc));
        emit_i32_const(&mut encoded, block_id.0);
        encoded.instruction(&Instruction::I32Eq);
        encoded.instruction(&Instruction::If(BlockType::Empty));
        for statement in &block.statements {
            context.emit_statement(&mut encoded, statement)?;
        }
        context.emit_terminator(&mut encoded, &block.terminator)?;
        encoded.instruction(&Instruction::End);
    }

    encoded.instruction(&Instruction::Unreachable);
    encoded.instruction(&Instruction::End);
    encoded.instruction(&Instruction::Unreachable);
    encoded.instruction(&Instruction::End);
    Ok(encoded)
}

#[derive(Clone, Copy, Debug)]
struct LocalLayout {
    start: u32,
    len: usize,
}

struct FunctionLayout {
    locals: Vec<LocalLayout>,
    non_parameter_i64_count: usize,
    scratch_start: u32,
    scratch_len: usize,
    reference_targets: Vec<reference_encoding::ReferenceTarget>,
    reference_handle_scratch: Option<u32>,
    f32_scratch: u32,
    f64_scratch: u32,
    pc: u32,
    status: u32,
}

impl FunctionLayout {
    fn new(types: &TypeTable, function: &Function) -> Result<Self, RealizationError> {
        let mut locals = vec![None; function.body.locals.len()];
        let mut next = 0_u32;

        for local in function.parameters.iter().copied() {
            let local_index = local.0 as usize;
            let declaration = function
                .body
                .local(local)
                .ok_or_else(|| invariant("validated parameter local is missing"))?
                .ty;
            let len = storage_carrier_count(types, declaration)?;
            let target = locals
                .get_mut(local_index)
                .ok_or_else(|| invariant("validated parameter local is outside the body"))?;
            if target.is_some() {
                return Err(invariant("validated function repeats a parameter local"));
            }
            *target = Some(LocalLayout { start: next, len });
            next = add_carriers(next, len, "Wasm parameter local index overflow")?;
        }

        let parameter_carrier_count = next as usize;
        let mut maximum_local_len = 0_usize;
        for (index, local) in function.body.locals.iter().enumerate() {
            let len = storage_carrier_count(types, local.ty)?;
            maximum_local_len = maximum_local_len.max(len);
            if locals[index].is_none() {
                locals[index] = Some(LocalLayout { start: next, len });
                next = add_carriers(next, len, "Wasm local index overflow")?;
            }
        }

        let scratch_len = maximum_local_len.max(1);
        let scratch_start = next;
        next = add_carriers(next, scratch_len, "Wasm scratch local index overflow")?;
        let reference_targets = reference_encoding::collect_reference_targets(types, function)?;
        let reference_handle_scratch = if reference_targets.is_empty() {
            None
        } else {
            let scratch = next;
            next = add_carriers(next, 1, "Wasm reference handle scratch index overflow")?;
            Some(scratch)
        };
        let total_i64_slots = next as usize;
        let non_parameter_i64_count = total_i64_slots
            .checked_sub(parameter_carrier_count)
            .ok_or_else(|| invariant("Wasm local carrier accounting underflow"))?;
        let f32_scratch = next;
        let f64_scratch = f32_scratch
            .checked_add(1)
            .ok_or_else(|| invariant("Wasm f64 scratch local index overflow"))?;
        let pc = f64_scratch
            .checked_add(1)
            .ok_or_else(|| invariant("Wasm pc local index overflow"))?;
        let status = pc
            .checked_add(1)
            .ok_or_else(|| invariant("Wasm status local index overflow"))?;

        Ok(Self {
            locals: locals
                .into_iter()
                .map(|local| local.ok_or_else(|| invariant("Wasm local mapping is incomplete")))
                .collect::<Result<Vec<_>, _>>()?,
            non_parameter_i64_count,
            scratch_start,
            scratch_len,
            reference_targets,
            reference_handle_scratch,
            f32_scratch,
            f64_scratch,
            pc,
            status,
        })
    }

    fn local(&self, local: LocalId) -> Result<LocalLayout, RealizationError> {
        self.locals
            .get(local.0 as usize)
            .copied()
            .ok_or_else(|| invariant("validated local identity is outside the function body"))
    }

    fn scratch(&self, index: usize) -> Result<u32, RealizationError> {
        if index >= self.scratch_len {
            return Err(invariant("private scratch carrier index is out of bounds"));
        }
        add_carriers(
            self.scratch_start,
            index,
            "private scratch carrier index overflow",
        )
    }

    fn reference_handle_scratch(&self) -> Result<u32, RealizationError> {
        self.reference_handle_scratch
            .ok_or_else(|| invariant("reference encoding has no private handle scratch"))
    }
}

fn add_carriers(start: u32, count: usize, message: &str) -> Result<u32, RealizationError> {
    let count = u32::try_from(count).map_err(|_| invariant(message))?;
    start.checked_add(count).ok_or_else(|| invariant(message))
}

struct FunctionEncoder<'a> {
    types: &'a TypeTable,
    function: &'a Function,
    layout: &'a FunctionLayout,
    callable_type_indices: &'a BTreeMap<TypeId, u32>,
    defined_function_offset: u32,
    faults: &'a mut Vec<Fault>,
    result_payload_count: usize,
}

impl FunctionEncoder<'_> {
    fn emit_statement(
        &mut self,
        encoded: &mut WasmFunction,
        statement: &Statement,
    ) -> Result<(), RealizationError> {
        match statement {
            Statement::Init { dst, src } => self.emit_store(encoded, dst, src),
            Statement::IntegerAdd {
                dst, left, right, ..
            } => self.emit_binary_integer(encoded, dst, left, right, Instruction::I64Add),
            Statement::IntegerSub {
                dst, left, right, ..
            } => self.emit_binary_integer(encoded, dst, left, right, Instruction::I64Sub),
            Statement::IntegerMul {
                dst, left, right, ..
            } => self.emit_binary_integer(encoded, dst, left, right, Instruction::I64Mul),
            Statement::IntegerXor {
                dst, left, right, ..
            } => self.emit_binary_integer(encoded, dst, left, right, Instruction::I64Xor),
            Statement::IntegerOr {
                dst, left, right, ..
            } => self.emit_binary_integer(encoded, dst, left, right, Instruction::I64Or),
            Statement::IntegerEq {
                dst, left, right, ..
            } => {
                self.emit_scalar_operand(encoded, left)?;
                self.emit_scalar_operand(encoded, right)?;
                encoded.instruction(&Instruction::I64Eq);
                encoded.instruction(&Instruction::I64ExtendI32U);
                self.emit_scalar_place_set(encoded, dst)
            }
            Statement::IntegerLt {
                dst,
                operand_type,
                left,
                right,
            } => {
                let kind = supported_kind(self.types, *operand_type)?;
                self.emit_scalar_operand(encoded, left)?;
                if kind.is_signed() {
                    emit_sign_extension(encoded, kind.width());
                }
                self.emit_scalar_operand(encoded, right)?;
                if kind.is_signed() {
                    emit_sign_extension(encoded, kind.width());
                    encoded.instruction(&Instruction::I64LtS);
                } else {
                    encoded.instruction(&Instruction::I64LtU);
                }
                encoded.instruction(&Instruction::I64ExtendI32U);
                self.emit_scalar_place_set(encoded, dst)
            }
            Statement::FloatAdd {
                dst, left, right, ..
            } => self.emit_binary_float(
                encoded,
                dst,
                left,
                right,
                Instruction::F32Add,
                Instruction::F64Add,
            ),
            Statement::FloatSub {
                dst, left, right, ..
            } => self.emit_binary_float(
                encoded,
                dst,
                left,
                right,
                Instruction::F32Sub,
                Instruction::F64Sub,
            ),
            Statement::FloatMul {
                dst, left, right, ..
            } => self.emit_binary_float(
                encoded,
                dst,
                left,
                right,
                Instruction::F32Mul,
                Instruction::F64Mul,
            ),
            Statement::FloatDiv {
                dst, left, right, ..
            } => self.emit_binary_float(
                encoded,
                dst,
                left,
                right,
                Instruction::F32Div,
                Instruction::F64Div,
            ),
            Statement::Read { src } => {
                let place = direct_access_place(src)?;
                let range = self.place_layout(place)?;
                for index in 0..range.len {
                    encoded.instruction(&Instruction::LocalGet(add_carriers(
                        range.start,
                        index,
                        "projected read carrier index overflow",
                    )?));
                    encoded.instruction(&Instruction::Drop);
                }
                Ok(())
            }
            Statement::Assign { dst, src } => {
                let place = direct_access_place(dst)?;
                self.emit_store(encoded, place, src)
            }
            Statement::Drop { .. } => Ok(()),
            Statement::ReferenceRead { src } => self.emit_reference_read_or_drop(encoded, src),
            Statement::ReferenceAssign { dst, src } => {
                self.emit_reference_assign(encoded, dst, src)
            }
            Statement::ReferenceDrop { place } => self.emit_reference_read_or_drop(encoded, place),
            Statement::Borrow { .. }
            | Statement::EndBorrow { .. }
            | Statement::RawRead { .. }
            | Statement::RawAssign { .. }
            | Statement::InteriorAssign { .. }
            | Statement::ReferenceInteriorAssign { .. } => Err(invariant(
                "coverage admission allowed an unsupported Core statement",
            )),
        }
    }

    fn emit_terminator(
        &mut self,
        encoded: &mut WasmFunction,
        terminator: &Terminator,
    ) -> Result<(), RealizationError> {
        match terminator {
            Terminator::Goto(target) => {
                self.emit_dispatch(encoded, *target);
                Ok(())
            }
            Terminator::Branch {
                condition,
                true_target,
                false_target,
            } => {
                self.emit_scalar_operand(encoded, condition)?;
                encoded.instruction(&Instruction::I64Eqz);
                encoded.instruction(&Instruction::If(BlockType::Empty));
                emit_i32_const(encoded, false_target.0);
                encoded.instruction(&Instruction::LocalSet(self.layout.pc));
                encoded.instruction(&Instruction::Else);
                emit_i32_const(encoded, true_target.0);
                encoded.instruction(&Instruction::LocalSet(self.layout.pc));
                encoded.instruction(&Instruction::End);
                encoded.instruction(&Instruction::Br(1));
                Ok(())
            }
            Terminator::Call {
                function,
                arguments,
                destination,
                target,
            } => {
                for argument in arguments {
                    self.emit_operand(encoded, argument)?;
                }
                encoded.instruction(&Instruction::Call(defined_function_index(
                    self.defined_function_offset,
                    *function,
                )?));
                let payload_count = self.call_payload_count(destination.as_ref())?;
                self.emit_call_completion(encoded, destination.as_ref(), *target, payload_count)
            }
            Terminator::ExternalCall {
                external,
                arguments,
                destination,
                target,
            } => {
                for argument in arguments {
                    self.emit_scalar_operand(encoded, argument)?;
                }
                encoded.instruction(&Instruction::Call(external.0));
                if let Some(destination) = destination {
                    self.emit_scalar_place_set(encoded, destination)?;
                }
                self.emit_dispatch(encoded, *target);
                Ok(())
            }
            Terminator::IndirectCall {
                callable,
                callee,
                arguments,
                destination,
                target,
            } => {
                self.emit_scalar_operand(encoded, callee)?;
                encoded.instruction(&Instruction::LocalSet(self.layout.scratch(0)?));
                for argument in arguments {
                    self.emit_operand(encoded, argument)?;
                }
                encoded.instruction(&Instruction::LocalGet(self.layout.scratch(0)?));
                encoded.instruction(&Instruction::I32WrapI64);
                let type_index = self
                    .callable_type_indices
                    .get(callable)
                    .copied()
                    .ok_or_else(|| invariant("missing Wasm type for admitted indirect call"))?;
                encoded.instruction(&Instruction::CallIndirect {
                    type_index,
                    table_index: 0,
                });
                let payload_count = self.call_payload_count(destination.as_ref())?;
                self.emit_call_completion(encoded, destination.as_ref(), *target, payload_count)
            }
            Terminator::Return(result) => {
                encoded.instruction(&Instruction::I32Const(0));
                let count = if let Some(result) = result {
                    self.emit_operand(encoded, result)?
                } else {
                    0
                };
                if count > self.result_payload_count {
                    return Err(invariant(
                        "validated return produced more carriers than the function result",
                    ));
                }
                for _ in count..self.result_payload_count {
                    encoded.instruction(&Instruction::I64Const(0));
                }
                encoded.instruction(&Instruction::Return);
                Ok(())
            }
            Terminator::Fault(fault) => {
                let index = self.faults.len();
                let payload = i64::try_from(index)
                    .map_err(|_| invariant("fault table index exceeds i64::MAX"))?;
                self.faults.push(fault.clone());
                encoded.instruction(&Instruction::I32Const(1));
                encoded.instruction(&Instruction::I64Const(payload));
                for _ in 1..self.result_payload_count {
                    encoded.instruction(&Instruction::I64Const(0));
                }
                encoded.instruction(&Instruction::Return);
                Ok(())
            }
        }
    }

    fn call_payload_count(&self, destination: Option<&Place>) -> Result<usize, RealizationError> {
        Ok(match destination {
            Some(destination) => self.place_layout(destination)?.len.max(1),
            None => 1,
        })
    }

    fn emit_call_completion(
        &self,
        encoded: &mut WasmFunction,
        destination: Option<&Place>,
        target: BasicBlockId,
        payload_count: usize,
    ) -> Result<(), RealizationError> {
        if payload_count > self.layout.scratch_len {
            return Err(invariant(
                "call result exceeds private scratch carrier capacity",
            ));
        }
        for index in (0..payload_count).rev() {
            encoded.instruction(&Instruction::LocalSet(self.layout.scratch(index)?));
        }
        encoded.instruction(&Instruction::LocalSet(self.layout.status));
        encoded.instruction(&Instruction::LocalGet(self.layout.status));
        encoded.instruction(&Instruction::If(BlockType::Empty));
        encoded.instruction(&Instruction::LocalGet(self.layout.status));
        encoded.instruction(&Instruction::LocalGet(self.layout.scratch(0)?));
        for _ in 1..self.result_payload_count {
            encoded.instruction(&Instruction::I64Const(0));
        }
        encoded.instruction(&Instruction::Return);
        encoded.instruction(&Instruction::End);
        if let Some(destination) = destination {
            let range = self.place_layout(destination)?;
            for index in 0..range.len {
                encoded.instruction(&Instruction::LocalGet(self.layout.scratch(index)?));
                encoded.instruction(&Instruction::LocalSet(add_carriers(
                    range.start,
                    index,
                    "call destination carrier index overflow",
                )?));
            }
        }
        self.emit_dispatch(encoded, target);
        Ok(())
    }

    fn emit_binary_integer(
        &self,
        encoded: &mut WasmFunction,
        dst: &Place,
        left: &Operand,
        right: &Operand,
        operation: Instruction<'static>,
    ) -> Result<(), RealizationError> {
        self.emit_scalar_operand(encoded, left)?;
        self.emit_scalar_operand(encoded, right)?;
        encoded.instruction(&operation);
        let kind = self.place_kind(dst)?;
        emit_canonicalization(encoded, kind.width());
        self.emit_scalar_place_set(encoded, dst)
    }

    fn emit_binary_float(
        &self,
        encoded: &mut WasmFunction,
        dst: &Place,
        left: &Operand,
        right: &Operand,
        f32_operation: Instruction<'static>,
        f64_operation: Instruction<'static>,
    ) -> Result<(), RealizationError> {
        let kind = self.place_kind(dst)?;
        match kind {
            ScalarKind::F16 => {
                let scratch = self.layout.scratch(0)?;
                self.emit_scalar_operand(encoded, left)?;
                f16_encoding::emit_widen_carrier_to_f64(encoded, scratch)?;
                self.emit_scalar_operand(encoded, right)?;
                f16_encoding::emit_widen_carrier_to_f64(encoded, scratch)?;
                encoded.instruction(&f64_operation);
                encoded.instruction(&Instruction::LocalSet(self.layout.f64_scratch));
                f16_encoding::emit_narrow_f64_to_carrier(
                    encoded,
                    self.layout.f64_scratch,
                    scratch,
                )?;
                self.emit_scalar_place_set(encoded, dst)
            }
            ScalarKind::F32 => {
                self.emit_scalar_operand(encoded, left)?;
                encoded.instruction(&Instruction::I32WrapI64);
                encoded.instruction(&Instruction::F32ReinterpretI32);
                self.emit_scalar_operand(encoded, right)?;
                encoded.instruction(&Instruction::I32WrapI64);
                encoded.instruction(&Instruction::F32ReinterpretI32);
                encoded.instruction(&f32_operation);
                encoded.instruction(&Instruction::LocalSet(self.layout.f32_scratch));
                self.emit_normalized_float_result(encoded, kind)?;
                self.emit_scalar_place_set(encoded, dst)
            }
            ScalarKind::F64 => {
                self.emit_scalar_operand(encoded, left)?;
                encoded.instruction(&Instruction::F64ReinterpretI64);
                self.emit_scalar_operand(encoded, right)?;
                encoded.instruction(&Instruction::F64ReinterpretI64);
                encoded.instruction(&f64_operation);
                encoded.instruction(&Instruction::LocalSet(self.layout.f64_scratch));
                self.emit_normalized_float_result(encoded, kind)?;
                self.emit_scalar_place_set(encoded, dst)
            }
            ScalarKind::Bool
            | ScalarKind::I8
            | ScalarKind::I16
            | ScalarKind::I32
            | ScalarKind::I64
            | ScalarKind::U8
            | ScalarKind::U16
            | ScalarKind::U32
            | ScalarKind::U64 => Err(invariant(
                "coverage admission allowed unsupported floating arithmetic type",
            )),
        }
    }

    fn emit_normalized_float_result(
        &self,
        encoded: &mut WasmFunction,
        kind: ScalarKind,
    ) -> Result<(), RealizationError> {
        let canonical_nan = kind.floating_residue(FloatingScalarValue::NaNClass)?;
        emit_i64_const(encoded, canonical_nan);
        match kind {
            ScalarKind::F32 => {
                encoded.instruction(&Instruction::LocalGet(self.layout.f32_scratch));
                encoded.instruction(&Instruction::I32ReinterpretF32);
                encoded.instruction(&Instruction::I64ExtendI32U);
                encoded.instruction(&Instruction::LocalGet(self.layout.f32_scratch));
                encoded.instruction(&Instruction::LocalGet(self.layout.f32_scratch));
                encoded.instruction(&Instruction::F32Ne);
            }
            ScalarKind::F64 => {
                encoded.instruction(&Instruction::LocalGet(self.layout.f64_scratch));
                encoded.instruction(&Instruction::I64ReinterpretF64);
                encoded.instruction(&Instruction::LocalGet(self.layout.f64_scratch));
                encoded.instruction(&Instruction::LocalGet(self.layout.f64_scratch));
                encoded.instruction(&Instruction::F64Ne);
            }
            ScalarKind::Bool
            | ScalarKind::I8
            | ScalarKind::I16
            | ScalarKind::I32
            | ScalarKind::I64
            | ScalarKind::U8
            | ScalarKind::U16
            | ScalarKind::U32
            | ScalarKind::U64
            | ScalarKind::F16 => {
                return Err(invariant(
                    "private floating normalization received unsupported scalar kind",
                ));
            }
        }
        encoded.instruction(&Instruction::Select);
        Ok(())
    }

    fn emit_store(
        &self,
        encoded: &mut WasmFunction,
        dst: &Place,
        src: &Operand,
    ) -> Result<(), RealizationError> {
        let destination = self.place_layout(dst)?;
        let emitted = self.emit_operand(encoded, src)?;
        if emitted != destination.len {
            return Err(invariant(
                "validated structural store has mismatched carrier counts",
            ));
        }
        if emitted > self.layout.scratch_len {
            return Err(invariant(
                "structural store exceeds private scratch carrier capacity",
            ));
        }
        for index in (0..emitted).rev() {
            encoded.instruction(&Instruction::LocalSet(self.layout.scratch(index)?));
        }
        for index in 0..emitted {
            encoded.instruction(&Instruction::LocalGet(self.layout.scratch(index)?));
            encoded.instruction(&Instruction::LocalSet(add_carriers(
                destination.start,
                index,
                "structural destination carrier index overflow",
            )?));
        }
        Ok(())
    }

    fn emit_operand(
        &self,
        encoded: &mut WasmFunction,
        operand: &Operand,
    ) -> Result<usize, RealizationError> {
        match operand {
            Operand::Constant(value) => {
                let carriers = constant_carriers(value)?;
                for carrier in &carriers {
                    encoded.instruction(&Instruction::I64Const(*carrier));
                }
                Ok(carriers.len())
            }
            Operand::Move(access) | Operand::Copy(access) => {
                let place = direct_access_place(access)?;
                let range = self.place_layout(place)?;
                for index in 0..range.len {
                    encoded.instruction(&Instruction::LocalGet(add_carriers(
                        range.start,
                        index,
                        "projected operand carrier index overflow",
                    )?));
                }
                Ok(range.len)
            }
            Operand::PersistentRead(persistent) => {
                encoded.instruction(&Instruction::GlobalGet(persistent.0));
                Ok(1)
            }
            Operand::FunctionValue(function) => {
                encoded.instruction(&Instruction::I64Const(i64::from(function.0)));
                Ok(1)
            }
            Operand::ReferenceRoot { place, .. } => self.emit_reference_root(encoded, place),
            Operand::ReferenceReborrow { src, .. } => self.emit_reference_reborrow(encoded, src),
            Operand::ReferenceMove(src) | Operand::ReferenceCopy(src) => {
                self.emit_reference_value(encoded, src)
            }
            Operand::PersistentSharedRoot(_) | Operand::RawMove(_) | Operand::AddressOf(_) => Err(
                invariant("coverage admission allowed an unsupported Core operand"),
            ),
        }
    }

    fn emit_scalar_operand(
        &self,
        encoded: &mut WasmFunction,
        operand: &Operand,
    ) -> Result<(), RealizationError> {
        if self.emit_operand(encoded, operand)? != 1 {
            return Err(invariant(
                "validated scalar operation produced a non-scalar carrier count",
            ));
        }
        Ok(())
    }

    fn emit_scalar_place_set(
        &self,
        encoded: &mut WasmFunction,
        place: &Place,
    ) -> Result<(), RealizationError> {
        let range = self.place_layout(place)?;
        if range.len != 1 {
            return Err(invariant(
                "validated scalar destination has a non-scalar carrier count",
            ));
        }
        encoded.instruction(&Instruction::LocalSet(range.start));
        Ok(())
    }

    fn place_layout(&self, place: &Place) -> Result<LocalLayout, RealizationError> {
        let root = self
            .function
            .body
            .local(place.local)
            .ok_or_else(|| invariant("validated local is missing"))?
            .ty;
        let local = self.layout.local(place.local)?;
        let span = projected_span(self.types, root, &place.projections)?;
        let start = add_carriers(
            local.start,
            span.offset,
            "projected local carrier index overflow",
        )?;
        if span
            .offset
            .checked_add(span.len)
            .is_none_or(|end| end > local.len)
        {
            return Err(invariant(
                "projected carrier span exceeds validated local storage",
            ));
        }
        Ok(LocalLayout {
            start,
            len: span.len,
        })
    }

    fn place_kind(&self, place: &Place) -> Result<ScalarKind, RealizationError> {
        let root = self
            .function
            .body
            .local(place.local)
            .ok_or_else(|| invariant("validated local is missing"))?
            .ty;
        let span = projected_span(self.types, root, &place.projections)?;
        supported_kind(self.types, span.ty)
    }

    fn emit_dispatch(&self, encoded: &mut WasmFunction, target: BasicBlockId) {
        emit_i32_const(encoded, target.0);
        encoded.instruction(&Instruction::LocalSet(self.layout.pc));
        encoded.instruction(&Instruction::Br(1));
    }
}

fn supported_kind(types: &TypeTable, ty: TypeId) -> Result<ScalarKind, RealizationError> {
    ScalarKind::from_type(types, ty).ok_or_else(|| {
        invariant(format!(
            "coverage admission allowed unsupported Core scalar type {:?}",
            ty
        ))
    })
}

fn defined_function_index(
    defined_function_offset: u32,
    function: FunctionId,
) -> Result<u32, RealizationError> {
    defined_function_offset
        .checked_add(function.0)
        .ok_or_else(|| invariant("Wasm defined function index overflow"))
}

fn persistent_global_type() -> GlobalType {
    GlobalType {
        val_type: ValType::I64,
        mutable: false,
        shared: false,
    }
}

fn direct_access_place(access: &PlaceAccess) -> Result<&Place, RealizationError> {
    match access {
        PlaceAccess::Direct(place) => Ok(place),
        PlaceAccess::Loan { .. } => Err(invariant(
            "coverage admission allowed a loan-relative access",
        )),
    }
}

fn emit_canonicalization(encoded: &mut WasmFunction, width: u32) {
    if width < 64 {
        emit_i64_const(encoded, mask(width));
        encoded.instruction(&Instruction::I64And);
    }
}

fn emit_sign_extension(encoded: &mut WasmFunction, width: u32) {
    if width < 64 {
        let shift = i64::from(64 - width);
        encoded.instruction(&Instruction::I64Const(shift));
        encoded.instruction(&Instruction::I64Shl);
        encoded.instruction(&Instruction::I64Const(shift));
        encoded.instruction(&Instruction::I64ShrS);
    }
}

fn residue_as_i64(residue: u64) -> i64 {
    i64::from_ne_bytes(residue.to_ne_bytes())
}

fn emit_i64_const(encoded: &mut WasmFunction, residue: u64) {
    encoded.instruction(&Instruction::I64Const(residue_as_i64(residue)));
}

fn emit_i32_const(encoded: &mut WasmFunction, bits: u32) {
    encoded.instruction(&Instruction::I32Const(i32::from_ne_bytes(
        bits.to_ne_bytes(),
    )));
}

fn checked_function_id(index: usize) -> Result<FunctionId, RealizationError> {
    u32::try_from(index)
        .map(FunctionId)
        .map_err(|_| invariant("Core function index exceeds u32::MAX"))
}

fn invariant(message: impl Into<String>) -> RealizationError {
    RealizationError::BackendInvariant(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use runen_core_ir::{ScalarType, TypeDef};

    #[test]
    fn scalar_kind_lookup_rejects_non_scalar_types() {
        let mut types = TypeTable::new();
        let structure = types.push(TypeDef::structure("S", Vec::new()));
        assert!(ScalarKind::from_type(&types, structure).is_none());
    }

    #[test]
    fn supported_kind_accepts_integer_scalar() {
        let mut types = TypeTable::new();
        let ty = types.push(TypeDef::scalar("I32", ScalarType::I32));
        assert_eq!(supported_kind(&types, ty), Ok(ScalarKind::I32));
    }

    #[test]
    fn imported_functions_shift_only_defined_wasm_function_indices() {
        assert_eq!(defined_function_index(3, FunctionId(0)), Ok(3));
        assert_eq!(defined_function_index(3, FunctionId(7)), Ok(10));
    }

    #[test]
    fn persistent_global_carrier_is_private_immutable_i64_storage() {
        let global = persistent_global_type();
        assert_eq!(global.val_type, ValType::I64);
        assert!(!global.mutable);
        assert!(!global.shared);
    }
}
