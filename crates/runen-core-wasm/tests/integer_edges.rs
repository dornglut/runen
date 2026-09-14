use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeTable,
    Value, validate_program,
};
use runen_core_wasm::{ExecutionOutcome, RealizedProgram};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

#[derive(Clone, Copy, Debug)]
enum ArithmeticOp {
    Add,
    Sub,
    Mul,
}

#[derive(Clone, Debug)]
struct Case {
    ty: ScalarType,
    op: ArithmeticOp,
    left: Value,
    right: Value,
    expected: Value,
}

fn observed(value: &Value) -> ObservedValue {
    match value {
        Value::I8(value) => ObservedValue::I8(*value),
        Value::I16(value) => ObservedValue::I16(*value),
        Value::I32(value) => ObservedValue::I32(*value),
        Value::I64(value) => ObservedValue::I64(*value),
        Value::U8(value) => ObservedValue::U8(*value),
        Value::U16(value) => ObservedValue::U16(*value),
        Value::U32(value) => ObservedValue::U32(*value),
        Value::U64(value) => ObservedValue::U64(*value),
        other => panic!("integer arithmetic fixture produced non-integer value: {other:?}"),
    }
}

fn run(case: Case) {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("integer", case.ty.clone()));
    let dst = Place::local(LocalId(0));
    let statement = match case.op {
        ArithmeticOp::Add => Statement::IntegerAdd {
            dst: dst.clone(),
            left: Operand::Constant(case.left.clone()),
            right: Operand::Constant(case.right.clone()),
        },
        ArithmeticOp::Sub => Statement::IntegerSub {
            dst: dst.clone(),
            left: Operand::Constant(case.left.clone()),
            right: Operand::Constant(case.right.clone()),
        },
        ArithmeticOp::Mul => Statement::IntegerMul {
            dst: dst.clone(),
            left: Operand::Constant(case.left.clone()),
            right: Operand::Constant(case.right.clone()),
        },
    };
    let validated = validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
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
                    vec![statement],
                    Terminator::Return(Some(Operand::Move(dst.into()))),
                )],
            },
        }],
    })
    .expect("integer boundary fixture must be valid Core");

    let reference = Machine::new(validated.clone(), FunctionId(0))
        .expect("entry is zero-parameter")
        .execute()
        .expect("integer arithmetic fixture is defined");
    assert_eq!(reference.terminal, TerminalStatus::Returned);
    assert_eq!(reference.result, Some(observed(&case.expected)));

    let realized = RealizedProgram::new(&validated)
        .expect("integer boundary fixture is within realization coverage")
        .execute(FunctionId(0))
        .expect("integer boundary fixture must execute physically");
    assert_eq!(realized, ExecutionOutcome::Returned(Some(case.expected)));
}

#[test]
fn add_sub_mul_preserve_zero_and_wrap_boundaries_for_every_integer_kind() {
    let cases = vec![
        Case { ty: ScalarType::I8, op: ArithmeticOp::Add, left: Value::I8(i8::MIN), right: Value::I8(0), expected: Value::I8(i8::MIN) },
        Case { ty: ScalarType::I8, op: ArithmeticOp::Add, left: Value::I8(i8::MAX), right: Value::I8(1), expected: Value::I8(i8::MIN) },
        Case { ty: ScalarType::I8, op: ArithmeticOp::Sub, left: Value::I8(i8::MIN), right: Value::I8(1), expected: Value::I8(i8::MAX) },
        Case { ty: ScalarType::I8, op: ArithmeticOp::Mul, left: Value::I8(i8::MAX), right: Value::I8(0), expected: Value::I8(0) },
        Case { ty: ScalarType::I8, op: ArithmeticOp::Mul, left: Value::I8(i8::MAX), right: Value::I8(2), expected: Value::I8(-2) },

        Case { ty: ScalarType::I16, op: ArithmeticOp::Add, left: Value::I16(i16::MIN), right: Value::I16(0), expected: Value::I16(i16::MIN) },
        Case { ty: ScalarType::I16, op: ArithmeticOp::Add, left: Value::I16(i16::MAX), right: Value::I16(1), expected: Value::I16(i16::MIN) },
        Case { ty: ScalarType::I16, op: ArithmeticOp::Sub, left: Value::I16(i16::MIN), right: Value::I16(1), expected: Value::I16(i16::MAX) },
        Case { ty: ScalarType::I16, op: ArithmeticOp::Mul, left: Value::I16(i16::MAX), right: Value::I16(0), expected: Value::I16(0) },
        Case { ty: ScalarType::I16, op: ArithmeticOp::Mul, left: Value::I16(i16::MAX), right: Value::I16(2), expected: Value::I16(-2) },

        Case { ty: ScalarType::I32, op: ArithmeticOp::Add, left: Value::I32(i32::MIN), right: Value::I32(0), expected: Value::I32(i32::MIN) },
        Case { ty: ScalarType::I32, op: ArithmeticOp::Add, left: Value::I32(i32::MAX), right: Value::I32(1), expected: Value::I32(i32::MIN) },
        Case { ty: ScalarType::I32, op: ArithmeticOp::Sub, left: Value::I32(i32::MIN), right: Value::I32(1), expected: Value::I32(i32::MAX) },
        Case { ty: ScalarType::I32, op: ArithmeticOp::Mul, left: Value::I32(i32::MAX), right: Value::I32(0), expected: Value::I32(0) },
        Case { ty: ScalarType::I32, op: ArithmeticOp::Mul, left: Value::I32(i32::MAX), right: Value::I32(2), expected: Value::I32(-2) },

        Case { ty: ScalarType::I64, op: ArithmeticOp::Add, left: Value::I64(i64::MIN), right: Value::I64(0), expected: Value::I64(i64::MIN) },
        Case { ty: ScalarType::I64, op: ArithmeticOp::Add, left: Value::I64(i64::MAX), right: Value::I64(1), expected: Value::I64(i64::MIN) },
        Case { ty: ScalarType::I64, op: ArithmeticOp::Sub, left: Value::I64(i64::MIN), right: Value::I64(1), expected: Value::I64(i64::MAX) },
        Case { ty: ScalarType::I64, op: ArithmeticOp::Mul, left: Value::I64(i64::MAX), right: Value::I64(0), expected: Value::I64(0) },
        Case { ty: ScalarType::I64, op: ArithmeticOp::Mul, left: Value::I64(i64::MAX), right: Value::I64(2), expected: Value::I64(-2) },

        Case { ty: ScalarType::U8, op: ArithmeticOp::Add, left: Value::U8(0), right: Value::U8(0), expected: Value::U8(0) },
        Case { ty: ScalarType::U8, op: ArithmeticOp::Add, left: Value::U8(u8::MAX), right: Value::U8(1), expected: Value::U8(0) },
        Case { ty: ScalarType::U8, op: ArithmeticOp::Sub, left: Value::U8(0), right: Value::U8(1), expected: Value::U8(u8::MAX) },
        Case { ty: ScalarType::U8, op: ArithmeticOp::Mul, left: Value::U8(u8::MAX), right: Value::U8(0), expected: Value::U8(0) },
        Case { ty: ScalarType::U8, op: ArithmeticOp::Mul, left: Value::U8(u8::MAX), right: Value::U8(2), expected: Value::U8(u8::MAX - 1) },

        Case { ty: ScalarType::U16, op: ArithmeticOp::Add, left: Value::U16(0), right: Value::U16(0), expected: Value::U16(0) },
        Case { ty: ScalarType::U16, op: ArithmeticOp::Add, left: Value::U16(u16::MAX), right: Value::U16(1), expected: Value::U16(0) },
        Case { ty: ScalarType::U16, op: ArithmeticOp::Sub, left: Value::U16(0), right: Value::U16(1), expected: Value::U16(u16::MAX) },
        Case { ty: ScalarType::U16, op: ArithmeticOp::Mul, left: Value::U16(u16::MAX), right: Value::U16(0), expected: Value::U16(0) },
        Case { ty: ScalarType::U16, op: ArithmeticOp::Mul, left: Value::U16(u16::MAX), right: Value::U16(2), expected: Value::U16(u16::MAX - 1) },

        Case { ty: ScalarType::U32, op: ArithmeticOp::Add, left: Value::U32(0), right: Value::U32(0), expected: Value::U32(0) },
        Case { ty: ScalarType::U32, op: ArithmeticOp::Add, left: Value::U32(u32::MAX), right: Value::U32(1), expected: Value::U32(0) },
        Case { ty: ScalarType::U32, op: ArithmeticOp::Sub, left: Value::U32(0), right: Value::U32(1), expected: Value::U32(u32::MAX) },
        Case { ty: ScalarType::U32, op: ArithmeticOp::Mul, left: Value::U32(u32::MAX), right: Value::U32(0), expected: Value::U32(0) },
        Case { ty: ScalarType::U32, op: ArithmeticOp::Mul, left: Value::U32(u32::MAX), right: Value::U32(2), expected: Value::U32(u32::MAX - 1) },

        Case { ty: ScalarType::U64, op: ArithmeticOp::Add, left: Value::U64(0), right: Value::U64(0), expected: Value::U64(0) },
        Case { ty: ScalarType::U64, op: ArithmeticOp::Add, left: Value::U64(u64::MAX), right: Value::U64(1), expected: Value::U64(0) },
        Case { ty: ScalarType::U64, op: ArithmeticOp::Sub, left: Value::U64(0), right: Value::U64(1), expected: Value::U64(u64::MAX) },
        Case { ty: ScalarType::U64, op: ArithmeticOp::Mul, left: Value::U64(u64::MAX), right: Value::U64(0), expected: Value::U64(0) },
        Case { ty: ScalarType::U64, op: ArithmeticOp::Mul, left: Value::U64(u64::MAX), right: Value::U64(2), expected: Value::U64(u64::MAX - 1) },
    ];

    for case in cases {
        run(case);
    }
}
