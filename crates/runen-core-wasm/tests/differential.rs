use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId,
    TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{ExecutionOutcome, RealizedProgram};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

#[derive(Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Xor,
    Or,
    Eq,
    Lt,
}

fn empty_body(blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals: Vec::new(),
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn function(name: &str, result: Option<TypeId>, body: Body) -> Function {
    Function {
        name: name.into(),
        parameters: Vec::new(),
        result,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body,
    }
}

fn program(types: TypeTable, functions: Vec<Function>) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions,
    })
    .expect("differential fixture must be valid Core")
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
        other => panic!("unsupported differential observation: {other:?}"),
    }
}

fn reference_outcome(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(validated, entry)
        .expect("differential entry must be admitted by reference machine")
        .execute()
        .expect("supported realization subset contains no Core UB operations");
    match report.terminal {
        TerminalStatus::Returned => {
            ExecutionOutcome::Returned(report.result.map(observed_to_value))
        }
        TerminalStatus::Faulted(code) => ExecutionOutcome::Faulted(Fault::new(code)),
    }
}

fn assert_differential(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let expected = reference_outcome(validated.clone(), entry);
    let actual = RealizedProgram::new(&validated)
        .expect("fixture must be inside Wasm realization coverage")
        .execute(entry)
        .expect("supported fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

fn scalar_return_program(scalar: ScalarType, value: Value) -> ValidatedProgram {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("T", scalar));
    program(
        types,
        vec![function(
            "entry",
            Some(ty),
            empty_body(vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(value))),
            )]),
        )],
    )
}

fn binary_program(
    scalar: ScalarType,
    left: Value,
    right: Value,
    operation: BinaryOp,
) -> ValidatedProgram {
    let mut types = TypeTable::new();
    let operand_ty = types.push(TypeDef::scalar("T", scalar));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let result_ty = match operation {
        BinaryOp::Eq | BinaryOp::Lt => bool_ty,
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Xor | BinaryOp::Or => operand_ty,
    };
    let destination = Place::local(LocalId(0));
    let left = Operand::Constant(left);
    let right = Operand::Constant(right);
    let statement = match operation {
        BinaryOp::Add => Statement::IntegerAdd {
            dst: destination.clone(),
            left,
            right,
        },
        BinaryOp::Sub => Statement::IntegerSub {
            dst: destination.clone(),
            left,
            right,
        },
        BinaryOp::Mul => Statement::IntegerMul {
            dst: destination.clone(),
            left,
            right,
        },
        BinaryOp::Xor => Statement::IntegerXor {
            dst: destination.clone(),
            left,
            right,
        },
        BinaryOp::Or => Statement::IntegerOr {
            dst: destination.clone(),
            left,
            right,
        },
        BinaryOp::Eq => Statement::IntegerEq {
            dst: destination.clone(),
            operand_type: operand_ty,
            left,
            right,
        },
        BinaryOp::Lt => Statement::IntegerLt {
            dst: destination.clone(),
            operand_type: operand_ty,
            left,
            right,
        },
    };
    let body = Body {
        locals: vec![LocalDecl::new("result", result_ty, false)],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![statement],
            Terminator::Return(Some(Operand::Move(destination.into()))),
        )],
    };
    program(types, vec![function("entry", Some(result_ty), body)])
}

#[test]
fn scalar_carrier_round_trips_all_supported_kinds_at_boundaries() {
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
        assert_eq!(
            assert_differential(scalar_return_program(scalar, value.clone()), FunctionId(0)),
            ExecutionOutcome::Returned(Some(value))
        );
    }
}

#[test]
fn integer_arithmetic_explicitly_preserves_modulo_residues() {
    let cases = [
        (
            ScalarType::I8,
            Value::I8(i8::MAX),
            Value::I8(1),
            BinaryOp::Add,
            Value::I8(i8::MIN),
        ),
        (
            ScalarType::U16,
            Value::U16(0),
            Value::U16(1),
            BinaryOp::Sub,
            Value::U16(u16::MAX),
        ),
        (
            ScalarType::I32,
            Value::I32(i32::MAX),
            Value::I32(2),
            BinaryOp::Mul,
            Value::I32(-2),
        ),
        (
            ScalarType::U64,
            Value::U64(u64::MAX),
            Value::U64(1),
            BinaryOp::Add,
            Value::U64(0),
        ),
    ];

    for (scalar, left, right, operation, expected) in cases {
        assert_eq!(
            assert_differential(binary_program(scalar, left, right, operation), FunctionId(0)),
            ExecutionOutcome::Returned(Some(expected))
        );
    }
}

#[test]
fn integer_bitwise_equality_and_order_match_reference_semantics() {
    let cases = [
        (
            ScalarType::I8,
            Value::I8(-1),
            Value::I8(0b0101_0101),
            BinaryOp::Xor,
            Value::I8(-86),
        ),
        (
            ScalarType::U32,
            Value::U32(0x0f00),
            Value::U32(0x00f0),
            BinaryOp::Or,
            Value::U32(0x0ff0),
        ),
        (
            ScalarType::I64,
            Value::I64(i64::MIN),
            Value::I64(i64::MIN),
            BinaryOp::Eq,
            Value::Bool(true),
        ),
        (
            ScalarType::I16,
            Value::I16(-1),
            Value::I16(1),
            BinaryOp::Lt,
            Value::Bool(true),
        ),
        (
            ScalarType::U16,
            Value::U16(u16::MAX),
            Value::U16(1),
            BinaryOp::Lt,
            Value::Bool(false),
        ),
    ];

    for (scalar, left, right, operation, expected) in cases {
        assert_eq!(
            assert_differential(binary_program(scalar, left, right, operation), FunctionId(0)),
            ExecutionOutcome::Returned(Some(expected))
        );
    }
}

#[test]
fn direct_scalar_storage_transport_matches_reference_machine() {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("U32", ScalarType::U32));
    let source = Place::local(LocalId(0));
    let target = Place::local(LocalId(1));
    let body = Body {
        locals: vec![
            LocalDecl::new("source", ty, false),
            LocalDecl::new("target", ty, true),
        ],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(
            vec![
                Statement::Init {
                    dst: source.clone(),
                    src: Operand::Constant(Value::U32(7)),
                },
                Statement::Read {
                    src: source.clone().into(),
                },
                Statement::Init {
                    dst: target.clone(),
                    src: Operand::Copy(source.clone().into()),
                },
                Statement::Drop {
                    place: source.clone().into(),
                },
                Statement::Init {
                    dst: source.clone(),
                    src: Operand::Constant(Value::U32(11)),
                },
                Statement::Assign {
                    dst: target.clone().into(),
                    src: Operand::Move(source.into()),
                },
            ],
            Terminator::Return(Some(Operand::Move(target.into()))),
        )],
    };
    let validated = program(types, vec![function("entry", Some(ty), body)]);
    assert_eq!(
        assert_differential(validated, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::U32(11)))
    );
}

#[test]
fn branch_goto_and_backedge_loop_match_reference_machine() {
    let mut types = TypeTable::new();
    let u64_ty = types.push(TypeDef::scalar("U64", ScalarType::U64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let counter = Place::local(LocalId(0));
    let condition = Place::local(LocalId(1));
    let next = Place::local(LocalId(2));
    let body = Body {
        locals: vec![
            LocalDecl::new("counter", u64_ty, true),
            LocalDecl::new("condition", bool_ty, false),
            LocalDecl::new("next", u64_ty, false),
        ],
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![
            BasicBlock::new(
                vec![Statement::Init {
                    dst: counter.clone(),
                    src: Operand::Constant(Value::U64(0)),
                }],
                Terminator::Goto(BasicBlockId(1)),
            ),
            BasicBlock::new(
                vec![Statement::IntegerLt {
                    dst: condition.clone(),
                    operand_type: u64_ty,
                    left: Operand::Copy(counter.clone().into()),
                    right: Operand::Constant(Value::U64(3)),
                }],
                Terminator::Branch {
                    condition: Operand::Move(condition.into()),
                    true_target: BasicBlockId(2),
                    false_target: BasicBlockId(3),
                },
            ),
            BasicBlock::new(
                vec![
                    Statement::IntegerAdd {
                        dst: next.clone(),
                        left: Operand::Copy(counter.clone().into()),
                        right: Operand::Constant(Value::U64(1)),
                    },
                    Statement::Assign {
                        dst: counter.clone().into(),
                        src: Operand::Move(next.into()),
                    },
                ],
                Terminator::Goto(BasicBlockId(1)),
            ),
            BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Move(counter.into()))),
            ),
        ],
    };
    let validated = program(types, vec![function("entry", Some(u64_ty), body)]);
    assert_eq!(
        assert_differential(validated, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::U64(3)))
    );
}

#[test]
fn direct_calls_nested_calls_and_scalar_results_match_reference_machine() {
    let mut types = TypeTable::new();
    let ty = types.push(TypeDef::scalar("U64", ScalarType::U64));

    let entry = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![LocalDecl::new("result", ty, false)],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Constant(Value::U64(40))],
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

    let middle = Function {
        name: "middle".into(),
        parameters: vec![LocalId(0)],
        result: Some(ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![
                LocalDecl::new("input", ty, false),
                LocalDecl::new("result", ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(2),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
                ),
            ],
        },
    };

    let leaf = Function {
        name: "leaf".into(),
        parameters: vec![LocalId(0)],
        result: Some(ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![
                LocalDecl::new("input", ty, false),
                LocalDecl::new("result", ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![Statement::IntegerAdd {
                    dst: Place::local(LocalId(1)),
                    left: Operand::Move(Place::local(LocalId(0)).into()),
                    right: Operand::Constant(Value::U64(2)),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(1)).into()))),
            )],
        },
    };

    let validated = program(types, vec![entry, middle, leaf]);
    assert_eq!(
        assert_differential(validated, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::U64(42)))
    );
}

#[test]
fn finite_direct_recursion_matches_reference_machine() {
    let mut types = TypeTable::new();
    let u64_ty = types.push(TypeDef::scalar("U64", ScalarType::U64));
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));

    let entry = Function {
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
        },
    };

    let recursive = Function {
        name: "countdown".into(),
        parameters: vec![LocalId(0)],
        result: Some(u64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals: vec![
                LocalDecl::new("n", u64_ty, false),
                LocalDecl::new("zero", bool_ty, false),
                LocalDecl::new("next", u64_ty, false),
                LocalDecl::new("result", u64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
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
                    vec![Statement::IntegerSub {
                        dst: Place::local(LocalId(2)),
                        left: Operand::Move(Place::local(LocalId(0)).into()),
                        right: Operand::Constant(Value::U64(1)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(2)).into())],
                        destination: Some(Place::local(LocalId(3))),
                        target: BasicBlockId(3),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(3)).into()))),
                ),
            ],
        },
    };

    let validated = program(types, vec![entry, recursive]);
    assert_eq!(
        assert_differential(validated, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::U64(1)))
    );
}

#[test]
fn no_result_return_matches_reference_machine() {
    let types = TypeTable::new();
    let validated = program(
        types,
        vec![function(
            "entry",
            None,
            empty_body(vec![BasicBlock::new(Vec::new(), Terminator::Return(None))]),
        )],
    );
    assert_eq!(
        assert_differential(validated, FunctionId(0)),
        ExecutionOutcome::Returned(None)
    );
}

fn nested_fault_program(code: &str) -> ValidatedProgram {
    let types = TypeTable::new();
    let entry = function(
        "entry",
        None,
        empty_body(vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::Call {
                    function: FunctionId(1),
                    arguments: Vec::new(),
                    destination: None,
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(Vec::new(), Terminator::Return(None)),
        ]),
    );
    let middle = function(
        "middle",
        None,
        empty_body(vec![
            BasicBlock::new(
                Vec::new(),
                Terminator::Call {
                    function: FunctionId(2),
                    arguments: Vec::new(),
                    destination: None,
                    target: BasicBlockId(1),
                },
            ),
            BasicBlock::new(Vec::new(), Terminator::Return(None)),
        ]),
    );
    let leaf = function(
        "leaf",
        None,
        empty_body(vec![BasicBlock::new(
            Vec::new(),
            Terminator::Fault(Fault::new(code)),
        )]),
    );
    program(types, vec![entry, middle, leaf])
}

#[test]
fn defined_fault_identity_propagates_without_wasm_traps() {
    for code in ["fault-alpha", "fault-beta"] {
        assert_eq!(
            assert_differential(nested_fault_program(code), FunctionId(0)),
            ExecutionOutcome::Faulted(Fault::new(code))
        );
    }
}
