use runen_numeric_oracle::{
    BinaryFormat, ExactDyadic, NumericOracleError, RoundedBinaryValue, Sign,
    convert_signed_integer, convert_unsigned_integer, round_dyadic,
};

fn tiny_format() -> BinaryFormat {
    BinaryFormat::new(4, -2, 2).expect("valid test format")
}

#[test]
fn validates_fixture_capacity_without_claiming_normative_limits() {
    assert_eq!(
        BinaryFormat::new(1, -2, 2),
        Err(NumericOracleError::PrecisionTooSmall)
    );
    assert_eq!(
        BinaryFormat::new(128, -2, 2),
        Err(NumericOracleError::PrecisionTooLarge)
    );
    assert_eq!(
        BinaryFormat::new(4, 3, 2),
        Err(NumericOracleError::InvalidExponentRange)
    );
}

#[test]
fn generic_rounding_rejects_exact_zero() {
    assert_eq!(
        round_dyadic(tiny_format(), ExactDyadic::from_parts(Sign::Positive, 0, 0)),
        Err(NumericOracleError::ZeroExactInput)
    );
}

#[test]
fn preserves_exact_normal_values() {
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Positive, 3, -1)
        ),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 12,
            exponent: 0,
        })
    );
}

#[test]
fn interior_halfway_chooses_even_canonical_significand() {
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Positive, 17, -4)
        ),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 8,
            exponent: 0,
        })
    );
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Positive, 19, -4)
        ),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 10,
            exponent: 0,
        })
    );
}

#[test]
fn lower_boundary_halfway_selects_same_sign_zero() {
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Positive, 1, -6)
        ),
        Ok(RoundedBinaryValue::Zero(Sign::Positive))
    );
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Negative, 1, -6)
        ),
        Ok(RoundedBinaryValue::Zero(Sign::Negative))
    );
}

#[test]
fn rounds_into_subnormal_range() {
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Positive, 1, -5)
        ),
        Ok(RoundedBinaryValue::Subnormal {
            sign: Sign::Positive,
            significand: 1,
        })
    );
}

#[test]
fn subnormal_normal_halfway_selects_even_normal_candidate() {
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Positive, 15, -6)
        ),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 8,
            exponent: -2,
        })
    );
}

#[test]
fn upper_boundary_uses_max_finite_below_halfway_and_infinity_at_halfway() {
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Positive, 123, -4)
        ),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 15,
            exponent: 2,
        })
    );
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Positive, 31, -2)
        ),
        Ok(RoundedBinaryValue::Infinity(Sign::Positive))
    );
    assert_eq!(
        round_dyadic(
            tiny_format(),
            ExactDyadic::from_parts(Sign::Negative, 31, -2)
        ),
        Ok(RoundedBinaryValue::Infinity(Sign::Negative))
    );
}

#[test]
fn integer_conversion_zero_is_positive_zero_and_halfway_ties_are_even() {
    let format = BinaryFormat::new(4, -2, 5).expect("valid test format");

    assert_eq!(
        convert_signed_integer(format, 0),
        Ok(RoundedBinaryValue::Zero(Sign::Positive))
    );
    assert_eq!(
        convert_unsigned_integer(format, 0),
        Ok(RoundedBinaryValue::Zero(Sign::Positive))
    );
    assert_eq!(
        convert_unsigned_integer(format, 17),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 8,
            exponent: 4,
        })
    );
    assert_eq!(
        convert_signed_integer(format, -17),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Negative,
            significand: 8,
            exponent: 4,
        })
    );
}
