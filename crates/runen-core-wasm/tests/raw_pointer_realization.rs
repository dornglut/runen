use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, Function, FunctionId, LocalDecl, LocalId, Operand, Place,
    Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId,
    TypeTable, ValidatedProgram, Value, validate_program,
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

fn validate(types: TypeTable, functions: Vec<Function>) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions,
    })
    .expect("raw-pointer realization fixture must be valid Core")
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
        other => panic!("unexpected raw-pointer differential result: {other:?}"),
    }
}

fn reference_outcome(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(validated, entry)
        .expect("raw-pointer differential entry must be admitted")
        .execute()
        .expect("accepted raw-pointer fixture must execute in reference semantics");
    match report.terminal {
        TerminalStatus::Returned => {
            ExecutionOutcome::Returned(report.result.map(observed_to_value))
        }
        TerminalStatus::Faulted(code) => {
            ExecutionOutcome::Faulted(runen_core_ir::Fault::new(code))
        }
    }
}

fn assert_differential(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let expected = reference_outcome(validated.clone(), entry);
    let actual = RealizedProgram::new(&validated)
        .expect("raw-pointer fixture must be inside bounded Core-Wasm coverage")
        .execute(entry)
        .expect("accepted raw-pointer fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

fn raw_i64_types() -> (TypeTable, TypeId, TypeId) {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_i64 = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    (types, i64_ty, raw_i64)
}

#[test]
fn raw_move_scalar_matches_reference_semantics() {
    let (types, i64_ty, raw_i64) = raw_i64_types();
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", raw_i64, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(41)),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(target.into()),
                    },
                    Statement::Init {
                        dst: result.clone(),
                        src: Operand::RawMove(pointer.into()),
                    },
                ],
                Terminator::Return(Some(Operand::Move(result.into()))),
            )],
        },
    );

    assert_eq!(
        assert_differential(validate(types, vec![entry]), FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(41)))
    );
}

#[test]
fn raw_read_is_non_consuming_on_a_defined_target() {
    let (types, i64_ty, raw_i64) = raw_i64_types();
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", raw_i64, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(43)),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(target.clone().into()),
                    },
                    Statement::RawRead {
                        pointer: pointer.into(),
                    },
                ],
                Terminator::Return(Some(Operand::Move(target.into()))),
            )],
        },
    );

    assert_eq!(
        assert_differential(validate(types, vec![entry]), FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(43)))
    );
}

#[test]
fn raw_pointer_copy_and_retarget_select_exact_runtime_roots() {
    let (types, i64_ty, raw_i64) = raw_i64_types();
    let left = Place::local(LocalId(0));
    let right = Place::local(LocalId(1));
    let pointer = Place::local(LocalId(2));
    let copy = Place::local(LocalId(3));
    let left_value = Place::local(LocalId(4));
    let right_value = Place::local(LocalId(5));
    let sum = Place::local(LocalId(6));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("left", i64_ty, false),
                LocalDecl::new("right", i64_ty, false),
                LocalDecl::new("pointer", raw_i64, false),
                LocalDecl::new("copy", raw_i64, false),
                LocalDecl::new("left_value", i64_ty, false),
                LocalDecl::new("right_value", i64_ty, false),
                LocalDecl::new("sum", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: left.clone(),
                        src: Operand::Constant(Value::I64(11)),
                    },
                    Statement::Init {
                        dst: right.clone(),
                        src: Operand::Constant(Value::I64(22)),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(left.into()),
                    },
                    Statement::Init {
                        dst: copy.clone(),
                        src: Operand::Copy(pointer.clone().into()),
                    },
                    Statement::Assign {
                        dst: pointer.clone().into(),
                        src: Operand::AddressOf(right.into()),
                    },
                    Statement::Init {
                        dst: left_value.clone(),
                        src: Operand::RawMove(copy.into()),
                    },
                    Statement::Init {
                        dst: right_value.clone(),
                        src: Operand::RawMove(pointer.into()),
                    },
                    Statement::IntegerAdd {
                        dst: sum.clone(),
                        left: Operand::Move(left_value.into()),
                        right: Operand::Move(right_value.into()),
                    },
                ],
                Terminator::Return(Some(Operand::Move(sum.into()))),
            )],
        },
    );

    assert_eq!(
        assert_differential(validate(types, vec![entry]), FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(33)))
    );
}

#[test]
fn raw_assign_from_raw_move_uses_the_snapshotted_target() {
    let (types, i64_ty, raw_i64) = raw_i64_types();
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", raw_i64, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(55)),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(target.clone().into()),
                    },
                    Statement::RawAssign {
                        pointer: pointer.clone().into(),
                        src: Operand::RawMove(pointer.into()),
                    },
                ],
                Terminator::Return(Some(Operand::Move(target.into()))),
            )],
        },
    );

    assert_eq!(
        assert_differential(validate(types, vec![entry]), FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(55)))
    );
}

#[test]
fn aggregate_raw_move_then_replace_matches_reference_semantics() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let raw_pair = types.push(TypeDef::raw_pointer("RawPair", pair_ty));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let moved = Place::local(LocalId(2));
    let entry = function(
        "entry",
        Vec::new(),
        Some(pair_ty),
        Body {
            locals: vec![
                LocalDecl::new("target", pair_ty, false),
                LocalDecl::new("pointer", raw_pair, false),
                LocalDecl::new("moved", pair_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(target.clone().into()),
                    },
                    Statement::Init {
                        dst: moved.clone(),
                        src: Operand::RawMove(pointer.clone().into()),
                    },
                    Statement::RawAssign {
                        pointer: pointer.into(),
                        src: Operand::Constant(Value::Struct(vec![Value::I64(7), Value::I64(8)])),
                    },
                    Statement::Drop {
                        place: moved.into(),
                    },
                ],
                Terminator::Return(Some(Operand::Move(target.into()))),
            )],
        },
    );

    assert_eq!(
        assert_differential(validate(types, vec![entry]), FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::Struct(vec![Value::I64(7), Value::I64(8)])))
    );
}

#[test]
fn caller_raw_target_survives_nested_activation_with_its_own_handles() {
    let (types, i64_ty, raw_i64) = raw_i64_types();
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", raw_i64, false),
                LocalDecl::new("call_result", i64_ty, false),
                LocalDecl::new("raw_result", i64_ty, false),
                LocalDecl::new("sum", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::I64(41)),
                        },
                        Statement::Init {
                            dst: Place::local(LocalId(1)),
                            src: Operand::AddressOf(Place::local(LocalId(0)).into()),
                        },
                    ],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(Place::local(LocalId(2))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(3)),
                            src: Operand::RawMove(Place::local(LocalId(1)).into()),
                        },
                        Statement::IntegerAdd {
                            dst: Place::local(LocalId(4)),
                            left: Operand::Move(Place::local(LocalId(2)).into()),
                            right: Operand::Move(Place::local(LocalId(3)).into()),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(4)).into()))),
                ),
            ],
        },
    );
    let helper = function(
        "helper",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", raw_i64, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(5)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::AddressOf(Place::local(LocalId(0)).into()),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(2)),
                        src: Operand::RawMove(Place::local(LocalId(1)).into()),
                    },
                ],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
            )],
        },
    );

    assert_eq!(
        assert_differential(validate(types, vec![entry, helper]), FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(46)))
    );
}
