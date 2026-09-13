use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand,
    PersistentDecl, PersistentId, Place, Program, ReferenceAccess, ReferencePermission,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_program,
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

#[test]
fn persistent_instances_are_distinct_from_each_other_and_from_frame_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("local", i64_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };
    let validated = validate_program(Program {
        external_callables: vec![],
        types,
        persistent: vec![
            PersistentDecl::new(i64_ty, Value::I64(1)),
            PersistentDecl::new(i64_ty, Value::I64(1)),
        ],
        functions: vec![function],
    })
    .expect("valid persistent program");
    let machine = Machine::new(validated, FunctionId(0)).expect("zero-parameter entry");
    let first = machine
        .persistent_storage_instance(PersistentId(0))
        .expect("first persistent instance");
    let second = machine
        .persistent_storage_instance(PersistentId(1))
        .expect("second persistent instance");
    assert_ne!(first, second);
}

#[test]
fn repeated_persistent_reads_are_non_consuming() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("sum", i64_ty, false)],
            vec![BasicBlock::new(
                vec![Statement::IntegerAdd {
                    dst: Place::local(LocalId(0)),
                    left: Operand::PersistentRead(PersistentId(0)),
                    right: Operand::PersistentRead(PersistentId(0)),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };
    let validated = validate_program(Program {
        external_callables: vec![],
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(21))],
        functions: vec![function],
    })
    .expect("valid repeated persistent reads");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined persistent read execution");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn persistent_shared_root_crosses_activation_and_reads_through_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::PersistentSharedRoot(PersistentId(0))],
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
    };
    let callee = Function {
        name: "read".into(),
        parameters: vec![LocalId(0)],
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::ReferenceCopy(ReferenceAccess::new(
                    Place::local(LocalId(0)),
                )))),
            )],
        ),
    };
    let validated = validate_program(Program {
        external_callables: vec![],
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(73))],
        functions: vec![caller, callee],
    })
    .expect("persistent Shared reference can cross a call boundary");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined cross-activation persistent reference execution");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(73)));
}

#[test]
fn normal_terminal_cleanup_drops_outer_persistent_reference_before_extent_end() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("persistent_ref", shared_i64, false)],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::PersistentSharedRoot(PersistentId(0)),
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let validated = validate_program(Program {
        external_callables: vec![],
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(5))],
        functions: vec![function],
    })
    .expect("outer persistent reference is valid until normal cleanup");

    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("outer frame cleanup must release the persistent carrier first");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, None);
}

#[test]
fn fault_terminal_cleanup_drops_outer_persistent_reference_before_extent_end() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("persistent_ref", shared_i64, false)],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::PersistentSharedRoot(PersistentId(0)),
                }],
                Terminator::Fault(Fault::new("persistent-cleanup")),
            )],
        ),
    };
    let validated = validate_program(Program {
        external_callables: vec![],
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(8))],
        functions: vec![function],
    })
    .expect("outer persistent reference is valid until fault cleanup");

    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("outer fault cleanup must release the persistent carrier first");
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("persistent-cleanup".into())
    );
    assert_eq!(report.result, None);
}
