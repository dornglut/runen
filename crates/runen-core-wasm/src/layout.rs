use runen_core_ir::{Projection, ScalarType, TypeId, TypeKind, TypeTable, Value};

use crate::scalar::{ScalarKind, constant_residue};
use crate::RealizationError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CarrierSpan {
    pub(crate) offset: usize,
    pub(crate) len: usize,
    pub(crate) ty: TypeId,
}

pub(crate) fn storage_carrier_count(
    types: &TypeTable,
    ty: TypeId,
) -> Result<usize, RealizationError> {
    carrier_count(types, ty, true, true, true)
}

pub(crate) fn result_carrier_count(
    types: &TypeTable,
    ty: TypeId,
) -> Result<usize, RealizationError> {
    // Coverage remains the admission authority for each result context. Ordinary
    // function results may now be a supported callable root, while callable-valued
    // interface components still reject before encoding. Safe-reference and raw-
    // pointer results remain outside the realized subset even though admitted local
    // carriers use one private i64 slot.
    carrier_count(types, ty, true, false, false)
}

fn carrier_count(
    types: &TypeTable,
    ty: TypeId,
    allow_callable_root: bool,
    allow_reference_root: bool,
    allow_raw_pointer_root: bool,
) -> Result<usize, RealizationError> {
    let definition = types
        .get(ty)
        .ok_or_else(|| invariant("validated Core type is missing"))?;
    if definition.interior_mutable {
        return Err(invariant(
            "coverage admission allowed interior-mutable carrier storage",
        ));
    }
    match &definition.kind {
        TypeKind::Scalar(_) if ScalarKind::from_type(types, ty).is_some() => Ok(1),
        TypeKind::Scalar(ScalarType::Callable(_)) if allow_callable_root => Ok(1),
        TypeKind::Scalar(ScalarType::Reference { .. }) if allow_reference_root => Ok(1),
        TypeKind::Scalar(ScalarType::RawPointer(_)) if allow_raw_pointer_root => Ok(1),
        TypeKind::Struct(fields) => fields.iter().try_fold(0_usize, |count, field| {
            let field_count = carrier_count(types, field.ty, false, false, false)?;
            count
                .checked_add(field_count)
                .ok_or_else(|| invariant("structural carrier count overflow"))
        }),
        TypeKind::Scalar(_) => Err(invariant(
            "coverage admission allowed an unsupported carrier type",
        )),
    }
}

pub(crate) fn projected_span(
    types: &TypeTable,
    root: TypeId,
    projections: &[Projection],
) -> Result<CarrierSpan, RealizationError> {
    let mut ty = root;
    let mut offset = 0_usize;
    for projection in projections {
        let Projection::Field(index) = projection;
        let definition = types
            .get(ty)
            .ok_or_else(|| invariant("validated projected Core type is missing"))?;
        let TypeKind::Struct(fields) = &definition.kind else {
            return Err(invariant(
                "coverage admission allowed a projection through a non-structural type",
            ));
        };
        let field_index = *index as usize;
        let field = fields
            .get(field_index)
            .ok_or_else(|| invariant("validated Core projection is out of bounds"))?;
        for preceding in &fields[..field_index] {
            offset = offset
                .checked_add(carrier_count(types, preceding.ty, false, false, false)?)
                .ok_or_else(|| invariant("projected carrier offset overflow"))?;
        }
        ty = field.ty;
    }
    Ok(CarrierSpan {
        offset,
        len: carrier_count(
            types,
            ty,
            projections.is_empty(),
            projections.is_empty(),
            projections.is_empty(),
        )?,
        ty,
    })
}

pub(crate) fn constant_carriers(value: &Value) -> Result<Vec<i64>, RealizationError> {
    let mut carriers = Vec::new();
    append_constant_carriers(value, &mut carriers)?;
    Ok(carriers)
}

fn append_constant_carriers(
    value: &Value,
    carriers: &mut Vec<i64>,
) -> Result<(), RealizationError> {
    if let Value::Struct(fields) = value {
        for field in fields {
            append_constant_carriers(field, carriers)?;
        }
        return Ok(());
    }
    let residue = constant_residue(value)
        .ok_or_else(|| invariant("coverage admission allowed an unsupported constant carrier"))?;
    carriers.push(i64::from_ne_bytes(residue.to_ne_bytes()));
    Ok(())
}

fn invariant(message: impl Into<String>) -> RealizationError {
    RealizationError::BackendInvariant(message.into())
}
