/*
 * Offline generator for the checked Runen correctly-rounded sine reference corpus.
 *
 * This program is verification tooling only. It is deliberately outside Cargo and
 * is not part of canonical `cargo validate`. The checked corpus was generated with
 * GNU MPFR 4.2.2.
 *
 * Regeneration on a system with MPFR development headers installed:
 *
 *   cc -std=c11 -O2 -Wall -Wextra -Werror -pedantic \
 *      crates/runen-numeric-oracle/reference/generate_sine_mpfr.c \
 *      -lmpfr -lgmp -o /tmp/runen-generate-sine-reference
 *   /tmp/runen-generate-sine-reference > /tmp/sine_reference.generated.rs
 *
 * Compare the two generated const blocks with the corresponding blocks in
 * `tests/sine_reference.rs`. No host `sin` implementation participates.
 *
 * For Runen fixture (p, emin, emax), MPFR is configured with precision p,
 * minimum exponent emin - p + 2, and maximum exponent emax + 1. Finite inputs
 * are constructed as exact dyadics. The target result is produced by
 * `mpfr_sin(..., MPFR_RNDN)` followed by
 * `mpfr_subnormalize(..., MPFR_RNDN)`. MPFR subnormalization therefore rounds
 * on the fixed grid 2^(emin-p+1), matching the Runen fixture's minimum
 * subnormal quantum while preserving the original ternary relation.
 */

#include <mpfr.h>

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    mpfr_prec_t precision;
    mpfr_exp_t emin;
    mpfr_exp_t emax;
} RunenFormat;

typedef struct {
    bool negative;
    unsigned long magnitude;
    mpfr_exp_t exponent;
} ExactDyadic;

typedef enum {
    ENCODED_ZERO,
    ENCODED_SUBNORMAL,
    ENCODED_NORMAL,
    ENCODED_INFINITY,
} EncodedKind;

typedef struct {
    EncodedKind kind;
    bool negative;
    unsigned long significand;
    mpfr_exp_t exponent;
} EncodedValue;

typedef struct {
    const char *label;
    ExactDyadic input;
    bool verify_midpoint_nearness;
} StressSpec;

static void fail(const char *message)
{
    fprintf(stderr, "sine reference generator: %s\n", message);
    exit(EXIT_FAILURE);
}

static unsigned int bit_length(unsigned long value)
{
    unsigned int bits = 0;
    while (value != 0) {
        ++bits;
        value >>= 1;
    }
    return bits;
}

static void configure_target_range(
    RunenFormat format,
    mpfr_exp_t *previous_emin,
    mpfr_exp_t *previous_emax)
{
    *previous_emin = mpfr_get_emin();
    *previous_emax = mpfr_get_emax();

    if (mpfr_set_emin(format.emin - format.precision + 2) != 0) {
        fail("unable to set MPFR minimum exponent");
    }
    if (mpfr_set_emax(format.emax + 1) != 0) {
        fail("unable to set MPFR maximum exponent");
    }
}

static void restore_range(mpfr_exp_t previous_emin, mpfr_exp_t previous_emax)
{
    if (mpfr_set_emin(previous_emin) != 0 || mpfr_set_emax(previous_emax) != 0) {
        fail("unable to restore MPFR exponent range");
    }
}

static void set_exact_dyadic(mpfr_t target, ExactDyadic input)
{
    if (input.magnitude == 0) {
        fail("finite corpus input must be nonzero");
    }

    if (mpfr_set_ui_2exp(
            target, input.magnitude, input.exponent, MPFR_RNDN) != 0) {
        fail("input dyadic was not exactly representable at fixture precision");
    }
    if (input.negative && mpfr_neg(target, target, MPFR_RNDN) != 0) {
        fail("negating exact input unexpectedly rounded");
    }
}

static unsigned long shift_exact(
    unsigned long magnitude,
    mpfr_exp_t source_exponent,
    mpfr_exp_t target_exponent)
{
    mpfr_exp_t distance = source_exponent - target_exponent;
    if (distance >= 0) {
        if ((unsigned long)distance >= sizeof(unsigned long) * 8) {
            fail("left shift exceeds generator carrier");
        }
        return magnitude << (unsigned int)distance;
    }

    distance = -distance;
    if ((unsigned long)distance >= sizeof(unsigned long) * 8) {
        fail("right shift exceeds generator carrier");
    }
    unsigned long divisor = 1UL << (unsigned int)distance;
    if (magnitude % divisor != 0) {
        fail("rounded MPFR value does not align with target grid");
    }
    return magnitude / divisor;
}

static EncodedValue encode_target_value(mpfr_t value, RunenFormat format)
{
    if (mpfr_nan_p(value)) {
        fail("finite sine reference unexpectedly produced NaN");
    }
    if (mpfr_inf_p(value)) {
        return (EncodedValue){
            .kind = ENCODED_INFINITY,
            .negative = mpfr_signbit(value) != 0,
            .significand = 0,
            .exponent = 0,
        };
    }
    if (mpfr_zero_p(value)) {
        return (EncodedValue){
            .kind = ENCODED_ZERO,
            .negative = mpfr_signbit(value) != 0,
            .significand = 0,
            .exponent = 0,
        };
    }

    mpfr_exp_t string_exponent = 0;
    char *digits = mpfr_get_str(
        NULL,
        &string_exponent,
        2,
        (size_t)format.precision,
        value,
        MPFR_RNDN);
    if (digits == NULL) {
        fail("mpfr_get_str allocation failed");
    }

    bool negative = digits[0] == '-';
    const char *magnitude_digits = negative ? digits + 1 : digits;
    size_t digit_count = strlen(magnitude_digits);
    char *end = NULL;
    unsigned long magnitude = strtoul(magnitude_digits, &end, 2);
    if (end == magnitude_digits || *end != '\0' || magnitude == 0) {
        mpfr_free_str(digits);
        fail("unable to decode finite MPFR binary significand");
    }

    mpfr_exp_t exact_exponent = string_exponent - (mpfr_exp_t)digit_count;
    while ((magnitude & 1UL) == 0) {
        magnitude >>= 1;
        ++exact_exponent;
    }
    mpfr_free_str(digits);

    mpfr_exp_t value_exponent =
        (mpfr_exp_t)bit_length(magnitude) - 1 + exact_exponent;
    if (value_exponent < format.emin) {
        mpfr_exp_t quantum_exponent = format.emin - format.precision + 1;
        unsigned long significand =
            shift_exact(magnitude, exact_exponent, quantum_exponent);
        unsigned long normal_threshold = 1UL << (format.precision - 1);
        if (significand == 0 || significand >= normal_threshold) {
            fail("decoded subnormal is outside target range");
        }
        return (EncodedValue){
            .kind = ENCODED_SUBNORMAL,
            .negative = negative,
            .significand = significand,
            .exponent = 0,
        };
    }

    mpfr_exp_t unit_exponent = value_exponent - format.precision + 1;
    unsigned long significand =
        shift_exact(magnitude, exact_exponent, unit_exponent);
    unsigned long normal_threshold = 1UL << (format.precision - 1);
    unsigned long carry = 1UL << format.precision;
    if (significand < normal_threshold || significand >= carry) {
        fail("decoded normal is outside target precision");
    }
    return (EncodedValue){
        .kind = ENCODED_NORMAL,
        .negative = negative,
        .significand = significand,
        .exponent = value_exponent,
    };
}

static EncodedValue reference_sine(ExactDyadic input, RunenFormat format)
{
    mpfr_exp_t previous_emin = 0;
    mpfr_exp_t previous_emax = 0;
    configure_target_range(format, &previous_emin, &previous_emax);

    mpfr_t x;
    mpfr_t y;
    mpfr_init2(x, format.precision);
    mpfr_init2(y, format.precision);

    set_exact_dyadic(x, input);
    int ternary = mpfr_sin(y, x, MPFR_RNDN);
    (void)mpfr_subnormalize(y, ternary, MPFR_RNDN);
    EncodedValue result = encode_target_value(y, format);

    mpfr_clear(x);
    mpfr_clear(y);
    restore_range(previous_emin, previous_emax);
    return result;
}

static void verify_near_midpoint(ExactDyadic input, RunenFormat target)
{
    const mpfr_prec_t work_precision = 256;
    mpfr_t x;
    mpfr_t y;
    mpfr_t magnitude;
    mpfr_t scaled;
    mpfr_t integral;
    mpfr_t fraction;
    mpfr_t half;
    mpfr_t distance;
    mpfr_t threshold;

    mpfr_init2(x, work_precision);
    mpfr_init2(y, work_precision);
    mpfr_init2(magnitude, work_precision);
    mpfr_init2(scaled, work_precision);
    mpfr_init2(integral, work_precision);
    mpfr_init2(fraction, work_precision);
    mpfr_init2(half, work_precision);
    mpfr_init2(distance, work_precision);
    mpfr_init2(threshold, work_precision);

    if (mpfr_set_ui_2exp(x, input.magnitude, input.exponent, MPFR_RNDN) != 0) {
        fail("midpoint input was not exact at high working precision");
    }
    if (input.negative && mpfr_neg(x, x, MPFR_RNDN) != 0) {
        fail("midpoint input negation unexpectedly rounded");
    }
    (void)mpfr_sin(y, x, MPFR_RNDN);
    (void)mpfr_abs(magnitude, y, MPFR_RNDN);

    mpfr_exp_t value_exponent = mpfr_get_exp(magnitude) - 1;
    if (value_exponent < target.emin) {
        fail("midpoint checker expects a normal target result");
    }
    mpfr_exp_t target_unit_exponent = value_exponent - target.precision + 1;
    (void)mpfr_mul_2si(
        scaled, magnitude, -target_unit_exponent, MPFR_RNDN);
    (void)mpfr_floor(integral, scaled);
    (void)mpfr_sub(fraction, scaled, integral, MPFR_RNDN);
    if (mpfr_set_ui_2exp(half, 1, -1, MPFR_RNDN) != 0) {
        fail("unable to construct exact midpoint fraction");
    }
    (void)mpfr_sub(distance, fraction, half, MPFR_RNDN);
    (void)mpfr_abs(distance, distance, MPFR_RNDN);
    if (mpfr_set_ui_2exp(threshold, 1, -14, MPFR_RNDN) != 0) {
        fail("unable to construct exact midpoint-distance threshold");
    }

    /*
     * The selected case must lie within 2^-14 of a target-ulp midpoint.
     * At 256-bit working precision the sine rounding error is negligible
     * relative to this classification threshold for the p=16 fixtures.
     */
    if (mpfr_cmp(distance, threshold) > 0) {
        fail("adversarial fixture is not sufficiently near a target midpoint");
    }

    mpfr_clear(x);
    mpfr_clear(y);
    mpfr_clear(magnitude);
    mpfr_clear(scaled);
    mpfr_clear(integral);
    mpfr_clear(fraction);
    mpfr_clear(half);
    mpfr_clear(distance);
    mpfr_clear(threshold);
}

static void print_case(ExactDyadic input, EncodedValue expected)
{
    switch (expected.kind) {
    case ENCODED_SUBNORMAL:
        printf(
            "    ReferenceCase::subnormal(%s, %lu, %ld, %s, %lu),\n",
            input.negative ? "NEG" : "POS",
            input.magnitude,
            (long)input.exponent,
            expected.negative ? "NEG" : "POS",
            expected.significand);
        break;
    case ENCODED_NORMAL:
        printf(
            "    ReferenceCase::normal(%s, %lu, %ld, %s, %lu, %ld),\n",
            input.negative ? "NEG" : "POS",
            input.magnitude,
            (long)input.exponent,
            expected.negative ? "NEG" : "POS",
            expected.significand,
            (long)expected.exponent);
        break;
    case ENCODED_ZERO:
    case ENCODED_INFINITY:
        fail("finite sine corpus unexpectedly produced a non-finite target value");
    }
}

static void emit_tiny_cases(void)
{
    const RunenFormat format = {
        .precision = 4,
        .emin = -2,
        .emax = 2,
    };
    const unsigned long normal_threshold = 1UL << (format.precision - 1);
    const unsigned long carry = 1UL << format.precision;
    const mpfr_exp_t quantum_exponent = format.emin - format.precision + 1;

    printf("const TINY_FINITE_CASES: &[ReferenceCase] = &[\n");
    for (int sign = 0; sign < 2; ++sign) {
        bool negative = sign != 0;
        for (unsigned long significand = 1;
             significand < normal_threshold;
             ++significand) {
            ExactDyadic input = {
                .negative = negative,
                .magnitude = significand,
                .exponent = quantum_exponent,
            };
            print_case(input, reference_sine(input, format));
        }
        for (mpfr_exp_t exponent = format.emin;
             exponent <= format.emax;
             ++exponent) {
            for (unsigned long significand = normal_threshold;
                 significand < carry;
                 ++significand) {
                ExactDyadic input = {
                    .negative = negative,
                    .magnitude = significand,
                    .exponent = exponent - format.precision + 1,
                };
                print_case(input, reference_sine(input, format));
            }
        }
    }
    printf("];\n\n");
}

static void emit_stress_cases(void)
{
    const RunenFormat format = {
        .precision = 16,
        .emin = -20,
        .emax = 200,
    };
    const StressSpec cases[] = {
        {"minimum subnormal", {false, 1, -35}, false},
        {"minimum normal", {false, 1, -20}, false},
        {"ordinary one", {false, 1, 0}, false},
        {"ordinary three", {false, 3, 0}, false},
        {"ordinary ten", {false, 10, 0}, false},
        {"negative ordinary ten", {true, 10, 0}, false},
        {"range reduction 2^50", {false, 1, 50}, false},
        {"range reduction 2^100", {false, 1, 100}, false},
        {"range reduction 2^180", {false, 1, 180}, false},
        {"near maximum finite input", {false, 65535, 185}, false},
        {"near midpoint A", {false, 57112, 163}, true},
        {"near midpoint B", {false, 35938, 6}, true},
        {"near midpoint C", {false, 41026, -4}, true},
    };

    printf("const STRESS_CASES: &[ReferenceCase] = &[\n");
    for (size_t index = 0; index < sizeof(cases) / sizeof(cases[0]); ++index) {
        const StressSpec *spec = &cases[index];
        if (spec->verify_midpoint_nearness) {
            verify_near_midpoint(spec->input, format);
        }
        printf("    // %s\n", spec->label);
        print_case(spec->input, reference_sine(spec->input, format));
    }
    printf("];\n");
}

int main(void)
{
    printf("// Generated with GNU MPFR %s; see reference/generate_sine_mpfr.c.\n", mpfr_get_version());
    printf("// BEGIN GENERATED SINE REFERENCE CORPUS\n");
    emit_tiny_cases();
    emit_stress_cases();
    printf("// END GENERATED SINE REFERENCE CORPUS\n");
    mpfr_free_cache();
    return EXIT_SUCCESS;
}
