use runen_core_ir::{
    Function, LocalId, PersistentDecl, PersistentId, Place, ReferenceAccess, ScalarType, TypeId,
    TypeKind, TypeTable,
};
use wasm_encoder::{Function as WasmFunction, Instruction};

use crate::RealizationError;
use crate::layout::storage_carrier_count;

use super::{FunctionEncoder, add_carriers, direct_access_place, invariant};

#[derive(Clone, Debug)]
pub(super) enum ReferenceTargetStorage {
    Local(Place),
    Persistent(PersistentId),
}

#[derive(Clone, Debug)]
pub(super) struct ReferenceTarget {
    pub(super) handle: u32,
    pub(super) storage: ReferenceTargetStorage,
    pub(super) ty: TypeId,
}

pub(super) fn collect_reference_targets(
    types: &TypeTable,
    function: &Function,
    persistent: &[PersistentDecl],
) -> Result<Vec<ReferenceTarget>, RealizationError> {
    let has_reference_local = function.body.locals.iter().any(|local| {
        matches!(
            types.get(local.ty).map(|definition| &definition.kind),
            Some(TypeKind::Scalar(ScalarType::Reference { .. }))
        )
    });
    if !has_reference_local {
        return Ok(Vec::new());
    }

    let mut targets = Vec::new();
    for (index, local) in function.body.locals.iter().enumerate() {
        let local_id = LocalId(
            u32::try_from(index).map_err(|_| invariant("Core local index exceeds u32::MAX"))?,
        );
        collect_target_regions(types, Place::local(local_id), local.ty, &mut targets)?;
    }
    for (index, declaration) in persistent.iter().enumerate() {
        if !reference_target_type_supported(types, declaration.ty) {
            continue;
        }
        let persistent = PersistentId(
            u32::try_from(index)
                .map_err(|_| invariant("Core persistent index exceeds u32::MAX"))?,
        );
        push_target(
            &mut targets,
            ReferenceTargetStorage::Persistent(persistent),
            declaration.ty,
        )?;
    }
    Ok(targets)
}

fn collect_target_regions(
    types: &TypeTable,
    place: Place,
    ty: TypeId,
    targets: &mut Vec<ReferenceTarget>,
) -> Result<(), RealizationError> {
    if !reference_target_type_supported(types, ty) {
        return Ok(());
    }

    push_target(targets, ReferenceTargetStorage::Local(place.clone()), ty)?;

    let definition = types
        .get(ty)
        .ok_or_else(|| invariant("validated reference target type is missing"))?;
    if let TypeKind::Struct(fields) = &definition.kind {
        for (index, field) in fields.iter().enumerate() {
            let field_index = u32::try_from(index)
                .map_err(|_| invariant("Core structural field index exceeds u32::MAX"))?;
            collect_target_regions(types, place.clone().field(field_index), field.ty, targets)?;
        }
    }
    Ok(())
}

fn push_target(
    targets: &mut Vec<ReferenceTarget>,
    storage: ReferenceTargetStorage,
    ty: TypeId,
) -> Result<(), RealizationError> {
    let handle = u32::try_from(targets.len())
        .map_err(|_| invariant("private reference target count exceeds u32::MAX"))?
        .checked_add(1)
        .ok_or_else(|| invariant("private reference target handle overflow"))?;
    targets.push(ReferenceTarget {
        handle,
        storage,
        ty,
    });
    Ok(())
}

fn reference_target_type_supported(types: &TypeTable, ty: TypeId) -> bool {
    let Some(definition) = types.get(ty) else {
        return false;
    };
    if definition.interior_mutable {
        return false;
    }
    match &definition.kind {
        TypeKind::Scalar(
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
            | ScalarType::F64,
        ) => true,
        TypeKind::Struct(fields) => fields
            .iter()
            .all(|field| reference_target_type_supported(types, field.ty)),
        TypeKind::Scalar(
            ScalarType::RawPointer(_)
            | ScalarType::Reference { .. }
            | ScalarType::Callable(_)
            | ScalarType::TrackedFixture,
        ) => false,
    }
}

impl FunctionEncoder<'_> {
    pub(super) fn emit_reference_root(
        &self,
        encoded: &mut WasmFunction,
        place: &Place,
    ) -> Result<usize, RealizationError> {
        let target = self
            .reference_target_for_place(place)
            .ok_or_else(|| invariant("admitted reference root has no private target handle"))?;
        encoded.instruction(&Instruction::I64Const(i64::from(target.handle)));
        Ok(1)
    }

    pub(super) fn emit_persistent_shared_root(
        &self,
        encoded: &mut WasmFunction,
        persistent: PersistentId,
    ) -> Result<usize, RealizationError> {
        let target = self
            .reference_target_for_persistent(persistent)
            .ok_or_else(|| {
                invariant("admitted persistent Shared root has no private target handle")
            })?;
        encoded.instruction(&Instruction::I64Const(i64::from(target.handle)));
        Ok(1)
    }

    pub(super) fn emit_reference_reborrow(
        &self,
        encoded: &mut WasmFunction,
        access: &ReferenceAccess,
    ) -> Result<usize, RealizationError> {
        let (_, referent, _) = self.reference_access_shape(access)?;
        self.emit_reference_handle_to_scratch(encoded, access)?;
        let value_scratch = self.layout.scratch(0)?;

        for parent in self
            .layout
            .reference_targets
            .iter()
            .filter(|target| target.ty == referent)
        {
            let child = match &parent.storage {
                ReferenceTargetStorage::Local(parent_place) => {
                    let child_place = projected_place(parent_place, &access.projections);
                    self.reference_target_for_place(&child_place).ok_or_else(|| {
                        invariant(
                            "admitted reference reborrow has no projected private target handle",
                        )
                    })?
                }
                ReferenceTargetStorage::Persistent(persistent) => {
                    if !access.projections.is_empty() {
                        return Err(invariant(
                            "admitted persistent Shared reborrow has a structural projection",
                        ));
                    }
                    self.reference_target_for_persistent(*persistent)
                        .ok_or_else(|| {
                            invariant(
                                "admitted persistent Shared reborrow has no private target handle",
                            )
                        })?
                }
            };
            self.emit_handle_match_start(encoded, parent.handle)?;
            encoded.instruction(&Instruction::I64Const(i64::from(child.handle)));
            encoded.instruction(&Instruction::LocalSet(value_scratch));
            self.emit_handle_match_end(encoded);
        }
        self.emit_handle_dispatch_finish(encoded)?;
        encoded.instruction(&Instruction::LocalGet(value_scratch));
        Ok(1)
    }

    pub(super) fn emit_reference_value(
        &self,
        encoded: &mut WasmFunction,
        access: &ReferenceAccess,
    ) -> Result<usize, RealizationError> {
        let (_, referent, selected_ty) = self.reference_access_shape(access)?;
        let carrier_count = storage_carrier_count(self.types, selected_ty)?;
        if carrier_count > self.layout.scratch_len {
            return Err(invariant(
                "reference-selected value exceeds private scratch carrier capacity",
            ));
        }

        self.emit_reference_handle_to_scratch(encoded, access)?;
        for parent in self
            .layout
            .reference_targets
            .iter()
            .filter(|target| target.ty == referent)
        {
            self.emit_handle_match_start(encoded, parent.handle)?;
            match &parent.storage {
                ReferenceTargetStorage::Local(parent_place) => {
                    let selected_place = projected_place(parent_place, &access.projections);
                    let selected = self.place_layout(&selected_place)?;
                    if selected.len != carrier_count {
                        return Err(invariant(
                            "private reference target carrier count disagrees with validated referent",
                        ));
                    }
                    for index in 0..carrier_count {
                        encoded.instruction(&Instruction::LocalGet(add_carriers(
                            selected.start,
                            index,
                            "reference target carrier index overflow",
                        )?));
                        encoded.instruction(&Instruction::LocalSet(self.layout.scratch(index)?));
                    }
                }
                ReferenceTargetStorage::Persistent(persistent) => {
                    if !access.projections.is_empty() || carrier_count != 1 {
                        return Err(invariant(
                            "admitted persistent Shared access is not one complete scalar root",
                        ));
                    }
                    encoded.instruction(&Instruction::GlobalGet(persistent.0));
                    encoded.instruction(&Instruction::LocalSet(self.layout.scratch(0)?));
                }
            }
            self.emit_handle_match_end(encoded);
        }
        self.emit_handle_dispatch_finish(encoded)?;

        for index in 0..carrier_count {
            encoded.instruction(&Instruction::LocalGet(self.layout.scratch(index)?));
        }
        Ok(carrier_count)
    }

    pub(super) fn emit_reference_read_or_drop(
        &self,
        encoded: &mut WasmFunction,
        access: &ReferenceAccess,
    ) -> Result<(), RealizationError> {
        let carrier_count = self.emit_reference_value(encoded, access)?;
        for _ in 0..carrier_count {
            encoded.instruction(&Instruction::Drop);
        }
        Ok(())
    }

    pub(super) fn emit_reference_assign(
        &self,
        encoded: &mut WasmFunction,
        destination: &ReferenceAccess,
        source: &runen_core_ir::Operand,
    ) -> Result<(), RealizationError> {
        let (_, referent, selected_ty) = self.reference_access_shape(destination)?;
        let carrier_count = storage_carrier_count(self.types, selected_ty)?;
        let emitted = self.emit_operand(encoded, source)?;
        if emitted != carrier_count {
            return Err(invariant(
                "validated reference replacement has mismatched carrier counts",
            ));
        }
        if emitted > self.layout.scratch_len {
            return Err(invariant(
                "reference replacement exceeds private scratch carrier capacity",
            ));
        }
        for index in (0..emitted).rev() {
            encoded.instruction(&Instruction::LocalSet(self.layout.scratch(index)?));
        }

        self.emit_reference_handle_to_scratch(encoded, destination)?;
        for parent in self
            .layout
            .reference_targets
            .iter()
            .filter(|target| target.ty == referent)
        {
            let ReferenceTargetStorage::Local(parent_place) = &parent.storage else {
                continue;
            };
            let selected_place = projected_place(parent_place, &destination.projections);
            let selected = self.place_layout(&selected_place)?;
            if selected.len != carrier_count {
                return Err(invariant(
                    "private reference replacement target has mismatched carrier count",
                ));
            }
            self.emit_handle_match_start(encoded, parent.handle)?;
            for index in 0..carrier_count {
                encoded.instruction(&Instruction::LocalGet(self.layout.scratch(index)?));
                encoded.instruction(&Instruction::LocalSet(add_carriers(
                    selected.start,
                    index,
                    "reference replacement carrier index overflow",
                )?));
            }
            self.emit_handle_match_end(encoded);
        }
        self.emit_handle_dispatch_finish(encoded)
    }

    fn reference_access_shape(
        &self,
        access: &ReferenceAccess,
    ) -> Result<(Place, TypeId, TypeId), RealizationError> {
        let carrier = direct_access_place(&access.reference)?.clone();
        let carrier_root = self
            .function
            .body
            .local(carrier.local)
            .ok_or_else(|| invariant("validated reference carrier local is missing"))?
            .ty;
        let carrier_ty = self
            .types
            .project_type(carrier_root, &carrier.projections)
            .ok_or_else(|| invariant("validated reference carrier projection is invalid"))?;
        let (referent, _) = self
            .types
            .reference(carrier_ty)
            .ok_or_else(|| invariant("validated reference access carrier is not a reference"))?;
        let selected_ty = self
            .types
            .project_type(referent, &access.projections)
            .ok_or_else(|| invariant("validated reference target projection is invalid"))?;
        Ok((carrier, referent, selected_ty))
    }

    fn emit_reference_handle_to_scratch(
        &self,
        encoded: &mut WasmFunction,
        access: &ReferenceAccess,
    ) -> Result<(), RealizationError> {
        let (carrier, _, _) = self.reference_access_shape(access)?;
        let carrier_layout = self.place_layout(&carrier)?;
        if carrier_layout.len != 1 {
            return Err(invariant(
                "admitted reference carrier does not occupy exactly one private slot",
            ));
        }
        encoded.instruction(&Instruction::LocalGet(carrier_layout.start));
        encoded.instruction(&Instruction::LocalSet(
            self.layout.reference_handle_scratch()?,
        ));
        Ok(())
    }

    fn reference_target_for_place(&self, place: &Place) -> Option<&ReferenceTarget> {
        self.layout.reference_targets.iter().find(|target| {
            matches!(&target.storage, ReferenceTargetStorage::Local(target_place) if target_place == place)
        })
    }

    fn reference_target_for_persistent(
        &self,
        persistent: PersistentId,
    ) -> Option<&ReferenceTarget> {
        self.layout.reference_targets.iter().find(|target| {
            matches!(
                target.storage,
                ReferenceTargetStorage::Persistent(target_persistent)
                    if target_persistent == persistent
            )
        })
    }

    fn emit_handle_match_start(
        &self,
        encoded: &mut WasmFunction,
        handle: u32,
    ) -> Result<(), RealizationError> {
        encoded.instruction(&Instruction::LocalGet(
            self.layout.reference_handle_scratch()?,
        ));
        encoded.instruction(&Instruction::I64Const(i64::from(handle)));
        encoded.instruction(&Instruction::I64Eq);
        encoded.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        Ok(())
    }

    fn emit_handle_match_end(&self, encoded: &mut WasmFunction) {
        encoded.instruction(&Instruction::I64Const(0));
        encoded.instruction(&Instruction::LocalSet(
            self.layout
                .reference_handle_scratch
                .expect("reference dispatch has a private handle scratch"),
        ));
        encoded.instruction(&Instruction::End);
    }

    fn emit_handle_dispatch_finish(
        &self,
        encoded: &mut WasmFunction,
    ) -> Result<(), RealizationError> {
        encoded.instruction(&Instruction::LocalGet(
            self.layout.reference_handle_scratch()?,
        ));
        encoded.instruction(&Instruction::I64Eqz);
        encoded.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        encoded.instruction(&Instruction::Else);
        encoded.instruction(&Instruction::Unreachable);
        encoded.instruction(&Instruction::End);
        Ok(())
    }
}

fn projected_place(root: &Place, projections: &[runen_core_ir::Projection]) -> Place {
    let mut place = root.clone();
    place.projections.extend_from_slice(projections);
    place
}
