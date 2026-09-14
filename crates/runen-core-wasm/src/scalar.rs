use runen_core_ir::{ScalarType, TypeId, TypeKind, TypeTable, Value};

use crate::{invalid_backend_result, RealizationError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarKind {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
}

impl ScalarKind {
    pub(crate) fn from_type(types: &TypeTable, ty: TypeId) -> Option<Self> {
        match &types.get(ty)?.kind {
            TypeKind::Scalar(ScalarType::Bool) => Some(Self::Bool),
            TypeKind::Scalar(ScalarType::I8) => Some(Self::I8),
            TypeKind::Scalar(ScalarType::I16) => Some(Self::I16),
            TypeKind::Scalar(ScalarType::I32) => Some(Self::I32),
            TypeKind::Scalar(ScalarType::I64) => Some(Self::I64),
            TypeKind::Scalar(ScalarType::U8) => Some(Self::U8),
            TypeKind::Scalar(ScalarType::U16) => Some(Self::U16),
            TypeKind::Scalar(ScalarType::U32) => Some(Self::U32),
            TypeKind::Scalar(ScalarType::U64) => Some(Self::U64),
            TypeKind::Scalar(_) | TypeKind::Struct(_) => None,
        }
    }

    pub(crate) const fn width(self) -> u32 {
        match self {
            Self::Bool => 1,
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 => 32,
            Self::I64 | Self::U64 => 64,
        }
    }

    pub(crate) const fn is_signed(self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }

    pub(crate) fn decode(self, payload: i64) -> Result<Value, RealizationError> {
        let residue = u64::from_ne_bytes(payload.to_ne_bytes());
        match self {
            Self::Bool => match residue {
                0 => Ok(Value::Bool(false)),
                1 => Ok(Value::Bool(true)),
                _ => Err(invalid_backend_result()),
            },
            Self::I8 => narrow_signed::<i8>(residue, 8).map(Value::I8),
            Self::I16 => narrow_signed::<i16>(residue, 16).map(Value::I16),
            Self::I32 => narrow_signed::<i32>(residue, 32).map(Value::I32),
            Self::I64 => decode_signed(residue, 64).map(Value::I64),
            Self::U8 => narrow_unsigned::<u8>(residue, 8).map(Value::U8),
            Self::U16 => narrow_unsigned::<u16>(residue, 16).map(Value::U16),
            Self::U32 => narrow_unsigned::<u32>(residue, 32).map(Value::U32),
            Self::U64 => Ok(Value::U64(residue)),
        }
    }
}

pub(crate) fn constant_residue(value: &Value) -> Option<u64> {
    match value {
        Value::Bool(value) => Some(u64::from(*value)),
        Value::I8(value) => Some(signed_bits(i64::from(*value), 8)),
        Value::I16(value) => Some(signed_bits(i64::from(*value), 16)),
        Value::I32(value) => Some(signed_bits(i64::from(*value), 32)),
        Value::I64(value) => Some(u64::from_ne_bytes(value.to_ne_bytes())),
        Value::U8(value) => Some(u64::from(*value)),
        Value::U16(value) => Some(u64::from(*value)),
        Value::U32(value) => Some(u64::from(*value)),
        Value::U64(value) => Some(*value),
        Value::F16(_)
        | Value::F32(_)
        | Value::F64(_)
        | Value::TrackedFixture(_)
        | Value::Struct(_) => None,
    }
}

pub(crate) const fn mask(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1_u64 << width) - 1
    }
}

fn signed_bits(value: i64, width: u32) -> u64 {
    u64::from_ne_bytes(value.to_ne_bytes()) & mask(width)
}

fn decode_unsigned(residue: u64, width: u32) -> Result<u64, RealizationError> {
    if residue & !mask(width) == 0 {
        Ok(residue)
    } else {
        Err(invalid_backend_result())
    }
}

fn decode_signed(residue: u64, width: u32) -> Result<i64, RealizationError> {
    let residue = decode_unsigned(residue, width)?;
    if width == 64 {
        return Ok(i64::from_ne_bytes(residue.to_ne_bytes()));
    }
    let sign_bit = 1_u64 << (width - 1);
    let extended = if residue & sign_bit == 0 {
        residue
    } else {
        residue | !mask(width)
    };
    Ok(i64::from_ne_bytes(extended.to_ne_bytes()))
}

fn narrow_signed<T>(residue: u64, width: u32) -> Result<T, RealizationError>
where
    T: TryFrom<i64>,
{
    let value = decode_signed(residue, width)?;
    T::try_from(value).map_err(|_| invalid_backend_result())
}

fn narrow_unsigned<T>(residue: u64, width: u32) -> Result<T, RealizationError>
where
    T: TryFrom<u64>,
{
    let value = decode_unsigned(residue, width)?;
    T::try_from(value).map_err(|_| invalid_backend_result())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_private_carriers_remain_realization_failures() {
        assert!(matches!(
            ScalarKind::Bool.decode(2),
            Err(RealizationError::BackendInvariant(_))
        ));
        assert!(matches!(
            ScalarKind::U8.decode(256),
            Err(RealizationError::BackendInvariant(_))
        ));
    }
}
