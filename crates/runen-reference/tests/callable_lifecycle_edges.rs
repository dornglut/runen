use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, CallableInterface, Function, FunctionId, LocalDecl, LocalId,
    NumericContract, Operand, Place, Program, SafeReferenceResultContract, ScalarType, Statement,
    Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn return_i64(name: &str, ty: runen_core_ir::TypeId, value: i64) -> Function {
    Function {
        name: name.into(),
        parameters: Vec::new(),
        result: Some(ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::I64(value)))),
            )],
        ),
    }
}

#[test]
fn callable_copy_move_reinitialization_and_dispatch_preserve_exact_function_identity() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let callable = types.push(TypeDef::callable(
        "ReturnsI64",
        CallableInterface::new(Vec::new(), Some(i64_ty), SafeReferenceResultContract::None),
    ));

    let source = Place::local(LocalId(0));
    let copy = Place::local(LocalId(1));
    let first = Place::local(LocalId(2));
    let second = Place::local(LocalId(3));
    let third = Place::local(LocalId(4));
    let subtotal = Place::local(LocalId(5));
    let total = Place::local(LocalId(6));

    let entry = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("source", callable, true),
                LocalDecl::new("copy", callable, false),
                LocalDecl::new("first", i64_ty, false),
                LocalDecl::new("second", i64_ty, false),
                LocalDecl::new("third", i64_ty, false),
                LocalDecl::new("subtotal", i64_ty, false),
                LocalDecl::new("total", i64_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: source.clone(),
                            src: Operand::FunctionValue(FunctionId(1)),
                        },
                        Statement::Init {
                            dst: copy.clone(),
                            src: Operand::Copy(source.clone().into()),
                        },
                    ],
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::Move(copy.into()),
                        arguments: Vec::new(),
                        destination: Some(first.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::Move(source.clone().into()),
                        arguments: Vec::new(),
                        destination: Some(second.clone()),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Assign {
                        dst: source.clone().into(),
                        src: Operand::FunctionValue(FunctionId(2)),
                    }],
                    Terminator::IndirectCall {
                        callable,
                        callee: Operand::Copy(source.into()),
                        arguments: Vec::new(),
                        destination: Some(third.clone()),
                        target: BasicBlockId(3),
                    },
                ),
                BasicBlock::new(
                    vec![
                        Statement::IntegerAdd {
                            dst: subtotal.clone(),
                            left: Operand::Move(first.into()),
                            right: Operand::Move(second.into()),
                        },
                        Statement::IntegerAdd {
                            dst: total.clone(),
                            left: Operand::Move(subtotal.into()),
                            right: Operand::Move(third.into()),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(total.into()))),
                ),
            ],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![
            entry,
            return_i64("first-target", i64_ty, 7),
            return_i64("second-target", i64_ty, 9),
        ],
    })
    .expect("callable lifecycle fixture is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("entry has zero parameters")
        .execute()
        .expect("callable lifecycle fixture has defined execution");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(23)));
}
