use wasm_encoder::{BlockType, Function as WasmFunction, Instruction, ValType};

use crate::RealizationError;
use crate::scalar::{FloatFormat, ScalarKind, mask};

/// Consume one private F16 `i64` carrier and produce its exact transient WebAssembly
/// `f64` value. The wider value is private realization machinery only.
pub(crate) fn emit_widen_carrier_to_f64(
    encoded: &mut WasmFunction,
    scratch: u32,
) -> Result<(), RealizationError> {
    let f16 = float_format(ScalarKind::F16)?;
    let f64 = float_format(ScalarKind::F64)?;
    let fraction_mask = mask(f16.fraction_bits());

    encoded.instruction(&Instruction::LocalSet(scratch));
    emit_raw_exponent(encoded, scratch, f16);
    encoded.instruction(&Instruction::I64Eqz);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::F64)));

    emit_fraction(encoded, scratch, f16);
    encoded.instruction(&Instruction::I64Eqz);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::F64)));
    emit_rebased_sign(encoded, scratch, f16, f64);
    encoded.instruction(&Instruction::F64ReinterpretI64);
    encoded.instruction(&Instruction::Else);

    // F16 subnormal: value = fraction * 2^(emin - (precision - 1)).
    // Normalize that exact dyadic value directly into F64 bits.
    emit_rebased_sign(encoded, scratch, f16, f64);
    emit_i64_const(
        encoded,
        nonnegative_i64(
            i64::from(f64.bias()) + i64::from(f16.emin()) - i64::from(f16.precision() - 1),
            "private F16-to-F64 subnormal exponent underflow",
        )?,
    );
    emit_fraction_floor_log2(encoded, scratch, f16);
    encoded.instruction(&Instruction::I64Add);
    emit_i64_const(encoded, u64::from(f64.fraction_bits()));
    encoded.instruction(&Instruction::I64Shl);
    encoded.instruction(&Instruction::I64Or);

    emit_fraction(encoded, scratch, f16);
    emit_i64_const(encoded, u64::from(f64.fraction_bits()));
    emit_fraction_floor_log2(encoded, scratch, f16);
    encoded.instruction(&Instruction::I64Sub);
    encoded.instruction(&Instruction::I64Shl);
    emit_i64_const(encoded, mask(f64.fraction_bits()));
    encoded.instruction(&Instruction::I64And);
    encoded.instruction(&Instruction::I64Or);
    encoded.instruction(&Instruction::F64ReinterpretI64);
    encoded.instruction(&Instruction::End);

    encoded.instruction(&Instruction::Else);
    emit_raw_exponent(encoded, scratch, f16);
    emit_i64_const(encoded, f16.exponent_mask());
    encoded.instruction(&Instruction::I64Eq);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::F64)));

    emit_fraction(encoded, scratch, f16);
    encoded.instruction(&Instruction::I64Eqz);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::F64)));
    emit_rebased_sign(encoded, scratch, f16, f64);
    emit_i64_const(encoded, f64.exponent_mask() << f64.fraction_bits());
    encoded.instruction(&Instruction::I64Or);
    encoded.instruction(&Instruction::F64ReinterpretI64);
    encoded.instruction(&Instruction::Else);
    emit_i64_const(encoded, f64.canonical_nan_residue());
    encoded.instruction(&Instruction::F64ReinterpretI64);
    encoded.instruction(&Instruction::End);

    encoded.instruction(&Instruction::Else);
    // F16 normal: rebias the exponent and left-align the explicit fraction.
    emit_rebased_sign(encoded, scratch, f16, f64);
    emit_raw_exponent(encoded, scratch, f16);
    emit_i64_const(
        encoded,
        nonnegative_i64(
            i64::from(f64.bias()) - i64::from(f16.bias()),
            "private F16-to-F64 exponent rebias underflow",
        )?,
    );
    encoded.instruction(&Instruction::I64Add);
    emit_i64_const(encoded, u64::from(f64.fraction_bits()));
    encoded.instruction(&Instruction::I64Shl);
    encoded.instruction(&Instruction::I64Or);
    emit_fraction(encoded, scratch, f16);
    emit_i64_const(
        encoded,
        u64::from(f64.fraction_bits() - f16.fraction_bits()),
    );
    encoded.instruction(&Instruction::I64Shl);
    encoded.instruction(&Instruction::I64Or);
    encoded.instruction(&Instruction::F64ReinterpretI64);
    encoded.instruction(&Instruction::End);
    encoded.instruction(&Instruction::End);

    debug_assert_eq!(fraction_mask, 0x03ff);
    Ok(())
}

/// Read the transient `f64` result local and produce the private F16 `i64` carrier
/// obtained by direct round-to-nearest, ties-to-even narrowing.
pub(crate) fn emit_narrow_f64_to_carrier(
    encoded: &mut WasmFunction,
    f64_scratch: u32,
    scratch: u32,
) -> Result<(), RealizationError> {
    let f16 = float_format(ScalarKind::F16)?;
    let f64 = float_format(ScalarKind::F64)?;

    encoded.instruction(&Instruction::LocalGet(f64_scratch));
    encoded.instruction(&Instruction::I64ReinterpretF64);
    encoded.instruction(&Instruction::LocalSet(scratch));

    emit_raw_exponent(encoded, scratch, f64);
    emit_i64_const(encoded, f64.exponent_mask());
    encoded.instruction(&Instruction::I64Eq);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));

    emit_fraction(encoded, scratch, f64);
    encoded.instruction(&Instruction::I64Eqz);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
    emit_signed_infinity(encoded, scratch, f16, f64);
    encoded.instruction(&Instruction::Else);
    emit_i64_const(encoded, f16.canonical_nan_residue());
    encoded.instruction(&Instruction::End);

    encoded.instruction(&Instruction::Else);
    emit_finite_f64_to_f16(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::End);
    Ok(())
}

fn emit_finite_f64_to_f16(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
) -> Result<(), RealizationError> {
    let upper_raw = nonnegative_i64(
        i64::from(f64.bias()) + i64::from(f16.emax()),
        "private F16 upper exponent underflow",
    )?;
    let normal_min_raw = nonnegative_i64(
        i64::from(f64.bias()) + i64::from(f16.emin()),
        "private F16 minimum-normal exponent underflow",
    )?;
    let underflow_midpoint_raw = nonnegative_i64(
        i64::from(f64.bias()) + i64::from(f16.emin()) - i64::from(f16.precision()),
        "private F16 underflow-midpoint exponent underflow",
    )?;

    emit_raw_exponent(encoded, scratch, f64);
    emit_i64_const(encoded, upper_raw);
    encoded.instruction(&Instruction::I64GtU);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
    emit_signed_infinity(encoded, scratch, f16, f64);
    encoded.instruction(&Instruction::Else);

    emit_raw_exponent(encoded, scratch, f64);
    emit_i64_const(encoded, normal_min_raw);
    encoded.instruction(&Instruction::I64GeU);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
    emit_normal_f16_carrier(encoded, scratch, f16, f64, upper_raw)?;
    encoded.instruction(&Instruction::Else);

    emit_raw_exponent(encoded, scratch, f64);
    emit_i64_const(encoded, underflow_midpoint_raw);
    encoded.instruction(&Instruction::I64LtU);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
    emit_rebased_sign(encoded, scratch, f64, f16);
    encoded.instruction(&Instruction::Else);
    emit_subnormal_f16_carrier(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::End);
    encoded.instruction(&Instruction::End);
    encoded.instruction(&Instruction::End);
    Ok(())
}

fn emit_normal_f16_carrier(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
    upper_raw: u64,
) -> Result<(), RealizationError> {
    let shift = f64
        .precision()
        .checked_sub(f16.precision())
        .ok_or_else(|| invariant("private F16 normal narrowing precision inversion"))?;
    emit_round_up_fixed(encoded, scratch, f64, shift);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
    emit_normal_magnitude(encoded, scratch, f16, f64, upper_raw, shift, 1)?;
    encoded.instruction(&Instruction::Else);
    emit_normal_magnitude(encoded, scratch, f16, f64, upper_raw, shift, 0)?;
    encoded.instruction(&Instruction::End);
    emit_rebased_sign(encoded, scratch, f64, f16);
    encoded.instruction(&Instruction::I64Or);
    Ok(())
}

fn emit_normal_magnitude(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
    upper_raw: u64,
    shift: u32,
    increment: u64,
) -> Result<(), RealizationError> {
    emit_rounded_significand(encoded, scratch, f64, shift, increment);
    emit_i64_const(encoded, 1_u64 << f16.precision());
    encoded.instruction(&Instruction::I64Eq);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));

    emit_raw_exponent(encoded, scratch, f64);
    emit_i64_const(encoded, upper_raw);
    encoded.instruction(&Instruction::I64Eq);
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
    emit_i64_const(encoded, f16.exponent_mask() << f16.fraction_bits());
    encoded.instruction(&Instruction::Else);
    emit_raw_exponent(encoded, scratch, f64);
    emit_i64_const(
        encoded,
        nonnegative_i64(
            i64::from(f64.bias()) - i64::from(f16.bias()) - 1,
            "private F16 carry exponent rebias underflow",
        )?,
    );
    encoded.instruction(&Instruction::I64Sub);
    emit_i64_const(encoded, u64::from(f16.fraction_bits()));
    encoded.instruction(&Instruction::I64Shl);
    encoded.instruction(&Instruction::End);

    encoded.instruction(&Instruction::Else);
    emit_raw_exponent(encoded, scratch, f64);
    emit_i64_const(
        encoded,
        nonnegative_i64(
            i64::from(f64.bias()) - i64::from(f16.bias()),
            "private F16 exponent rebias underflow",
        )?,
    );
    encoded.instruction(&Instruction::I64Sub);
    emit_i64_const(encoded, u64::from(f16.fraction_bits()));
    encoded.instruction(&Instruction::I64Shl);
    emit_rounded_significand(encoded, scratch, f64, shift, increment);
    emit_i64_const(encoded, 1_u64 << f16.fraction_bits());
    encoded.instruction(&Instruction::I64Sub);
    encoded.instruction(&Instruction::I64Or);
    encoded.instruction(&Instruction::End);
    Ok(())
}

fn emit_subnormal_f16_carrier(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
) -> Result<(), RealizationError> {
    emit_round_up_dynamic(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::If(BlockType::Result(ValType::I64)));
    emit_subnormal_retained(encoded, scratch, f16, f64)?;
    emit_i64_const(encoded, 1);
    encoded.instruction(&Instruction::I64Add);
    encoded.instruction(&Instruction::Else);
    emit_subnormal_retained(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::End);
    emit_rebased_sign(encoded, scratch, f64, f16);
    encoded.instruction(&Instruction::I64Or);
    Ok(())
}

fn emit_round_up_fixed(encoded: &mut WasmFunction, scratch: u32, format: FloatFormat, shift: u32) {
    emit_significand(encoded, scratch, format);
    emit_i64_const(encoded, mask(shift));
    encoded.instruction(&Instruction::I64And);
    emit_i64_const(encoded, 1_u64 << (shift - 1));
    encoded.instruction(&Instruction::I64GtU);

    emit_significand(encoded, scratch, format);
    emit_i64_const(encoded, mask(shift));
    encoded.instruction(&Instruction::I64And);
    emit_i64_const(encoded, 1_u64 << (shift - 1));
    encoded.instruction(&Instruction::I64Eq);
    emit_significand(encoded, scratch, format);
    emit_i64_const(encoded, u64::from(shift));
    encoded.instruction(&Instruction::I64ShrU);
    emit_i64_const(encoded, 1);
    encoded.instruction(&Instruction::I64And);
    emit_i64_const(encoded, 1);
    encoded.instruction(&Instruction::I64Eq);
    encoded.instruction(&Instruction::I32And);
    encoded.instruction(&Instruction::I32Or);
}

fn emit_round_up_dynamic(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
) -> Result<(), RealizationError> {
    emit_significand(encoded, scratch, f64);
    emit_dynamic_remainder_mask(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::I64And);
    emit_dynamic_half(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::I64GtU);

    emit_significand(encoded, scratch, f64);
    emit_dynamic_remainder_mask(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::I64And);
    emit_dynamic_half(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::I64Eq);
    emit_subnormal_retained(encoded, scratch, f16, f64)?;
    emit_i64_const(encoded, 1);
    encoded.instruction(&Instruction::I64And);
    emit_i64_const(encoded, 1);
    encoded.instruction(&Instruction::I64Eq);
    encoded.instruction(&Instruction::I32And);
    encoded.instruction(&Instruction::I32Or);
    Ok(())
}

fn emit_dynamic_remainder_mask(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
) -> Result<(), RealizationError> {
    emit_i64_const(encoded, 1);
    emit_subnormal_shift(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::I64Shl);
    emit_i64_const(encoded, 1);
    encoded.instruction(&Instruction::I64Sub);
    Ok(())
}

fn emit_dynamic_half(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
) -> Result<(), RealizationError> {
    emit_i64_const(encoded, 1);
    emit_subnormal_shift(encoded, scratch, f16, f64)?;
    emit_i64_const(encoded, 1);
    encoded.instruction(&Instruction::I64Sub);
    encoded.instruction(&Instruction::I64Shl);
    Ok(())
}

fn emit_subnormal_retained(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
) -> Result<(), RealizationError> {
    emit_significand(encoded, scratch, f64);
    emit_subnormal_shift(encoded, scratch, f16, f64)?;
    encoded.instruction(&Instruction::I64ShrU);
    Ok(())
}

fn emit_subnormal_shift(
    encoded: &mut WasmFunction,
    scratch: u32,
    f16: FloatFormat,
    f64: FloatFormat,
) -> Result<(), RealizationError> {
    let quantum_exponent = i64::from(f16.emin()) - i64::from(f16.precision() - 1);
    emit_i64_const(
        encoded,
        nonnegative_i64(
            i64::from(f64.fraction_bits()) + i64::from(f64.bias()) + quantum_exponent,
            "private F16 subnormal shift base underflow",
        )?,
    );
    emit_raw_exponent(encoded, scratch, f64);
    encoded.instruction(&Instruction::I64Sub);
    Ok(())
}

fn emit_rounded_significand(
    encoded: &mut WasmFunction,
    scratch: u32,
    format: FloatFormat,
    shift: u32,
    increment: u64,
) {
    emit_significand(encoded, scratch, format);
    emit_i64_const(encoded, u64::from(shift));
    encoded.instruction(&Instruction::I64ShrU);
    if increment != 0 {
        emit_i64_const(encoded, increment);
        encoded.instruction(&Instruction::I64Add);
    }
}

fn emit_significand(encoded: &mut WasmFunction, scratch: u32, format: FloatFormat) {
    emit_fraction(encoded, scratch, format);
    emit_i64_const(encoded, 1_u64 << format.fraction_bits());
    encoded.instruction(&Instruction::I64Or);
}

fn emit_fraction_floor_log2(encoded: &mut WasmFunction, scratch: u32, format: FloatFormat) {
    emit_i64_const(encoded, 63);
    emit_fraction(encoded, scratch, format);
    encoded.instruction(&Instruction::I64Clz);
    encoded.instruction(&Instruction::I64Sub);
}

fn emit_raw_exponent(encoded: &mut WasmFunction, scratch: u32, format: FloatFormat) {
    encoded.instruction(&Instruction::LocalGet(scratch));
    emit_i64_const(encoded, u64::from(format.fraction_bits()));
    encoded.instruction(&Instruction::I64ShrU);
    emit_i64_const(encoded, format.exponent_mask());
    encoded.instruction(&Instruction::I64And);
}

fn emit_fraction(encoded: &mut WasmFunction, scratch: u32, format: FloatFormat) {
    encoded.instruction(&Instruction::LocalGet(scratch));
    emit_i64_const(encoded, mask(format.fraction_bits()));
    encoded.instruction(&Instruction::I64And);
}

fn emit_rebased_sign(
    encoded: &mut WasmFunction,
    scratch: u32,
    source: FloatFormat,
    target: FloatFormat,
) {
    encoded.instruction(&Instruction::LocalGet(scratch));
    emit_i64_const(encoded, source.sign_mask());
    encoded.instruction(&Instruction::I64And);
    if target.width() > source.width() {
        emit_i64_const(encoded, u64::from(target.width() - source.width()));
        encoded.instruction(&Instruction::I64Shl);
    } else if source.width() > target.width() {
        emit_i64_const(encoded, u64::from(source.width() - target.width()));
        encoded.instruction(&Instruction::I64ShrU);
    }
}

fn emit_signed_infinity(
    encoded: &mut WasmFunction,
    scratch: u32,
    target: FloatFormat,
    source: FloatFormat,
) {
    emit_rebased_sign(encoded, scratch, source, target);
    emit_i64_const(encoded, target.exponent_mask() << target.fraction_bits());
    encoded.instruction(&Instruction::I64Or);
}

fn float_format(kind: ScalarKind) -> Result<FloatFormat, RealizationError> {
    kind.float_format().ok_or_else(|| {
        invariant("private F16 arithmetic requested metadata for a non-floating scalar")
    })
}

fn nonnegative_i64(value: i64, message: &str) -> Result<u64, RealizationError> {
    u64::try_from(value).map_err(|_| invariant(message))
}

fn emit_i64_const(encoded: &mut WasmFunction, residue: u64) {
    encoded.instruction(&Instruction::I64Const(i64::from_ne_bytes(
        residue.to_ne_bytes(),
    )));
}

fn invariant(message: impl Into<String>) -> RealizationError {
    RealizationError::BackendInvariant(message.into())
}
