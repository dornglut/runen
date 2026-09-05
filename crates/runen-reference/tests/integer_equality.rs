use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeTable,
    Value, validate_program,
};
use runen_reference::{
    ExecutionReport, Machine, ObservedValue, TerminalStatus, VerificationEventKind,
    VerificationWriteKind,
};

fn execute_integer_eq(scalar: ScalarType, left: Value, right: Value) -> ExecutionReport {
    let mut types = TypeTable::new();
    let operand_type = types.push(TypeDef::scalar("integer", scalar));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let left_place = Place::local(LocalId(0));
    let right_place = Place::local(LocalId(1));
    let result_place = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(bool_type),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![
                LocalDecl::new("left", operand_type, false),
                LocalDecl::new("right", operand_type, false),
                LocalDecl::new("result", bool_type, false),
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
                    Statement::IntegerEq {
                        dst: result_place.clone(),
                        operand_type,
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
    .expect("integer-equality fixture must be valid Core");
    Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("safe IntegerEq execution is defined")
}

#[test]
fn integer_eq_executes_true_and_false_for_all_eight_fixed_width_integer_kinds() {
    let cases = [
        (ScalarType::I8, Value::I8(-5), Value::I8(-5), true),
        (ScalarType::I8, Value::I8(i8::MIN), Value::I8(i8::MAX), false),
        (ScalarType::I16, Value::I16(i16::MIN), Value::I16(i16::MIN), true),
        (ScalarType::I16, Value::I16(-1), Value::I16(0), false),
        (ScalarType::I32, Value::I32(i32::MAX), Value::I32(i32::MAX), true),
        (ScalarType::I32, Value::I32(-17), Value::I32(17), false),
        (ScalarType::I64, Value::I64(i64::MIN), Value::I64(i64::MIN), true),
        (ScalarType::I64, Value::I64(i64::MAX), Value::I64(i64::MIN), false),
        (ScalarType::U8, Value::U8(0), Value::U8(0), true),
        (ScalarType::U8, Value::U8(0), Value::U8(u8::MAX), false),
        (ScalarType::U16, Value::U16(u16::MAX), Value::U16(u16::MAX), true),
        (ScalarType::U16, Value::U16(1), Value::U16(2), false),
        (ScalarType::U32, Value::U32(42), Value::U32(42), true),
        (ScalarType::U32, Value::U32(u32::MAX), Value::U32(0), false),
        (ScalarType::U64, Value::U64(u64::MAX), Value::U64(u64::MAX), true),
        (ScalarType::U64, Value::U64(123), Value::U64(124), false),
    ];

    for (scalar, left, right, expected) in cases {
        let report = execute_integer_eq(scalar, left, right);
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(ObservedValue::Bool(expected)));
    }
}

#[test]
fn integer_eq_constant_constant_execution_uses_explicit_operand_type() {
    let mut types = TypeTable::new();
    let operand_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let result = Place::local(LocalId(0));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(bool_type),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("result", bool_type, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![Statement::IntegerEq {
                    dst: result.clone(),
                    operand_type,
                    left: Operand::Constant(Value::I8(-7)),
                    right: Operand::Constant(Value::I8(-7)),
                }],
                Terminator::Return(Some(Operand::Move(result.into()))),
            )],
        },
    };
    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("constant/constant equality must be typed by explicit operand_type");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("constant equality execution is defined");

    assert_eq!(report.result, Some(ObservedValue::Bool(true)));
}

#[test]
fn integer_eq_moves_left_then_right_once_and_records_one_bool_result_write() {
    let report = execute_integer_eq(ScalarType::I8, Value::I8(6), Value::I8(6));

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
                    kind: VerificationWriteKind::IntegerEq,
                } if *place == result
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();

    assert_eq!(left_moves.len(), 1, "left operand must be consumed once");
    assert_eq!(right_moves.len(), 1, "right operand must be consumed once");
    assert_eq!(writes.len(), 1, "IntegerEq must write its Bool result once");
    assert!(left_moves[0] < right_moves[0]);
    assert!(right_moves[0] < writes[0]);
}

fn execute_branching_integer_eq(left: i8, right: i8) -> ExecutionReport {
    let mut types = TypeTable::new();
    let operand_type = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let bool_type = types.push(TypeDef::scalar("bool", ScalarType::Bool));
    let result = Place::local(LocalId(0));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(bool_type),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("eq-result", bool_type, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    vec![Statement::IntegerEq {
                        dst: result.clone(),
                        operand_type,
                        left: Operand::Constant(Value::I8(left)),
                        right: Operand::Constant(Value::I8(right)),
                    }],
                    Terminator::Branch {
                        condition: Operand::Move(result.into()),
                        true_target: BasicBlockId(1),
                        false_target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(Value::Bool(true)))),
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(Value::Bool(false)))),
                ),
            ],
        },
    };
    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("IntegerEq Bool result must be an ordinary Branch condition");
    Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("branching equality execution is defined")
}

#[test]
fn integer_eq_result_drives_existing_bool_branch_without_fused_predicate() {
    assert_eq!(
        execute_branching_integer_eq(-9, -9).result,
        Some(ObservedValue::Bool(true))
    );
    assert_eq!(
        execute_branching_integer_eq(-9, 9).result,
        Some(ObservedValue::Bool(false))
    );
}
