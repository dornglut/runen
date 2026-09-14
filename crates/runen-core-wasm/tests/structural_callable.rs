use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Fault, Field, Function, FunctionId,
    LocalDecl, LocalId, Operand, Place, Program, SafeReferenceResultContract, ScalarType, Statement,
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
    .expect("structural callable fixture must be valid Core")
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
        ObservedValue::Struct(fields) => {
            Value::Struct(fields.into_iter().map(observed_to_value).collect())
        }
        other => panic!("unsupported structural callable observation: {other:?}"),
    }
}

fn reference_outcome(program: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(program, entry)
        .expect("structural callable entry must be admitted by reference machine")
        .execute()
        .expect("structural callable realization subset contains no Core UB operations");
    match report.terminal {
        TerminalStatus::Returned => {
            ExecutionOutcome::Returned(report.result.map(observed_to_value))
        }
        TerminalStatus::Faulted(code) => ExecutionOutcome::Faulted(Fault::new(code)),
    }
}

fn assert_differential(program: &ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let expected = reference_outcome(program.clone(), entry);
    let actual = RealizedProgram::new(program)
        .expect("structural callable fixture must be inside Wasm realization coverage")
        .execute(entry)
        .expect("structural callable fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

#[test]
fn flat_and_empty_parameters_with_aggregate_result_match_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let empty_ty = types.push(TypeDef::structure("Empty", Vec::new()));
    let callable = callable_type(
        &mut types,
        "PairIdentity",
        vec![pair_ty, empty_ty],
        Some(pair_ty),
    );
    let value = Value::Struct(vec![Value::I64(20), Value::I64(22)]);

    let entry = function(
        "entry",
        Vec::new(),
        Some(pair_ty),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("result", pair_ty, false),
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
                        Operand::Constant(value.clone()),
                        Operand::Constant(Value::Struct(Vec::new())),
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
    let identity = function(
        "identity",
        vec![LocalId(0), LocalId(1)],
        Some(pair_ty),
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("empty", empty_ty, false),
        ],
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
        )],
    );
    let program = validated(types, vec![entry, identity]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(value))
    );
}

#[test]
fn nested_aggregate_and_empty_results_match_reference() {
    let mut types = TypeTable::new();
    let i8_ty = types.push(TypeDef::scalar("I8", ScalarType::I8));
    let u64_ty = types.push(TypeDef::scalar("U64", ScalarType::U64));
    let empty_ty = types.push(TypeDef::structure("Empty", Vec::new()));
    let inner_ty = types.push(TypeDef::structure(
        "Inner",
        vec![Field::new("small", i8_ty), Field::new("empty", empty_ty)],
    ));
    let outer_ty = types.push(TypeDef::structure(
        "Outer",
        vec![Field::new("inner", inner_ty), Field::new("large", u64_ty)],
    ));
    let outer_callable = callable_type(&mut types, "OuterIdentity", vec![outer_ty], Some(outer_ty));
    let empty_callable = callable_type(&mut types, "EmptyProducer", Vec::new(), Some(empty_ty));
    let value = Value::Struct(vec![
        Value::Struct(vec![Value::I8(-7), Value::Struct(Vec::new())]),
        Value::U64(u64::MAX),
    ]);

    let nested_entry = function(
        "nested_entry",
        Vec::new(),
        Some(outer_ty),
        vec![
            LocalDecl::new("callee", outer_callable, false),
            LocalDecl::new("result", outer_ty, false),
        ],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(2)),
                }],
                Terminator::IndirectCall {
                    callable: outer_callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: vec![Operand::Constant(value.clone())],
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
    let empty_entry = function(
        "empty_entry",
        Vec::new(),
        Some(empty_ty),
        vec![
            LocalDecl::new("callee", empty_callable, false),
            LocalDecl::new("result", empty_ty, false),
        ],
        vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::FunctionValue(FunctionId(3)),
                }],
                Terminator::IndirectCall {
                    callable: empty_callable,
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
    let outer_identity = function(
        "outer_identity",
        vec![LocalId(0)],
        Some(outer_ty),
        vec![LocalDecl::new("value", outer_ty, false)],
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
        )],
    );
    let empty_producer = function(
        "empty_producer",
        Vec::new(),
        Some(empty_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::Struct(Vec::new()))),
        )],
    );
    let program = validated(
        types,
        vec![nested_entry, empty_entry, outer_identity, empty_producer],
    );

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(value))
    );
    assert_eq!(
        assert_differential(&program, FunctionId(1)),
        ExecutionOutcome::Returned(Some(Value::Struct(Vec::new())))
    );
}

#[test]
fn equal_interface_target_identities_remain_distinct_for_aggregate_results() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let callable = callable_type(&mut types, "Producer", Vec::new(), Some(pair_ty));

    let entry = |name: &str, selected: FunctionId| {
        function(
            name,
            Vec::new(),
            Some(pair_ty),
            vec![
                LocalDecl::new("callee", callable, false),
                LocalDecl::new("result", pair_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::FunctionValue(selected),
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
    };
    let left = Value::Struct(vec![Value::I64(1), Value::I64(2)]);
    let right = Value::Struct(vec![Value::I64(20), Value::I64(22)]);
    let left_target = function(
        "left_target",
        Vec::new(),
        Some(pair_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(left.clone()))),
        )],
    );
    let right_target = function(
        "right_target",
        Vec::new(),
        Some(pair_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(right.clone()))),
        )],
    );
    let program = validated(
        types,
        vec![
            entry("left_entry", FunctionId(2)),
            entry("right_entry", FunctionId(3)),
            left_target,
            right_target,
        ],
    );

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(left))
    );
    assert_eq!(
        assert_differential(&program, FunctionId(1)),
        ExecutionOutcome::Returned(Some(right))
    );
}

#[test]
fn direct_callable_transport_then_structural_indirect_call_matches_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let callable = callable_type(&mut types, "PairIdentity", vec![pair_ty], Some(pair_ty));
    let value = Value::Struct(vec![Value::I64(20), Value::I64(22)]);

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
                    arguments: vec![
                        Operand::FunctionValue(FunctionId(2)),
                        Operand::Constant(value.clone()),
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
        Some(pair_ty),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("argument", pair_ty, false),
            LocalDecl::new("result", pair_ty, false),
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
    let program = validated(types, vec![entry, apply, identity]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(value))
    );
}

#[test]
fn aggregate_result_fault_propagates_without_normal_continuation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let callable = callable_type(&mut types, "FaultingProducer", Vec::new(), Some(pair_ty));
    let entry = function(
        "entry",
        Vec::new(),
        Some(pair_ty),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("result", pair_ty, false),
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
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(1))),
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
            Terminator::Fault(Fault::new("structural-callable-fault")),
        )],
    );
    let program = validated(types, vec![entry, faulting]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Faulted(Fault::new("structural-callable-fault"))
    );
}

#[test]
fn returned_aggregate_projection_composes_with_integer_operation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let callable = callable_type(&mut types, "Producer", Vec::new(), Some(pair_ty));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("sum", i64_ty, false),
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
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(
                vec![Statement::IntegerAdd {
                    dst: Place::local(LocalId(2)),
                    left: Operand::Copy(Place::local(LocalId(1)).field(0).into()),
                    right: Operand::Move(Place::local(LocalId(1)).field(1).into()),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            ),
        ],
    );
    let producer = function(
        "producer",
        Vec::new(),
        Some(pair_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::Struct(vec![
                Value::I64(20),
                Value::I64(22),
            ])))),
        )],
    );
    let program = validated(types, vec![entry, producer]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn finite_structural_indirect_recursion_matches_reference() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let state_ty = types.push(TypeDef::structure(
        "State",
        vec![Field::new("again", bool_ty), Field::new("value", i64_ty)],
    ));
    let callable = callable_type(&mut types, "Recursive", vec![state_ty], Some(state_ty));

    let entry = function(
        "entry",
        Vec::new(),
        Some(state_ty),
        vec![
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("result", state_ty, false),
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
                    arguments: vec![Operand::Constant(Value::Struct(vec![
                        Value::Bool(true),
                        Value::I64(0),
                    ]))],
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
    let recursive = function(
        "recursive",
        vec![LocalId(0)],
        Some(state_ty),
        vec![
            LocalDecl::new("state", state_ty, false),
            LocalDecl::new("callee", callable, false),
            LocalDecl::new("recursive_result", state_ty, false),
        ],
        vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::Branch {
                    condition: Operand::Copy(Place::local(LocalId(0)).field(0).into()),
                    true_target: BasicBlockId(1),
                    false_target: BasicBlockId(2),
                },
            ),
            BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::FunctionValue(FunctionId(1)),
                }],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(1)).into()),
                    arguments: vec![Operand::Constant(Value::Struct(vec![
                        Value::Bool(false),
                        Value::I64(42),
                    ]))],
                    destination: Some(Place::local(LocalId(2))),
                    target: BasicBlockId(3),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            ),
        ],
    );
    let program = validated(types, vec![entry, recursive]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::Struct(vec![
            Value::Bool(false),
            Value::I64(42),
        ])))
    );
}
