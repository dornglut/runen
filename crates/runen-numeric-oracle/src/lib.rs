#![forbid(unsafe_code)]
//! Verification-only executable oracle for accepted Runen numeric relations.
//!
//! This crate is not Runen source syntax, compiler IR, a runtime floating-point
//! implementation, a backend model, or a normative semantic owner. It covers
//! exact dyadic inputs to the accepted binary floating rounding relation plus
//! class-level scalar integer/floating conversion and finite sum-reduction evidence.
//!
//! The `i128`/`u128` carriers, `i32` exponents, integer widths up to 128 bits,
//! and exact finite-sum accumulator capacity are executable fixture limits only.
//! They do not define Runen integer widths, floating exponent limits, or a
//! physical reduction-accumulator representation.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sign {
    Positive,
    Negative,
}

impl Sign {
    const fn from_negative(negative: bool) -> Self {
        if negative {
            Self::Negative
        } else {
            Self::Positive
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundedBinaryValue {
    Zero(Sign),
    Subnormal {
        sign: Sign,
        significand: u128,
    },
    Normal {
        sign: Sign,
        significand: u128,
        exponent: i32,
    },
    Infinity(Sign),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryValueFixture {
    Finite(ExactDyadic),
    Zero(Sign),
    Infinity(Sign),
    NaNClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryConversionResult {
    Value(RoundedBinaryValue),
    NaNClass,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerSignedness {
    Signed,
    Unsigned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerConversionResult {
    Signed(i128),
    Unsigned(u128),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericOracleError {
    PrecisionTooSmall,
    PrecisionTooLarge,
    InvalidExponentRange,
    IntegerWidthZero,
    IntegerWidthTooLarge,
    ExponentArithmeticOverflow,
    InternalRangeExceeded,
    ZeroExactInput,
    NonFiniteReductionInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryFormat {
    precision: u32,
    emin: i32,
    emax: i32,
}

impl BinaryFormat {
    pub fn new(precision: u32, emin: i32, emax: i32) -> Result<Self, NumericOracleError> {
        if precision < 2 {
            return Err(NumericOracleError::PrecisionTooSmall);
        }

        // Runen does not have this bound. This verification fixture uses u128
        // significands, so its executable capacity does.
        if precision > 127 {
            return Err(NumericOracleError::PrecisionTooLarge);
        }
        if emin > emax {
            return Err(NumericOracleError::InvalidExponentRange);
        }

        let precision_tail = (precision - 1) as i32;
        emin.checked_sub(precision_tail)
            .ok_or(NumericOracleError::ExponentArithmeticOverflow)?;

        Ok(Self {
            precision,
            emin,
            emax,
        })
    }

    fn q_exponent(self) -> Result<i32, NumericOracleError> {
        let precision_tail = (self.precision - 1) as i32;
        self.emin
            .checked_sub(precision_tail)
            .ok_or(NumericOracleError::ExponentArithmeticOverflow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegerFormat {
    width: u32,
    signedness: IntegerSignedness,
}

impl IntegerFormat {
    pub fn new(width: u32, signedness: IntegerSignedness) -> Result<Self, NumericOracleError> {
        if width == 0 {
            return Err(NumericOracleError::IntegerWidthZero);
        }
        if width > 128 {
            return Err(NumericOracleError::IntegerWidthTooLarge);
        }

        Ok(Self { width, signedness })
    }

    fn signed_bounds(self) -> (i128, i128) {
        if self.width == 128 {
            return (i128::MIN, i128::MAX);
        }

        let magnitude = 1_i128 << (self.width - 1);
        (-magnitude, magnitude - 1)
    }

    fn signed_minimum_magnitude(self) -> u128 {
        1_u128 << (self.width - 1)
    }

    fn unsigned_maximum(self) -> u128 {
        if self.width == 128 {
            u128::MAX
        } else {
            (1_u128 << self.width) - 1
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactDyadic {
    sign: Sign,
    magnitude: u128,
    exponent: i32,
}

impl ExactDyadic {
    #[must_use]
    pub const fn from_parts(sign: Sign, magnitude: u128, exponent: i32) -> Self {
        Self {
            sign,
            magnitude,
            exponent,
        }
    }

    #[must_use]
    pub fn from_signed_integer(value: i128) -> Self {
        Self {
            sign: Sign::from_negative(value.is_negative()),
            magnitude: value.unsigned_abs(),
            exponent: 0,
        }
    }

    #[must_use]
    pub const fn from_unsigned_integer(value: u128) -> Self {
        Self {
            sign: Sign::Positive,
            magnitude: value,
            exponent: 0,
        }
    }
}

pub fn round_dyadic(
    format: BinaryFormat,
    exact: ExactDyadic,
) -> Result<RoundedBinaryValue, NumericOracleError> {
    if exact.magnitude == 0 {
        return Err(NumericOracleError::ZeroExactInput);
    }

    // A nonzero u128 magnitude has floor(log2) in [0, 127], so this cast is
    // within the fixture representation capacity.
    let magnitude_floor_log2 = exact.magnitude.ilog2() as i32;
    let mut value_exponent = magnitude_floor_log2
        .checked_add(exact.exponent)
        .ok_or(NumericOracleError::ExponentArithmeticOverflow)?;

    if value_exponent < format.emin {
        let significand =
            round_to_integer_grid(exact.magnitude, exact.exponent, format.q_exponent()?)?;
        if significand == 0 {
            return Ok(RoundedBinaryValue::Zero(exact.sign));
        }

        let normal_threshold = 1_u128 << (format.precision - 1);
        if significand < normal_threshold {
            return Ok(RoundedBinaryValue::Subnormal {
                sign: exact.sign,
                significand,
            });
        }
        if significand == normal_threshold {
            return Ok(RoundedBinaryValue::Normal {
                sign: exact.sign,
                significand,
                exponent: format.emin,
            });
        }
        return Err(NumericOracleError::InternalRangeExceeded);
    }

    if value_exponent > format.emax {
        return Ok(RoundedBinaryValue::Infinity(exact.sign));
    }

    let precision_tail = (format.precision - 1) as i32;
    let unit_exponent = value_exponent
        .checked_sub(precision_tail)
        .ok_or(NumericOracleError::ExponentArithmeticOverflow)?;
    let mut significand = round_to_integer_grid(exact.magnitude, exact.exponent, unit_exponent)?;
    let carry = 1_u128 << format.precision;

    if significand == carry {
        significand >>= 1;
        value_exponent = value_exponent
            .checked_add(1)
            .ok_or(NumericOracleError::ExponentArithmeticOverflow)?;
    }

    if value_exponent > format.emax {
        return Ok(RoundedBinaryValue::Infinity(exact.sign));
    }

    let normal_minimum = 1_u128 << (format.precision - 1);
    if !(normal_minimum..carry).contains(&significand) {
        return Err(NumericOracleError::InternalRangeExceeded);
    }

    Ok(RoundedBinaryValue::Normal {
        sign: exact.sign,
        significand,
        exponent: value_exponent,
    })
}

pub fn convert_signed_integer(
    format: BinaryFormat,
    value: i128,
) -> Result<RoundedBinaryValue, NumericOracleError> {
    if value == 0 {
        return Ok(RoundedBinaryValue::Zero(Sign::Positive));
    }
    round_dyadic(format, ExactDyadic::from_signed_integer(value))
}

pub fn convert_unsigned_integer(
    format: BinaryFormat,
    value: u128,
) -> Result<RoundedBinaryValue, NumericOracleError> {
    if value == 0 {
        return Ok(RoundedBinaryValue::Zero(Sign::Positive));
    }
    round_dyadic(format, ExactDyadic::from_unsigned_integer(value))
}

/// Verification-only oracle for the accepted finite same-format unordered
/// floating sum reduction.
///
/// The exact mathematical state is represented with a bounded common dyadic
/// scale and separate checked positive/negative u128 magnitudes. Capacity errors
/// are fixture limitations, not Runen semantic reduction limits.
pub fn reduce_finite_sum(
    format: BinaryFormat,
    contributions: &[BinaryValueFixture],
) -> Result<RoundedBinaryValue, NumericOracleError> {
    let mut minimum_exponent: Option<i32> = None;
    let mut empty = true;
    let mut negative_zero_only = true;

    for contribution in contributions {
        match contribution {
            BinaryValueFixture::Finite(exact) => {
                if exact.magnitude == 0 {
                    return Err(NumericOracleError::ZeroExactInput);
                }
                empty = false;
                negative_zero_only = false;
                minimum_exponent = Some(match minimum_exponent {
                    Some(current) => current.min(exact.exponent),
                    None => exact.exponent,
                });
            }
            BinaryValueFixture::Zero(sign) => {
                empty = false;
                if *sign != Sign::Negative {
                    negative_zero_only = false;
                }
            }
            BinaryValueFixture::Infinity(_) | BinaryValueFixture::NaNClass => {
                return Err(NumericOracleError::NonFiniteReductionInput);
            }
        }
    }

    let Some(scale_exponent) = minimum_exponent else {
        return Ok(RoundedBinaryValue::Zero(if !empty && negative_zero_only {
            Sign::Negative
        } else {
            Sign::Positive
        }));
    };

    let mut positive_magnitude = 0_u128;
    let mut negative_magnitude = 0_u128;

    for contribution in contributions {
        let BinaryValueFixture::Finite(exact) = contribution else {
            continue;
        };

        let exponent_distance = i64::from(exact.exponent) - i64::from(scale_exponent);
        let shift = u32::try_from(exponent_distance)
            .map_err(|_| NumericOracleError::InternalRangeExceeded)?;
        let scaled = checked_scale_u128(exact.magnitude, shift)?;

        let accumulator = match exact.sign {
            Sign::Positive => &mut positive_magnitude,
            Sign::Negative => &mut negative_magnitude,
        };
        *accumulator = accumulator
            .checked_add(scaled)
            .ok_or(NumericOracleError::InternalRangeExceeded)?;
    }

    match positive_magnitude.cmp(&negative_magnitude) {
        std::cmp::Ordering::Equal => Ok(RoundedBinaryValue::Zero(Sign::Positive)),
        std::cmp::Ordering::Greater => round_dyadic(
            format,
            ExactDyadic::from_parts(
                Sign::Positive,
                positive_magnitude - negative_magnitude,
                scale_exponent,
            ),
        ),
        std::cmp::Ordering::Less => round_dyadic(
            format,
            ExactDyadic::from_parts(
                Sign::Negative,
                negative_magnitude - positive_magnitude,
                scale_exponent,
            ),
        ),
    }
}

pub fn convert_binary_to_binary(
    destination: BinaryFormat,
    source: BinaryValueFixture,
) -> Result<BinaryConversionResult, NumericOracleError> {
    Ok(match source {
        BinaryValueFixture::Finite(exact) => {
            BinaryConversionResult::Value(round_dyadic(destination, exact)?)
        }
        BinaryValueFixture::Zero(sign) => {
            BinaryConversionResult::Value(RoundedBinaryValue::Zero(sign))
        }
        BinaryValueFixture::Infinity(sign) => {
            BinaryConversionResult::Value(RoundedBinaryValue::Infinity(sign))
        }
        BinaryValueFixture::NaNClass => BinaryConversionResult::NaNClass,
    })
}

pub fn convert_binary_to_integer(
    destination: IntegerFormat,
    source: BinaryValueFixture,
) -> Result<IntegerConversionResult, NumericOracleError> {
    match destination.signedness {
        IntegerSignedness::Signed => convert_binary_to_signed(destination, source),
        IntegerSignedness::Unsigned => convert_binary_to_unsigned(destination, source),
    }
}

fn convert_binary_to_signed(
    destination: IntegerFormat,
    source: BinaryValueFixture,
) -> Result<IntegerConversionResult, NumericOracleError> {
    let (minimum, maximum) = destination.signed_bounds();
    let value = match source {
        BinaryValueFixture::Zero(_) | BinaryValueFixture::NaNClass => 0,
        BinaryValueFixture::Infinity(Sign::Positive) => maximum,
        BinaryValueFixture::Infinity(Sign::Negative) => minimum,
        BinaryValueFixture::Finite(exact) => {
            let truncated = truncate_dyadic_magnitude(exact)?;
            match (exact.sign, truncated) {
                (Sign::Positive, TruncatedMagnitude::AboveU128) => maximum,
                (Sign::Positive, TruncatedMagnitude::Exact(magnitude)) => {
                    if magnitude > maximum as u128 {
                        maximum
                    } else {
                        magnitude as i128
                    }
                }
                (Sign::Negative, TruncatedMagnitude::AboveU128) => minimum,
                (Sign::Negative, TruncatedMagnitude::Exact(magnitude)) => {
                    if magnitude >= destination.signed_minimum_magnitude() {
                        minimum
                    } else {
                        -(magnitude as i128)
                    }
                }
            }
        }
    };

    Ok(IntegerConversionResult::Signed(value))
}

fn convert_binary_to_unsigned(
    destination: IntegerFormat,
    source: BinaryValueFixture,
) -> Result<IntegerConversionResult, NumericOracleError> {
    let maximum = destination.unsigned_maximum();
    let value = match source {
        BinaryValueFixture::Zero(_)
        | BinaryValueFixture::NaNClass
        | BinaryValueFixture::Infinity(Sign::Negative) => 0,
        BinaryValueFixture::Infinity(Sign::Positive) => maximum,
        BinaryValueFixture::Finite(exact) if exact.sign == Sign::Negative => {
            if exact.magnitude == 0 {
                return Err(NumericOracleError::ZeroExactInput);
            }
            0
        }
        BinaryValueFixture::Finite(exact) => match truncate_dyadic_magnitude(exact)? {
            TruncatedMagnitude::AboveU128 => maximum,
            TruncatedMagnitude::Exact(magnitude) => magnitude.min(maximum),
        },
    };

    Ok(IntegerConversionResult::Unsigned(value))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TruncatedMagnitude {
    Exact(u128),
    AboveU128,
}

fn truncate_dyadic_magnitude(exact: ExactDyadic) -> Result<TruncatedMagnitude, NumericOracleError> {
    if exact.magnitude == 0 {
        return Err(NumericOracleError::ZeroExactInput);
    }

    if exact.exponent >= 0 {
        let shift = exact.exponent as u32;
        if shift >= u128::BITS || exact.magnitude > (u128::MAX >> shift) {
            return Ok(TruncatedMagnitude::AboveU128);
        }
        return Ok(TruncatedMagnitude::Exact(exact.magnitude << shift));
    }

    let distance = exact.exponent.unsigned_abs();
    if distance >= u128::BITS {
        return Ok(TruncatedMagnitude::Exact(0));
    }

    Ok(TruncatedMagnitude::Exact(exact.magnitude >> distance))
}

fn checked_scale_u128(magnitude: u128, shift: u32) -> Result<u128, NumericOracleError> {
    if shift >= u128::BITS || magnitude > (u128::MAX >> shift) {
        return Err(NumericOracleError::InternalRangeExceeded);
    }
    Ok(magnitude << shift)
}

fn round_to_integer_grid(
    magnitude: u128,
    exact_exponent: i32,
    grid_exponent: i32,
) -> Result<u128, NumericOracleError> {
    let shift = exact_exponent
        .checked_sub(grid_exponent)
        .ok_or(NumericOracleError::ExponentArithmeticOverflow)?;

    if shift >= 0 {
        let shift = shift as u32;
        if shift >= u128::BITS || magnitude > (u128::MAX >> shift) {
            return Err(NumericOracleError::InternalRangeExceeded);
        }
        return Ok(magnitude << shift);
    }

    let distance = shift.unsigned_abs();
    if distance > 128 {
        return Ok(0);
    }
    if distance == 128 {
        let half = 1_u128 << 127;
        return Ok(if magnitude > half { 1 } else { 0 });
    }

    let quotient = magnitude >> distance;
    let mask = (1_u128 << distance) - 1;
    let remainder = magnitude & mask;
    let half = 1_u128 << (distance - 1);

    Ok(match remainder.cmp(&half) {
        std::cmp::Ordering::Less => quotient,
        std::cmp::Ordering::Greater => quotient + 1,
        std::cmp::Ordering::Equal if (quotient & 1) == 0 => quotient,
        std::cmp::Ordering::Equal => quotient + 1,
    })
}
