use runen_core_ir::{Function, LocalId, Place, PlaceAccess, ScalarType, TypeId, TypeKind, TypeTable};
use wasm_encoder::{Function as WasmFunction, Instruction};

use crate::RealizationError;
use crate::coverage::is_supported_raw_pointer_pointee_type;
use crate::layout::storage_carrier_count;

use super::{FunctionEncoder, add_carriers, direct_access_place, invariant};

#[derive(Clone, Debug)]
pub(super) struct RawPointerTarget {
    pub(super) handle: u32,
    pub(super) place: Place,
    pub(super) ty: TypeId,
}

pub(super) fn collect_raw_pointer_targets(
    types: &TypeTable,
    function: &Function,
) -> Result<Vec<RawPointerTarget>, RealizationError> {
    let has_raw_pointer_local = function
        .body
        .locals
        .iter()
        .enumerate()
        .any(|(index, local)| {
            !function.parameters.contains(&LocalId(index as u32))
                && matches!(
                    types.get(local.ty).map(|definition| &definition.kind),
                    Some(TypeKind::Scalar(ScalarType::RawPointer(_)))
                )
        });
    if !has_raw_pointer_local {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for (index, local) in function.body.locals.iter().enumerate() {
        if !is_supported_raw_pointer_pointee_type(types, local.ty) {
            continue;
        }
        let local_id = LocalId(
            u32::try_from(index).map_err(|_| invariant("Core local index exceeds u32::MAX"))?,
        );
        let handle = u32::try_from(targets.len())
            .map_err(|_| invariant("private raw-pointer target count exceeds u32::MAX"))?
            .checked_add(1)
            .ok_or_else(|| invariant("private raw-pointer target handle overflow"))?;
        targets.push(RawPointerTarget {
            handle,
            place: Place::local(local_id),
            ty: local.ty,
        });
    }
    Ok(targets)
}

impl FunctionEncoder<'_> {
    pub(super) fn emit_raw_address_of(
        &self,
        encoded: &mut WasmFunction,
        access: &PlaceAccess,
    ) -> Result<usize, RealizationError> {
        let place = direct_access_place(access)?;
        if !place.projections.is_empty() {
            return Err(invariant(
                "coverage admission allowed projected raw address formation",
            ));
        }
        let target = self
            .raw_pointer_target_for_place(place)
            .ok_or_else(|| invariant("admitted raw address has no private target handle"))?;
        encoded.instruction(&Instruction::I64Const(i64::from(target.handle)));
        Ok(1)
    }

    pub(super) fn emit_raw_read(
        &self,
        encoded: &mut WasmFunction,
        pointer: &PlaceAccess,
    ) -> Result<(), RealizationError> {
        let carrier_count = self.emit_raw_target_value(encoded, pointer)?;
        for _ in 0..carrier_count {
            encoded.instruction(&Instruction::Drop);
        }
        Ok(())
    }

    pub(super) fn emit_raw_move(
        &self,
        encoded: &mut WasmFunction,
        pointer: &PlaceAccess,
    ) -> Result<usize, RealizationError> {
        self.emit_raw_target_value(encoded, pointer)
    }

    pub(super) fn emit_raw_assign(
        &self,
        encoded: &mut WasmFunction,
        pointer: &PlaceAccess,
        source: &runen_core_ir::Operand,
    ) -> Result<(), RealizationError> {
        let pointee = self.raw_pointer_pointee(pointer)?;
        let carrier_count = storage_carrier_count(self.types, pointee)?;
        let target_scratch = self.layout.raw_assign_target_scratch()?;

        // Core RawAssign snapshots the target before source evaluation. Keep that
        // snapshot in a distinct scratch slot so a RawMove source may perform its
        // own raw-target dispatch without retargeting the outer replacement.
        self.emit_raw_pointer_handle_to_scratch(encoded, pointer, target_scratch)?;

        let emitted = self.emit_operand(encoded, source)?;
        if emitted != carrier_count {
            return Err(invariant(
                "validated RawAssign source carrier count disagrees with pointee",
            ));
        }
        if emitted > self.layout.scratch_len {
            return Err(invariant(
                "RawAssign source exceeds private scratch carrier capacity",
            ));
        }
        for index in (0..emitted).rev() {
            encoded.instruction(&Instruction::LocalSet(self.layout.scratch(index)?));
        }

        for target in self
            .layout
            .raw_pointer_targets
            .iter()
            .filter(|target| target.ty == pointee)
        {
            let target_layout = self.place_layout(&target.place)?;
            if target_layout.len != carrier_count {
                return Err(invariant(
                    "private raw-pointer target carrier count disagrees with pointee",
                ));
            }
            self.emit_raw_handle_match_start(encoded, target_scratch, target.handle);
            for index in 0..carrier_count {
                encoded.instruction(&Instruction::LocalGet(self.layout.scratch(index)?));
                encoded.instruction(&Instruction::LocalSet(add_carriers(
                    target_layout.start,
                    index,
                    "raw replacement target carrier index overflow",
                )?));
            }
            self.emit_raw_handle_match_end(encoded, target_scratch);
        }
        self.emit_raw_handle_dispatch_finish(encoded, target_scratch)
    }

    fn emit_raw_target_value(
        &self,
        encoded: &mut WasmFunction,
        pointer: &PlaceAccess,
    ) -> Result<usize, RealizationError> {
        let pointee = self.raw_pointer_pointee(pointer)?;
        let carrier_count = storage_carrier_count(self.types, pointee)?;
        if carrier_count > self.layout.scratch_len {
            return Err(invariant(
                "raw-pointer target exceeds private scratch carrier capacity",
            ));
        }
        let handle_scratch = self.layout.raw_pointer_handle_scratch()?;
        self.emit_raw_pointer_handle_to_scratch(encoded, pointer, handle_scratch)?;

        for target in self
            .layout
            .raw_pointer_targets
            .iter()
            .filter(|target| target.ty == pointee)
        {
            let target_layout = self.place_layout(&target.place)?;
            if target_layout.len != carrier_count {
                return Err(invariant(
                    "private raw-pointer target carrier count disagrees with pointee",
                ));
            }
            self.emit_raw_handle_match_start(encoded, handle_scratch, target.handle);
            for index in 0..carrier_count {
                encoded.instruction(&Instruction::LocalGet(add_carriers(
                    target_layout.start,
                    index,
                    "raw target carrier index overflow",
                )?));
                encoded.instruction(&Instruction::LocalSet(self.layout.scratch(index)?));
            }
            self.emit_raw_handle_match_end(encoded, handle_scratch);
        }
        self.emit_raw_handle_dispatch_finish(encoded, handle_scratch)?;

        for index in 0..carrier_count {
            encoded.instruction(&Instruction::LocalGet(self.layout.scratch(index)?));
        }
        Ok(carrier_count)
    }

    fn raw_pointer_pointee(&self, access: &PlaceAccess) -> Result<TypeId, RealizationError> {
        let pointer = direct_access_place(access)?;
        if !pointer.projections.is_empty() {
            return Err(invariant(
                "admitted raw-pointer value access is not a complete local root",
            ));
        }
        let pointer_ty = self
            .function
            .body
            .local(pointer.local)
            .ok_or_else(|| invariant("validated raw-pointer local is missing"))?
            .ty;
        let definition = self
            .types
            .get(pointer_ty)
            .ok_or_else(|| invariant("validated raw-pointer type is missing"))?;
        let TypeKind::Scalar(ScalarType::RawPointer(pointee)) = definition.kind else {
            return Err(invariant(
                "coverage admission allowed raw operation on non-pointer storage",
            ));
        };
        Ok(pointee)
    }

    fn emit_raw_pointer_handle_to_scratch(
        &self,
        encoded: &mut WasmFunction,
        access: &PlaceAccess,
        scratch: u32,
    ) -> Result<(), RealizationError> {
        let pointer = direct_access_place(access)?;
        let layout = self.place_layout(pointer)?;
        if layout.len != 1 {
            return Err(invariant(
                "admitted raw-pointer carrier does not occupy exactly one private slot",
            ));
        }
        encoded.instruction(&Instruction::LocalGet(layout.start));
        encoded.instruction(&Instruction::LocalSet(scratch));
        Ok(())
    }

    fn raw_pointer_target_for_place(&self, place: &Place) -> Option<&RawPointerTarget> {
        self.layout
            .raw_pointer_targets
            .iter()
            .find(|target| target.place == *place)
    }

    fn emit_raw_handle_match_start(
        &self,
        encoded: &mut WasmFunction,
        scratch: u32,
        handle: u32,
    ) {
        encoded.instruction(&Instruction::LocalGet(scratch));
        encoded.instruction(&Instruction::I64Const(i64::from(handle)));
        encoded.instruction(&Instruction::I64Eq);
        encoded.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
    }

    fn emit_raw_handle_match_end(&self, encoded: &mut WasmFunction, scratch: u32) {
        encoded.instruction(&Instruction::I64Const(0));
        encoded.instruction(&Instruction::LocalSet(scratch));
        encoded.instruction(&Instruction::End);
    }

    fn emit_raw_handle_dispatch_finish(
        &self,
        encoded: &mut WasmFunction,
        scratch: u32,
    ) -> Result<(), RealizationError> {
        encoded.instruction(&Instruction::LocalGet(scratch));
        encoded.instruction(&Instruction::I64Eqz);
        encoded.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        encoded.instruction(&Instruction::Else);
        encoded.instruction(&Instruction::Unreachable);
        encoded.instruction(&Instruction::End);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runen_core_ir::{
        BasicBlock, BasicBlockId, Body, LocalDecl, SafeReferenceResultContract, Terminator, TypeDef,
    };

    #[test]
    fn equal_local_roots_receive_distinct_private_raw_target_handles() {
        let mut types = TypeTable::new();
        let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
        let raw_i64 = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
        let function = Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![
                    LocalDecl::new("left", i64_ty, false),
                    LocalDecl::new("right", i64_ty, false),
                    LocalDecl::new("pointer", raw_i64, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            },
        };

        let targets = collect_raw_pointer_targets(&types, &function)
            .expect("private raw target collection must succeed");
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].place, Place::local(LocalId(0)));
        assert_eq!(targets[1].place, Place::local(LocalId(1)));
        assert_ne!(targets[0].handle, targets[1].handle);
    }
}
