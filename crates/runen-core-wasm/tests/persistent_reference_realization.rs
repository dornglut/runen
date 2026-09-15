use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand,
    PersistentDecl, PersistentId, Place, Program, ReferenceAccess, ReferencePermission,
    SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef, TypeId, TypeTable,
    ValidatedProgram, Value, validate_program,
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

fn validate(
    types: TypeTable,
    persistent: Vec<PersistentDecl>,
    functions: Vec<Function>,
) -> ValidatedProgram {
    validate_program(Program {
        types,
        persistent,
        external_callables: Vec::new(),
        functions,
    })
    .expect("persistent reference realization fixture must be valid Core")
}

fn reference_outcome(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let report = Machine::new(validated, entry)
        .expect("persistent reference differential entry must be admitted")
        .execute()
        .expect("accepted persistent reference fixture must execute in reference semantics");
    match report.terminal {
        TerminalStatus::Returned => ExecutionOutcome::Returned(match report.result {
            None => None,
            Some(ObservedValue::I64(value)) => Some(Value::I64(value)),
            other => panic!("unexpected persistent reference differential result: {other:?}"),
        }),
        TerminalStatus::Faulted(code) => ExecutionOutcome::Faulted(Fault::new(code)),
    }
}

fn assert_differential(validated: ValidatedProgram, entry: FunctionId) -> ExecutionOutcome {
    let expected = reference_outcome(validated.clone(), entry);
    let actual = RealizedProgram::new(&validated)
        .expect("persistent reference fixture must be inside bounded Core-Wasm coverage")
        .execute(entry)
        .expect("accepted persistent reference fixture must execute without backend failure");
    assert_eq!(actual, expected);
    actual
}

fn shared_i64_types() -> (TypeTable, TypeId, TypeId) {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    (types, i64_ty, shared_i64)
}

#[test]
fn persistent_shared_i64_dereference_matches_reference_semantics() {
    let (types, i64_ty, shared_i64) = shared_i64_types();
    let reference = Place::local(LocalId(0));
    let result = Place::local(LocalId(1));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("reference", shared_i64, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: reference.clone(),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    },
                    Statement::Init {
                        dst: result.clone(),
                        src: Operand::ReferenceCopy(ReferenceAccess::new(reference.clone())),
                    },
                    Statement::Drop {
                        place: reference.into(),
                    },
                ],
                Terminator::Return(Some(Operand::Move(result.into()))),
            )],
        },
    );
    let program = validate(
        types,
        vec![PersistentDecl::new(i64_ty, Value::I64(41))],
        vec![entry],
    );

    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(41)))
    );
}

#[test]
fn dynamic_persistent_reference_handle_flow_selects_each_runtime_target() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));

    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("left_result", i64_ty, false),
                LocalDecl::new("right_result", i64_ty, false),
                LocalDecl::new("sum", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Constant(Value::Bool(true))],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Constant(Value::Bool(false))],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::IntegerAdd {
                        dst: Place::local(LocalId(2)),
                        left: Operand::Move(Place::local(LocalId(0)).into()),
                        right: Operand::Move(Place::local(LocalId(1)).into()),
                    }],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
                ),
            ],
        },
    );

    let helper = function(
        "select",
        vec![LocalId(0)],
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("condition", bool_ty, false),
                LocalDecl::new("selected", shared_i64, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Branch {
                        condition: Operand::Copy(Place::local(LocalId(0)).into()),
                        true_target: BasicBlockId(1),
                        false_target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    }],
                    Terminator::Goto(BasicBlockId(3)),
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::PersistentSharedRoot(PersistentId(1)),
                    }],
                    Terminator::Goto(BasicBlockId(3)),
                ),
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: Place::local(LocalId(2)),
                            src: Operand::ReferenceCopy(ReferenceAccess::new(Place::local(
                                LocalId(1),
                            ))),
                        },
                        Statement::Drop {
                            place: Place::local(LocalId(1)).into(),
                        },
                    ],
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(2)).into()))),
                ),
            ],
        },
    );

    let program = validate(
        types,
        vec![
            PersistentDecl::new(i64_ty, Value::I64(11)),
            PersistentDecl::new(i64_ty, Value::I64(22)),
        ],
        vec![entry, helper],
    );
    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(33)))
    );
}

#[test]
fn persistent_shared_carrier_copy_and_move_preserve_the_target() {
    let (types, i64_ty, shared_i64) = shared_i64_types();
    let original = Place::local(LocalId(0));
    let copied = Place::local(LocalId(1));
    let moved = Place::local(LocalId(2));
    let result = Place::local(LocalId(3));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("original", shared_i64, false),
                LocalDecl::new("copied", shared_i64, false),
                LocalDecl::new("moved", shared_i64, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: original.clone(),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    },
                    Statement::Init {
                        dst: copied.clone(),
                        src: Operand::Copy(original.clone().into()),
                    },
                    Statement::Init {
                        dst: moved.clone(),
                        src: Operand::Move(copied.into()),
                    },
                    Statement::Drop {
                        place: original.into(),
                    },
                    Statement::Init {
                        dst: result.clone(),
                        src: Operand::ReferenceCopy(ReferenceAccess::new(moved.clone())),
                    },
                    Statement::Drop {
                        place: moved.into(),
                    },
                ],
                Terminator::Return(Some(Operand::Move(result.into()))),
            )],
        },
    );
    let program = validate(
        types,
        vec![PersistentDecl::new(i64_ty, Value::I64(53))],
        vec![entry],
    );

    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(53)))
    );
}

#[test]
fn complete_shared_reborrow_of_persistent_root_preserves_the_target() {
    let (types, i64_ty, shared_i64) = shared_i64_types();
    let parent = Place::local(LocalId(0));
    let child = Place::local(LocalId(1));
    let result = Place::local(LocalId(2));
    let entry = function(
        "entry",
        Vec::new(),
        Some(i64_ty),
        Body {
            locals: vec![
                LocalDecl::new("parent", shared_i64, false),
                LocalDecl::new("child", shared_i64, false),
                LocalDecl::new("result", i64_ty, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: parent.clone(),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    },
                    Statement::Init {
                        dst: child.clone(),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(parent.clone()),
                        },
                    },
                    Statement::Drop {
                        place: parent.into(),
                    },
                    Statement::Init {
                        dst: result.clone(),
                        src: Operand::ReferenceCopy(ReferenceAccess::new(child.clone())),
                    },
                    Statement::Drop {
                        place: child.into(),
                    },
                ],
                Terminator::Return(Some(Operand::Move(result.into()))),
            )],
        },
    );
    let program = validate(
        types,
        vec![PersistentDecl::new(i64_ty, Value::I64(67))],
        vec![entry],
    );

    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Returned(Some(Value::I64(67)))
    );
}

#[test]
fn persistent_shared_carrier_remains_executable_on_defined_fault_path() {
    let (types, i64_ty, shared_i64) = shared_i64_types();
    let fault = Fault::new("persistent-reference-fault");
    let entry = function(
        "entry",
        Vec::new(),
        None,
        Body {
            locals: vec![
                LocalDecl::new("original", shared_i64, false),
                LocalDecl::new("copy", shared_i64, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Copy(Place::local(LocalId(0)).into()),
                    },
                ],
                Terminator::Fault(fault.clone()),
            )],
        },
    );
    let program = validate(
        types,
        vec![PersistentDecl::new(i64_ty, Value::I64(73))],
        vec![entry],
    );

    assert_eq!(
        assert_differential(program, FunctionId(0)),
        ExecutionOutcome::Faulted(fault)
    );
}
