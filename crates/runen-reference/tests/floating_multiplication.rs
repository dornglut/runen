use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, Function, FunctionId,
    LocalDecl, LocalId, NumericContract, Operand, Place, Program, SafeReferenceResultContract,
    ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_numeric_oracle::{BinaryFormat, ExactDyadic, RoundedBinaryValue, Sign, round_dyadic};
use runen_reference::{
    Machine, ObservedBinaryFloatValue, ObservedValue, TerminalStatus, VerificationEventKind,
    VerificationWriteKind,
};

fn constant(scalar: ScalarType, value: BinaryFloatValue) -> Value {
    match scalar {
        ScalarType::F16 => Value::F16(value),
        ScalarType::F32 => Value::F32(value),
        ScalarType::F64 => Value::F64(value),
        _ => unreachable!("FloatMul fixture requires a represented floating kind"),
    }
}

fn observed(scalar: ScalarType, value: ObservedBinaryFloatValue) -> ObservedValue {
    match scalar {
        ScalarType::F16 => ObservedValue::F16(value),
        ScalarType::F32 => ObservedValue::F32(value),
        ScalarType::F64 => ObservedValue::F64(value),
        _ => unreachable!("FloatMul fixture requires a represented floating kind"),
    }
}

fn execute_float_mul_with_contract(
    contract: NumericContract,
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> runen_reference::ExecutionReport {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("float", scalar));
    let result = Place::local(LocalId(0));
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("result", ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::FloatMul {
                        contract,
                        dst: result.clone(),
                        left: Operand::Constant(constant(scalar, left)),
                        right: Operand::Constant(constant(scalar, right)),
                    }],
                    Terminator::Return(Some(Operand::Move(result.into()))),
                )],
            },
        }],
    };
    let validated = validate_program(program).expect("same-format FloatMul fixture must validate");
    Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("safe FloatMul execution is defined")
}

fn assert_mul_with_contract(
    contract: NumericContract,
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
    expected: ObservedBinaryFloatValue,
) {
    let report = execute_float_mul_with_contract(contract, scalar, left, right);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(observed(scalar, expected)));
}

fn assert_mul(
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
    expected: ObservedBinaryFloatValue,
) {
    assert_mul_with_contract(NumericContract::Standard, scalar, left, right, expected);
}

fn positive_normal(significand: u64, exponent: i16) -> BinaryFloatValue {
    BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand,
        exponent,
    }
}

fn negative_normal(significand: u64, exponent: i16) -> BinaryFloatValue {
    BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Negative,
        significand,
        exponent,
    }
}

fn signed_subnormal(sign: BinaryFloatSign, significand: u64) -> BinaryFloatValue {
    BinaryFloatValue::Subnormal { sign, significand }
}

#[test]
fn exact_multiplication_executes_in_all_three_formats_and_all_contracts() {
    for (scalar, precision) in [
        (ScalarType::F16, 11_u32),
        (ScalarType::F32, 24_u32),
        (ScalarType::F64, 53_u32),
    ] {
        let one_significand = 1_u64 << (precision - 1);
        let three_significand = 3_u64 << (precision - 2);
        for contract in [
            NumericContract::Standard,
            NumericContract::Reproducible,
            NumericContract::Fast,
        ] {
            assert_mul_with_contract(
                contract,
                scalar,
                positive_normal(one_significand, 1),
                positive_normal(three_significand, 1),
                ObservedBinaryFloatValue::Represented(positive_normal(three_significand, 2)),
            );
        }
    }
}

#[test]
fn signed_zero_products_follow_operand_sign_product() {
    use BinaryFloatSign::{Negative, Positive};

    let one = positive_normal(1_u64 << 23, 0);
    let negative_one = negative_normal(1_u64 << 23, 0);
    for (left, right, expected) in [
        (BinaryFloatValue::Zero(Positive), one, Positive),
        (BinaryFloatValue::Zero(Negative), one, Negative),
        (BinaryFloatValue::Zero(Positive), negative_one, Negative),
        (BinaryFloatValue::Zero(Negative), negative_one, Positive),
        (
            BinaryFloatValue::Zero(Negative),
            BinaryFloatValue::Zero(Positive),
            Negative,
        ),
        (
            BinaryFloatValue::Zero(Negative),
            BinaryFloatValue::Zero(Negative),
            Positive,
        ),
    ] {
        assert_mul(
            ScalarType::F32,
            left,
            right,
            ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(expected)),
        );
    }
}

#[test]
fn infinity_products_follow_operand_sign_and_zero_times_infinity_is_nan() {
    use BinaryFloatSign::{Negative, Positive};

    let positive_infinity = BinaryFloatValue::Infinity(Positive);
    let negative_infinity = BinaryFloatValue::Infinity(Negative);
    let one = positive_normal(1_u64 << 23, 0);
    let negative_one = negative_normal(1_u64 << 23, 0);

    for (left, right, expected) in [
        (positive_infinity, one, positive_infinity),
        (negative_infinity, one, negative_infinity),
        (positive_infinity, negative_one, negative_infinity),
        (negative_infinity, negative_one, positive_infinity),
        (positive_infinity, negative_infinity, negative_infinity),
        (negative_infinity, negative_infinity, positive_infinity),
    ] {
        assert_mul(
            ScalarType::F32,
            left,
            right,
            ObservedBinaryFloatValue::Represented(expected),
        );
    }

    for (left, right) in [
        (BinaryFloatValue::Zero(Positive), positive_infinity),
        (negative_infinity, BinaryFloatValue::Zero(Negative)),
    ] {
        assert_mul(
            ScalarType::F32,
            left,
            right,
            ObservedBinaryFloatValue::NaNClass,
        );
    }
}

#[test]
fn produced_nan_is_a_runtime_operand_and_propagates_through_float_mul() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let nan = Place::local(LocalId(0));
    let result = Place::local(LocalId(1));
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![
                    LocalDecl::new("nan", f32_ty, false),
                    LocalDecl::new("result", f32_ty, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::FloatMul {
                            contract: NumericContract::Standard,
                            dst: nan.clone(),
                            left: Operand::Constant(Value::F32(BinaryFloatValue::Zero(
                                BinaryFloatSign::Positive,
                            ))),
                            right: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                                BinaryFloatSign::Positive,
                            ))),
                        },
                        Statement::FloatMul {
                            contract: NumericContract::Fast,
                            dst: result.clone(),
                            left: Operand::Move(nan.into()),
                            right: Operand::Constant(Value::F32(positive_normal(1_u64 << 23, 0))),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(result.into()))),
                )],
            },
        }],
    };

    let validated =
        validate_program(program).expect("NaN FloatMul propagation fixture must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("NaN FloatMul propagation is defined");
    assert_eq!(
        report.result,
        Some(ObservedValue::F32(ObservedBinaryFloatValue::NaNClass))
    );
}

#[test]
fn halfway_rounding_uses_ties_to_even_in_both_directions() {
    assert_mul(
        ScalarType::F16,
        positive_normal(1026, -1),
        positive_normal(1280, -1),
        ObservedBinaryFloatValue::Represented(positive_normal(1282, -2)),
    );
    assert_mul(
        ScalarType::F16,
        positive_normal(1025, -1),
        positive_normal(1536, -1),
        ObservedBinaryFloatValue::Represented(positive_normal(1538, -2)),
    );
}

#[test]
fn lower_normal_subnormal_boundary_rounding_is_exact() {
    let minimum_normal = positive_normal(1024, -14);
    assert_mul(
        ScalarType::F16,
        minimum_normal,
        positive_normal(2046, -1),
        ObservedBinaryFloatValue::Represented(signed_subnormal(BinaryFloatSign::Positive, 1023)),
    );
    assert_mul(
        ScalarType::F16,
        minimum_normal,
        positive_normal(2047, -1),
        ObservedBinaryFloatValue::Represented(minimum_normal),
    );
}

#[test]
fn upper_rounding_boundary_selects_max_finite_then_infinity_at_midpoint() {
    let maximum = positive_normal(2047, 15);
    assert_mul(
        ScalarType::F16,
        maximum,
        positive_normal(1024, 0),
        ObservedBinaryFloatValue::Represented(maximum),
    );
    assert_mul(
        ScalarType::F16,
        positive_normal(1040, 0),
        positive_normal(2016, 15),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
    );
}

#[test]
fn fast_reference_representative_preserves_subnormal_inputs_results_and_sign() {
    let one = positive_normal(1024, 0);
    for (subnormal, expected) in [
        (
            signed_subnormal(BinaryFloatSign::Positive, 1),
            signed_subnormal(BinaryFloatSign::Positive, 1),
        ),
        (
            signed_subnormal(BinaryFloatSign::Negative, 1),
            signed_subnormal(BinaryFloatSign::Negative, 1),
        ),
    ] {
        assert_mul_with_contract(
            NumericContract::Fast,
            ScalarType::F16,
            subnormal,
            one,
            ObservedBinaryFloatValue::Represented(expected),
        );
    }

    assert_mul_with_contract(
        NumericContract::Fast,
        ScalarType::F16,
        positive_normal(1024, -14),
        positive_normal(1024, -1),
        ObservedBinaryFloatValue::Represented(signed_subnormal(BinaryFloatSign::Positive, 512)),
    );
}

#[test]
fn extreme_f64_underflow_and_overflow_are_complete_without_oracle_dependency() {
    let minimum_subnormal = signed_subnormal(BinaryFloatSign::Positive, 1);
    assert_mul(
        ScalarType::F64,
        minimum_subnormal,
        minimum_subnormal,
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(BinaryFloatSign::Positive)),
    );

    let maximum = positive_normal((1_u64 << 53) - 1, 1023);
    let two = positive_normal(1_u64 << 52, 1);
    assert_mul(
        ScalarType::F64,
        maximum,
        two,
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
    );
}

#[test]
fn operand_effects_precede_exactly_one_distinct_float_mul_write() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let left = Place::local(LocalId(0));
    let right = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let two = positive_normal(1_u64 << 23, 1);
    let three = positive_normal(3_u64 << 22, 1);
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![
                    LocalDecl::new("left", f32_ty, false),
                    LocalDecl::new("right", f32_ty, false),
                    LocalDecl::new("result", f32_ty, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: left.clone(),
                            src: Operand::Constant(Value::F32(two)),
                        },
                        Statement::Init {
                            dst: right.clone(),
                            src: Operand::Constant(Value::F32(three)),
                        },
                        Statement::FloatMul {
                            contract: NumericContract::Fast,
                            dst: result.clone(),
                            left: Operand::Move(left.clone().into()),
                            right: Operand::Move(right.clone().into()),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(result.clone().into()))),
                )],
            },
        }],
    };
    let report = Machine::new(validate_program(program).unwrap(), FunctionId(0))
        .unwrap()
        .execute()
        .unwrap();

    let left_move = report
        .verification_events
        .iter()
        .position(|event| event.kind == VerificationEventKind::Move(left.clone()))
        .expect("left move event");
    let right_move = report
        .verification_events
        .iter()
        .position(|event| event.kind == VerificationEventKind::Move(right.clone()))
        .expect("right move event");
    let mul_writes = report
        .verification_events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            (event.kind
                == VerificationEventKind::Write {
                    place: result.clone(),
                    kind: VerificationWriteKind::FloatMul,
                })
            .then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(mul_writes.len(), 1);
    assert!(left_move < right_move);
    assert!(right_move < mul_writes[0]);
    assert!(report.verification_events.iter().all(|event| {
        !matches!(
            event.kind,
            VerificationEventKind::Write {
                kind: VerificationWriteKind::IntegerMul
                    | VerificationWriteKind::FloatAdd
                    | VerificationWriteKind::FloatSub,
                ..
            }
        )
    }));
}

fn oracle_format(scalar: ScalarType) -> (BinaryFormat, u32, i32, i32) {
    match scalar {
        ScalarType::F16 => (BinaryFormat::new(11, -14, 15).unwrap(), 11, -14, 15),
        ScalarType::F32 => (BinaryFormat::new(24, -126, 127).unwrap(), 24, -126, 127),
        ScalarType::F64 => (BinaryFormat::new(53, -1022, 1023).unwrap(), 53, -1022, 1023),
        _ => unreachable!("oracle fixture requires a represented floating kind"),
    }
}

fn oracle_sign(sign: BinaryFloatSign) -> Sign {
    match sign {
        BinaryFloatSign::Positive => Sign::Positive,
        BinaryFloatSign::Negative => Sign::Negative,
    }
}

fn product_sign(left: Sign, right: Sign) -> Sign {
    if left == right {
        Sign::Positive
    } else {
        Sign::Negative
    }
}

fn exact_nonzero_finite(scalar: ScalarType, value: BinaryFloatValue) -> (Sign, u128, i32) {
    let (_, precision, emin, _) = oracle_format(scalar);
    let precision_tail = i32::try_from(precision - 1).unwrap();
    match value {
        BinaryFloatValue::Subnormal { sign, significand } => (
            oracle_sign(sign),
            u128::from(significand),
            emin - precision_tail,
        ),
        BinaryFloatValue::Normal {
            sign,
            significand,
            exponent,
        } => (
            oracle_sign(sign),
            u128::from(significand),
            i32::from(exponent) - precision_tail,
        ),
        BinaryFloatValue::Zero(_) | BinaryFloatValue::Infinity(_) => {
            panic!("differential product fixture requires nonzero finite input")
        }
    }
}

fn observed_from_oracle(result: RoundedBinaryValue) -> ObservedBinaryFloatValue {
    ObservedBinaryFloatValue::Represented(match result {
        RoundedBinaryValue::Zero(sign) => BinaryFloatValue::Zero(match sign {
            Sign::Positive => BinaryFloatSign::Positive,
            Sign::Negative => BinaryFloatSign::Negative,
        }),
        RoundedBinaryValue::Subnormal { sign, significand } => BinaryFloatValue::Subnormal {
            sign: match sign {
                Sign::Positive => BinaryFloatSign::Positive,
                Sign::Negative => BinaryFloatSign::Negative,
            },
            significand: u64::try_from(significand).unwrap(),
        },
        RoundedBinaryValue::Normal {
            sign,
            significand,
            exponent,
        } => BinaryFloatValue::Normal {
            sign: match sign {
                Sign::Positive => BinaryFloatSign::Positive,
                Sign::Negative => BinaryFloatSign::Negative,
            },
            significand: u64::try_from(significand).unwrap(),
            exponent: i16::try_from(exponent).unwrap(),
        },
        RoundedBinaryValue::Infinity(sign) => BinaryFloatValue::Infinity(match sign {
            Sign::Positive => BinaryFloatSign::Positive,
            Sign::Negative => BinaryFloatSign::Negative,
        }),
    })
}

fn deterministic_nonzero_finite_values(scalar: ScalarType) -> Vec<BinaryFloatValue> {
    let (_, precision, emin, emax) = oracle_format(scalar);
    let minimum_normal = 1_u64 << (precision - 1);
    let maximum_significand = (1_u64 << precision) - 1;
    vec![
        signed_subnormal(BinaryFloatSign::Positive, 1),
        signed_subnormal(BinaryFloatSign::Negative, 2),
        signed_subnormal(BinaryFloatSign::Positive, minimum_normal - 1),
        positive_normal(minimum_normal, i16::try_from(emin).unwrap()),
        negative_normal(minimum_normal, -1),
        positive_normal(minimum_normal, 0),
        positive_normal(3_u64 << (precision - 2), 0),
        negative_normal(maximum_significand, i16::try_from(emax).unwrap()),
    ]
}

#[test]
fn standard_finite_products_match_independent_exact_dyadic_oracle_without_capacity_skips() {
    let mut compared = 0_usize;

    for scalar in [ScalarType::F16, ScalarType::F32, ScalarType::F64] {
        let (format, _, _, _) = oracle_format(scalar);
        let values = deterministic_nonzero_finite_values(scalar);
        for left in &values {
            for right in &values {
                let (left_sign, left_magnitude, left_exponent) =
                    exact_nonzero_finite(scalar, *left);
                let (right_sign, right_magnitude, right_exponent) =
                    exact_nonzero_finite(scalar, *right);
                let magnitude = left_magnitude
                    .checked_mul(right_magnitude)
                    .expect("represented significand product fits u128");
                let exponent = left_exponent
                    .checked_add(right_exponent)
                    .expect("represented exponent sum fits i32");
                let exact = ExactDyadic::from_parts(
                    product_sign(left_sign, right_sign),
                    magnitude,
                    exponent,
                );
                let expected = round_dyadic(format, exact)
                    .expect("represented finite product fits independent oracle capacity");
                compared += 1;
                assert_mul(scalar, *left, *right, observed_from_oracle(expected));
            }
        }
    }

    assert_eq!(compared, 3 * 8 * 8);
}
