use std::cmp::Ordering;

use runen_core_ir::{BinaryFloatSign, BinaryFloatValue};

use crate::ObservedBinaryFloatValue;

const MAGNITUDE_LIMBS: usize = 33;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FloatFormat {
    precision: u32,
    emin: i16,
    emax: i16,
}

const F16: FloatFormat = FloatFormat {
    precision: 11,
    emin: -14,
    emax: 15,
};
const F32: FloatFormat = FloatFormat {
    precision: 24,
    emin: -126,
    emax: 127,
};
const F64: FloatFormat = FloatFormat {
    precision: 53,
    emin: -1022,
    emax: 1023,
};

/// Reference-runtime floating leaf.
///
/// `NaNClass` represents only semantic class membership. The oracle deliberately
/// carries no member identity or physical NaN representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RuntimeFloatValue {
    Represented(BinaryFloatValue),
    NaNClass,
}

impl RuntimeFloatValue {
    pub(super) fn from_constant(value: BinaryFloatValue) -> Self {
        Self::Represented(value)
    }

    pub(super) fn into_observed(self) -> ObservedBinaryFloatValue {
        match self {
            Self::Represented(value) => ObservedBinaryFloatValue::Represented(value),
            Self::NaNClass => ObservedBinaryFloatValue::NaNClass,
        }
    }
}

pub(super) fn add_f16(left: RuntimeFloatValue, right: RuntimeFloatValue) -> RuntimeFloatValue {
    add_standard(F16, left, right)
}

pub(super) fn add_f32(left: RuntimeFloatValue, right: RuntimeFloatValue) -> RuntimeFloatValue {
    add_standard(F32, left, right)
}

pub(super) fn add_f64(left: RuntimeFloatValue, right: RuntimeFloatValue) -> RuntimeFloatValue {
    add_standard(F64, left, right)
}

fn add_standard(
    format: FloatFormat,
    left: RuntimeFloatValue,
    right: RuntimeFloatValue,
) -> RuntimeFloatValue {
    match (left, right) {
        (RuntimeFloatValue::NaNClass, _) | (_, RuntimeFloatValue::NaNClass) => {
            RuntimeFloatValue::NaNClass
        }
        (RuntimeFloatValue::Represented(left), RuntimeFloatValue::Represented(right)) => {
            add_represented(format, left, right)
        }
    }
}

fn add_represented(
    format: FloatFormat,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> RuntimeFloatValue {
    use BinaryFloatSign::{Negative, Positive};
    use BinaryFloatValue::{Infinity, Zero};

    match (left, right) {
        (Zero(left_sign), Zero(right_sign)) => {
            let sign = if left_sign == Negative && right_sign == Negative {
                Negative
            } else {
                Positive
            };
            RuntimeFloatValue::Represented(Zero(sign))
        }
        (Zero(_), right) => RuntimeFloatValue::Represented(right),
        (left, Zero(_)) => RuntimeFloatValue::Represented(left),
        (Infinity(left_sign), Infinity(right_sign)) if left_sign == right_sign => {
            RuntimeFloatValue::Represented(Infinity(left_sign))
        }
        (Infinity(_), Infinity(_)) => RuntimeFloatValue::NaNClass,
        (Infinity(sign), _) | (_, Infinity(sign)) => RuntimeFloatValue::Represented(Infinity(sign)),
        (left, right) => add_nonzero_finite(format, left, right),
    }
}

fn add_nonzero_finite(
    format: FloatFormat,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> RuntimeFloatValue {
    let (left_sign, left_magnitude) = exact_finite_magnitude(format, left);
    let (right_sign, right_magnitude) = exact_finite_magnitude(format, right);

    let (sign, magnitude) = if left_sign == right_sign {
        (left_sign, left_magnitude.add(&right_magnitude))
    } else {
        match left_magnitude.cmp(&right_magnitude) {
            Ordering::Greater => (left_sign, left_magnitude.sub(&right_magnitude)),
            Ordering::Less => (right_sign, right_magnitude.sub(&left_magnitude)),
            Ordering::Equal => {
                return RuntimeFloatValue::Represented(BinaryFloatValue::Zero(
                    BinaryFloatSign::Positive,
                ));
            }
        }
    };

    RuntimeFloatValue::Represented(round_exact_magnitude(format, sign, &magnitude))
}

fn exact_finite_magnitude(
    format: FloatFormat,
    value: BinaryFloatValue,
) -> (BinaryFloatSign, ExactMagnitude) {
    match value {
        BinaryFloatValue::Subnormal { sign, significand } => {
            (sign, ExactMagnitude::from_shifted(significand, 0))
        }
        BinaryFloatValue::Normal {
            sign,
            significand,
            exponent,
        } => {
            let shift = i32::from(exponent) - i32::from(format.emin);
            let shift = u32::try_from(shift)
                .expect("validated same-format normal exponent is not below emin");
            (sign, ExactMagnitude::from_shifted(significand, shift))
        }
        BinaryFloatValue::Zero(_) | BinaryFloatValue::Infinity(_) => {
            unreachable!("special floating values are handled before exact finite addition")
        }
    }
}

fn round_exact_magnitude(
    format: FloatFormat,
    sign: BinaryFloatSign,
    magnitude: &ExactMagnitude,
) -> BinaryFloatValue {
    let highest = magnitude
        .highest_bit()
        .expect("nonzero finite addition result has one highest bit");
    let normal_threshold_bit = format.precision - 1;

    if highest < normal_threshold_bit {
        return BinaryFloatValue::Subnormal {
            sign,
            significand: magnitude.low_u64(),
        };
    }

    let shift = highest - normal_threshold_bit;
    let mut significand = magnitude.shifted_down_u64(shift);

    if shift != 0 {
        let half_bit = shift - 1;
        let greater_than_half = magnitude.bit(half_bit) && magnitude.any_bits_below(half_bit);
        let exact_half = magnitude.bit(half_bit) && !magnitude.any_bits_below(half_bit);
        if greater_than_half || (exact_half && significand & 1 == 1) {
            significand += 1;
        }
    }

    let mut exponent = i32::from(format.emin) + i32::try_from(shift).expect("shift fits i32");
    let carry = 1_u64 << format.precision;
    if significand == carry {
        significand >>= 1;
        exponent += 1;
    }

    if exponent > i32::from(format.emax) {
        BinaryFloatValue::Infinity(sign)
    } else {
        BinaryFloatValue::Normal {
            sign,
            significand,
            exponent: i16::try_from(exponent).expect("represented floating exponent fits i16"),
        }
    }
}

/// Exact unsigned magnitude on the format's smallest-subnormal quantum grid.
///
/// The represented F64 format needs at most bit 2098 for the exact sum of two
/// finite operands. Thirty-three 64-bit limbs provide bits 0..2111, so this fixed
/// carrier is complete for F16/F32/F64 and has no semantic capacity failure mode.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ExactMagnitude([u64; MAGNITUDE_LIMBS]);

impl ExactMagnitude {
    fn from_shifted(value: u64, shift: u32) -> Self {
        debug_assert_ne!(value, 0);
        let mut limbs = [0_u64; MAGNITUDE_LIMBS];
        let word = usize::try_from(shift / 64).expect("word index fits usize");
        let offset = shift % 64;
        assert!(
            word < MAGNITUDE_LIMBS,
            "validated floating magnitude fits fixed carrier"
        );
        limbs[word] = value << offset;
        if offset != 0 {
            let next = word + 1;
            assert!(
                next < MAGNITUDE_LIMBS,
                "validated floating magnitude fits fixed carrier"
            );
            limbs[next] = value >> (64 - offset);
        }
        Self(limbs)
    }

    fn cmp(&self, other: &Self) -> Ordering {
        for index in (0..MAGNITUDE_LIMBS).rev() {
            match self.0[index].cmp(&other.0[index]) {
                Ordering::Equal => {}
                ordering => return ordering,
            }
        }
        Ordering::Equal
    }

    fn add(&self, other: &Self) -> Self {
        let mut result = [0_u64; MAGNITUDE_LIMBS];
        let mut carry = false;
        for (index, slot) in result.iter_mut().enumerate() {
            let (sum, carry_left) = self.0[index].overflowing_add(other.0[index]);
            let (sum, carry_in) = sum.overflowing_add(u64::from(carry));
            *slot = sum;
            carry = carry_left || carry_in;
        }
        assert!(
            !carry,
            "F16/F32/F64 exact addition fits the proven 2112-bit carrier"
        );
        Self(result)
    }

    fn sub(&self, other: &Self) -> Self {
        debug_assert!(self.cmp(other) != Ordering::Less);
        let mut result = [0_u64; MAGNITUDE_LIMBS];
        let mut borrow = false;
        for (index, slot) in result.iter_mut().enumerate() {
            let (difference, borrow_right) = self.0[index].overflowing_sub(other.0[index]);
            let (difference, borrow_in) = difference.overflowing_sub(u64::from(borrow));
            *slot = difference;
            borrow = borrow_right || borrow_in;
        }
        debug_assert!(!borrow);
        Self(result)
    }

    fn highest_bit(&self) -> Option<u32> {
        self.0.iter().enumerate().rev().find_map(|(index, word)| {
            if *word == 0 {
                None
            } else {
                let within_word = 63 - word.leading_zeros();
                Some(u32::try_from(index).expect("limb index fits u32") * 64 + within_word)
            }
        })
    }

    fn bit(&self, index: u32) -> bool {
        let word = usize::try_from(index / 64).expect("word index fits usize");
        let offset = index % 64;
        self.0
            .get(word)
            .is_some_and(|word| word & (1_u64 << offset) != 0)
    }

    /// Whether any bit with index strictly below `exclusive_upper` is set.
    fn any_bits_below(&self, exclusive_upper: u32) -> bool {
        let full_words = usize::try_from(exclusive_upper / 64).expect("word count fits usize");
        if self.0[..full_words.min(MAGNITUDE_LIMBS)]
            .iter()
            .any(|word| *word != 0)
        {
            return true;
        }

        let remainder = exclusive_upper % 64;
        if remainder == 0 || full_words >= MAGNITUDE_LIMBS {
            return false;
        }
        let mask = (1_u64 << remainder) - 1;
        self.0[full_words] & mask != 0
    }

    /// Shift right by `shift`, returning the low 64 bits of the quotient.
    ///
    /// Callers choose `shift` so the complete quotient contains at most the
    /// represented precision (53 bits), therefore no significant quotient bit is
    /// discarded by this return type.
    fn shifted_down_u64(&self, shift: u32) -> u64 {
        let word = usize::try_from(shift / 64).expect("word index fits usize");
        let offset = shift % 64;
        let Some(low_word) = self.0.get(word) else {
            return 0;
        };
        let mut value = *low_word >> offset;
        if offset != 0
            && let Some(high_word) = self.0.get(word + 1)
        {
            value |= *high_word << (64 - offset);
        }
        value
    }

    fn low_u64(&self) -> u64 {
        debug_assert!(self.0[1..].iter().all(|word| *word == 0));
        self.0[0]
    }
}
