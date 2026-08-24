use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{
    ExecutionReport, Machine, TerminalStatus, VerificationEventKind, VerificationWriteKind,
};

fn execute_integer_sub(scalar: ScalarType, left: Value, right: Value) -> ExecutionReport {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("integer", scalar));
    let left_place = Place::local(LocalId(0));
    let right_place = Place::local(LocalId(1));
    let result_place = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(ty),
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
                    Statement::IntegerSub {
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
    .expect("integer-sub fixture must be valid Core");
    Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("safe IntegerSub execution is defined")
}

#[test]
fn integer_sub_executes_all_eight_fixed_width_integer_kinds() {
    let cases = [
        (ScalarType::I8, Value::I8(2), Value::I8(3), Value::I8(-1)),
        (
            ScalarType::I16,
            Value::I16(-8),
            Value::I16(11),
            Value::I16(-19),
        ),
        (
            ScalarType::I32,
            Value::I32(20),
            Value::I32(-7),
            Value::I32(27),
        ),
        (
            ScalarType::I64,
            Value::I64(-30),
            Value::I64(-12),
            Value::I64(-18),
        ),
        (ScalarType::U8, Value::U8(5), Value::U8(3), Value::U8(2)),
        (
            ScalarType::U16,
            Value::U16(19),
            Value::U16(11),
            Value::U16(8),
        ),
        (
            ScalarType::U32,
            Value::U32(27),
            Value::U32(7),
            Value::U32(20),
        ),
        (
            ScalarType::U64,
            Value::U64(42),
            Value::U64(12),
            Value::U64(30),
        ),
    ];

    for (scalar, left, right, expected) in cases {
        let report = execute_integer_sub(scalar, left, right);
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn integer_sub_wraps_at_every_signed_and_unsigned_width_boundary() {
    let cases = [
        (
            ScalarType::I8,
            Value::I8(i8::MIN),
            Value::I8(1),
            Value::I8(i8::MAX),
        ),
        (
            ScalarType::I16,
            Value::I16(i16::MIN),
            Value::I16(1),
            Value::I16(i16::MAX),
        ),
        (
            ScalarType::I32,
            Value::I32(i32::MIN),
            Value::I32(1),
            Value::I32(i32::MAX),
        ),
        (
            ScalarType::I64,
            Value::I64(i64::MIN),
            Value::I64(1),
            Value::I64(i64::MAX),
        ),
        (
            ScalarType::U8,
            Value::U8(0),
            Value::U8(1),
            Value::U8(u8::MAX),
        ),
        (
            ScalarType::U16,
            Value::U16(0),
            Value::U16(1),
            Value::U16(u16::MAX),
        ),
        (
            ScalarType::U32,
            Value::U32(0),
            Value::U32(1),
            Value::U32(u32::MAX),
        ),
        (
            ScalarType::U64,
            Value::U64(0),
            Value::U64(1),
            Value::U64(u64::MAX),
        ),
    ];

    for (scalar, left, right, expected) in cases {
        let report = execute_integer_sub(scalar, left, right);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn integer_sub_moves_left_then_right_once_and_records_one_integer_sub_write() {
    let report = execute_integer_sub(ScalarType::I8, Value::I8(40), Value::I8(2));

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
                    kind: VerificationWriteKind::IntegerSub,
                } if *place == result
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();

    assert_eq!(left_moves.len(), 1, "left operand must be consumed once");
    assert_eq!(right_moves.len(), 1, "right operand must be consumed once");
    assert_eq!(writes.len(), 1, "IntegerSub must write its result once");
    assert!(left_moves[0] < right_moves[0]);
    assert!(right_moves[0] < writes[0]);
}
