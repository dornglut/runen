use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{
    ExecutionReport, Machine, ObservedValue, TerminalStatus, VerificationEventKind,
    VerificationWriteKind,
};

fn execute_integer_mul(scalar: ScalarType, left: Value, right: Value) -> ExecutionReport {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("integer", scalar));
    let left_place = Place::local(LocalId(0));
    let right_place = Place::local(LocalId(1));
    let result_place = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(ty),
        shared_reference_result_origin: None,
        body: Body {
            locals: vec![
                LocalDecl::new("left", ty, false),
                LocalDecl::new("right", ty, false),
                LocalDecl::new("result", ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: left_place.clone(),
                        src: Operand::Constant(left),
                    },
                    Statement::Init {
                        dst: right_place.clone(),
                        src: Operand::Constant(right),
                    },
                    Statement::IntegerMul {
                        dst: result_place.clone(),
                        left: Operand::Move(left_place.into()),
                        right: Operand::Move(right_place.into()),
                    },
                ],
                Terminator::Return(Some(Operand::Move(result_place.into()))),
            )],
        },
    };
    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("integer-mul fixture must be valid Core");
    Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("safe IntegerMul execution is defined")
}

#[test]
fn integer_mul_executes_all_eight_fixed_width_integer_kinds() {
    let cases = [
        (
            ScalarType::I8,
            Value::I8(-3),
            Value::I8(4),
            ObservedValue::I8(-12),
        ),
        (
            ScalarType::I16,
            Value::I16(-8),
            Value::I16(-11),
            ObservedValue::I16(88),
        ),
        (
            ScalarType::I32,
            Value::I32(20),
            Value::I32(-7),
            ObservedValue::I32(-140),
        ),
        (
            ScalarType::I64,
            Value::I64(30),
            Value::I64(12),
            ObservedValue::I64(360),
        ),
        (
            ScalarType::U8,
            Value::U8(5),
            Value::U8(3),
            ObservedValue::U8(15),
        ),
        (
            ScalarType::U16,
            Value::U16(19),
            Value::U16(11),
            ObservedValue::U16(209),
        ),
        (
            ScalarType::U32,
            Value::U32(27),
            Value::U32(7),
            ObservedValue::U32(189),
        ),
        (
            ScalarType::U64,
            Value::U64(42),
            Value::U64(12),
            ObservedValue::U64(504),
        ),
    ];

    for (scalar, left, right, expected) in cases {
        let report = execute_integer_mul(scalar, left, right);
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn integer_mul_wraps_signed_positive_and_negative_overflow_at_every_width() {
    let cases = [
        (
            ScalarType::I8,
            Value::I8(i8::MAX),
            Value::I8(2),
            ObservedValue::I8(-2),
        ),
        (
            ScalarType::I8,
            Value::I8(i8::MIN),
            Value::I8(2),
            ObservedValue::I8(0),
        ),
        (
            ScalarType::I16,
            Value::I16(i16::MAX),
            Value::I16(2),
            ObservedValue::I16(-2),
        ),
        (
            ScalarType::I16,
            Value::I16(i16::MIN),
            Value::I16(2),
            ObservedValue::I16(0),
        ),
        (
            ScalarType::I32,
            Value::I32(i32::MAX),
            Value::I32(2),
            ObservedValue::I32(-2),
        ),
        (
            ScalarType::I32,
            Value::I32(i32::MIN),
            Value::I32(2),
            ObservedValue::I32(0),
        ),
        (
            ScalarType::I64,
            Value::I64(i64::MAX),
            Value::I64(2),
            ObservedValue::I64(-2),
        ),
        (
            ScalarType::I64,
            Value::I64(i64::MIN),
            Value::I64(2),
            ObservedValue::I64(0),
        ),
    ];

    for (scalar, left, right, expected) in cases {
        let report = execute_integer_mul(scalar, left, right);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn integer_mul_wraps_unsigned_overflow_at_every_width() {
    let cases = [
        (
            ScalarType::U8,
            Value::U8(u8::MAX),
            Value::U8(2),
            ObservedValue::U8(u8::MAX - 1),
        ),
        (
            ScalarType::U16,
            Value::U16(u16::MAX),
            Value::U16(2),
            ObservedValue::U16(u16::MAX - 1),
        ),
        (
            ScalarType::U32,
            Value::U32(u32::MAX),
            Value::U32(2),
            ObservedValue::U32(u32::MAX - 1),
        ),
        (
            ScalarType::U64,
            Value::U64(u64::MAX),
            Value::U64(2),
            ObservedValue::U64(u64::MAX - 1),
        ),
    ];

    for (scalar, left, right, expected) in cases {
        let report = execute_integer_mul(scalar, left, right);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn integer_mul_moves_left_then_right_once_and_records_one_integer_mul_write() {
    let report = execute_integer_mul(ScalarType::I8, Value::I8(6), Value::I8(7));

    let left = Place::local(LocalId(0));
    let right = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let left_moves = report
        .verification_events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            matches!(
                &event.kind,
                VerificationEventKind::Move(place) if *place == left
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();
    let right_moves = report
        .verification_events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            matches!(
                &event.kind,
                VerificationEventKind::Move(place) if *place == right
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();
    let writes = report
        .verification_events
        .iter()
        .enumerate()
        .filter_map(|(index, event)| {
            matches!(
                &event.kind,
                VerificationEventKind::Write {
                    place,
                    kind: VerificationWriteKind::IntegerMul,
                } if *place == result
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();

    assert_eq!(left_moves.len(), 1, "left operand must be consumed once");
    assert_eq!(right_moves.len(), 1, "right operand must be consumed once");
    assert_eq!(writes.len(), 1, "IntegerMul must write its result once");
    assert!(left_moves[0] < right_moves[0]);
    assert!(right_moves[0] < writes[0]);
}
