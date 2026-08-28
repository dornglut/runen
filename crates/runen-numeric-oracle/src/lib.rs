#![forbid(unsafe_code)]
//! Verification-only executable oracle for accepted Runen numeric relations.
//!
//! This crate is not Runen source syntax, compiler IR, a runtime floating-point
//! implementation, a backend model, or a normative semantic owner. It covers
//! exact dyadic and exact binary-ratio inputs to the accepted binary floating
//! rounding relation plus class-level scalar integer/floating conversion and
//! sum-reduction evidence.
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
pub enum SumReductionResult {
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
    ZeroDenominator,
    NonFiniteReductionInput,
    InvalidRoundedValueFixture,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactBinaryRatio {
    sign: Sign,
    numerator: u128,
    denominator: u128,
    exponent: i32,
}

impl ExactBinaryRatio {
    #[must_use]
    pub const fn from_parts(sign: Sign, numerator: u128, denominator: u128, exponent: i32) -> Self {
        Self {
            sign,
            numerator,
            denominator,
            exponent,
        }
    }
}

pub fn round_binary_ratio(
    format: BinaryFormat,
    exact: ExactBinaryRatio,
) -> Result<RoundedBinaryValue, NumericOracleError> {
    if exact.numerator == 0 {
        return Err(NumericOracleError::ZeroExactInput);
    }
    if exact.denominator == 0 {
        return Err(NumericOracleError::ZeroDenominator);
    }

    let ratio_exponent = ratio_floor_log2(exact.numerator, exact.denominator)?;
    let mut value_exponent = ratio_exponent
        .checked_add(exact.exponent)
        .ok_or(NumericOracleError::ExponentArithmeticOverflow)?;

    if value_exponent > format.emax {
        return Ok(RoundedBinaryValue::Infinity(exact.sign));
    }

    if value_exponent < format.emin {
        let quantum_exponent = format.q_exponent()?;
        let shift = value_exponent
            .checked_sub(quantum_exponent)
            .ok_or(NumericOracleError::ExponentArithmeticOverflow)?;
        if shift < -1 {
            return Ok(RoundedBinaryValue::Zero(exact.sign));
        }

        let (numerator, denominator) =
            normalize_binary_ratio(exact.numerator, exact.denominator, ratio_exponent)?;
        let significand = if shift == -1 {
            if numerator == denominator { 0 } else { 1 }
        } else {
            round_normalized_ratio_to_integer(
                numerator,
                denominator,
                u32::try_from(shift).map_err(|_| NumericOracleError::InternalRangeExceeded)?,
            )?
        };

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

    let (numerator, denominator) =
        normalize_binary_ratio(exact.numerator, exact.denominator, ratio_exponent)?;
    let mut significand =
        round_normalized_ratio_to_integer(numerator, denominator, format.precision - 1)?;
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

fn ratio_floor_log2(numerator: u128, denominator: u128) -> Result<i32, NumericOracleError> {
    debug_assert_ne!(numerator, 0);
    debug_assert_ne!(denominator, 0);

    let candidate = numerator.ilog2() as i32 - denominator.ilog2() as i32;
    let reaches_candidate = if candidate >= 0 {
        numerator >= checked_scale_u128(denominator, candidate as u32)?
    } else {
        checked_scale_u128(numerator, candidate.unsigned_abs())? >= denominator
    };

    Ok(if reaches_candidate {
        candidate
    } else {
        candidate - 1
    })
}

fn normalize_binary_ratio(
    numerator: u128,
    denominator: u128,
    ratio_exponent: i32,
) -> Result<(u128, u128), NumericOracleError> {
    let (numerator, denominator) = if ratio_exponent >= 0 {
        (
            numerator,
            checked_scale_u128(denominator, ratio_exponent as u32)?,
        )
    } else {
        (
            checked_scale_u128(numerator, ratio_exponent.unsigned_abs())?,
            denominator,
        )
    };

    if numerator < denominator || numerator - denominator >= denominator {
        return Err(NumericOracleError::InternalRangeExceeded);
    }

    Ok((numerator, denominator))
}

fn round_normalized_ratio_to_integer(
    numerator: u128,
    denominator: u128,
    fraction_bits: u32,
) -> Result<u128, NumericOracleError> {
    debug_assert!(numerator >= denominator);
    debug_assert!(numerator - denominator < denominator);

    let mut significand = 1_u128;
    let mut remainder = numerator - denominator;

    for _ in 0..fraction_bits {
        significand = significand
            .checked_mul(2)
            .ok_or(NumericOracleError::InternalRangeExceeded)?;
        let complement = denominator - remainder;
        if remainder >= complement {
            significand |= 1;
            remainder -= complement;
        } else {
            remainder = remainder
                .checked_add(remainder)
                .ok_or(NumericOracleError::InternalRangeExceeded)?;
        }
    }

    let complement = denominator - remainder;
    if remainder > complement || (remainder == complement && (significand & 1) == 1) {
        significand = significand
            .checked_add(1)
            .ok_or(NumericOracleError::InternalRangeExceeded)?;
    }

    Ok(significand)
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

/// Verification-only oracle for the accepted same-format unordered floating
/// sum reduction including its signed-infinity and submitted-NaN extensions.
///
/// Finite-only inputs delegate to the accepted bounded exact finite-sum oracle
/// below. Special-value-determined results avoid unnecessary bounded finite
/// accumulation after all fixture values have been structurally validated.
pub fn reduce_sum(
    format: BinaryFormat,
    contributions: &[BinaryValueFixture],
) -> Result<SumReductionResult, NumericOracleError> {
    let mut positive_infinity = false;
    let mut negative_infinity = false;
    let mut nan_present = false;

    for contribution in contributions {
        match contribution {
            BinaryValueFixture::Finite(exact) if exact.magnitude == 0 => {
                return Err(NumericOracleError::ZeroExactInput);
            }
            BinaryValueFixture::Finite(_) | BinaryValueFixture::Zero(_) => {}
            BinaryValueFixture::Infinity(Sign::Positive) => positive_infinity = true,
            BinaryValueFixture::Infinity(Sign::Negative) => negative_infinity = true,
            BinaryValueFixture::NaNClass => nan_present = true,
        }
    }

    if nan_present {
        return Ok(SumReductionResult::NaNClass);
    }
    if positive_infinity && negative_infinity {
        return Ok(SumReductionResult::NaNClass);
    }
    if positive_infinity {
        return Ok(SumReductionResult::Value(RoundedBinaryValue::Infinity(
            Sign::Positive,
        )));
    }
    if negative_infinity {
        return Ok(SumReductionResult::Value(RoundedBinaryValue::Infinity(
            Sign::Negative,
        )));
    }

    reduce_finite_sum(format, contributions).map(SumReductionResult::Value)
}

/// Evaluate one verification-only tree-rounded internal addition using the
/// baseline `standard` addition relation. This follows the `fast` unordered-sum
/// tree-candidate rule; it does not model a source-visible tree or Exec combine.
pub fn add_standard_tree_node(
    format: BinaryFormat,
    left: SumReductionResult,
    right: SumReductionResult,
) -> Result<SumReductionResult, NumericOracleError> {
    let (SumReductionResult::Value(left), SumReductionResult::Value(right)) = (left, right) else {
        return Ok(SumReductionResult::NaNClass);
    };

    let left = rounded_value_to_fixture(format, left)?;
    let right = rounded_value_to_fixture(format, right)?;
    reduce_sum(format, &[left, right])
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

fn rounded_value_to_fixture(
    format: BinaryFormat,
    value: RoundedBinaryValue,
) -> Result<BinaryValueFixture, NumericOracleError> {
    let normal_minimum = 1_u128 << (format.precision - 1);
    let carry = 1_u128 << format.precision;

    Ok(match value {
        RoundedBinaryValue::Zero(sign) => BinaryValueFixture::Zero(sign),
        RoundedBinaryValue::Infinity(sign) => BinaryValueFixture::Infinity(sign),
        RoundedBinaryValue::Subnormal { sign, significand }
            if (1..normal_minimum).contains(&significand) =>
        {
            BinaryValueFixture::Finite(ExactDyadic::from_parts(
                sign,
                significand,
                format.q_exponent()?,
            ))
        }
        RoundedBinaryValue::Normal {
            sign,
            significand,
            exponent,
        } if (normal_minimum..carry).contains(&significand)
            && (format.emin..=format.emax).contains(&exponent) =>
        {
            let precision_tail = (format.precision - 1) as i32;
            let exact_exponent = exponent
                .checked_sub(precision_tail)
                .ok_or(NumericOracleError::ExponentArithmeticOverflow)?;
            BinaryValueFixture::Finite(ExactDyadic::from_parts(sign, significand, exact_exponent))
        }
        _ => return Err(NumericOracleError::InvalidRoundedValueFixture),
    })
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
