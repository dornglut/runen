use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef,
    TypeTable, ValidatedProgram, Value, validate_program,
};
use runen_core_wasm::{ExecutionOutcome, RealizedProgram};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

fn function(
    result: runen_core_ir::TypeId,
    locals: Vec<LocalDecl>,
    statements: Vec<Statement>,
    returned: Operand,
) -> Function {
    Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(result),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: Body {
            locals,
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                statements,
                Terminator::Return(Some(returned)),
            )],
        },
    }
}

fn validate(types: TypeTable, entry: Function) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent: Vec::new(),
        external_callables: Vec::new(),
        functions: vec![entry],
    })
    .expect("raw-pointer target-identity fixture must be valid Core")
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
        other => panic!("unexpected target-identity differential result: {other:?}"),
    }
}

fn assert_differential(validated: ValidatedProgram) -> ExecutionOutcome {
    let reference = Machine::new(validated.clone(), FunctionId(0))
        .expect("target-identity entry must be admitted")
        .execute()
        .expect("target-identity fixture must execute in reference semantics");
    let expected = match reference.terminal {
        TerminalStatus::Returned => {
            ExecutionOutcome::Returned(reference.result.map(observed_to_value))
        }
        TerminalStatus::Faulted(code) => ExecutionOutcome::Faulted(runen_core_ir::Fault::new(code)),
    };
    let actual = RealizedProgram::new(&validated)
        .expect("target-identity fixture must realize")
        .execute(FunctionId(0))
        .expect("target-identity fixture must execute in Core-Wasm");
    assert_eq!(actual, expected);
    actual
}

#[test]
fn type_and_value_equal_roots_remain_distinct_private_targets() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let raw_i64 = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let left = Place::local(LocalId(0));
    let right = Place::local(LocalId(1));
    let pointer = Place::local(LocalId(2));
    let sum = Place::local(LocalId(3));
    let entry = function(
        i64_ty,
        vec![
            LocalDecl::new("left", i64_ty, false),
            LocalDecl::new("right", i64_ty, false),
            LocalDecl::new("pointer", raw_i64, false),
            LocalDecl::new("sum", i64_ty, false),
        ],
        vec![
            Statement::Init {
                dst: left.clone(),
                src: Operand::Constant(Value::I64(7)),
            },
            Statement::Init {
                dst: right.clone(),
                src: Operand::Constant(Value::I64(7)),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(left.clone().into()),
            },
            Statement::RawAssign {
                pointer: pointer.into(),
                src: Operand::Constant(Value::I64(9)),
            },
            Statement::IntegerAdd {
                dst: sum.clone(),
                left: Operand::Move(left.into()),
                right: Operand::Move(right.into()),
            },
        ],
        Operand::Move(sum.into()),
    );

    assert_eq!(
        assert_differential(validate(types, entry)),
        ExecutionOutcome::Returned(Some(Value::I64(16)))
    );
}

#[test]
fn aggregate_raw_move_preserves_the_complete_carrier_sequence_before_replacement() {
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
        pair_ty,
        vec![
            LocalDecl::new("target", pair_ty, false),
            LocalDecl::new("pointer", raw_pair, false),
            LocalDecl::new("moved", pair_ty, false),
        ],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(1), Value::I64(2)])),
            },
            Statement::Init {
                dst: pointer.clone(),
                src: Operand::AddressOf(target.into()),
            },
            Statement::Init {
                dst: moved.clone(),
                src: Operand::RawMove(pointer.clone().into()),
            },
            Statement::RawAssign {
                pointer: pointer.into(),
                src: Operand::Constant(Value::Struct(vec![Value::I64(7), Value::I64(8)])),
            },
        ],
        Operand::Move(moved.into()),
    );

    assert_eq!(
        assert_differential(validate(types, entry)),
        ExecutionOutcome::Returned(Some(Value::Struct(vec![Value::I64(1), Value::I64(2)])))
    );
}
