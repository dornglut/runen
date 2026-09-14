use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Fault, Field, Function, FunctionId,
    LocalDecl, LocalId, Operand, Place, Program, SafeReferenceResultContract, ScalarType,
    Statement, Terminator, TypeDef, TypeId, TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{ExecutionOutcome, RealizationError, RealizedProgram};
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
    .expect("callable-result fixture must be valid Core")
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
        other => panic!("unsupported callable-result observation: {other:?}"),
    }
}

fn reference_outcome(program: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(program, entry)
        .expect("callable-result entry must be admitted by reference machine")
        .execute()
        .expect("callable-result realization subset contains no Core UB operations");
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
        .expect("callable-result fixture must be inside Wasm realization coverage")
        .execute(entry)
        .expect("observable callable-result fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

#[test]
fn direct_callable_result_copies_then_invokes_and_entry_identity_stays_private() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = callable_type(&mut types, "Thunk", Vec::new(), Some(i64_ty));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![
            LocalDecl::new("returned", callable, false),
            LocalDecl::new("copy", callable, false),
            LocalDecl::new("result", i64_ty, false),
        ],
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
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::Copy(Place::local(LocalId(0)).into()),
                }],
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(1)).into()),
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(2))),
                    target: BasicBlockId(2),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            ),
        ],
    );
    let producer = function(
        "producer",
        Vec::new(),
        Some(callable),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(2)))),
        )],
    );
    let target = function(
        "target",
        Vec::new(),
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
        )],
    );
    let program = validated(types, vec![entry, producer, target]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );

    let realized = RealizedProgram::new(&program).expect("program must realize");
    assert_eq!(
        realized.execute(FunctionId(1)),
        Err(RealizationError::EntryResultUnsupported(FunctionId(1)))
    );
}

#[test]
fn same_interface_callable_results_preserve_distinct_function_identity() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = callable_type(&mut types, "Thunk", Vec::new(), Some(i64_ty));

    let make_entry = |name: &str, producer: FunctionId| {
        function(
            name,
            Vec::new(),
            Some(i64_ty),
            vec![
                LocalDecl::new("returned", callable, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: producer,
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::Move(Place::local(LocalId(0)).into()),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
                ),
            ],
        )
    };

    let left_entry = make_entry("left_entry", FunctionId(2));
    let right_entry = make_entry("right_entry", FunctionId(3));
    let left_producer = function(
        "left_producer",
        Vec::new(),
        Some(callable),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(4)))),
        )],
    );
    let right_producer = function(
        "right_producer",
        Vec::new(),
        Some(callable),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(5)))),
        )],
    );
    let left = function(
        "left",
        Vec::new(),
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(11)))),
        )],
    );
    let right = function(
        "right",
        Vec::new(),
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(22)))),
        )],
    );
    let program = validated(
        types,
        vec![
            left_entry,
            right_entry,
            left_producer,
            right_producer,
            left,
            right,
        ],
    );

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(11)))
    );
    assert_eq!(
        assert_differential(&program, FunctionId(1)),
        ExecutionOutcome::Returned(Some(Value::I64(22)))
    );
}

#[test]
fn callable_result_survives_multiple_direct_result_boundaries_before_invocation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = callable_type(&mut types, "Thunk", Vec::new(), Some(i64_ty));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![
            LocalDecl::new("returned", callable, false),
            LocalDecl::new("result", i64_ty, false),
        ],
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
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: Vec::new(),
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(2),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            ),
        ],
    );
    let second_hop = function(
        "second_hop",
        Vec::new(),
        Some(callable),
        vec![LocalDecl::new("value", callable, false)],
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
    );
    let first_hop = function(
        "first_hop",
        Vec::new(),
        Some(callable),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(3)))),
        )],
    );
    let target = function(
        "target",
        Vec::new(),
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
        )],
    );
    let program = validated(types, vec![entry, second_hop, first_hop, target]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn callable_result_with_structural_interface_composes_with_existing_indirect_transport() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let callable = callable_type(&mut types, "PairSum", vec![pair_ty], Some(i64_ty));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![
            LocalDecl::new("returned", callable, false),
            LocalDecl::new("result", i64_ty, false),
        ],
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
                Terminator::IndirectCall {
                    callable,
                    callee: Operand::Move(Place::local(LocalId(0)).into()),
                    arguments: vec![Operand::Constant(Value::Struct(vec![
                        Value::I64(20),
                        Value::I64(22),
                    ]))],
                    destination: Some(Place::local(LocalId(1))),
                    target: BasicBlockId(2),
                },
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            ),
        ],
    );
    let producer = function(
        "producer",
        Vec::new(),
        Some(callable),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(2)))),
        )],
    );
    let sum = function(
        "sum",
        vec![LocalId(0)],
        Some(i64_ty),
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("sum", i64_ty, false),
        ],
        vec![BasicBlock::new(
            vec![Statement::IntegerAdd {
                dst: Place::local(LocalId(1)),
                left: Operand::Copy(Place::local(LocalId(0)).field(0).into()),
                right: Operand::Copy(Place::local(LocalId(0)).field(1).into()),
            }],
            Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
        )],
    );
    let program = validated(types, vec![entry, producer, sum]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(42)))
    );
}

#[test]
fn faulting_callable_result_callee_propagates_before_normal_continuation() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = callable_type(&mut types, "Thunk", Vec::new(), Some(i64_ty));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        vec![LocalDecl::new("returned", callable, false)],
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
                Terminator::Fault(Fault::new("normal-continuation-ran")),
            ),
        ],
    );
    let producer = function(
        "producer",
        Vec::new(),
        Some(callable),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Fault(Fault::new("callable-result-fault")),
        )],
    );
    let program = validated(types, vec![entry, producer]);

    assert_eq!(
        assert_differential(&program, FunctionId(0)),
        ExecutionOutcome::Faulted(Fault::new("callable-result-fault"))
    );
}

#[test]
fn higher_order_callable_function_result_is_admitted_but_entry_identity_stays_private() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let inner = callable_type(&mut types, "Inner", Vec::new(), Some(i64_ty));
    let outer = callable_type(&mut types, "Outer", Vec::new(), Some(inner));

    let producer = function(
        "producer",
        Vec::new(),
        Some(outer),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(1)))),
        )],
    );
    let outer_target = function(
        "outer_target",
        Vec::new(),
        Some(inner),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::FunctionValue(FunctionId(2)))),
        )],
    );
    let inner_target = function(
        "inner_target",
        Vec::new(),
        Some(i64_ty),
        Vec::new(),
        vec![BasicBlock::new(
            Vec::new(),
            Terminator::Return(Some(Operand::Constant(Value::I64(42)))),
        )],
    );
    let program = validated(types, vec![producer, outer_target, inner_target]);

    let realized = RealizedProgram::new(&program)
        .expect("higher-order callable function result must realize for internal transport");
    assert_eq!(
        realized.execute(FunctionId(0)),
        Err(RealizationError::EntryResultUnsupported(FunctionId(0)))
    );
}
