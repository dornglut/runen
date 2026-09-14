use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand,
    PersistentDecl, PersistentId, Place, Program, SafeReferenceResultContract, ScalarType,
    Statement, Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{ExecutionOutcome, RealizedProgram};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

fn function(name: &str, parameters: Vec<LocalId>, result: Option<TypeId>, body: Body) -> Function {
    Function {
        name: name.into(),
        parameters,
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body,
    }
}

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn validated(
    types: TypeTable,
    persistent: Vec<PersistentDecl>,
    functions: Vec<Function>,
) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent,
        external_callables: Vec::new(),
        functions,
    })
    .expect("persistent realization fixture must be valid Core")
}

fn observed_to_value(value: ObservedValue) -> Value {
    match value {
        ObservedValue::Bool(value) => Value::Bool(value),
        ObservedValue::I8(value) => Value::I8(value),
        ObservedValue::I16(value) => Value::I16(value),
        ObservedValue::I32(value) => Value::I32(value),
        ObservedValue::I64(value) => Value::I64(value),
        ObservedValue::U8(value) => Value::U8(value),
        ObservedValue::U16(value) => Value::U16(value),
        ObservedValue::U32(value) => Value::U32(value),
        ObservedValue::U64(value) => Value::U64(value),
        other => panic!("unsupported persistent differential observation: {other:?}"),
    }
}

fn reference_outcome(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(validated, entry)
        .expect("persistent differential entry must be admitted by reference machine")
        .execute()
        .expect("persistent realization subset contains no Core UB operations");
    match report.terminal {
        TerminalStatus::Returned => {
            ExecutionOutcome::Returned(report.result.map(observed_to_value))
        }
        TerminalStatus::Faulted(code) => ExecutionOutcome::Faulted(Fault::new(code)),
    }
}

fn assert_differential(validated: &ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let expected = reference_outcome(validated.clone(), entry);
    let actual = RealizedProgram::new(validated)
        .expect("fixture must be inside persistent Wasm realization coverage")
        .execute(entry)
        .expect("supported persistent fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

fn persistent_return_program(scalar: ScalarType, value: Value) -> ValidatedProgram {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("T", scalar));
    validated(
        types,
        vec![PersistentDecl::new(ty, value)],
        vec![function(
            "entry",
            Vec::new(),
            Some(ty),
            body(
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::PersistentRead(PersistentId(0)))),
                )],
            ),
        )],
    )
}

#[test]
fn persistent_reads_round_trip_all_supported_scalar_boundaries() {
    let cases = [
        (ScalarType::Bool, Value::Bool(false)),
        (ScalarType::Bool, Value::Bool(true)),
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
        let program = persistent_return_program(scalar, value.clone());
        assert_eq!(
            assert_differential(&program, FunctionId(0)),
            ExecutionOutcome::Returned(Some(value))
        );
    }
}

#[test]
fn persistent_reads_compose_with_integer_ops_and_nested_direct_calls() {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("U64", ScalarType::U64));

    let entry = function(
        "entry",
        Vec::new(),
        Some(ty),
        body(
            vec![LocalDecl::new("result", ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        ),
    );

    let middle = function(
        "middle",
        Vec::new(),
        Some(ty),
        body(
            vec![LocalDecl::new("result", ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(2),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        ),
    );

    let leaf = function(
        "leaf",
        Vec::new(),
        Some(ty),
        body(
            vec![LocalDecl::new("sum", ty, false)],
            vec![BasicBlock::new(
                vec![Statement::IntegerAdd {
                    dst: Place::local(LocalId(0)),
                    left: Operand::PersistentRead(PersistentId(0)),
                    right: Operand::PersistentRead(PersistentId(1)),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    );

    let program = validated(
        types,
        vec![
            PersistentDecl::new(ty, Value::U64(40)),
            PersistentDecl::new(ty, Value::U64(2)),
        ],
        vec![entry, middle, leaf],
    );

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::U64(42)))
    );
}

#[test]
fn repeated_execute_reestablishes_the_same_persistent_initial_state() {
    let program = persistent_return_program(ScalarType::I64, Value::I64(-42));
    let realized = RealizedProgram::new(&program).expect("persistent program must realize");
    let expected = ExecutionOutcome::Returned(Some(Value::I64(-42)));

    assert_eq!(
        realized
            .execute(FunctionId(0))
            .expect("first represented execution must succeed"),
        expected
    );
    assert_eq!(
        realized
            .execute(FunctionId(0))
            .expect("second represented execution must succeed"),
        expected
    );
}

#[test]
fn equal_valued_declarations_remain_distinct_and_unused_persistents_are_admitted() {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let program = validated(
        types,
        vec![
            PersistentDecl::new(ty, Value::I64(7)),
            PersistentDecl::new(ty, Value::I64(7)),
        ],
        vec![function(
            "entry",
            Vec::new(),
            None,
            body(
                Vec::new(),
                vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
            ),
        )],
    );

    assert_eq!(program.as_program().persistent.len(), 2);
    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(None)
    );
}

#[test]
fn defined_fault_identity_is_unchanged_by_persistent_establishment() {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("U32", ScalarType::U32));
    let program = validated(
        types,
        vec![PersistentDecl::new(ty, Value::U32(9))],
        vec![function(
            "entry",
            Vec::new(),
            None,
            body(
                Vec::new(),
                vec![BasicBlock::new(
                    Vec::new(),
                    Terminator::Fault(Fault::new("persistent-fault")),
                )],
            ),
        )],
    );

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Faulted(Fault::new("persistent-fault"))
    );
}
