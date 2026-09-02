use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, Value, validate_program,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus, VerificationEventKind};

fn execute_direct_result(scalar: ScalarType, value: Value) -> runen_reference::ExecutionReport {
    let mut types = TypeTable::new();
    let result_ty = types.push(TypeDef::scalar("result", scalar));
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(result_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: Vec::new(),
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(value))),
                )],
            },
        }],
    };
    let validated = validate_program(program).expect("matching constant result must validate");
    Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("fixed-width integer return is defined")
}

#[test]
fn entry_results_preserve_every_fixed_width_integer_variant() {
    let cases = [
        (
            ScalarType::I8,
            Value::I8(i8::MIN),
            ObservedValue::I8(i8::MIN),
        ),
        (
            ScalarType::I16,
            Value::I16(i16::MIN),
            ObservedValue::I16(i16::MIN),
        ),
        (
            ScalarType::I32,
            Value::I32(i32::MIN),
            ObservedValue::I32(i32::MIN),
        ),
        (
            ScalarType::I64,
            Value::I64(i64::MIN),
            ObservedValue::I64(i64::MIN),
        ),
        (
            ScalarType::U8,
            Value::U8(u8::MAX),
            ObservedValue::U8(u8::MAX),
        ),
        (
            ScalarType::U16,
            Value::U16(u16::MAX),
            ObservedValue::U16(u16::MAX),
        ),
        (
            ScalarType::U32,
            Value::U32(u32::MAX),
            ObservedValue::U32(u32::MAX),
        ),
        (
            ScalarType::U64,
            Value::U64(u64::MAX),
            ObservedValue::U64(u64::MAX),
        ),
    ];

    for (scalar, value, expected) in cases {
        let report = execute_direct_result(scalar, value);
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
        assert!(report.verification_events.is_empty());
    }
}

#[test]
fn non_i64_values_survive_init_copy_assign_and_move() {
    let mut types = TypeTable::new();
    let u32_ty = types.push(TypeDef::scalar("u32", ScalarType::U32));
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(u32_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![
                    LocalDecl::new("source", u32_ty, false),
                    LocalDecl::new("target", u32_ty, true),
                ],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::U32(7)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::Copy(Place::local(LocalId(0)).into()),
                        },
                        Statement::Assign {
                            dst: Place::local(LocalId(1)).into(),
                            src: Operand::Constant(Value::U32(u32::MAX)),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
                )],
            },
        }],
    };

    let validated = validate_program(program).expect("u32 transport program must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("u32 transport is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::U32(u32::MAX)));
    assert!(
        !report
            .verification_events
            .iter()
            .any(|event| matches!(event.kind, VerificationEventKind::DropTrackedFixture { .. })),
        "ordinary integer transport must not invent integer-specific cleanup events"
    );
}

#[test]
fn u64_max_round_trips_through_call_argument_and_result() {
    let mut types = TypeTable::new();
    let u64_ty = types.push(TypeDef::scalar("u64", ScalarType::U64));

    let caller = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(u64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("result", u64_ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Constant(Value::U64(u64::MAX))],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        },
    };
    let callee = Function {
        name: "identity".into(),
        parameters: vec![LocalId(0)],
        result: Some(u64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("value", u64_ty, false)],
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
        functions: vec![caller, callee],
    })
    .expect("u64 boundary call must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("u64 boundary call is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::U64(u64::MAX)));
}

#[test]
fn mixed_fixed_width_struct_round_trips_through_storage() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("i8", ScalarType::I8));
    let u64_ty = types.push(TypeDef::scalar("u64", ScalarType::U64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("small", i8_ty), Field::new("large", u64_ty)],
    ));
    let input = Value::Struct(vec![Value::I8(i8::MIN), Value::U64(u64::MAX)]);
    let expected = ObservedValue::Struct(vec![
        ObservedValue::I8(i8::MIN),
        ObservedValue::U64(u64::MAX),
    ]);
    let program = Program {
        types,
        functions: vec![Function {
            name: "entry".into(),
            parameters: Vec::new(),
            result: Some(pair_ty),
            safe_reference_result_contract: SafeReferenceResultContract::None,
            body: Body {
                locals: vec![LocalDecl::new("pair", pair_ty, false)],
                loans: Vec::new(),
                entry: BasicBlockId(0),
                blocks: vec![BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(input),
                    }],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                )],
            },
        }],
    };

    let validated = validate_program(program).expect("mixed integer struct must validate");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("mixed integer struct execution is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(expected));
}
