use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, Function, FunctionId,
    LocalDecl, LocalId, NumericContract, Operand, Place, Program, SafeReferenceResultContract,
    ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_numeric_oracle::{
    BinaryFormat, ExactBinaryRatio, RoundedBinaryValue, Sign, round_binary_ratio,
};
use runen_reference::{
    Machine, ObservedBinaryFloatValue, ObservedValue, TerminalStatus, VerificationEventKind,
    VerificationWriteKind,
};

fn format(scalar: ScalarType) -> (BinaryFormat, u32, i32) {
    match scalar {
        ScalarType::F16 => (BinaryFormat::new(11, -14, 15).unwrap(), 11, -14),
        ScalarType::F32 => (BinaryFormat::new(24, -126, 127).unwrap(), 24, -126),
        ScalarType::F64 => (BinaryFormat::new(53, -1022, 1023).unwrap(), 53, -1022),
        _ => unreachable!("FloatDiv fixture requires a represented floating kind"),
    }
}

fn constant(scalar: ScalarType, value: BinaryFloatValue) -> Value {
    match scalar {
        ScalarType::F16 => Value::F16(value),
        ScalarType::F32 => Value::F32(value),
        ScalarType::F64 => Value::F64(value),
        _ => unreachable!("FloatDiv fixture requires a represented floating kind"),
    }
}

fn observed(scalar: ScalarType, value: ObservedBinaryFloatValue) -> ObservedValue {
    match scalar {
        ScalarType::F16 => ObservedValue::F16(value),
        ScalarType::F32 => ObservedValue::F32(value),
        ScalarType::F64 => ObservedValue::F64(value),
        _ => unreachable!("FloatDiv fixture requires a represented floating kind"),
    }
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

fn execute_float_div_with_contract(
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
                    vec![Statement::FloatDiv {
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
    let validated = validate_program(program).expect("same-format FloatDiv fixture must validate");
    Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("safe FloatDiv execution is defined")
}

fn assert_div_with_contract(
    contract: NumericContract,
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
    expected: ObservedBinaryFloatValue,
) {
    let report = execute_float_div_with_contract(contract, scalar, left, right);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(observed(scalar, expected)));
}

fn assert_div(
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
    expected: ObservedBinaryFloatValue,
) {
    assert_div_with_contract(NumericContract::Standard, scalar, left, right, expected);
}

fn finite_parts(scalar: ScalarType, value: BinaryFloatValue) -> (Sign, u128, i32) {
    let (_, precision, emin) = format(scalar);
    let tail = i32::try_from(precision - 1).unwrap();
    match value {
        BinaryFloatValue::Subnormal { sign, significand } => {
            (oracle_sign(sign), u128::from(significand), emin - tail)
        }
        BinaryFloatValue::Normal {
            sign,
            significand,
            exponent,
        } => (
            oracle_sign(sign),
            u128::from(significand),
            i32::from(exponent) - tail,
        ),
        BinaryFloatValue::Zero(_) | BinaryFloatValue::Infinity(_) => {
            unreachable!("oracle cross-check accepts only nonzero finite operands")
        }
    }
}

fn oracle_sign(sign: BinaryFloatSign) -> Sign {
    match sign {
        BinaryFloatSign::Positive => Sign::Positive,
        BinaryFloatSign::Negative => Sign::Negative,
    }
}

fn binary_sign(sign: Sign) -> BinaryFloatSign {
    match sign {
        Sign::Positive => BinaryFloatSign::Positive,
        Sign::Negative => BinaryFloatSign::Negative,
    }
}

fn quotient_sign(left: Sign, right: Sign) -> Sign {
    if left == right {
        Sign::Positive
    } else {
        Sign::Negative
    }
}

fn oracle_quotient(
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
) -> BinaryFloatValue {
    let (format, _, _) = format(scalar);
    let (left_sign, numerator, left_exponent) = finite_parts(scalar, left);
    let (right_sign, denominator, right_exponent) = finite_parts(scalar, right);
    let rounded = round_binary_ratio(
        format,
        ExactBinaryRatio::from_parts(
            quotient_sign(left_sign, right_sign),
            numerator,
            denominator,
            left_exponent - right_exponent,
        ),
    )
    .expect("represented nonzero finite quotient is within oracle fixture capacity");
    match rounded {
        RoundedBinaryValue::Zero(sign) => BinaryFloatValue::Zero(binary_sign(sign)),
        RoundedBinaryValue::Subnormal { sign, significand } => BinaryFloatValue::Subnormal {
            sign: binary_sign(sign),
            significand: u64::try_from(significand).unwrap(),
        },
        RoundedBinaryValue::Normal {
            sign,
            significand,
            exponent,
        } => BinaryFloatValue::Normal {
            sign: binary_sign(sign),
            significand: u64::try_from(significand).unwrap(),
            exponent: i16::try_from(exponent).unwrap(),
        },
        RoundedBinaryValue::Infinity(sign) => BinaryFloatValue::Infinity(binary_sign(sign)),
    }
}

#[test]
fn exact_division_executes_in_all_three_formats_and_all_contracts() {
    for (scalar, precision) in [
        (ScalarType::F16, 11_u32),
        (ScalarType::F32, 24_u32),
        (ScalarType::F64, 53_u32),
    ] {
        let one = 1_u64 << (precision - 1);
        let three = 3_u64 << (precision - 2);
        let left = positive_normal(three, 1);
        let right = positive_normal(one, 1);
        let expected = positive_normal(three, 0);
        for contract in [
            NumericContract::Standard,
            NumericContract::Reproducible,
            NumericContract::Fast,
        ] {
            assert_div_with_contract(
                contract,
                scalar,
                left,
                right,
                ObservedBinaryFloatValue::Represented(expected),
            );
        }
    }
}

#[test]
fn recurring_one_third_matches_direct_expectations_and_independent_ratio_oracle() {
    for (scalar, precision, expected_significand) in [
        (ScalarType::F16, 11_u32, 1_365_u64),
        (ScalarType::F32, 24_u32, 11_184_811_u64),
        (ScalarType::F64, 53_u32, 6_004_799_503_160_661_u64),
    ] {
        let one = positive_normal(1_u64 << (precision - 1), 0);
        let three = positive_normal(3_u64 << (precision - 2), 1);
        let expected = positive_normal(expected_significand, -2);
        assert_eq!(oracle_quotient(scalar, one, three), expected);
        for contract in [
            NumericContract::Standard,
            NumericContract::Reproducible,
            NumericContract::Fast,
        ] {
            assert_div_with_contract(
                contract,
                scalar,
                one,
                three,
                ObservedBinaryFloatValue::Represented(expected),
            );
        }
    }
}

#[test]
fn signed_zero_and_infinity_division_special_values_follow_sign_product() {
    use BinaryFloatSign::{Negative, Positive};

    let one = positive_normal(1_u64 << 23, 0);
    let negative_one = negative_normal(1_u64 << 23, 0);
    let positive_infinity = BinaryFloatValue::Infinity(Positive);
    let negative_infinity = BinaryFloatValue::Infinity(Negative);

    for (left, right) in [
        (
            BinaryFloatValue::Zero(Positive),
            BinaryFloatValue::Zero(Negative),
        ),
        (positive_infinity, negative_infinity),
    ] {
        assert_div(
            ScalarType::F32,
            left,
            right,
            ObservedBinaryFloatValue::NaNClass,
        );
    }

    for (left, right, expected) in [
        (
            one,
            BinaryFloatValue::Zero(Positive),
            BinaryFloatValue::Infinity(Positive),
        ),
        (
            negative_one,
            BinaryFloatValue::Zero(Positive),
            BinaryFloatValue::Infinity(Negative),
        ),
        (
            one,
            BinaryFloatValue::Zero(Negative),
            BinaryFloatValue::Infinity(Negative),
        ),
        (positive_infinity, negative_one, negative_infinity),
        (negative_infinity, negative_one, positive_infinity),
        (
            positive_infinity,
            BinaryFloatValue::Zero(Negative),
            negative_infinity,
        ),
        (
            BinaryFloatValue::Zero(Negative),
            one,
            BinaryFloatValue::Zero(Negative),
        ),
        (
            BinaryFloatValue::Zero(Negative),
            negative_one,
            BinaryFloatValue::Zero(Positive),
        ),
        (
            negative_one,
            positive_infinity,
            BinaryFloatValue::Zero(Negative),
        ),
        (
            BinaryFloatValue::Zero(Negative),
            positive_infinity,
            BinaryFloatValue::Zero(Negative),
        ),
    ] {
        assert_div(
            ScalarType::F32,
            left,
            right,
            ObservedBinaryFloatValue::Represented(expected),
        );
    }
}

#[test]
fn produced_nan_is_a_runtime_operand_and_propagates_through_float_div() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let nan = Place::local(LocalId(0));
    let result = Place::local(LocalId(1));
    let one = positive_normal(1_u64 << 23, 0);
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
                        Statement::FloatDiv {
                            contract: NumericContract::Fast,
                            dst: result.clone(),
                            left: Operand::Move(nan.into()),
                            right: Operand::Constant(Value::F32(one)),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(result.into()))),
                )],
            },
        }],
    };

    let report = Machine::new(validate_program(program).unwrap(), FunctionId(0))
        .unwrap()
        .execute()
        .unwrap();
    assert_eq!(
        report.result,
        Some(ObservedValue::F32(ObservedBinaryFloatValue::NaNClass))
    );
}

#[test]
fn lower_normal_subnormal_boundary_and_underflow_midpoint_round_ties_to_even() {
    let minimum_normal = positive_normal(1024, -14);
    let two = positive_normal(1024, 1);

    let boundary_midpoint_left = positive_normal(2047, -14);
    assert_eq!(
        oracle_quotient(ScalarType::F16, boundary_midpoint_left, two),
        minimum_normal
    );
    assert_div(
        ScalarType::F16,
        boundary_midpoint_left,
        two,
        ObservedBinaryFloatValue::Represented(minimum_normal),
    );

    let minimum_subnormal = signed_subnormal(BinaryFloatSign::Positive, 1);
    assert_div(
        ScalarType::F16,
        minimum_subnormal,
        two,
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(BinaryFloatSign::Positive)),
    );
    assert_div(
        ScalarType::F16,
        signed_subnormal(BinaryFloatSign::Positive, 3),
        positive_normal(1024, 2),
        ObservedBinaryFloatValue::Represented(minimum_subnormal),
    );
}

#[test]
fn finite_overflow_boundary_and_extreme_f64_ranges_are_complete_without_oracle_capacity_dependency()
{
    let f16_maximum = positive_normal(2047, 15);
    assert_div(
        ScalarType::F16,
        f16_maximum,
        positive_normal(1024, 0),
        ObservedBinaryFloatValue::Represented(f16_maximum),
    );

    // The binary16 overflow rounding midpoint is 65520. A same-format finite
    // quotient cannot equal that midpoint: after reduction it requires the
    // p+1-bit odd numerator 4095. The closest realizable quotient below is the
    // maximum finite 65504; dividing that value by the predecessor of one yields
    // the closest realizable quotient above, exactly 65536, and must carry to
    // infinity.
    assert_div(
        ScalarType::F16,
        f16_maximum,
        positive_normal(2047, -1),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
    );

    let f64_minimum_subnormal = signed_subnormal(BinaryFloatSign::Positive, 1);
    let f64_maximum = positive_normal((1_u64 << 53) - 1, 1023);
    assert_div(
        ScalarType::F64,
        f64_minimum_subnormal,
        f64_maximum,
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(BinaryFloatSign::Positive)),
    );
    assert_div(
        ScalarType::F64,
        f64_maximum,
        f64_minimum_subnormal,
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
    );
}

#[test]
fn fast_reference_representative_preserves_subnormal_results_and_sign() {
    let minimum_normal = positive_normal(1024, -14);
    let two = positive_normal(1024, 1);
    assert_div_with_contract(
        NumericContract::Fast,
        ScalarType::F16,
        minimum_normal,
        two,
        ObservedBinaryFloatValue::Represented(signed_subnormal(BinaryFloatSign::Positive, 512)),
    );
    assert_div_with_contract(
        NumericContract::Fast,
        ScalarType::F16,
        negative_normal(1024, -14),
        two,
        ObservedBinaryFloatValue::Represented(signed_subnormal(BinaryFloatSign::Negative, 512)),
    );
}

#[test]
fn operand_effects_precede_exactly_one_distinct_float_div_write() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let left = Place::local(LocalId(0));
    let right = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let three = positive_normal(3_u64 << 22, 1);
    let two = positive_normal(1_u64 << 23, 1);
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
                            src: Operand::Constant(Value::F32(three)),
                        },
                        Statement::Init {
                            dst: right.clone(),
                            src: Operand::Constant(Value::F32(two)),
                        },
                        Statement::FloatDiv {
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
    let div_writes = report
        .verification_events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            (event.kind
                == VerificationEventKind::Write {
                    place: result.clone(),
                    kind: VerificationWriteKind::FloatDiv,
                })
            .then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(div_writes.len(), 1);
    assert!(left_move < right_move);
    assert!(right_move < div_writes[0]);
    assert!(report.verification_events.iter().all(|event| {
        !matches!(
            event.kind,
            VerificationEventKind::Write {
                kind: VerificationWriteKind::FloatMul
                    | VerificationWriteKind::IntegerMul
                    | VerificationWriteKind::FloatAdd
                    | VerificationWriteKind::FloatSub,
                ..
            }
        )
    }));
}
