use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, LocalDecl, LocalId, Operand, Place, Program,
    ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

#[test]
fn explicit_fault_is_valid_without_return_value_in_result_function() {
    let mut types = TypeTable::new();
    let result_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let first = Fault::new("first");
    let second = Fault::new("second");
    assert_ne!(first, second, "represented fault reasons remain distinguishable");

    let function = Function {
        name: "faulting_result".into(),
        parameters: Vec::new(),
        result: Some(result_ty),
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Fault(first))],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("Fault(F) is an abnormal terminator and requires no result operand");
}

#[test]
fn explicit_fault_contributes_no_cfg_successor_state() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "no_fault_successor".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![LocalDecl::new("uninitialized", i64_ty, false)],
            vec![
                BasicBlock::new(Vec::new(), Terminator::Fault(Fault::new("stop"))),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(0)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("disconnected state-invalid block is not reached through Fault(F)");
}

#[test]
fn explicit_fault_has_no_operand_or_target_to_validate() {
    let types = TypeTable::new();
    let function = Function {
        name: "fault_only".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Fault(Fault::new("opaque-reason")),
            )],
        ),
    };

    validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("Fault(F) needs neither an operand nor a target block");

    let _ = Operand::Constant(Value::Bool(true));
}