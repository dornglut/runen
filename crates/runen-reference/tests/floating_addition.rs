use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, Field, Function, FunctionId,
    LocalDecl, LocalId, Operand, Place, Program, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_program,
};
use runen_numeric_oracle::{
    BinaryFormat, NumericOracleError, RoundedBinaryValue, Sign, SumReductionResult,
    add_standard_tree_node,
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
        _ => unreachable!("FloatAdd fixture requires a represented floating kind"),
    }
}

fn observed(scalar: ScalarType, value: ObservedBinaryFloatValue) -> ObservedValue {
    match scalar {
        ScalarType::F16 => ObservedValue::F16(value),
        ScalarType::F32 => ObservedValue::F32(value),
        ScalarType::F64 => ObservedValue::F64(value),
        _ => unreachable!("FloatAdd fixture requires a represented floating kind"),
    }
}

fn execute_float_add(
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
            body: Body {
                locals: vec![LocalDecl::new("result", ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::FloatAdd {
                        dst: result.clone(),
                        left: Operand::Constant(constant(scalar, left)),
                        right: Operand::Constant(constant(scalar, right)),
                    }],
                    Terminator::Return(Some(Operand::Move(result.into()))),
                )],
            },
        }],
    };
    let validated = validate_program(program).expect("same-format FloatAdd fixture must validate");
    Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("safe FloatAdd execution is defined")
}

fn assert_add(
    scalar: ScalarType,
    left: BinaryFloatValue,
    right: BinaryFloatValue,
    expected: ObservedBinaryFloatValue,
) {
    let report = execute_float_add(scalar, left, right);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(observed(scalar, expected)));
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
fn exact_addition_executes_in_all_three_formats() {
    let cases = [
        (ScalarType::F16, 1_u64 << 10),
        (ScalarType::F32, 1_u64 << 23),
        (ScalarType::F64, 1_u64 << 52),
    ];

    for (scalar, one) in cases {
        assert_add(
            scalar,
            positive_normal(one, 0),
            positive_normal(one, 0),
            ObservedBinaryFloatValue::Represented(positive_normal(one, 1)),
        );
    }
}

#[test]
fn halfway_rounding_uses_ties_to_even_in_both_directions() {
    let half_ulp_at_one = positive_normal(1_u64 << 10, -11);

    assert_add(
        ScalarType::F16,
        positive_normal(1024, 0),
        half_ulp_at_one,
        ObservedBinaryFloatValue::Represented(positive_normal(1024, 0)),
    );
    assert_add(
        ScalarType::F16,
        positive_normal(1025, 0),
        half_ulp_at_one,
        ObservedBinaryFloatValue::Represented(positive_normal(1026, 0)),
    );
}

#[test]
fn subnormal_boundary_and_rounding_carry_are_exact() {
    assert_add(
        ScalarType::F16,
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1023,
        },
        BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Positive,
            significand: 1,
        },
        ObservedBinaryFloatValue::Represented(positive_normal(1024, -14)),
    );

    assert_add(
        ScalarType::F16,
        positive_normal(2047, 0),
        positive_normal(1024, -10),
        ObservedBinaryFloatValue::Represented(positive_normal(1024, 1)),
    );
}

#[test]
fn upper_rounding_boundary_selects_max_finite_then_infinity_at_midpoint() {
    let maximum = positive_normal(2047, 15);

    assert_add(
        ScalarType::F16,
        maximum,
        positive_normal(1024, 3),
        ObservedBinaryFloatValue::Represented(maximum),
    );
    assert_add(
        ScalarType::F16,
        maximum,
        positive_normal(1024, 4),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
    );
    assert_add(
        ScalarType::F16,
        maximum,
        positive_normal(1024, 5),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Infinity(
            BinaryFloatSign::Positive,
        )),
    );
}

#[test]
fn extreme_f64_exponent_gap_is_complete_without_oracle_capacity() {
    let maximum = positive_normal((1_u64 << 53) - 1, 1023);
    let minimum_subnormal = BinaryFloatValue::Subnormal {
        sign: BinaryFloatSign::Positive,
        significand: 1,
    };

    assert_add(
        ScalarType::F64,
        maximum,
        minimum_subnormal,
        ObservedBinaryFloatValue::Represented(maximum),
    );
}

#[test]
fn cancellation_and_signed_zero_follow_the_accepted_addition_rule() {
    assert_add(
        ScalarType::F64,
        positive_normal(1_u64 << 52, 10),
        negative_normal(1_u64 << 52, 10),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(
            BinaryFloatSign::Positive,
        )),
    );
    assert_add(
        ScalarType::F32,
        BinaryFloatValue::Zero(BinaryFloatSign::Negative),
        BinaryFloatValue::Zero(BinaryFloatSign::Negative),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(
            BinaryFloatSign::Negative,
        )),
    );
    assert_add(
        ScalarType::F32,
        BinaryFloatValue::Zero(BinaryFloatSign::Positive),
        BinaryFloatValue::Zero(BinaryFloatSign::Negative),
        ObservedBinaryFloatValue::Represented(BinaryFloatValue::Zero(
            BinaryFloatSign::Positive,
        )),
    );
}

#[test]
fn infinity_cases_and_nan_class_are_runtime_values_not_constants() {
    let positive_infinity = BinaryFloatValue::Infinity(BinaryFloatSign::Positive);
    let negative_infinity = BinaryFloatValue::Infinity(BinaryFloatSign::Negative);

    assert_add(
        ScalarType::F32,
        positive_infinity,
        positive_normal(1_u64 << 23, 0),
        ObservedBinaryFloatValue::Represented(positive_infinity),
    );
    assert_add(
        ScalarType::F32,
        negative_infinity,
        negative_infinity,
        ObservedBinaryFloatValue::Represented(negative_infinity),
    );
    assert_add(
        ScalarType::F32,
        positive_infinity,
        negative_infinity,
        ObservedBinaryFloatValue::NaNClass,
    );
}

#[test]
fn produced_nan_survives_copy_move_and_later_float_add() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let first = Place::local(LocalId(0));
    let copied = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
            body: Body {
                locals: vec![
                    LocalDecl::new("first", f32_ty, false),
                    LocalDecl::new("copied", f32_ty, false),
                    LocalDecl::new("result", f32_ty, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::FloatAdd {
                            dst: first.clone(),
                            left: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                                BinaryFloatSign::Positive,
                            ))),
                            right: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                                BinaryFloatSign::Negative,
                            ))),
                        },
                        Statement::Init {
                            dst: copied.clone(),
                            src: Operand::Copy(first.into()),
                        },
                        Statement::FloatAdd {
                            dst: result.clone(),
                            left: Operand::Move(copied.into()),
                            right: Operand::Constant(Value::F32(positive_normal(1_u64 << 23, 0))),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(result.into()))),
                )],
            },
        }],
    };

    let validated = validate_program(program).expect("NaN transport fixture must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("NaN class transport is defined");

    assert_eq!(
        report.result,
        Some(ObservedValue::F32(ObservedBinaryFloatValue::NaNClass))
    );
    assert!(report.verification_events.iter().any(|event| {
        matches!(event.kind, VerificationEventKind::Copy(ref place) if *place == Place::local(LocalId(0)))
    }));
    assert!(report.verification_events.iter().any(|event| {
        matches!(event.kind, VerificationEventKind::Move(ref place) if *place == Place::local(LocalId(1)))
    }));
}

#[test]
fn nan_class_round_trips_through_call_argument_and_result() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));

    let caller = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(f32_ty),
        body: Body {
            locals: vec![
                LocalDecl::new("nan", f32_ty, false),
                LocalDecl::new("result", f32_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    vec![Statement::FloatAdd {
                        dst: Place::local(LocalId(0)),
                        left: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                            BinaryFloatSign::Positive,
                        ))),
                        right: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                            BinaryFloatSign::Negative,
                        ))),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
                ),
            ],
        },
    };
    let identity = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(f32_ty),
        body: Body {
            locals: vec![LocalDecl::new("value", f32_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        },
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, identity],
    })
    .expect("NaN call transport fixture must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("NaN call transport is defined");

    assert_eq!(
        report.result,
        Some(ObservedValue::F32(ObservedBinaryFloatValue::NaNClass))
    );
}

#[test]
fn struct_transport_preserves_nan_class_without_member_identity() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("float", f32_ty), Field::new("tag", i8_ty)],
    ));

    let nan = Place::local(LocalId(0));
    let pair = Place::local(LocalId(1));
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(pair_ty),
            body: Body {
                locals: vec![
                    LocalDecl::new("nan", f32_ty, false),
                    LocalDecl::new("pair", pair_ty, false),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::FloatAdd {
                            dst: nan.clone(),
                            left: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                                BinaryFloatSign::Positive,
                            ))),
                            right: Operand::Constant(Value::F32(BinaryFloatValue::Infinity(
                                BinaryFloatSign::Negative,
                            ))),
                        },
                        Statement::Init {
                            dst: pair.clone().field(0),
                            src: Operand::Move(nan.into()),
                        },
                        Statement::Init {
                            dst: pair.clone().field(1),
                            src: Operand::Constant(Value::I8(7)),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(pair.into()))),
                )],
            },
        }],
    };

    let validated = validate_program(program).expect("NaN struct fixture must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("NaN struct transport is defined");

    assert_eq!(
        report.result,
        Some(ObservedValue::Struct(vec![
            ObservedValue::F32(ObservedBinaryFloatValue::NaNClass),
            ObservedValue::I8(7),
        ]))
    );
}

#[test]
fn operand_effects_precede_exactly_one_float_add_write() {
    let mut types = TypeTable::new();
    let f32_ty = types.push(TypeDef::scalar("f32", ScalarType::F32));
    let left = Place::local(LocalId(0));
    let right = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let one = positive_normal(1_u64 << 23, 0);
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(f32_ty),
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
                            src: Operand::Constant(Value::F32(one)),
                        },
                        Statement::Init {
                            dst: right.clone(),
                            src: Operand::Constant(Value::F32(one)),
                        },
                        Statement::FloatAdd {
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

    let validated = validate_program(program).expect("trace fixture must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("trace fixture is defined");

    let left_move = report
        .verification_events
        .iter()
        .position(|event| matches!(&event.kind, VerificationEventKind::Move(place) if *place == left))
        .expect("left move event");
    let right_move = report
        .verification_events
        .iter()
        .position(|event| matches!(&event.kind, VerificationEventKind::Move(place) if *place == right))
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
                    kind: VerificationWriteKind::FloatAdd,
                } if *place == result
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();

    assert_eq!(writes.len(), 1);
    assert!(left_move < right_move);
    assert!(right_move < writes[0]);
    assert!(!report.verification_events.iter().any(|event| {
        matches!(event.kind, VerificationEventKind::DropTrackedFixture { .. })
    }));
}

fn oracle_format(scalar: ScalarType) -> BinaryFormat {
    match scalar {
        ScalarType::F16 => BinaryFormat::new(11, -14, 15).unwrap(),
        ScalarType::F32 => BinaryFormat::new(24, -126, 127).unwrap(),
        ScalarType::F64 => BinaryFormat::new(53, -1022, 1023).unwrap(),
        _ => unreachable!("oracle fixture requires a represented floating kind"),
    }
}

fn oracle_sign(sign: BinaryFloatSign) -> Sign {
    match sign {
        BinaryFloatSign::Positive => Sign::Positive,
        BinaryFloatSign::Negative => Sign::Negative,
    }
}

fn oracle_value(value: BinaryFloatValue) -> SumReductionResult {
    SumReductionResult::Value(match value {
        BinaryFloatValue::Zero(sign) => RoundedBinaryValue::Zero(oracle_sign(sign)),
        BinaryFloatValue::Subnormal { sign, significand } => RoundedBinaryValue::Subnormal {
            sign: oracle_sign(sign),
            significand: u128::from(significand),
        },
        BinaryFloatValue::Normal {
            sign,
            significand,
            exponent,
        } => RoundedBinaryValue::Normal {
            sign: oracle_sign(sign),
            significand: u128::from(significand),
            exponent: i32::from(exponent),
        },
        BinaryFloatValue::Infinity(sign) => RoundedBinaryValue::Infinity(oracle_sign(sign)),
    })
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

fn deterministic_values(scalar: ScalarType) -> Vec<BinaryFloatValue> {
    let (precision, emin, emax) = match scalar {
        ScalarType::F16 => (11_u32, -14_i16, 15_i16),
        ScalarType::F32 => (24, -126, 127),
        ScalarType::F64 => (53, -1022, 1023),
        _ => unreachable!(),
    };
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
            exponent: emin,
        },
        BinaryFloatValue::Normal {
            sign: BinaryFloatSign::Negative,
            significand: minimum_normal + 1,
            exponent: 0_i16.clamp(emin, emax),
        },
        BinaryFloatValue::Normal {
            sign: BinaryFloatSign::Positive,
            significand: maximum_significand,
            exponent: emax,
        },
        BinaryFloatValue::Infinity(BinaryFloatSign::Positive),
        BinaryFloatValue::Infinity(BinaryFloatSign::Negative),
    ]
}

#[test]
fn reference_results_match_independent_numeric_oracle_within_fixture_capacity() {
    let mut compared = 0_usize;
    let mut capacity_limited = 0_usize;

    for scalar in [ScalarType::F16, ScalarType::F32, ScalarType::F64] {
        let format = oracle_format(scalar);
        let values = deterministic_values(scalar);
        for left in &values {
            for right in &values {
                match add_standard_tree_node(format, oracle_value(*left), oracle_value(*right)) {
                    Ok(expected) => {
                        compared += 1;
                        assert_add(scalar, *left, *right, observed_from_oracle(expected));
                    }
                    Err(NumericOracleError::InternalRangeExceeded) => capacity_limited += 1,
                    Err(error) => panic!("well-formed represented oracle fixture failed: {error:?}"),
                }
            }
        }
    }

    assert!(compared > 150, "differential corpus must exercise a broad defined domain");
    assert!(
        capacity_limited > 0,
        "corpus must expose the oracle's documented extreme-range capacity limit"
    );
}
