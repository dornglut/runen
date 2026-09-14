use runen_core_ir::{
    BinaryFloatSign, BinaryFloatValue, ScalarType, TypeId, TypeKind, TypeTable, Value,
};

use crate::{RealizationError, invalid_backend_result};

/// Semantic floating value exposed at the Core-Wasm provider boundary.
///
/// `NaNClass` records only membership in the Runen semantic NaN class. It does not
/// assign a NaN member identity, sign, payload, quiet/signaling state, physical
/// encoding, ABI representation, or Runen equality relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatingScalarValue {
    Represented(BinaryFloatValue),
    NaNClass,
}

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
    F16,
    F32,
    F64,
}

#[derive(Clone, Copy)]
struct FloatFormat {
    width: u32,
    precision: u32,
    emin: i16,
    emax: i16,
    exponent_bits: u32,
}

impl FloatFormat {
    const fn fraction_bits(self) -> u32 {
        self.precision - 1
    }

    const fn exponent_mask(self) -> u64 {
        (1_u64 << self.exponent_bits) - 1
    }

    const fn bias(self) -> i16 {
        ((1_u16 << (self.exponent_bits - 1)) - 1) as i16
    }

    const fn sign_mask(self) -> u64 {
        1_u64 << (self.width - 1)
    }

    const fn canonical_nan_residue(self) -> u64 {
        (self.exponent_mask() << self.fraction_bits()) | 1
    }
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
            TypeKind::Scalar(ScalarType::F16) => Some(Self::F16),
            TypeKind::Scalar(ScalarType::F32) => Some(Self::F32),
            TypeKind::Scalar(ScalarType::F64) => Some(Self::F64),
            TypeKind::Scalar(_) | TypeKind::Struct(_) => None,
        }
    }

    pub(crate) const fn width(self) -> u32 {
        match self {
            Self::Bool => 1,
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 | Self::F16 => 16,
            Self::I32 | Self::U32 | Self::F32 => 32,
            Self::I64 | Self::U64 | Self::F64 => 64,
        }
    }

    pub(crate) const fn is_signed(self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }

    pub(crate) const fn is_floating(self) -> bool {
        matches!(self, Self::F16 | Self::F32 | Self::F64)
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
            Self::F16 => represented_float(self.decode_floating(payload)?).map(Value::F16),
            Self::F32 => represented_float(self.decode_floating(payload)?).map(Value::F32),
            Self::F64 => represented_float(self.decode_floating(payload)?).map(Value::F64),
        }
    }

    pub(crate) fn decode_floating(
        self,
        payload: i64,
    ) -> Result<FloatingScalarValue, RealizationError> {
        let Some(format) = self.float_format() else {
            return Err(invalid_backend_result());
        };
        let residue = decode_unsigned(u64::from_ne_bytes(payload.to_ne_bytes()), format.width)?;
        let fraction_bits = format.fraction_bits();
        let fraction_mask = mask(fraction_bits);
        let sign = if residue & format.sign_mask() == 0 {
            BinaryFloatSign::Positive
        } else {
            BinaryFloatSign::Negative
        };
        let exponent = (residue >> fraction_bits) & format.exponent_mask();
        let fraction = residue & fraction_mask;

        if exponent == 0 {
            return Ok(if fraction == 0 {
                FloatingScalarValue::Represented(BinaryFloatValue::Zero(sign))
            } else {
                FloatingScalarValue::Represented(BinaryFloatValue::Subnormal {
                    sign,
                    significand: fraction,
                })
            });
        }

        if exponent == format.exponent_mask() {
            if fraction == 0 {
                return Ok(FloatingScalarValue::Represented(
                    BinaryFloatValue::Infinity(sign),
                ));
            }
            if residue & !format.sign_mask() == format.canonical_nan_residue() {
                return Ok(FloatingScalarValue::NaNClass);
            }
            return Err(invalid_backend_result());
        }

        let significand = (1_u64 << fraction_bits) | fraction;
        let exponent =
            i16::try_from(exponent).map_err(|_| invalid_backend_result())? - format.bias();
        Ok(FloatingScalarValue::Represented(BinaryFloatValue::Normal {
            sign,
            significand,
            exponent,
        }))
    }

    pub(crate) fn floating_residue(
        self,
        value: FloatingScalarValue,
    ) -> Result<u64, RealizationError> {
        let Some(format) = self.float_format() else {
            return Err(invalid_backend_result());
        };
        match value {
            FloatingScalarValue::Represented(value) => {
                encode_binary_float(format, value).ok_or_else(invalid_backend_result)
            }
            FloatingScalarValue::NaNClass => Ok(format.canonical_nan_residue()),
        }
    }

    pub(crate) fn floating_value_matches(self, value: BinaryFloatValue) -> bool {
        self.float_format()
            .is_some_and(|format| encode_binary_float(format, value).is_some())
    }

    const fn float_format(self) -> Option<FloatFormat> {
        match self {
            Self::F16 => Some(FloatFormat {
                width: 16,
                precision: 11,
                emin: -14,
                emax: 15,
                exponent_bits: 5,
            }),
            Self::F32 => Some(FloatFormat {
                width: 32,
                precision: 24,
                emin: -126,
                emax: 127,
                exponent_bits: 8,
            }),
            Self::F64 => Some(FloatFormat {
                width: 64,
                precision: 53,
                emin: -1022,
                emax: 1023,
                exponent_bits: 11,
            }),
            Self::Bool
            | Self::I8
            | Self::I16
            | Self::I32
            | Self::I64
            | Self::U8
            | Self::U16
            | Self::U32
            | Self::U64 => None,
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
        Value::F16(value) => ScalarKind::F16
            .float_format()
            .and_then(|format| encode_binary_float(format, *value)),
        Value::F32(value) => ScalarKind::F32
            .float_format()
            .and_then(|format| encode_binary_float(format, *value)),
        Value::F64(value) => ScalarKind::F64
            .float_format()
            .and_then(|format| encode_binary_float(format, *value)),
        Value::TrackedFixture(_) | Value::Struct(_) => None,
    }
}

pub(crate) const fn mask(width: u32) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1_u64 << width) - 1
    }
}

fn represented_float(value: FloatingScalarValue) -> Result<BinaryFloatValue, RealizationError> {
    match value {
        FloatingScalarValue::Represented(value) => Ok(value),
        FloatingScalarValue::NaNClass => Err(invalid_backend_result()),
    }
}

fn encode_binary_float(format: FloatFormat, value: BinaryFloatValue) -> Option<u64> {
    let fraction_bits = format.fraction_bits();
    let normal_min = 1_u64 << fraction_bits;
    let significand_limit = 1_u64 << format.precision;
    let sign = match value {
        BinaryFloatValue::Zero(sign)
        | BinaryFloatValue::Subnormal { sign, .. }
        | BinaryFloatValue::Normal { sign, .. }
        | BinaryFloatValue::Infinity(sign) => sign,
    };
    let sign_bits = match sign {
        BinaryFloatSign::Positive => 0,
        BinaryFloatSign::Negative => format.sign_mask(),
    };

    let magnitude = match value {
        BinaryFloatValue::Zero(_) => 0,
        BinaryFloatValue::Subnormal { significand, .. } => {
            if !(1..normal_min).contains(&significand) {
                return None;
            }
            significand
        }
        BinaryFloatValue::Normal {
            significand,
            exponent,
            ..
        } => {
            if !(normal_min..significand_limit).contains(&significand)
                || !(format.emin..=format.emax).contains(&exponent)
            {
                return None;
            }
            let exponent = i32::from(exponent) + i32::from(format.bias());
            let exponent = u64::try_from(exponent).ok()?;
            (exponent << fraction_bits) | (significand - normal_min)
        }
        BinaryFloatValue::Infinity(_) => format.exponent_mask() << fraction_bits,
    };
    Some(sign_bits | magnitude)
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

    fn payload(residue: u64) -> i64 {
        i64::from_ne_bytes(residue.to_ne_bytes())
    }

    #[test]
    fn invalid_private_integer_carriers_remain_realization_failures() {
        assert!(matches!(
            ScalarKind::Bool.decode(2),
            Err(RealizationError::BackendInvariant(_))
        ));
        assert!(matches!(
            ScalarKind::U8.decode(256),
            Err(RealizationError::BackendInvariant(_))
        ));
    }

    #[test]
    fn floating_carriers_round_trip_semantic_values_without_host_arithmetic() {
        let cases = [
            (
                ScalarKind::F16,
                BinaryFloatValue::Zero(BinaryFloatSign::Negative),
            ),
            (
                ScalarKind::F16,
                BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Positive,
                    significand: 1,
                },
            ),
            (
                ScalarKind::F32,
                BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Negative,
                    significand: 1_u64 << 23,
                    exponent: -126,
                },
            ),
            (
                ScalarKind::F64,
                BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Positive,
                    significand: (1_u64 << 53) - 1,
                    exponent: 1023,
                },
            ),
            (
                ScalarKind::F64,
                BinaryFloatValue::Infinity(BinaryFloatSign::Negative),
            ),
        ];

        for (kind, value) in cases {
            let residue = kind
                .floating_residue(FloatingScalarValue::Represented(value))
                .expect("represented value fits the declared format");
            assert_eq!(
                kind.decode_floating(payload(residue)),
                Ok(FloatingScalarValue::Represented(value))
            );
        }
    }

    #[test]
    fn floating_nan_class_uses_one_private_canonical_carrier() {
        for kind in [ScalarKind::F16, ScalarKind::F32, ScalarKind::F64] {
            let residue = kind
                .floating_residue(FloatingScalarValue::NaNClass)
                .expect("floating kind accepts NaN class");
            assert_eq!(
                kind.decode_floating(payload(residue)),
                Ok(FloatingScalarValue::NaNClass)
            );
            assert!(matches!(
                kind.decode_floating(payload(residue | 2)),
                Err(RealizationError::BackendInvariant(_))
            ));
        }
    }

    #[test]
    fn narrow_floating_carriers_reject_nonzero_unused_high_bits() {
        assert!(matches!(
            ScalarKind::F16.decode_floating(payload(1_u64 << 16)),
            Err(RealizationError::BackendInvariant(_))
        ));
        assert!(matches!(
            ScalarKind::F32.decode_floating(payload(1_u64 << 32)),
            Err(RealizationError::BackendInvariant(_))
        ));
    }

    #[test]
    fn represented_provider_value_must_match_declared_floating_format() {
        let f32_only = BinaryFloatValue::Normal {
            sign: BinaryFloatSign::Positive,
            significand: 1_u64 << 23,
            exponent: -126,
        };
        assert!(ScalarKind::F32.floating_value_matches(f32_only));
        assert!(!ScalarKind::F16.floating_value_matches(f32_only));
    }
}
