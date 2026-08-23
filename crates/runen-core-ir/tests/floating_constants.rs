use runen_core_ir::{
    BasicBlock, BasicBlockId, BinaryFloatSign, BinaryFloatValue, Body, Field, Function, LocalDecl,
    LocalId, MirValidationErrorKind, Operand, Place, Program, ScalarType, Statement, Terminator,
    TypeDef, TypeId, TypeTable, Value, validate_program,
};

fn program_initializing(scalar: ScalarType, value: Value) -> Program {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("scalar", scalar));
    Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            body: Body {
                locals: vec![LocalDecl::new("value", ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(value),
                    }],
                    Terminator::Return(None),
                )],
            },
        }],
    }
}

fn wrapped(scalar: ScalarType, value: BinaryFloatValue) -> Value {
    match scalar {
        ScalarType::F16 => Value::F16(value),
        ScalarType::F32 => Value::F32(value),
        ScalarType::F64 => Value::F64(value),
        _ => panic!("test helper requires a represented floating scalar"),
    }
}

fn assert_valid(scalar: ScalarType, value: BinaryFloatValue) {
    validate_program(program_initializing(scalar, wrapped(scalar, value)))
        .expect("matching semantic floating constant must validate");
}

fn assert_invalid(scalar: ScalarType, value: BinaryFloatValue) {
    let error = validate_program(program_initializing(scalar, wrapped(scalar, value)))
        .expect_err("malformed semantic floating constant must be rejected");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch {
            expected: TypeId(0)
        }
    );
}

#[test]
fn signed_zero_and_infinity_validate_and_remain_distinct() {
    for scalar in [ScalarType::F16, ScalarType::F32, ScalarType::F64] {
        let positive_zero = BinaryFloatValue::Zero(BinaryFloatSign::Positive);
        let negative_zero = BinaryFloatValue::Zero(BinaryFloatSign::Negative);
        let positive_infinity = BinaryFloatValue::Infinity(BinaryFloatSign::Positive);
        let negative_infinity = BinaryFloatValue::Infinity(BinaryFloatSign::Negative);

        assert_valid(scalar, positive_zero);
        assert_valid(scalar, negative_zero);
        assert_valid(scalar, positive_infinity);
        assert_valid(scalar, negative_infinity);

        assert_ne!(wrapped(scalar, positive_zero), wrapped(scalar, negative_zero));
        assert_ne!(
            wrapped(scalar, positive_infinity),
            wrapped(scalar, negative_infinity)
        );
    }
}

#[test]
fn exact_floating_format_boundaries_validate() {
    let cases = [
        (ScalarType::F16, 11_u32, -14_i16, 15_i16),
        (ScalarType::F32, 24_u32, -126_i16, 127_i16),
        (ScalarType::F64, 53_u32, -1022_i16, 1023_i16),
    ];

    for (scalar, precision, emin, emax) in cases {
        let normal_min = 1_u64 << (precision - 1);
        let normal_max = (1_u64 << precision) - 1;
        let subnormal_max = normal_min - 1;

        for value in [
            BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Positive,
                significand: 1,
            },
            BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Negative,
                significand: subnormal_max,
            },
            BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Positive,
                significand: normal_min,
                exponent: emin,
            },
            BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Negative,
                significand: normal_max,
                exponent: emax,
            },
        ] {
            assert_valid(scalar, value);
        }
    }
}

#[test]
fn malformed_floating_payloads_reject_at_existing_type_boundary() {
    let cases = [
        (ScalarType::F16, 11_u32, -14_i16, 15_i16),
        (ScalarType::F32, 24_u32, -126_i16, 127_i16),
        (ScalarType::F64, 53_u32, -1022_i16, 1023_i16),
    ];

    for (scalar, precision, emin, emax) in cases {
        let normal_min = 1_u64 << (precision - 1);
        let significand_limit = 1_u64 << precision;

        for value in [
            BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Positive,
                significand: 0,
            },
            BinaryFloatValue::Subnormal {
                sign: BinaryFloatSign::Negative,
                significand: normal_min,
            },
            BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Positive,
                significand: normal_min - 1,
                exponent: emin,
            },
            BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Negative,
                significand: significand_limit,
                exponent: emax,
            },
            BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Positive,
                significand: normal_min,
                exponent: emin - 1,
            },
            BinaryFloatValue::Normal {
                sign: BinaryFloatSign::Negative,
                significand: normal_min,
                exponent: emax + 1,
            },
        ] {
            assert_invalid(scalar, value);
        }
    }
}

#[test]
fn floating_constant_matching_is_exact_across_formats() {
    let payload = BinaryFloatValue::Zero(BinaryFloatSign::Positive);
    let cases = [
        (ScalarType::F16, Value::F32(payload)),
        (ScalarType::F16, Value::F64(payload)),
        (ScalarType::F32, Value::F16(payload)),
        (ScalarType::F32, Value::F64(payload)),
        (ScalarType::F64, Value::F16(payload)),
        (ScalarType::F64, Value::F32(payload)),
    ];

    for (scalar, value) in cases {
        let error = validate_program(program_initializing(scalar, value))
            .expect_err("valid payload in the wrong format wrapper must be rejected");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::TypeMismatch {
                expected: TypeId(0)
            }
        );
    }
}

#[test]
fn structural_constants_recursively_preserve_mixed_floating_formats() {
    let mut types = TypeTable::new();
    let f16_ty = types.push(TypeDef::scalar("f16", ScalarType::F16));
    let f64_ty = types.push(TypeDef::scalar("f64", ScalarType::F64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("small", f16_ty), Field::new("large", f64_ty)],
    ));
    let value = Value::Struct(vec![
        Value::F16(BinaryFloatValue::Subnormal {
            sign: BinaryFloatSign::Negative,
            significand: 1,
        }),
        Value::F64(BinaryFloatValue::Normal {
            sign: BinaryFloatSign::Positive,
            significand: (1_u64 << 53) - 1,
            exponent: 1023,
        }),
    ]);
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            body: Body {
                locals: vec![LocalDecl::new("pair", pair_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(value),
                    }],
                    Terminator::Return(None),
                )],
            },
        }],
    };

    validate_program(program).expect("mixed floating structural constant must validate");
}
