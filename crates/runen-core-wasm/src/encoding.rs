use std::borrow::Cow;
use std::collections::BTreeMap;

use runen_core_ir::{
    BasicBlockId, Fault, Function, FunctionId, LocalId, Operand, Place, PlaceAccess, ScalarType,
    Statement, Terminator, TypeId, TypeKind, TypeTable, ValidatedProgram,
};
use wasm_encoder::{
    BlockType, CodeSection, ConstExpr, ElementSection, Elements, ExportKind, ExportSection,
    Function as WasmFunction, FunctionSection, GlobalSection, GlobalType, Instruction, Module,
    RefType, TableSection, TableType, TypeSection, ValType,
};

use crate::RealizationError;
use crate::scalar::{ScalarKind, constant_residue, mask};

pub(crate) struct EncodedProgram {
    pub(crate) bytes: Vec<u8>,
    pub(crate) faults: Vec<Fault>,
    pub(crate) entries: Vec<EntryInfo>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct EntryInfo {
    pub(crate) parameter_count: usize,
    pub(crate) result: Option<ScalarKind>,
}

pub(crate) fn entry_export_name(function: FunctionId) -> String {
    format!("__runen_core_entry_{}", function.0)
}

pub(crate) fn encode(program: &ValidatedProgram) -> Result<EncodedProgram, RealizationError> {
    let program = program.as_program();
    let mut module = Module::new();
    let mut types = TypeSection::new();
    let mut functions = FunctionSection::new();
    let mut tables = TableSection::new();
    let mut globals = GlobalSection::new();
    let mut exports = ExportSection::new();
    let mut elements = ElementSection::new();
    let mut entries = Vec::with_capacity(program.functions.len());

    for (index, function) in program.functions.iter().enumerate() {
        let function_id = checked_function_id(index)?;
        let params = vec![ValType::I64; function.parameters.len()];
        types.ty().function(params, [ValType::I32, ValType::I64]);
        functions.function(function_id.0);
        if function.parameters.is_empty() {
            exports.export(
                &entry_export_name(function_id),
                ExportKind::Func,
                function_id.0,
            );
        }
        entries.push(EntryInfo {
            parameter_count: function.parameters.len(),
            result: function
                .result
                .map(|ty| supported_kind(&program.types, ty))
                .transpose()?,
        });
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
                u32::try_from(index).map_err(|_| invariant("Core function index exceeds u32::MAX"))
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
            let parameter_count = callable_parameter_count(&program.types, callable)?;
            let type_index = types.len();
            types.ty().function(
                vec![ValType::I64; parameter_count],
                [ValType::I32, ValType::I64],
            );
            callable_type_indices.insert(callable, type_index);
        }
    }
    Ok(callable_type_indices)
}

fn callable_parameter_count(types: &TypeTable, ty: TypeId) -> Result<usize, RealizationError> {
    let definition = types
        .get(ty)
        .ok_or_else(|| invariant("validated callable type is missing"))?;
    let TypeKind::Scalar(ScalarType::Callable(interface)) = &definition.kind else {
        return Err(invariant(
            "coverage admission allowed a non-callable indirect-call type",
        ));
    };
    Ok(interface.parameters.len())
}

fn encode_function(
    types: &TypeTable,
    function: &Function,
    callable_type_indices: &BTreeMap<TypeId, u32>,
    faults: &mut Vec<Fault>,
) -> Result<WasmFunction, RealizationError> {
    let layout = FunctionLayout::new(function)?;
    let i64_local_count = layout
        .non_parameter_count
        .checked_add(1)
        .ok_or_else(|| invariant("Wasm local count overflow"))?;
    let i64_local_count = u32::try_from(i64_local_count)
        .map_err(|_| invariant("Wasm i64 local count exceeds u32::MAX"))?;
    let mut encoded = WasmFunction::new([(i64_local_count, ValType::I64), (2, ValType::I32)]);

    emit_i32_const(&mut encoded, function.body.entry.0);
    encoded.instruction(&Instruction::LocalSet(layout.pc));
    encoded.instruction(&Instruction::Loop(BlockType::Empty));

    let mut context = FunctionEncoder {
        types,
        function,
        layout: &layout,
        callable_type_indices,
        faults,
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

struct FunctionLayout {
    locals: Vec<u32>,
    non_parameter_count: usize,
    payload: u32,
    pc: u32,
    status: u32,
}

impl FunctionLayout {
    fn new(function: &Function) -> Result<Self, RealizationError> {
        let parameter_count = function.parameters.len();
        let mut locals = vec![None; function.body.locals.len()];
        for (slot, local) in function.parameters.iter().copied().enumerate() {
            let local_index = local.0 as usize;
            let target = locals
                .get_mut(local_index)
                .ok_or_else(|| invariant("validated parameter local is missing"))?;
            if target.is_some() {
                return Err(invariant("validated function repeats a parameter local"));
            }
            *target = Some(
                u32::try_from(slot)
                    .map_err(|_| invariant("Wasm parameter index exceeds u32::MAX"))?,
            );
        }

        let mut next = u32::try_from(parameter_count)
            .map_err(|_| invariant("Wasm parameter count exceeds u32::MAX"))?;
        let mut non_parameter_count = 0_usize;
        for local in &mut locals {
            if local.is_none() {
                *local = Some(next);
                next = next
                    .checked_add(1)
                    .ok_or_else(|| invariant("Wasm local index overflow"))?;
                non_parameter_count = non_parameter_count
                    .checked_add(1)
                    .ok_or_else(|| invariant("Wasm local count overflow"))?;
            }
        }

        let payload = next;
        next = next
            .checked_add(1)
            .ok_or_else(|| invariant("Wasm payload local index overflow"))?;
        let pc = next;
        next = next
            .checked_add(1)
            .ok_or_else(|| invariant("Wasm pc local index overflow"))?;
        let status = next;

        Ok(Self {
            locals: locals
                .into_iter()
                .map(|local| local.ok_or_else(|| invariant("Wasm local mapping is incomplete")))
                .collect::<Result<Vec<_>, _>>()?,
            non_parameter_count,
            payload,
            pc,
            status,
        })
    }

    fn local(&self, local: LocalId) -> Result<u32, RealizationError> {
        self.locals
            .get(local.0 as usize)
            .copied()
            .ok_or_else(|| invariant("validated local identity is outside the function body"))
    }
}

struct FunctionEncoder<'a> {
    types: &'a TypeTable,
    function: &'a Function,
    layout: &'a FunctionLayout,
    callable_type_indices: &'a BTreeMap<TypeId, u32>,
    faults: &'a mut Vec<Fault>,
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
                self.emit_operand(encoded, left)?;
                self.emit_operand(encoded, right)?;
                encoded.instruction(&Instruction::I64Eq);
                encoded.instruction(&Instruction::I64ExtendI32U);
                self.emit_local_set(encoded, dst.local)
            }
            Statement::IntegerLt {
                dst,
                operand_type,
                left,
                right,
            } => {
                let kind = supported_kind(self.types, *operand_type)?;
                self.emit_operand(encoded, left)?;
                if kind.is_signed() {
                    emit_sign_extension(encoded, kind.width());
                }
                self.emit_operand(encoded, right)?;
                if kind.is_signed() {
                    emit_sign_extension(encoded, kind.width());
                    encoded.instruction(&Instruction::I64LtS);
                } else {
                    encoded.instruction(&Instruction::I64LtU);
                }
                encoded.instruction(&Instruction::I64ExtendI32U);
                self.emit_local_set(encoded, dst.local)
            }
            Statement::Read { src } => {
                let local = direct_access_local(src)?;
                encoded.instruction(&Instruction::LocalGet(self.layout.local(local)?));
                encoded.instruction(&Instruction::Drop);
                Ok(())
            }
            Statement::Assign { dst, src } => {
                let local = direct_access_local(dst)?;
                self.emit_operand(encoded, src)?;
                encoded.instruction(&Instruction::LocalSet(self.layout.local(local)?));
                Ok(())
            }
            Statement::Drop { .. } => Ok(()),
            Statement::FloatAdd { .. }
            | Statement::FloatSub { .. }
            | Statement::FloatMul { .. }
            | Statement::FloatDiv { .. }
            | Statement::Borrow { .. }
            | Statement::EndBorrow { .. }
            | Statement::ReferenceRead { .. }
            | Statement::RawRead { .. }
            | Statement::RawAssign { .. }
            | Statement::ReferenceAssign { .. }
            | Statement::InteriorAssign { .. }
            | Statement::ReferenceInteriorAssign { .. }
            | Statement::ReferenceDrop { .. } => Err(invariant(
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
                self.emit_operand(encoded, condition)?;
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
                encoded.instruction(&Instruction::Call(function.0));
                self.emit_call_completion(encoded, destination.as_ref(), *target)
            }
            Terminator::IndirectCall {
                callable,
                callee,
                arguments,
                destination,
                target,
            } => {
                self.emit_operand(encoded, callee)?;
                encoded.instruction(&Instruction::LocalSet(self.layout.payload));
                for argument in arguments {
                    self.emit_operand(encoded, argument)?;
                }
                encoded.instruction(&Instruction::LocalGet(self.layout.payload));
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
                self.emit_call_completion(encoded, destination.as_ref(), *target)
            }
            Terminator::Return(result) => {
                if let Some(result) = result {
                    self.emit_operand(encoded, result)?;
                    encoded.instruction(&Instruction::LocalSet(self.layout.payload));
                    encoded.instruction(&Instruction::I32Const(0));
                    encoded.instruction(&Instruction::LocalGet(self.layout.payload));
                } else {
                    encoded.instruction(&Instruction::I32Const(0));
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
                encoded.instruction(&Instruction::Return);
                Ok(())
            }
            Terminator::ExternalCall { .. } => Err(invariant(
                "coverage admission allowed an unsupported Core terminator",
            )),
        }
    }

    fn emit_call_completion(
        &self,
        encoded: &mut WasmFunction,
        destination: Option<&Place>,
        target: BasicBlockId,
    ) -> Result<(), RealizationError> {
        encoded.instruction(&Instruction::LocalSet(self.layout.payload));
        encoded.instruction(&Instruction::LocalSet(self.layout.status));
        encoded.instruction(&Instruction::LocalGet(self.layout.status));
        encoded.instruction(&Instruction::If(BlockType::Empty));
        encoded.instruction(&Instruction::LocalGet(self.layout.status));
        encoded.instruction(&Instruction::LocalGet(self.layout.payload));
        encoded.instruction(&Instruction::Return);
        encoded.instruction(&Instruction::End);
        if let Some(destination) = destination {
            encoded.instruction(&Instruction::LocalGet(self.layout.payload));
            encoded.instruction(&Instruction::LocalSet(
                self.layout.local(destination.local)?,
            ));
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
        self.emit_operand(encoded, left)?;
        self.emit_operand(encoded, right)?;
        encoded.instruction(&operation);
        let kind = self.local_kind(dst.local)?;
        emit_canonicalization(encoded, kind.width());
        self.emit_local_set(encoded, dst.local)
    }

    fn emit_store(
        &self,
        encoded: &mut WasmFunction,
        dst: &Place,
        src: &Operand,
    ) -> Result<(), RealizationError> {
        self.emit_operand(encoded, src)?;
        self.emit_local_set(encoded, dst.local)
    }

    fn emit_operand(
        &self,
        encoded: &mut WasmFunction,
        operand: &Operand,
    ) -> Result<(), RealizationError> {
        match operand {
            Operand::Constant(value) => {
                let residue = constant_residue(value).ok_or_else(|| {
                    invariant("coverage admission allowed an unsupported Core constant")
                })?;
                emit_i64_const(encoded, residue);
                Ok(())
            }
            Operand::Move(access) | Operand::Copy(access) => {
                let local = direct_access_local(access)?;
                encoded.instruction(&Instruction::LocalGet(self.layout.local(local)?));
                Ok(())
            }
            Operand::PersistentRead(persistent) => {
                encoded.instruction(&Instruction::GlobalGet(persistent.0));
                Ok(())
            }
            Operand::FunctionValue(function) => {
                encoded.instruction(&Instruction::I64Const(i64::from(function.0)));
                Ok(())
            }
            Operand::PersistentSharedRoot(_)
            | Operand::RawMove(_)
            | Operand::AddressOf(_)
            | Operand::ReferenceRoot { .. }
            | Operand::ReferenceReborrow { .. }
            | Operand::ReferenceMove(_)
            | Operand::ReferenceCopy(_) => Err(invariant(
                "coverage admission allowed an unsupported Core operand",
            )),
        }
    }

    fn emit_local_set(
        &self,
        encoded: &mut WasmFunction,
        local: LocalId,
    ) -> Result<(), RealizationError> {
        encoded.instruction(&Instruction::LocalSet(self.layout.local(local)?));
        Ok(())
    }

    fn emit_dispatch(&self, encoded: &mut WasmFunction, target: BasicBlockId) {
        emit_i32_const(encoded, target.0);
        encoded.instruction(&Instruction::LocalSet(self.layout.pc));
        encoded.instruction(&Instruction::Br(1));
    }

    fn local_kind(&self, local: LocalId) -> Result<ScalarKind, RealizationError> {
        let ty = self
            .function
            .body
            .local(local)
            .ok_or_else(|| invariant("validated local is missing"))?
            .ty;
        supported_kind(self.types, ty)
    }
}

fn supported_kind(types: &TypeTable, ty: TypeId) -> Result<ScalarKind, RealizationError> {
    ScalarKind::from_type(types, ty).ok_or_else(|| {
        invariant(format!(
            "coverage admission allowed unsupported Core type {:?}",
            ty
        ))
    })
}

fn persistent_global_type() -> GlobalType {
    GlobalType {
        val_type: ValType::I64,
        mutable: false,
        shared: false,
    }
}

fn direct_access_local(access: &PlaceAccess) -> Result<LocalId, RealizationError> {
    match access {
        PlaceAccess::Direct(place) if place.projections.is_empty() => Ok(place.local),
        PlaceAccess::Direct(_) | PlaceAccess::Loan { .. } => Err(invariant(
            "coverage admission allowed a non-root or loan-relative access",
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
    fn persistent_global_carrier_is_private_immutable_i64_storage() {
        let global = persistent_global_type();
        assert_eq!(global.val_type, ValType::I64);
        assert!(!global.mutable);
        assert!(!global.shared);
    }
}
