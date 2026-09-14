use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Fault, Function, FunctionId, LocalDecl,
    LocalId, Operand, Place, Program, SafeReferenceResultContract, ScalarType, Statement,
    Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value, validate_program,
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
    .expect("callable realization fixture must be valid Core")
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
        other => panic!("unsupported callable differential observation: {other:?}"),
    }
}

fn reference_outcome(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(validated, entry)
        .expect("callable differential entry must be admitted by reference machine")
        .execute()
        .expect("callable realization subset contains no Core UB operations");
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
        .expect("fixture must be inside callable Wasm realization coverage")
        .execute(entry)
        .expect("supported callable fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

fn callable_type(
    types: &mut TypeTable,
    name: &str,
    parameters: Vec<TypeId>,
    result: Option<TypeId>,
) -> TypeId {
    types.push(TypeDef::callable(
        name,
        CallableInterface::new(parameters, result, SafeReferenceResultContract::None),
    ))
}

fn indirect_entry(name: &str, callable: TypeId, result: TypeId, target: FunctionId) -> Function {
    function(
        name,
        Vec::new(),
        Some(result),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("result", result, false),
        ],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(target),
                }],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            ),
        ],
    )
}

#[test]
fn equal_interface_function_identities_remain_distinct_through_indirect_dispatch() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = callable_type(&mut types, "Thunk", Vec::new(), Some(i64_ty));

    let left_entry = indirect_entry("left_entry", callable, i64_ty, FunctionId(2));
    let right_entry = indirect_entry("right_entry", callable, i64_ty, FunctionId(3));
    let left = function(
        "left",
        Vec::new(),
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(41)))),
        )],
    );
    let right = function(
        "right",
        Vec::new(),
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
        )],
    );
    let program = validated(types, vec![left_entry, right_entry, left, right]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(41)))
    );
    assert_eq!(
        assert_differential(&program, FunctionId(1)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn callable_parameter_composes_direct_and_indirect_calls() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = callable_type(&mut types, "Unary", vec![i64_ty], Some(i64_ty));

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
                        Operand::FunctionValue(FunctionId(2)),
                        Operand::Constant(Value::I64(41)),
                    ],
                    destination: Some(Place::local(LocalId(0))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            ),
        ],
    );
    let apply = function(
        "apply",
        vec![LocalId(0), LocalId(1)],
        Some(i64_ty),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("value", i64_ty, false),
            LocalDecl::new("result", i64_ty, false),
        ],
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: vec![Operand::Move(Place::local(LocalId(1)).into())],
                    destination: Some(Place::local(LocalId(2))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            ),
        ],
    );
    let plus_one = function(
        "plus_one",
        vec![LocalId(0)],
        Some(i64_ty),
        vec![
            LocalDecl::new("value", i64_ty, false),
            LocalDecl::new("result", i64_ty, false),
        ],
        vec![BasicBlock::new(
            vec![Statement::IntegerAdd {
                dst: Place::local(LocalId(1)),
                left: Operand::Move(Place::local(LocalId(0)).into()),
                right: Operand::Constant(Value::I64(1)),
            }],
            Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
        )],
    );
    let program = validated(types, vec![entry, apply, plus_one]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn indirect_call_with_multiple_arguments_matches_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = callable_type(&mut types, "Binary", vec![i64_ty, i64_ty], Some(i64_ty));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("result", i64_ty, false),
        ],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(1)),
                }],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: vec![
                        Operand::Constant(Value::I64(20)),
                        Operand::Constant(Value::I64(22)),
                    ],
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            ),
        ],
    );
    let add = function(
        "add",
        vec![LocalId(0), LocalId(1)],
        Some(i64_ty),
        vec![
            LocalDecl::new("left", i64_ty, false),
            LocalDecl::new("right", i64_ty, false),
            LocalDecl::new("result", i64_ty, false),
        ],
        vec![BasicBlock::new(
            vec![Statement::IntegerAdd {
                dst: Place::local(LocalId(2)),
                left: Operand::Move(Place::local(LocalId(0)).into()),
                right: Operand::Move(Place::local(LocalId(1)).into()),
            }],
            Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
        )],
    );
    let program = validated(types, vec![entry, add]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn no_result_and_defined_fault_indirect_calls_match_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let sink_callable = callable_type(&mut types, "Sink", vec![i64_ty], None);
    let fault_callable = callable_type(&mut types, "FaultThunk", Vec::new(), Some(i64_ty));

    let normal_entry = function(
        "normal_entry",
        Vec::new(),
        Some(i64_ty),
        vec![LocalDecl::new("callee", sink_callable, false)],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(2)),
                }],
                Terminator::IndirectCall {
                    callable: sink_callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: vec![Operand::Constant(Value::I64(7))],
                    destination: None,
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(9)))),
            ),
        ],
    );
    let fault_entry = function(
        "fault_entry",
        Vec::new(),
        Some(i64_ty),
        vec![
            LocalDecl::new("callee", fault_callable, false),
            LocalDecl::new("result", i64_ty, false),
        ],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(3)),
                }],
                Terminator::IndirectCall {
                    callable: fault_callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            ),
        ],
    );
    let sink = function(
        "sink",
        vec![LocalId(0)],
        None,
        vec![LocalDecl::new("value", i64_ty, false)],
        vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
    );
    let boom = function(
        "boom",
        Vec::new(),
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Fault(Fault::new("callable-fault")),
        )],
    );
    let program = validated(types, vec![normal_entry, fault_entry, sink, boom]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(9)))
    );
    assert_eq!(
        assert_differential(&program, FunctionId(1)),
        ExecutionOutcome::Faulted(Fault::new("callable-fault"))
    );
}

#[test]
fn finite_indirect_recursion_matches_reference() {
    let mut types = TypeTable::new();
    let u64_ty = types.push(TypeDef::scalar("U64", ScalarType::U64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let callable = callable_type(&mut types, "Recursive", vec![u64_ty], Some(u64_ty));

    let entry = function(
        "entry",
        Vec::new(),
        Some(u64_ty),
        vec![LocalDecl::new("result", u64_ty, false)],
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::Call {
                    function: FunctionId(1),
                    arguments: vec![Operand::Constant(Value::U64(4))],
                    destination: Some(Place::local(LocalId(0))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            ),
        ],
    );
    let recursive = function(
        "countdown",
        vec![LocalId(0)],
        Some(u64_ty),
        vec![
            LocalDecl::new("n", u64_ty, false),
            LocalDecl::new("zero", bool_ty, false),
            LocalDecl::new("next", u64_ty, false),
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("result", u64_ty, false),
        ],
        vec![
            BasicBlock::new(
                vec![Statement::IntegerEq {
                    dst: Place::local(LocalId(1)),
                    operand_type: u64_ty,
                    left: Operand::Copy(Place::local(LocalId(0)).into()),
                    right: Operand::Constant(Value::U64(0)),
                }],
                Terminator::Branch {
                    condition: Operand::Move(Place::local(LocalId(1)).into()),
                    true_target: BasicBlockId(1),
                    false_target: BasicBlockId(2),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::U64(1)))),
            ),
            BasicBlock::new(
                vec![
                    Statement::IntegerSub {
                        dst: Place::local(LocalId(2)),
                        left: Operand::Move(Place::local(LocalId(0)).into()),
                        right: Operand::Constant(Value::U64(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(3)),
                        src: Operand::FunctionValue(FunctionId(1)),
                    },
                ],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(3)).into()),
                    arguments: vec![Operand::Move(Place::local(LocalId(2)).into())],
                    destination: Some(Place::local(LocalId(4))),
                    target: BasicBlockId(3),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(4)).into()))),
            ),
        ],
    );
    let program = validated(types, vec![entry, recursive]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::U64(1)))
    );
}
