use runen_numeric_oracle::{
    BinaryConversionResult, BinaryFormat, BinaryValueFixture, ExactDyadic, IntegerConversionResult,
    IntegerFormat, IntegerSignedness, NumericOracleError, RoundedBinaryValue, Sign,
    convert_binary_to_binary, convert_binary_to_integer,
};

fn tiny_float_format() -> BinaryFormat {
    BinaryFormat::new(4, -2, 2).expect("valid test float format")
}

fn signed_i8() -> IntegerFormat {
    IntegerFormat::new(8, IntegerSignedness::Signed).expect("valid signed fixture")
}

fn unsigned_u8() -> IntegerFormat {
    IntegerFormat::new(8, IntegerSignedness::Unsigned).expect("valid unsigned fixture")
}

#[test]
fn binary_to_binary_finite_conversion_reuses_rounding_relation() {
    assert_eq!(
        convert_binary_to_binary(
            tiny_float_format(),
            BinaryValueFixture::Finite(ExactDyadic::from_parts(Sign::Positive, 17, -4))
        ),
        Ok(BinaryConversionResult::Value(RoundedBinaryValue::Normal {
            sign: Sign::Positive,
            significand: 8,
            exponent: 0,
        }))
    );
}

#[test]
fn binary_to_binary_preserves_signed_zero_and_infinity() {
    for sign in [Sign::Positive, Sign::Negative] {
        assert_eq!(
            convert_binary_to_binary(tiny_float_format(), BinaryValueFixture::Zero(sign)),
            Ok(BinaryConversionResult::Value(RoundedBinaryValue::Zero(
                sign
            )))
        );
        assert_eq!(
            convert_binary_to_binary(tiny_float_format(), BinaryValueFixture::Infinity(sign)),
            Ok(BinaryConversionResult::Value(
                RoundedBinaryValue::Infinity(sign)
            ))
        );
    }
}

#[test]
fn binary_to_binary_nan_preserves_only_class_membership() {
    assert_eq!(
        convert_binary_to_binary(tiny_float_format(), BinaryValueFixture::NaNClass),
        Ok(BinaryConversionResult::NaNClass)
    );
}

#[test]
fn binary_to_integer_truncates_finite_values_toward_zero() {
    assert_eq!(
        convert_binary_to_integer(
            signed_i8(),
            BinaryValueFixture::Finite(ExactDyadic::from_parts(Sign::Positive, 15, -2))
        ),
        Ok(IntegerConversionResult::Signed(3))
    );
    assert_eq!(
        convert_binary_to_integer(
            signed_i8(),
            BinaryValueFixture::Finite(ExactDyadic::from_parts(Sign::Negative, 15, -2))
        ),
        Ok(IntegerConversionResult::Signed(-3))
    );
}

#[test]
fn binary_to_integer_tests_range_after_truncation() {
    assert_eq!(
        convert_binary_to_integer(
            signed_i8(),
            BinaryValueFixture::Finite(ExactDyadic::from_parts(Sign::Positive, 255, -1))
        ),
        Ok(IntegerConversionResult::Signed(127))
    );
    assert_eq!(
        convert_binary_to_integer(
            unsigned_u8(),
            BinaryValueFixture::Finite(ExactDyadic::from_parts(Sign::Negative, 1, -1))
        ),
        Ok(IntegerConversionResult::Unsigned(0))
    );
}

#[test]
fn binary_to_integer_clamps_finite_overflow_to_destination_bounds() {
    assert_eq!(
        convert_binary_to_integer(
            signed_i8(),
            BinaryValueFixture::Finite(ExactDyadic::from_signed_integer(128))
        ),
        Ok(IntegerConversionResult::Signed(127))
    );
    assert_eq!(
        convert_binary_to_integer(
            signed_i8(),
            BinaryValueFixture::Finite(ExactDyadic::from_signed_integer(-129))
        ),
        Ok(IntegerConversionResult::Signed(-128))
    );
    assert_eq!(
        convert_binary_to_integer(
            unsigned_u8(),
            BinaryValueFixture::Finite(ExactDyadic::from_unsigned_integer(256))
        ),
        Ok(IntegerConversionResult::Unsigned(255))
    );
}

#[test]
fn binary_to_integer_maps_infinity_to_destination_bounds() {
    assert_eq!(
        convert_binary_to_integer(signed_i8(), BinaryValueFixture::Infinity(Sign::Positive)),
        Ok(IntegerConversionResult::Signed(127))
    );
    assert_eq!(
        convert_binary_to_integer(signed_i8(), BinaryValueFixture::Infinity(Sign::Negative)),
        Ok(IntegerConversionResult::Signed(-128))
    );
    assert_eq!(
        convert_binary_to_integer(
            unsigned_u8(),
            BinaryValueFixture::Infinity(Sign::Positive)
        ),
        Ok(IntegerConversionResult::Unsigned(255))
    );
    assert_eq!(
        convert_binary_to_integer(
            unsigned_u8(),
            BinaryValueFixture::Infinity(Sign::Negative)
        ),
        Ok(IntegerConversionResult::Unsigned(0))
    );
}

#[test]
fn binary_to_integer_maps_nan_class_to_zero() {
    assert_eq!(
        convert_binary_to_integer(signed_i8(), BinaryValueFixture::NaNClass),
        Ok(IntegerConversionResult::Signed(0))
    );
    assert_eq!(
        convert_binary_to_integer(unsigned_u8(), BinaryValueFixture::NaNClass),
        Ok(IntegerConversionResult::Unsigned(0))
    );
}

#[test]
fn integer_destination_validation_is_fixture_capacity_only() {
    assert_eq!(
        IntegerFormat::new(0, IntegerSignedness::Signed),
        Err(NumericOracleError::IntegerWidthZero)
    );
    assert_eq!(
        IntegerFormat::new(129, IntegerSignedness::Unsigned),
        Err(NumericOracleError::IntegerWidthTooLarge)
    );
    assert!(IntegerFormat::new(128, IntegerSignedness::Signed).is_ok());
    assert!(IntegerFormat::new(128, IntegerSignedness::Unsigned).is_ok());
}

#[test]
fn oversized_dyadic_integer_magnitude_clamps_without_host_overflow() {
    let huge_positive = BinaryValueFixture::Finite(ExactDyadic::from_parts(
        Sign::Positive,
        1,
        200,
    ));
    let huge_negative = BinaryValueFixture::Finite(ExactDyadic::from_parts(
        Sign::Negative,
        1,
        200,
    ));

    assert_eq!(
        convert_binary_to_integer(signed_i8(), huge_positive),
        Ok(IntegerConversionResult::Signed(127))
    );
    assert_eq!(
        convert_binary_to_integer(signed_i8(), huge_negative),
        Ok(IntegerConversionResult::Signed(-128))
    );
    assert_eq!(
        convert_binary_to_integer(unsigned_u8(), huge_positive),
        Ok(IntegerConversionResult::Unsigned(255))
    );
    assert_eq!(
        convert_binary_to_integer(unsigned_u8(), huge_negative),
        Ok(IntegerConversionResult::Unsigned(0))
    );
}
