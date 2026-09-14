use runen_core_ir::{Projection, ScalarType, TypeId, TypeKind, TypeTable, Value};

use crate::scalar::{ScalarKind, constant_residue};
use crate::{RealizationError, invalid_backend_result};

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
    carrier_count(types, ty, true)
}

pub(crate) fn result_carrier_count(
    types: &TypeTable,
    ty: TypeId,
) -> Result<usize, RealizationError> {
    // Coverage remains the admission authority for each result context. Ordinary
    // function results may now be a supported callable root, while callable-valued
    // interface components still reject before encoding.
    carrier_count(types, ty, true)
}

fn carrier_count(
    types: &TypeTable,
    ty: TypeId,
    allow_callable_root: bool,
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
        TypeKind::Struct(fields) => fields.iter().try_fold(0_usize, |count, field| {
            let field_count = carrier_count(types, field.ty, false)?;
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
                .checked_add(carrier_count(types, preceding.ty, false)?)
                .ok_or_else(|| invariant("projected carrier offset overflow"))?;
        }
        ty = field.ty;
    }
    Ok(CarrierSpan {
        offset,
        len: carrier_count(types, ty, projections.is_empty())?,
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

pub(crate) fn decode_value(
    types: &TypeTable,
    ty: TypeId,
    carriers: &[i64],
) -> Result<Value, RealizationError> {
    let mut cursor = 0_usize;
    let value = decode_value_at(types, ty, carriers, &mut cursor)?;
    if cursor != carriers.len() {
        return Err(invalid_backend_result());
    }
    Ok(value)
}

fn decode_value_at(
    types: &TypeTable,
    ty: TypeId,
    carriers: &[i64],
    cursor: &mut usize,
) -> Result<Value, RealizationError> {
    if let Some(kind) = ScalarKind::from_type(types, ty) {
        let carrier = carriers
            .get(*cursor)
            .copied()
            .ok_or_else(invalid_backend_result)?;
        *cursor += 1;
        return kind.decode(carrier);
    }
    let definition = types.get(ty).ok_or_else(invalid_backend_result)?;
    let TypeKind::Struct(fields) = &definition.kind else {
        return Err(invalid_backend_result());
    };
    let mut values = Vec::with_capacity(fields.len());
    for field in fields {
        values.push(decode_value_at(types, field.ty, carriers, cursor)?);
    }
    Ok(Value::Struct(values))
}

fn invariant(message: impl Into<String>) -> RealizationError {
    RealizationError::BackendInvariant(message.into())
}
