use runen_numeric_oracle::{
    BinaryFormat, BinaryValueFixture, ExactDyadic, NumericOracleError, RoundedBinaryValue, Sign,
    SumReductionResult, reduce_finite_sum, reduce_sum,
};

fn tiny_format() -> BinaryFormat {
    BinaryFormat::new(4, -2, 5).expect("valid test format")
}

fn finite(sign: Sign, magnitude: u128, exponent: i32) -> BinaryValueFixture {
    BinaryValueFixture::Finite(ExactDyadic::from_parts(sign, magnitude, exponent))
}

fn normal_as_finite_fixture(value: RoundedBinaryValue) -> BinaryValueFixture {
    match value {
        RoundedBinaryValue::Normal {
            sign,
            significand,
            exponent,
        } => finite(sign, significand, exponent - 3),
        other => panic!("expected normal tiny-format value, got {other:?}"),
    }
}

#[test]
fn empty_and_signed_zero_sums_have_explicit_zero_signs() {
    assert_eq!(
        reduce_finite_sum(tiny_format(), &[]),
        Ok(RoundedBinaryValue::Zero(Sign::Positive))
    );
    assert_eq!(
        reduce_finite_sum(
            tiny_format(),
            &[
                BinaryValueFixture::Zero(Sign::Negative),
                BinaryValueFixture::Zero(Sign::Negative),
            ],
        ),
        Ok(RoundedBinaryValue::Zero(Sign::Negative))
    );
    assert_eq!(
        reduce_finite_sum(
            tiny_format(),
            &[
                BinaryValueFixture::Zero(Sign::Negative),
                BinaryValueFixture::Zero(Sign::Positive),
            ],
        ),
        Ok(RoundedBinaryValue::Zero(Sign::Positive))
    );
}

#[test]
fn nonzero_exact_cancellation_produces_positive_zero() {
    assert_eq!(
        reduce_finite_sum(
            tiny_format(),
            &[
                finite(Sign::Positive, 3, 0),
                finite(Sign::Negative, 3, 0),
                BinaryValueFixture::Zero(Sign::Negative),
            ],
        ),
        Ok(RoundedBinaryValue::Zero(Sign::Positive))
    );
}

#[test]
fn exact_and_inexact_finite_sums_round_once() {
    assert_eq!(
        reduce_finite_sum(
            tiny_format(),
            &[finite(Sign::Positive, 3, -1), finite(Sign::Positive, 5, -1)],
        ),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 8,
            exponent: 2,
        })
    );

    assert_eq!(
        reduce_finite_sum(
            tiny_format(),
            &[finite(Sign::Positive, 1, 0), finite(Sign::Positive, 1, -4)],
        ),
        Ok(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 8,
            exponent: 0,
        })
    );
}

#[test]
fn exact_sum_overflow_uses_existing_upper_rounding_boundary() {
    assert_eq!(
        reduce_finite_sum(
            tiny_format(),
            &[finite(Sign::Positive, 15, 2), finite(Sign::Positive, 4, 0)],
        ),
        Ok(RoundedBinaryValue::Infinity(Sign::Positive))
    );
}

#[test]
fn exact_sum_is_permutation_invariant_within_fixture_capacity() {
    let first = [
        finite(Sign::Positive, 7, -2),
        finite(Sign::Negative, 3, -1),
        BinaryValueFixture::Zero(Sign::Negative),
        finite(Sign::Positive, 5, -3),
    ];
    let second = [first[3], first[1], first[0], first[2]];

    assert_eq!(
        reduce_finite_sum(tiny_format(), &first),
        reduce_finite_sum(tiny_format(), &second)
    );
}

#[test]
fn exact_sum_differs_from_one_legal_rounded_add_tree() {
    let a = finite(Sign::Negative, 8, 1); // -16
    let b = finite(Sign::Negative, 8, 1); // -16
    let c = finite(Sign::Negative, 11, -1); // -5.5

    let left_pair = reduce_finite_sum(tiny_format(), &[a, b]).unwrap();
    let left_tree =
        reduce_finite_sum(tiny_format(), &[normal_as_finite_fixture(left_pair), c]).unwrap();

    let right_pair = reduce_finite_sum(tiny_format(), &[b, c]).unwrap();
    let right_tree =
        reduce_finite_sum(tiny_format(), &[a, normal_as_finite_fixture(right_pair)]).unwrap();

    let exact_sum = reduce_finite_sum(tiny_format(), &[a, b, c]).unwrap();

    assert_eq!(
        left_tree,
        RoundedBinaryValue::Normal {
            sign: Sign::Negative,
            significand: 9,
            exponent: 5,
        }
    );
    assert_eq!(
        right_tree,
        RoundedBinaryValue::Normal {
            sign: Sign::Negative,
            significand: 10,
            exponent: 5,
        }
    );
    assert_ne!(left_tree, right_tree);
    assert_eq!(exact_sum, left_tree);
}

#[test]
fn nonfinite_contributions_are_outside_finite_oracle_slice() {
    for contribution in [
        BinaryValueFixture::Infinity(Sign::Positive),
        BinaryValueFixture::Infinity(Sign::Negative),
        BinaryValueFixture::NaNClass,
    ] {
        assert_eq!(
            reduce_finite_sum(tiny_format(), &[contribution]),
            Err(NumericOracleError::NonFiniteReductionInput)
        );
    }
}

#[test]
fn malformed_zero_finite_fixture_is_rejected() {
    assert_eq!(
        reduce_finite_sum(tiny_format(), &[finite(Sign::Positive, 0, 0)]),
        Err(NumericOracleError::ZeroExactInput)
    );
}

#[test]
fn exact_accumulator_capacity_overflow_is_reported_not_wrapped() {
    let wide_format = BinaryFormat::new(127, 0, 126).expect("valid wide fixture format");
    let large = finite(Sign::Positive, 1_u128 << 126, 0);

    assert_eq!(
        reduce_finite_sum(wide_format, &[large, large, large, large]),
        Err(NumericOracleError::InternalRangeExceeded)
    );
}

#[test]
fn single_infinity_sign_determines_sum_with_finite_and_zero_inputs() {
    let finite_value = finite(Sign::Negative, 7, -2);

    assert_eq!(
        reduce_sum(
            tiny_format(),
            &[
                BinaryValueFixture::Infinity(Sign::Positive),
                finite_value,
                BinaryValueFixture::Zero(Sign::Negative),
            ],
        ),
        Ok(SumReductionResult::Value(RoundedBinaryValue::Infinity(
            Sign::Positive,
        )))
    );
    assert_eq!(
        reduce_sum(
            tiny_format(),
            &[
                finite_value,
                BinaryValueFixture::Infinity(Sign::Negative),
                BinaryValueFixture::Zero(Sign::Positive),
            ],
        ),
        Ok(SumReductionResult::Value(RoundedBinaryValue::Infinity(
            Sign::Negative,
        )))
    );
}

#[test]
fn opposite_infinity_signs_produce_nan_class_independent_of_order() {
    let first = [
        BinaryValueFixture::Infinity(Sign::Positive),
        finite(Sign::Positive, 3, 0),
        BinaryValueFixture::Infinity(Sign::Negative),
    ];
    let second = [first[2], first[1], first[0]];

    assert_eq!(
        reduce_sum(tiny_format(), &first),
        Ok(SumReductionResult::NaNClass)
    );
    assert_eq!(
        reduce_sum(tiny_format(), &second),
        Ok(SumReductionResult::NaNClass)
    );
}

#[test]
fn finite_only_broader_sum_delegates_to_accepted_finite_oracle() {
    let contributions = [
        finite(Sign::Positive, 7, -2),
        finite(Sign::Negative, 3, -1),
        BinaryValueFixture::Zero(Sign::Negative),
    ];
    let finite_result = reduce_finite_sum(tiny_format(), &contributions).unwrap();

    assert_eq!(
        reduce_sum(tiny_format(), &contributions),
        Ok(SumReductionResult::Value(finite_result))
    );
}

#[test]
fn submitted_nan_remains_outside_broader_sum_slice() {
    assert_eq!(
        reduce_sum(tiny_format(), &[BinaryValueFixture::NaNClass]),
        Err(NumericOracleError::NaNReductionInput)
    );
    assert_eq!(
        reduce_sum(
            tiny_format(),
            &[
                BinaryValueFixture::Infinity(Sign::Positive),
                BinaryValueFixture::NaNClass,
            ],
        ),
        Err(NumericOracleError::NaNReductionInput)
    );
}

#[test]
fn infinity_result_does_not_require_unneeded_finite_accumulator_capacity() {
    let wide_format = BinaryFormat::new(127, 0, 126).expect("valid wide fixture format");
    let large = finite(Sign::Positive, 1_u128 << 126, 0);

    assert_eq!(
        reduce_sum(
            wide_format,
            &[
                large,
                large,
                large,
                large,
                BinaryValueFixture::Infinity(Sign::Positive),
            ],
        ),
        Ok(SumReductionResult::Value(RoundedBinaryValue::Infinity(
            Sign::Positive,
        )))
    );
}

#[test]
fn malformed_finite_fixture_is_rejected_even_when_infinity_determines_result() {
    assert_eq!(
        reduce_sum(
            tiny_format(),
            &[
                BinaryValueFixture::Infinity(Sign::Positive),
                finite(Sign::Negative, 0, 0),
            ],
        ),
        Err(NumericOracleError::ZeroExactInput)
    );
}
