//! Semantic values observed at the bounded Core-Wasm execution entry boundary.
//!
//! This is not a Runen value construction API, an independent nominal type
//! identity, an ABI, serialization, or a physical WebAssembly representation.

use runen_core_ir::{ScalarType, TypeId, TypeKind, TypeTable, Value};

use crate::scalar::{FloatingScalarValue, ScalarKind};
use crate::{RealizationError, invalid_backend_result};

/// One semantic observation of a selected, validated Core function's result.
///
/// A `Struct` retains its fields' declared order, but not a standalone nominal
/// or Core `TypeId`: the selected function's exact result type supplies that
/// context. Rust `PartialEq`/`Eq` compare *observation representations only*;
/// equal structural observations do not equate distinct Runen nominal types,
/// and equal `NaNClass` observations do not identify NaN members or define
/// Runen language equality, hashing, ABI identity, or serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionValue {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F16(FloatingScalarValue),
    F32(FloatingScalarValue),
    F64(FloatingScalarValue),
    Struct(Vec<Self>),
}

/// An entry result is observable precisely when every leaf is an admitted
/// semantic scalar. Callable identity and all other unsupported value kinds
/// stay unavailable through the public execution-result boundary.
pub(crate) fn is_observable_entry_result(types: &TypeTable, ty: TypeId) -> bool {
    let Some(definition) = types.get(ty) else {
        return false;
    };
    match &definition.kind {
        TypeKind::Scalar(ScalarType::Callable(_)) => false,
        TypeKind::Scalar(_) => ScalarKind::from_type(types, ty).is_some(),
        TypeKind::Struct(fields) => fields
            .iter()
            .all(|field| is_observable_entry_result(types, field.ty)),
    }
}

/// Decode the entire private result payload against the selected validated
/// result type. It must consume exactly the known number of carriers; no bits,
/// padding, private NaN representative, or callable identity escape this API.
pub(crate) fn decode_result(
    types: &TypeTable,
    ty: TypeId,
    carriers: &[i64],
) -> Result<ExecutionValue, RealizationError> {
    let mut cursor = 0_usize;
    let value = decode_result_at(types, ty, carriers, &mut cursor)?;
    if cursor != carriers.len() {
        return Err(invalid_backend_result());
    }
    Ok(value)
}

fn decode_result_at(
    types: &TypeTable,
    ty: TypeId,
    carriers: &[i64],
    cursor: &mut usize,
) -> Result<ExecutionValue, RealizationError> {
    if let Some(kind) = ScalarKind::from_type(types, ty) {
        let carrier = carriers
            .get(*cursor)
            .copied()
            .ok_or_else(invalid_backend_result)?;
        *cursor = cursor.checked_add(1).ok_or_else(invalid_backend_result)?;
        return match kind {
            ScalarKind::F16 => kind.decode_floating(carrier).map(ExecutionValue::F16),
            ScalarKind::F32 => kind.decode_floating(carrier).map(ExecutionValue::F32),
            ScalarKind::F64 => kind.decode_floating(carrier).map(ExecutionValue::F64),
            _ => match kind.decode(carrier)? {
                Value::Bool(value) => Ok(ExecutionValue::Bool(value)),
                Value::I8(value) => Ok(ExecutionValue::I8(value)),
                Value::I16(value) => Ok(ExecutionValue::I16(value)),
                Value::I32(value) => Ok(ExecutionValue::I32(value)),
                Value::I64(value) => Ok(ExecutionValue::I64(value)),
                Value::U8(value) => Ok(ExecutionValue::U8(value)),
                Value::U16(value) => Ok(ExecutionValue::U16(value)),
                Value::U32(value) => Ok(ExecutionValue::U32(value)),
                Value::U64(value) => Ok(ExecutionValue::U64(value)),
                _ => Err(invalid_backend_result()),
            },
        };
    }

    let definition = types.get(ty).ok_or_else(invalid_backend_result)?;
    let TypeKind::Struct(fields) = &definition.kind else {
        return Err(invalid_backend_result());
    };
    let mut values = Vec::with_capacity(fields.len());
    for field in fields {
        values.push(decode_result_at(types, field.ty, carriers, cursor)?);
    }
    Ok(ExecutionValue::Struct(values))
}
