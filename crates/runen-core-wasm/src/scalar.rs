use runen_core_ir::{ScalarType, TypeId, TypeKind, TypeTable, Value};

use crate::{BackendProtocolError, RealizationError};

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
            TypeKind::Scalar(_)|TypeKind::Struct(_) => None,
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
                _ => Err(RealizationError::BackendProtocol(
                    BackendProtocolError::InvalidBooleanPayload(payload),
                )),
            },
            Self::I8 => decode_signed(residue, 8).map(|value| Value::I8(value as i8)),
            Self::I16 => decode_signed(residue, 16).map(|value| Value::I16(value as i16)),
            Self::I32 => decode_signed(residue, 32).map(|value| Value::I32(value as i32)),
            Self::I64 => decode_signed(residue, 64).map(Value::I64),
            Self::U8 => decode_unsigned(residue, 8).map(|value| Value::U8(value as u8)),
            Self::U16 => decode_unsigned(residue, 16).map(|value| Value::U16(value as u16)),
            Self::U32 => decode_unsigned(residue, 32).map(|value| Value::U32(value as u32)),
            Self::U64 => Ok(Value::U64(residue)),
        }
    }
}

pub(crate) fn constant_residue(value: &Value) -> Option<u64> {
    match value {
        Value::Bool(value) => Some(u64::from(*value)),
        Value::I8(value) => Some(signed_residue(i128::from(*value), 8)),
        Value::I16(value) => Some(signed_residue(i128::from(*value), 16)),
        Value::I32(value) => Some(signed_residue(i128::from(*value), 32)),
        Value::I64(value) => Some(signed_residue(i128::from(*value), 64)),
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

fn signed_residue(value: i128, width: u32) -> u64 {
    let modulus = 1_u128 << width;
    let residue = if value >= 0 {
        value as u128
    } else {
        modulus - value.unsigned_abs()
    };
    u64::try_from(residue).expect("fixed-width signed residue fits u64")
}

fn decode_unsigned(residue: u64, width: u32) -> Result<u64, RealizationError> {
    if residue & !mask(width) == 0 {
        Ok(residue)
    } else {
        Err(RealizationError::BackendProtocol(
            BackendProtocolError::NonCanonicalIntegerPayload { payload: residue, width },
        ))
    }
}

fn decode_signed(residue: u64, width: u32) -> Result<i64, RealizationError> {
    let residue = decode_unsigned(residue, width)?;
    let signed_boundary = 1_u128 << (width - 1);
    let modulus = 1_u128 << width;
    let residue = u128::from(residue);
    let value = if residue < signed_boundary {
        i128::try_from(residue).expect("u64 residue fits i128")
    } else {
        i128::try_from(residue).expect("u64 residue fits i128")
            - i128::try_from(modulus).expect("2^64 fits i128")
    };
    i64::try_from(value).map_err(|_| {
        RealizationError::BackendProtocol(BackendProtocolError::NonCanonicalIntegerPayload {
            payload: u64::try_from(residue).expect("residue remains u64-sized"),
            width,
        })
    })
}
