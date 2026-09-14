use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Field, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId,
    TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{ExecutionOutcome, RealizedProgram};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

fn function(
    name: &str,
    parameters: Vec<LocalId>,
    result: Option<TypeId>,
    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
) -> Function {
    Function {
        name: name.into(),
        parameters,
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals,
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks,
        },
    }
}

fn validated(types: TypeTable, functions: Vec<Function>) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions,
    })
    .expect("aggregate fixture must be valid Core")
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
        ObservedValue::Struct(values) => {
            Value::Struct(values.into_iter().map(observed_to_value).collect())
        }
        other => panic!("unsupported aggregate differential observation: {other:?}"),
    }
}

fn reference_outcome(program: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(program, entry)
        .expect("aggregate differential entry must be admitted by reference machine")
        .execute()
        .expect("aggregate realization subset contains no Core UB operations");
    match report.terminal {
        TerminalStatus::Returned => {
            ExecutionOutcome::Returned(report.result.map(observed_to_value))
        }
        TerminalStatus::Faulted(code) => ExecutionOutcome::Faulted(Fault::new(code)),
    }
}

fn assert_differential(program: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let expected = reference_outcome(program.clone(), entry);
    let actual = RealizedProgram::new(&program)
        .expect("aggregate fixture must be inside Wasm realization coverage")
        .execute(entry)
        .expect("aggregate fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

#[test]
fn nested_and_empty_aggregate_entry_results_match_reference() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("I8", ScalarType::I8));
    let u64_ty = types.push(TypeDef::scalar("U64", ScalarType::U64));
    let inner_ty = types.push(TypeDef::structure(
        "Inner",
        vec![Field::new("small", i8_ty)],
    ));
    let outer_ty = types.push(TypeDef::structure(
        "Outer",
        vec![Field::new("inner", inner_ty), Field::new("large", u64_ty)],
    ));
    let value = Value::Struct(vec![
        Value::Struct(vec![Value::I8(-7)]),
        Value::U64(u64::MAX),
    ]);
    let nested = validated(
        types,
        vec![function(
            "entry",
            Vec::new(),
            Some(outer_ty),
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(value.clone()))),
            )],
        )],
    );
    assert_eq!(
        assert_differential(nested, FunctionId(0)),
        ExecutionOutcome::Returned(Some(value))
    );

    let mut empty_types = TypeTable::new();
    let empty_ty = empty_types.push(TypeDef::structure("Empty", Vec::new()));
    let empty = Value::Struct(Vec::new());
    let empty_program = validated(
        empty_types,
        vec![function(
            "entry",
            Vec::new(),
            Some(empty_ty),
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(empty.clone()))),
            )],
        )],
    );
    assert_eq!(
        assert_differential(empty_program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(empty))
    );
}

#[test]
fn projected_move_copy_and_integer_operation_match_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let pair = Place::local(LocalId(0));
    let result = Place::local(LocalId(1));
    let program = validated(
        types,
        vec![function(
            "entry",
            Vec::new(),
            Some(i64_ty),
            vec![
                LocalDecl::new("pair", pair_ty, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: pair.clone(),
                        src: Operand::Constant(Value::Struct(vec![
                            Value::I64(20),
                            Value::I64(22),
                        ])),
                    },
                    Statement::IntegerAdd {
                        dst: result.clone(),
                        left: Operand::Copy(pair.clone().field(0).into()),
                        right: Operand::Move(pair.field(1).into()),
                    },
                ],
                Terminator::Return(Some(Operand::Move(result.into()))),
            )],
        )],
    );
    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn whole_aggregate_copy_then_move_transport_matches_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let source = Place::local(LocalId(0));
    let copied = Place::local(LocalId(1));
    let moved = Place::local(LocalId(2));
    let value = Value::Struct(vec![Value::I64(17), Value::I64(25)]);
    let program = validated(
        types,
        vec![function(
            "entry",
            Vec::new(),
            Some(pair_ty),
            vec![
                LocalDecl::new("source", pair_ty, false),
                LocalDecl::new("copied", pair_ty, false),
                LocalDecl::new("moved", pair_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: source.clone(),
                        src: Operand::Constant(value.clone()),
                    },
                    Statement::Init {
                        dst: copied.clone(),
                        src: Operand::Copy(source.into()),
                    },
                    Statement::Init {
                        dst: moved.clone(),
                        src: Operand::Move(copied.into()),
                    },
                ],
                Terminator::Return(Some(Operand::Move(moved.into()))),
            )],
        )],
    );
    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(value))
    );
}

#[test]
fn projected_assign_read_and_drop_preserve_structural_state() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("kept", i64_ty), Field::new("dropped", i64_ty)],
    ));
    let pair = Place::local(LocalId(0));
    let program = validated(
        types,
        vec![function(
            "entry",
            Vec::new(),
            Some(i64_ty),
            vec![LocalDecl::new("pair", pair_ty, true)],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: pair.clone(),
                        src: Operand::Constant(Value::Struct(vec![
                            Value::I64(1),
                            Value::I64(99),
                        ])),
                    },
                    Statement::Assign {
                        dst: pair.clone().field(0).into(),
                        src: Operand::Constant(Value::I64(42)),
                    },
                    Statement::Read {
                        src: pair.clone().field(1).into(),
                    },
                    Statement::Drop {
                        place: pair.clone().field(1).into(),
                    },
                ],
                Terminator::Return(Some(Operand::Move(pair.field(0).into()))),
            )],
        )],
    );
    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn direct_aggregate_parameter_and_result_match_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let destination = Place::local(LocalId(0));
    let entry = function(
        "entry",
        Vec::new(),
        Some(pair_ty),
        vec![LocalDecl::new("result", pair_ty, false)],
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::Call {
                    function: FunctionId(1),
                    arguments: vec![Operand::Constant(Value::Struct(vec![
                        Value::I64(20),
                        Value::I64(22),
                    ]))],
                    destination: Some(destination.clone()),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(destination.into()))),
            ),
        ],
    );
    let identity = function(
        "identity",
        vec![LocalId(0)],
        Some(pair_ty),
        vec![LocalDecl::new("value", pair_ty, false)],
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
        )],
    );
    let program = validated(types, vec![entry, identity]);
    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::Struct(vec![
            Value::I64(20),
            Value::I64(22),
        ])))
    );
}

#[test]
fn aggregate_result_fault_propagates_exact_identity() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let destination = Place::local(LocalId(0));
    let entry = function(
        "entry",
        Vec::new(),
        Some(pair_ty),
        vec![LocalDecl::new("result", pair_ty, false)],
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::Call {
                    function: FunctionId(1),
                    arguments: Vec::new(),
                    destination: Some(destination.clone()),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Fault(Fault::new("normal-continuation-ran")),
            ),
        ],
    );
    let faulting = function(
        "faulting",
        Vec::new(),
        Some(pair_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Fault(Fault::new("aggregate-fault")),
        )],
    );
    let program = validated(types, vec![entry, faulting]);
    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Faulted(Fault::new("aggregate-fault"))
    );
}

#[test]
fn closure_environment_shape_uses_generic_structural_direct_call_transport() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let environment_ty = types.push(TypeDef::structure(
        "$closure-env-0",
        vec![Field::new("$capture0", i64_ty), Field::new("$capture1", i64_ty)],
    ));
    let result = Place::local(LocalId(0));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![LocalDecl::new("result", i64_ty, false)],
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::Call {
                    function: FunctionId(1),
                    arguments: vec![
                        Operand::Constant(Value::I64(0)),
                        Operand::Constant(Value::Struct(vec![Value::I64(20), Value::I64(22)])),
                    ],
                    destination: Some(result.clone()),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(result.into()))),
            ),
        ],
    );
    let argument = Place::local(LocalId(0));
    let environment = Place::local(LocalId(1));
    let first = Place::local(LocalId(2));
    let captures = Place::local(LocalId(3));
    let wrapper = function(
        "$closure-wrapper-0",
        vec![LocalId(0), LocalId(1)],
        Some(i64_ty),
        vec![
            LocalDecl::new("value", i64_ty, false),
            LocalDecl::new("$environment", environment_ty, false),
            LocalDecl::new("$first", i64_ty, false),
            LocalDecl::new("$captures", i64_ty, false),
        ],
        vec![BasicBlock::new(
            vec![
                Statement::IntegerAdd {
                    dst: captures.clone(),
                    left: Operand::Copy(environment.clone().field(0).into()),
                    right: Operand::Move(environment.field(1).into()),
                },
                Statement::IntegerAdd {
                    dst: first.clone(),
                    left: Operand::Move(argument.into()),
                    right: Operand::Move(captures.into()),
                },
            ],
            Terminator::Return(Some(Operand::Move(first.into()))),
        )],
    );
    let program = validated(types, vec![entry, wrapper]);
    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}
