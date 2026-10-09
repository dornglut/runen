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

#[cfg(test)]
mod tests {
    use super::*;
    use runen_core_ir::{BinaryFloatSign, BinaryFloatValue, Field, TypeDef};

    fn carrier(bits: u64) -> i64 {
        i64::from_ne_bytes(bits.to_ne_bytes())
    }

    #[test]
    fn every_float_format_observes_represented_values_and_nan_class() {
        for (scalar, kind) in [
            (ScalarType::F16, ScalarKind::F16),
            (ScalarType::F32, ScalarKind::F32),
            (ScalarType::F64, ScalarKind::F64),
        ] {
            let mut types = TypeTable::new();
            let ty = types.push(TypeDef::scalar("Floating", scalar));
            let significand = 1_u64
                << kind.float_format().expect("floating format").fraction_bits();
            let values = [
                FloatingScalarValue::Represented(BinaryFloatValue::Zero(
                    BinaryFloatSign::Positive,
                )),
                FloatingScalarValue::Represented(BinaryFloatValue::Zero(
                    BinaryFloatSign::Negative,
                )),
                FloatingScalarValue::Represented(BinaryFloatValue::Subnormal {
                    sign: BinaryFloatSign::Negative,
                    significand: 1,
                }),
                FloatingScalarValue::Represented(BinaryFloatValue::Normal {
                    sign: BinaryFloatSign::Positive,
                    significand,
                    exponent: 0,
                }),
                FloatingScalarValue::Represented(BinaryFloatValue::Infinity(
                    BinaryFloatSign::Positive,
                )),
                FloatingScalarValue::Represented(BinaryFloatValue::Infinity(
                    BinaryFloatSign::Negative,
                )),
                FloatingScalarValue::NaNClass,
            ];
            for value in values {
                let bits = kind.floating_residue(value).expect("valid floating carrier");
                let expected = match kind {
                    ScalarKind::F16 => ExecutionValue::F16(value),
                    ScalarKind::F32 => ExecutionValue::F32(value),
                    ScalarKind::F64 => ExecutionValue::F64(value),
                    _ => unreachable!(),
                };
                assert_eq!(decode_result(&types, ty, &[carrier(bits)]), Ok(expected));
            }
        }
    }

    #[test]
    fn decoder_preserves_nested_empty_fields_and_rejects_wrong_carrier_counts() {
        let mut types = TypeTable::new();
        let empty = types.push(TypeDef::structure("Empty", vec![]));
        let f32_ty = types.push(TypeDef::scalar("F32", ScalarType::F32));
        let i8_ty = types.push(TypeDef::scalar("I8", ScalarType::I8));
        let inner = types.push(TypeDef::structure(
            "Inner",
            vec![Field::new("number", f32_ty), Field::new("empty", empty)],
        ));
        let outer = types.push(TypeDef::structure(
            "Outer",
            vec![Field::new("inner", inner), Field::new("counter", i8_ty)],
        ));
        let positive_zero = ScalarKind::F32
            .floating_residue(FloatingScalarValue::Represented(BinaryFloatValue::Zero(
                BinaryFloatSign::Positive,
            )))
            .expect("valid zero");
        let payload = [carrier(positive_zero), carrier(7)];
        assert_eq!(
            decode_result(&types, outer, &payload),
            Ok(ExecutionValue::Struct(vec![
                ExecutionValue::Struct(vec![
                    ExecutionValue::F32(FloatingScalarValue::Represented(
                        BinaryFloatValue::Zero(BinaryFloatSign::Positive),
                    )),
                    ExecutionValue::Struct(vec![]),
                ]),
                ExecutionValue::I8(7),
            ]))
        );
        assert_eq!(decode_result(&types, empty, &[]), Ok(ExecutionValue::Struct(vec![])));
        assert!(matches!(
            decode_result(&types, outer, &payload[..1]),
            Err(RealizationError::BackendInvariant(_))
        ));
        assert!(matches!(
            decode_result(&types, outer, &[payload[0], payload[1], 0]),
            Err(RealizationError::BackendInvariant(_))
        ));
    }

    #[test]
    fn invalid_private_carriers_are_not_semantic_nan_or_runen_faults() {
        let mut types = TypeTable::new();
        let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
        let u8_ty = types.push(TypeDef::scalar("U8", ScalarType::U8));
        assert!(matches!(
            decode_result(&types, bool_ty, &[2]),
            Err(RealizationError::BackendInvariant(_))
        ));
        assert!(matches!(
            decode_result(&types, u8_ty, &[256]),
            Err(RealizationError::BackendInvariant(_))
        ));
        for (scalar, kind) in [
            (ScalarType::F16, ScalarKind::F16),
            (ScalarType::F32, ScalarKind::F32),
            (ScalarType::F64, ScalarKind::F64),
        ] {
            let ty = types.push(TypeDef::scalar("Float", scalar));
            let canonical = kind
                .floating_residue(FloatingScalarValue::NaNClass)
                .expect("NaN carrier");
            let sign = kind.float_format().expect("format").sign_mask();
            assert!(matches!(
                decode_result(&types, ty, &[carrier(canonical | sign)]),
                Err(RealizationError::BackendInvariant(_))
            ));
            assert!(matches!(
                decode_result(&types, ty, &[carrier(canonical | 2)]),
                Err(RealizationError::BackendInvariant(_))
            ));
        }
        for (scalar, kind) in [
            (ScalarType::F16, ScalarKind::F16),
            (ScalarType::F32, ScalarKind::F32),
        ] {
            let ty = types.push(TypeDef::scalar("Narrow", scalar));
            assert!(matches!(
                decode_result(&types, ty, &[carrier(1_u64 << kind.width())]),
                Err(RealizationError::BackendInvariant(_))
            ));
        }
    }

    #[test]
    fn structural_rust_observation_equality_is_not_nominal_type_equality() {
        let mut types = TypeTable::new();
        let boolean = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
        let left = types.push(TypeDef::structure(
            "Left",
            vec![Field::new("value", boolean)],
        ));
        let right = types.push(TypeDef::structure(
            "Right",
            vec![Field::new("value", boolean)],
        ));
        assert_ne!(left, right);
        assert_eq!(
            decode_result(&types, left, &[1]),
            decode_result(&types, right, &[1])
        );
        assert_eq!(
            ExecutionValue::F32(FloatingScalarValue::NaNClass),
            ExecutionValue::F32(FloatingScalarValue::NaNClass)
        );
    }
}
