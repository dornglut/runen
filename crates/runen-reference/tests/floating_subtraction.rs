use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, Function, FunctionId,
    LocalDecl, LocalId, NumericContract, Operand, Place, Program, ScalarType, Statement,
    Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_numeric_oracle::{
    BinaryFormat, BinaryValueFixture, ExactDyadic, NumericOracleError, RoundedBinaryValue, Sign,
    SumReductionResult, reduce_sum,
};
use runen_reference::{
    Machine, ObservedBinaryFloatValue, ObservedValue, TerminalStatus, VerificationEventKind,
    VerificationWriteKind,
};

fn constant(scalar: ScalarType, value: BinaryFloatValue) -> Value {
    match scalar {
        ScalarType::F16 => Value::F16(value),
        ScalarType::F32 => Value::F32(value),
        ScalarType::F64 => Value::F64(value),
        _ => unreachable!("FloatSub fixture requires a represented floating kind"),
    }
}

fn observed(scalar: ScalarType, value: ObservedBinaryFloatValue) -> ObservedValue {
    match scalar {
        ScalarType::F16 => ObservedValue::F16(value),
        ScalarType::F32 => ObservedValue::F32(value),
        ScalarType::F64 => ObservedValue::F64(value),
        _ => unreachable!("FloatSub fixture requires a represented floating kind"),
    }
}

fn execute_float_sub_with_contract(
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
            shared_reference_result_origin: None,
            body: Body {
                locals: vec![LocalDecl::new("result", ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::FloatSub {
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
    let validated = validate_program(program).expect("same-format FloatSub fixture must validate");
    Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("safe FloatSub execution is defined")
}

fn assert_sub_with_contract(
    contract: NumericContract,
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
    expected: ObservedBinaryFloatValue,
) {
    let report = execute_float_sub_with_contract(contract, scalar, left, right);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(observed(scalar, expected)));
}

fn assert_sub(
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
    expected: ObservedBinaryFloatValue,
) {
    assert_sub_with_contract(NumericContract::Standard, scalar, left, right, expected);
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

#[test]
fn exact_subtraction_executes_in_all_three_formats_and_all_contracts() {
    for (scalar, one) in [
        (ScalarType::F16, 1_u64 << 10),
        (ScalarType::F32, 1_u64 << 23),
        (ScalarType::F64, 1_u64 << 52),
    ] {
        for contract in [
            NumericContract::Standard,
            NumericContract::Reproducible,
            NumericContract::Fast,
        ] {
            assert_sub_with_contract(
                contract,
                scalar,
                positive_normal(one, 1),
                positive_normal(one, 0),
                ObservedBinaryFloatValue::Represented(positive_normal(one, 0)),
            );
        }
    }
}

#[test]
fn signed_zero_pairs_and_exact_finite_cancellation_follow_subtraction_rules() {
    use BinaryFloatSign::{Negative, Positive};

    for (left, right, expected) in [
        (Negative, Positive, Negative),
        (Positive, Positive, Positive),
        (Positive, Negative, Positive),
        (Negative, Negative, Positive),
    ] {
        assert_sub(
            ScalarType::F32,
            BinaryFloatValue::Zero(left),
            BinaryFloatValue::Zero(right),
            ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(expected)),
        );
    }

    assert_sub(
        ScalarType::F64,
        positive_normal(1_u64 << 52, 10),
        positive_normal(1_u64 << 52, 10),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(Positive)),
    );
    assert_sub(
        ScalarType::F64,
        negative_normal(1_u64 << 52, 10),
        negative_normal(1_u64 << 52, 10),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(Positive)),
    );
}

#[test]
fn zero_minus_nonzero_reverses_only_the_finite_result_sign() {
    assert_sub(
        ScalarType::F32,
        BinaryFloatValue::Zero(BinaryFloatSign::Positive),
        positive_normal(1_u64 << 23, 0),
        ObservedBinaryFloatValue::Represented(negative_normal(1_u64 << 23, 0)),
    );
    assert_sub(
        ScalarType::F32,
        BinaryFloatValue::Zero(BinaryFloatSign::Negative),
        negative_normal(1_u64 << 23, 0),
        ObservedBinaryFloatValue::Represented(positive_normal(1_u64 << 23, 0)),
    );
}

#[test]
fn infinity_cases_follow_operation_specific_subtraction_rules() {
    let positive_infinity = BinaryFloatValue::Infinity(BinaryFloatSign::Positive);
    let negative_infinity = BinaryFloatValue::Infinity(BinaryFloatSign::Negative);
    let one = positive_normal(1_u64 << 23, 0);
    let negative_one = negative_normal(1_u64 << 23, 0);

    assert_sub(
        ScalarType::F32,
        positive_infinity,
        negative_infinity,
        ObservedBinaryFloatValue::Represented(positive_infinity),
    );
    assert_sub(
        ScalarType::F32,
        negative_infinity,
        positive_infinity,
        ObservedBinaryFloatValue::Represented(negative_infinity),
    );
    assert_sub(
        ScalarType::F32,
        positive_infinity,
        one,
        ObservedBinaryFloatValue::Represented(positive_infinity),
    );
    assert_sub(
        ScalarType::F32,
        one,
        positive_infinity,
        ObservedBinaryFloatValue::Represented(negative_infinity),
    );
    assert_sub(
        ScalarType::F32,
        negative_one,
        negative_infinity,
        ObservedBinaryFloatValue::Represented(positive_infinity),
    );
    assert_sub(
        ScalarType::F32,
        positive_infinity,
        positive_infinity,
        ObservedBinaryFloatValue::NaNClass,
    );
    assert_sub(
        ScalarType::F32,
        negative_infinity,
        negative_infinity,
        ObservedBinaryFloatValue::NaNClass,
    );
}

#[test]
fn produced_nan_is_a_runtime_operand_and_propagates_through_float_sub() {
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
            shared_reference_result_origin: None,
            body: Body {
                locals: vec![
                    LocalDecl::new("nan", f32_ty, false),
                    LocalDecl::new("result", f32_ty, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::FloatSub {
                            contract: NumericContract::Standard,
                            dst: nan.clone(),
                            left: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                                BinaryFloatSign::Positive,
                            ))),
                            right: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                                BinaryFloatSign::Positive,
                            ))),
                        },
                        Statement::FloatSub {
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
        validate_program(program).expect("NaN FloatSub propagation fixture must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("NaN FloatSub propagation is defined");
    assert_eq!(
        report.result,
        Some(ObservedValue::F32(ObservedBinaryFloatValue::NaNClass))
    );
}

#[test]
fn halfway_rounding_uses_ties_to_even_in_both_directions() {
    let half_ulp_at_one = negative_normal(1_u64 << 10, -11);

    assert_sub(
        ScalarType::F16,
        positive_normal(1024, 0),
        half_ulp_at_one,
        ObservedBinaryFloatValue::Represented(positive_normal(1024, 0)),
    );
    assert_sub(
        ScalarType::F16,
        positive_normal(1025, 0),
        half_ulp_at_one,
        ObservedBinaryFloatValue::Represented(positive_normal(1026, 0)),
    );
}

#[test]
fn lower_normal_subnormal_boundary_is_exact() {
    assert_sub(
        ScalarType::F16,
        positive_normal(1024, -14),
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1,
        },
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1023,
        }),
    );
}

#[test]
fn upper_rounding_boundary_selects_max_finite_then_infinity_at_midpoint() {
    let maximum = positive_normal(2047, 15);

    assert_sub(
        ScalarType::F16,
        maximum,
        negative_normal(1024, 3),
        ObservedBinaryFloatValue::Represented(maximum),
    );
    assert_sub(
        ScalarType::F16,
        maximum,
        negative_normal(1024, 4),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
    );
    assert_sub(
        ScalarType::F16,
        maximum,
        negative_normal(1024, 5),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
    );
}

#[test]
fn fast_reference_representative_preserves_subnormal_inputs_results_and_sign() {
    assert_sub_with_contract(
        NumericContract::Fast,
        ScalarType::F16,
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 2,
        },
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1,
        },
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1,
        }),
    );
    assert_sub_with_contract(
        NumericContract::Fast,
        ScalarType::F16,
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Negative,
            significand: 2,
        },
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Negative,
            significand: 1,
        },
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Negative,
            significand: 1,
        }),
    );
    assert_sub_with_contract(
        NumericContract::Fast,
        ScalarType::F16,
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1,
        },
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Negative,
            significand: 1,
        },
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 2,
        }),
    );
}

#[test]
fn extreme_f64_exponent_gap_is_complete_without_oracle_capacity() {
    let maximum = positive_normal((1_u64 << 53) - 1, 1023);
    let minimum_subnormal = BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Positive,
        significand: 1,
    };

    assert_sub(
        ScalarType::F64,
        maximum,
        minimum_subnormal,
        ObservedBinaryFloatValue::Represented(maximum),
    );
    assert_sub_with_contract(
        NumericContract::Fast,
        ScalarType::F64,
        maximum,
        minimum_subnormal,
        ObservedBinaryFloatValue::Represented(maximum),
    );
}

#[test]
fn operand_effects_precede_exactly_one_distinct_float_sub_write() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let left = Place::local(LocalId(0));
    let right = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let two = positive_normal(1_u64 << 23, 1);
    let one = positive_normal(1_u64 << 23, 0);
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
            shared_reference_result_origin: None,
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
                            src: Operand::Constant(Value::F32(one)),
                        },
                        Statement::FloatSub {
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

    let validated = validate_program(program).expect("FloatSub trace fixture must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("FloatSub trace fixture is defined");

    let left_move = report
        .verification_events
        .iter()
        .position(
            |event| matches!(&event.kind, VerificationEventKind::Move(place) if *place == left),
        )
        .expect("left move event");
    let right_move = report
        .verification_events
        .iter()
        .position(
            |event| matches!(&event.kind, VerificationEventKind::Move(place) if *place == right),
        )
        .expect("right move event");
    let writes = report
        .verification_events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            matches!(
                &event.kind,
                VerificationEventKind::Write {
                    place,
                    kind: VerificationWriteKind::FloatSub,
                } if *place == result
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();

    assert_eq!(writes.len(), 1);
    assert!(left_move < right_move);
    assert!(right_move < writes[0]);
    assert!(!report.verification_events.iter().any(|event| {
        matches!(
            event.kind,
            VerificationEventKind::Write {
                kind: VerificationWriteKind::FloatAdd,
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

fn oracle_sign(sign: BinaryFloatSign, invert: bool) -> Sign {
    match (sign, invert) {
        (BinaryFloatSign::Positive, false) | (BinaryFloatSign::Negative, true) => Sign::Positive,
        (BinaryFloatSign::Negative, false) | (BinaryFloatSign::Positive, true) => Sign::Negative,
    }
}

fn oracle_contribution(
    scalar: ScalarType,
    value: BinaryFloatValue,
    invert: bool,
) -> BinaryValueFixture {
    let (_, precision, emin, _) = oracle_format(scalar);
    let precision_tail = i32::try_from(precision - 1).unwrap();
    match value {
        BinaryFloatValue::Zero(sign) => BinaryValueFixture::Zero(oracle_sign(sign, invert)),
        BinaryFloatValue::Subnormal { sign, significand } => {
            BinaryValueFixture::Finite(ExactDyadic::from_parts(
                oracle_sign(sign, invert),
                u128::from(significand),
                emin - precision_tail,
            ))
        }
        BinaryFloatValue::Normal {
            sign,
            significand,
            exponent,
        } => BinaryValueFixture::Finite(ExactDyadic::from_parts(
            oracle_sign(sign, invert),
            u128::from(significand),
            i32::from(exponent) - precision_tail,
        )),
        BinaryFloatValue::Infinity(sign) => BinaryValueFixture::Infinity(oracle_sign(sign, invert)),
    }
}

fn observed_from_oracle(result: SumReductionResult) -> ObservedBinaryFloatValue {
    match result {
        SumReductionResult::NaNClass => ObservedBinaryFloatValue::NaNClass,
        SumReductionResult::Value(value) => ObservedBinaryFloatValue::Represented(match value {
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
        }),
    }
}

fn deterministic_finite_values(scalar: ScalarType) -> Vec<BinaryFloatValue> {
    let (_, precision, emin, emax) = oracle_format(scalar);
    let minimum_normal = 1_u64 << (precision - 1);
    let maximum_significand = (1_u64 << precision) - 1;
    vec![
        BinaryFloatValue::Zero(BinaryFloatSign::Positive),
        BinaryFloatValue::Zero(BinaryFloatSign::Negative),
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1,
        },
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Negative,
            significand: minimum_normal - 1,
        },
        BinaryFloatValue::Normal {
            sign: BinaryFloatSign::Positive,
            significand: minimum_normal,
            exponent: i16::try_from(emin).unwrap(),
        },
        BinaryFloatValue::Normal {
            sign: BinaryFloatSign::Negative,
            significand: minimum_normal + 1,
            exponent: i16::try_from(0_i32.clamp(emin, emax)).unwrap(),
        },
        BinaryFloatValue::Normal {
            sign: BinaryFloatSign::Positive,
            significand: maximum_significand,
            exponent: i16::try_from(emax).unwrap(),
        },
    ]
}

#[test]
fn standard_finite_results_match_independent_numeric_oracle_within_fixture_capacity() {
    let mut compared = 0_usize;
    let mut capacity_limited = 0_usize;

    for scalar in [ScalarType::F16, ScalarType::F32, ScalarType::F64] {
        let (format, _, _, _) = oracle_format(scalar);
        let values = deterministic_finite_values(scalar);
        for left in &values {
            for right in &values {
                if matches!(left, BinaryFloatValue::Zero(_))
                    && matches!(right, BinaryFloatValue::Zero(_))
                {
                    continue;
                }

                let contributions = [
                    oracle_contribution(scalar, *left, false),
                    oracle_contribution(scalar, *right, true),
                ];
                match reduce_sum(format, &contributions) {
                    Ok(expected) => {
                        compared += 1;
                        assert_sub(scalar, *left, *right, observed_from_oracle(expected));
                    }
                    Err(NumericOracleError::InternalRangeExceeded) => capacity_limited += 1,
                    Err(error) => {
                        panic!("well-formed finite subtraction oracle fixture failed: {error:?}")
                    }
                }
            }
        }
    }

    assert!(
        compared > 80,
        "differential corpus must exercise a broad finite subtraction domain"
    );
    assert!(
        capacity_limited > 0,
        "corpus must expose the oracle's documented extreme-range capacity limit"
    );
}
