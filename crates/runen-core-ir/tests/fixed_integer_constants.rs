use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, Function, LocalDecl, LocalId, MirValidationErrorKind,
    Operand, Place, Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_program,
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
            shared_reference_result_origin: None,
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

#[test]
fn every_fixed_width_integer_type_accepts_its_boundary_constants() {
    let cases = [
        (ScalarType::I8, Value::I8(i8::MIN)),
        (ScalarType::I8, Value::I8(i8::MAX)),
        (ScalarType::I16, Value::I16(i16::MIN)),
        (ScalarType::I16, Value::I16(i16::MAX)),
        (ScalarType::I32, Value::I32(i32::MIN)),
        (ScalarType::I32, Value::I32(i32::MAX)),
        (ScalarType::I64, Value::I64(i64::MIN)),
        (ScalarType::I64, Value::I64(i64::MAX)),
        (ScalarType::U8, Value::U8(0)),
        (ScalarType::U8, Value::U8(u8::MAX)),
        (ScalarType::U16, Value::U16(0)),
        (ScalarType::U16, Value::U16(u16::MAX)),
        (ScalarType::U32, Value::U32(0)),
        (ScalarType::U32, Value::U32(u32::MAX)),
        (ScalarType::U64, Value::U64(0)),
        (ScalarType::U64, Value::U64(u64::MAX)),
    ];

    for (scalar, value) in cases {
        validate_program(program_initializing(scalar, value))
            .expect("matching fixed-width integer boundary constant must validate");
    }
}

#[test]
fn integer_constant_matching_is_exact_across_width_and_signedness() {
    for value in [Value::U8(1), Value::I16(1), Value::I64(1)] {
        let error = validate_program(program_initializing(ScalarType::I8, value))
            .expect_err("cross-width or cross-signedness constant must be rejected");
        assert_eq!(
            error.kind,
            MirValidationErrorKind::TypeMismatch {
                expected: runen_core_ir::TypeId(0)
            }
        );
    }

    let error = validate_program(program_initializing(ScalarType::U64, Value::I64(1)))
        .expect_err("signed constant must not match unsigned type");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::TypeMismatch {
            expected: runen_core_ir::TypeId(0)
        }
    );
}

#[test]
fn structural_constants_recursively_preserve_non_i64_integer_variants() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let u64_ty = types.push(TypeDef::scalar("u64", ScalarType::U64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("small", i8_ty), Field::new("large", u64_ty)],
    ));
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: None,
            shared_reference_result_origin: None,
            body: Body {
                locals: vec![LocalDecl::new("pair", pair_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::Struct(vec![
                            Value::I8(i8::MIN),
                            Value::U64(u64::MAX),
                        ])),
                    }],
                    Terminator::Return(None),
                )],
            },
        }],
    };

    validate_program(program).expect("mixed fixed-width structural constant must validate");
}
