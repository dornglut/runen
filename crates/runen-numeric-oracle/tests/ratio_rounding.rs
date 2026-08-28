use runen_numeric_oracle::{
    BinaryFormat, ExactBinaryRatio, ExactDyadic, NumericOracleError, RoundedBinaryValue, Sign,
    round_binary_ratio, round_dyadic,
};

fn ratio(sign: Sign, numerator: u128, denominator: u128, exponent: i32) -> ExactBinaryRatio {
    ExactBinaryRatio::from_parts(sign, numerator, denominator, exponent)
}

fn normal(sign: Sign, significand: u128, exponent: i32) -> RoundedBinaryValue {
    RoundedBinaryValue::Normal {
        sign,
        significand,
        exponent,
    }
}

fn subnormal(sign: Sign, significand: u128) -> RoundedBinaryValue {
    RoundedBinaryValue::Subnormal { sign, significand }
}

#[test]
fn exact_binary_ratios_agree_with_dyadic_rounding_when_the_quotient_is_dyadic() {
    let format = BinaryFormat::new(11, -14, 15).unwrap();

    for (ratio_input, dyadic_input) in [
        (
            ratio(Sign::Positive, 3, 2, 0),
            ExactDyadic::from_parts(Sign::Positive, 3, -1),
        ),
        (
            ratio(Sign::Positive, 5, 2, 0),
            ExactDyadic::from_parts(Sign::Positive, 5, -1),
        ),
        (
            ratio(Sign::Negative, 5, 4, 2),
            ExactDyadic::from_parts(Sign::Negative, 5, 0),
        ),
        (
            ratio(Sign::Positive, 6, 3, 0),
            ExactDyadic::from_parts(Sign::Positive, 1, 1),
        ),
    ] {
        assert_eq!(
            round_binary_ratio(format, ratio_input),
            round_dyadic(format, dyadic_input)
        );
    }
}

#[test]
fn recurring_one_third_rounds_exactly_in_all_represented_formats_and_signs() {
    for (format, expected_significand) in [
        (BinaryFormat::new(11, -14, 15).unwrap(), 1365_u128),
        (BinaryFormat::new(24, -126, 127).unwrap(), 11_184_811_u128),
        (
            BinaryFormat::new(53, -1022, 1023).unwrap(),
            6_004_799_503_160_661_u128,
        ),
    ] {
        for sign in [Sign::Positive, Sign::Negative] {
            assert_eq!(
                round_binary_ratio(format, ratio(sign, 1, 3, 0)),
                Ok(normal(sign, expected_significand, -2))
            );
        }
    }
}

#[test]
fn nearest_ties_to_even_distinguishes_below_half_both_tie_parities_and_above_half() {
    let format = BinaryFormat::new(3, -2, 2).unwrap();

    for (input, expected_significand) in [
        (ratio(Sign::Positive, 21, 16, 0), 5_u128),
        (ratio(Sign::Positive, 9, 8, 0), 4_u128),
        (ratio(Sign::Positive, 11, 8, 0), 6_u128),
        (ratio(Sign::Positive, 23, 16, 0), 6_u128),
    ] {
        assert_eq!(
            round_binary_ratio(format, input),
            Ok(normal(Sign::Positive, expected_significand, 0))
        );
    }
}

#[test]
fn lower_normal_subnormal_midpoint_carries_to_minimum_normal() {
    let format = BinaryFormat::new(3, -2, 2).unwrap();

    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 3, 1, -4)),
        Ok(subnormal(Sign::Positive, 3))
    );
    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 7, 1, -5)),
        Ok(normal(Sign::Positive, 4, -2))
    );
}

#[test]
fn subnormal_halfway_underflow_rounds_to_signed_zero_and_above_half_rounds_up() {
    let format = BinaryFormat::new(3, -2, 2).unwrap();

    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Negative, 1, 1, -5)),
        Ok(RoundedBinaryValue::Zero(Sign::Negative))
    );
    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 3, 1, -6)),
        Ok(subnormal(Sign::Positive, 1))
    );
}

#[test]
fn upper_rounding_boundary_selects_maximum_finite_then_infinity_at_midpoint() {
    let format = BinaryFormat::new(3, -2, 2).unwrap();

    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 29, 4, 0)),
        Ok(normal(Sign::Positive, 7, 2))
    );
    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 15, 2, 0)),
        Ok(RoundedBinaryValue::Infinity(Sign::Positive))
    );
}

#[test]
fn full_u128_normalization_does_not_materialize_an_out_of_range_leading_bit() {
    let format = BinaryFormat::new(3, -2, 2).unwrap();
    let denominator = (1_u128 << 127) + 1;

    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 1, denominator, 128)),
        Ok(normal(Sign::Positive, 4, 1))
    );
}

#[test]
fn extreme_exponents_select_zero_and_infinity_without_host_floating_arithmetic() {
    let format = BinaryFormat::new(53, -1022, 1023).unwrap();

    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 1, 3, -2000)),
        Ok(RoundedBinaryValue::Zero(Sign::Positive))
    );
    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Negative, 1, 3, 2000)),
        Ok(RoundedBinaryValue::Infinity(Sign::Negative))
    );
}

#[test]
fn invalid_zero_numerator_and_denominator_are_distinct() {
    let format = BinaryFormat::new(24, -126, 127).unwrap();

    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 0, 1, 0)),
        Err(NumericOracleError::ZeroExactInput)
    );
    assert_eq!(
        round_binary_ratio(format, ratio(Sign::Positive, 1, 0, 0)),
        Err(NumericOracleError::ZeroDenominator)
    );
}
