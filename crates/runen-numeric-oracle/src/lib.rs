#![forbid(unsafe_code)]
//! Verification-only executable oracle for accepted Runen numeric relations.
//!
//! This crate is not Runen source syntax, compiler IR, a runtime floating-point
//! implementation, a backend model, or a normative semantic owner. The first
//! slice covers only exact dyadic inputs to the accepted binary floating rounding
//! relation and semantic integer-to-floating conversion.
//!
//! The `i128`/`u128` carriers and `i32` exponents are executable fixture capacity
//! only. They do not define Runen integer widths or floating exponent limits.

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
pub enum NumericOracleError {
    PrecisionTooSmall,
    PrecisionTooLarge,
    InvalidExponentRange,
    ExponentArithmeticOverflow,
    InternalRangeExceeded,
    ZeroExactInput,
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
