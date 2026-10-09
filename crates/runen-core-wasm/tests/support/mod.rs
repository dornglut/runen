//! Test-only semantic observation conversions at a validated Core execution boundary.
//! The production realization never depends on the reference oracle.

use runen_core_ir::Value;
use runen_core_wasm::{ExecutionValue, FloatingScalarValue};
use runen_reference::{ObservedBinaryFloatValue, ObservedValue};

#[allow(dead_code)]
pub fn from_core_constant(value: Value) -> ExecutionValue {
    match value {
        Value::Bool(value) => ExecutionValue::Bool(value),
        Value::I8(value) => ExecutionValue::I8(value),
        Value::I16(value) => ExecutionValue::I16(value),
        Value::I32(value) => ExecutionValue::I32(value),
        Value::I64(value) => ExecutionValue::I64(value),
        Value::U8(value) => ExecutionValue::U8(value),
        Value::U16(value) => ExecutionValue::U16(value),
        Value::U32(value) => ExecutionValue::U32(value),
        Value::U64(value) => ExecutionValue::U64(value),
        Value::F16(value) => ExecutionValue::F16(FloatingScalarValue::Represented(value)),
        Value::F32(value) => ExecutionValue::F32(FloatingScalarValue::Represented(value)),
        Value::F64(value) => ExecutionValue::F64(FloatingScalarValue::Represented(value)),
        Value::Struct(fields) => ExecutionValue::Struct(
            fields.into_iter().map(from_core_constant).collect(),
        ),
        Value::TrackedFixture(_) => panic!("tracked fixtures cannot cross the Core-Wasm observation boundary"),
    }
}

fn floating(value: ObservedBinaryFloatValue) -> FloatingScalarValue {
    match value {
        ObservedBinaryFloatValue::Represented(value) => FloatingScalarValue::Represented(value),
        ObservedBinaryFloatValue::NaNClass => FloatingScalarValue::NaNClass,
    }
}

#[allow(dead_code)]
pub fn from_reference_observation(value: ObservedValue) -> ExecutionValue {
    match value {
        ObservedValue::Bool(value) => ExecutionValue::Bool(value),
        ObservedValue::I8(value) => ExecutionValue::I8(value),
        ObservedValue::I16(value) => ExecutionValue::I16(value),
        ObservedValue::I32(value) => ExecutionValue::I32(value),
        ObservedValue::I64(value) => ExecutionValue::I64(value),
        ObservedValue::U8(value) => ExecutionValue::U8(value),
        ObservedValue::U16(value) => ExecutionValue::U16(value),
        ObservedValue::U32(value) => ExecutionValue::U32(value),
        ObservedValue::U64(value) => ExecutionValue::U64(value),
        ObservedValue::F16(value) => ExecutionValue::F16(floating(value)),
        ObservedValue::F32(value) => ExecutionValue::F32(floating(value)),
        ObservedValue::F64(value) => ExecutionValue::F64(floating(value)),
        ObservedValue::Struct(fields) => ExecutionValue::Struct(
            fields.into_iter().map(from_reference_observation).collect(),
        ),
        ObservedValue::Function(_) | ObservedValue::TrackedFixture(_) => {
            panic!("oracle-only value cannot cross the Core-Wasm observation boundary")
        }
    }
}
