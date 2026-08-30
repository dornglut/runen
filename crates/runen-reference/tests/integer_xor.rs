use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{
    ExecutionReport, Machine, ObservedValue, TerminalStatus, VerificationEventKind,
    VerificationWriteKind,
};

fn execute_integer_xor(scalar: ScalarType, left: Value, right: Value) -> ExecutionReport {
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
                    Statement::IntegerXor {
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
    .expect("integer-XOR fixture must be valid Core");
    Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("safe IntegerXor execution is defined")
}

#[test]
fn integer_xor_executes_all_eight_fixed_width_integer_kinds() {
    let cases = [
        (
            ScalarType::I8,
            Value::I8(-5),
            Value::I8(3),
            ObservedValue::I8(-8),
        ),
        (
            ScalarType::I16,
            Value::I16(i16::MIN),
            Value::I16(-1),
            ObservedValue::I16(i16::MAX),
        ),
        (
            ScalarType::I32,
            Value::I32(i32::MAX),
            Value::I32(-1),
            ObservedValue::I32(i32::MIN),
        ),
        (
            ScalarType::I64,
            Value::I64(-1),
            Value::I64(0),
            ObservedValue::I64(-1),
        ),
        (
            ScalarType::U8,
            Value::U8(0),
            Value::U8(u8::MAX),
            ObservedValue::U8(u8::MAX),
        ),
        (
            ScalarType::U16,
            Value::U16(0x00ff),
            Value::U16(0x0f0f),
            ObservedValue::U16(0x0ff0),
        ),
        (
            ScalarType::U32,
            Value::U32(0xaaaa_5555),
            Value::U32(0x0f0f_f0f0),
            ObservedValue::U32(0xa5a5_a5a5),
        ),
        (
            ScalarType::U64,
            Value::U64(42),
            Value::U64(15),
            ObservedValue::U64(37),
        ),
    ];

    for (scalar, left, right, expected) in cases {
        let report = execute_integer_xor(scalar, left, right);
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn signed_xor_maps_canonical_residues_back_to_the_signed_domain() {
    let cases = [
        (
            ScalarType::I8,
            Value::I8(i8::MAX),
            Value::I8(-1),
            ObservedValue::I8(i8::MIN),
        ),
        (
            ScalarType::I8,
            Value::I8(i8::MIN),
            Value::I8(-1),
            ObservedValue::I8(i8::MAX),
        ),
        (
            ScalarType::I8,
            Value::I8(-5),
            Value::I8(-3),
            ObservedValue::I8(6),
        ),
        (
            ScalarType::I16,
            Value::I16(i16::MAX),
            Value::I16(-1),
            ObservedValue::I16(i16::MIN),
        ),
        (
            ScalarType::I32,
            Value::I32(i32::MIN),
            Value::I32(-1),
            ObservedValue::I32(i32::MAX),
        ),
        (
            ScalarType::I64,
            Value::I64(i64::MAX),
            Value::I64(-1),
            ObservedValue::I64(i64::MIN),
        ),
    ];

    for (scalar, left, right, expected) in cases {
        let report = execute_integer_xor(scalar, left, right);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn integer_xor_moves_left_then_right_once_and_records_one_integer_xor_write() {
    let report = execute_integer_xor(ScalarType::I8, Value::I8(6), Value::I8(3));

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
                    kind: VerificationWriteKind::IntegerXor,
                } if *place == result
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();

    assert_eq!(left_moves.len(), 1, "left operand must be consumed once");
    assert_eq!(right_moves.len(), 1, "right operand must be consumed once");
    assert_eq!(writes.len(), 1, "IntegerXor must write its result once");
    assert!(left_moves[0] < right_moves[0]);
    assert!(right_moves[0] < writes[0]);
}
