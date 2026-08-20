use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{
    Machine, TerminalStatus, UndefinedBehaviorKind, VerificationEvent, VerificationEventKind,
    VerificationWriteKind,
};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn tracked_types() -> (TypeTable, runen_core_ir::TypeId) {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    (types, tracked)
}

fn event_position(
    events: &[VerificationEvent],
    predicate: impl Fn(&VerificationEvent) -> bool,
) -> usize {
    events
        .iter()
        .position(predicate)
        .expect("expected verification event")
}

#[test]
fn result_is_preserved_across_callee_cleanup_and_caller_resumes_after_cleanup() {
    let (types, tracked) = tracked_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("argument", tracked, false),
                LocalDecl::new("result", tracked, false),
            ],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::TrackedFixture(1)),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::Move(Place::local(LocalId(0)).into())],
                        destination: Some(Place::local(LocalId(1))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: Place::local(LocalId(1)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "callee".into(),
        parameters: vec![LocalId(0)],
        result: Some(tracked),
        body: body(
            vec![
                LocalDecl::new("parameter", tracked, false),
                LocalDecl::new("temporary", tracked, false),
            ],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(1)),
                    src: Operand::Constant(Value::TrackedFixture(2)),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("valid result call");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined execution");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, None);

    let callee_move = event_position(&report.verification_events, |event| {
        event.activation.0 == 2
            && matches!(
                event.kind,
                VerificationEventKind::Move(ref place) if *place == Place::local(LocalId(0))
            )
    });
    let callee_cleanup = event_position(&report.verification_events, |event| {
        event.activation.0 == 2
            && matches!(
                event.kind,
                VerificationEventKind::DropTrackedFixture { id: 2, .. }
            )
    });
    let caller_result_write = event_position(&report.verification_events, |event| {
        event.activation.0 == 1
            && matches!(
                event.kind,
                VerificationEventKind::Write {
                    ref place,
                    kind: VerificationWriteKind::Init,
                } if *place == Place::local(LocalId(1))
            )
    });
    let caller_read = event_position(&report.verification_events, |event| {
        event.activation.0 == 1
            && matches!(
                event.kind,
                VerificationEventKind::Read(ref place) if *place == Place::local(LocalId(1))
            )
    });

    assert!(callee_move < callee_cleanup);
    assert!(callee_cleanup < caller_result_write);
    assert!(caller_result_write < caller_read);

    let returned_value_drops = report
        .verification_events
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                VerificationEventKind::DropTrackedFixture { id: 1, .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(returned_value_drops.len(), 1);
    assert_eq!(returned_value_drops[0].activation.0, 1);
}

#[test]
fn defined_fault_propagates_through_two_suspended_callers_with_exact_cleanup() {
    let (types, tracked) = tracked_types();

    let calling_function = |name: &str, fixture: u64, callee: FunctionId| -> Function {
        Function {
            name: name.into(),
            parameters: Vec::new(),
            result: None,
            body: body(
                vec![LocalDecl::new("owned", tracked, false)],
                vec![
                    BasicBlock::new(
                        vec![Statement::Init {
                            dst: Place::local(LocalId(0)),
                            src: Operand::Constant(Value::TrackedFixture(fixture)),
                        }],
                        Terminator::Call {
                            function: callee,
                            arguments: Vec::new(),
                            destination: None,
                            target: BasicBlockId(1),
                        },
                    ),
                    BasicBlock::new(Vec::new(), Terminator::Return(None)),
                ],
            ),
        }
    };

    let outer = calling_function("outer", 10, FunctionId(1));
    let middle = calling_function("middle", 20, FunctionId(2));
    let inner = Function {
        name: "inner".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![LocalDecl::new("owned", tracked, false)],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::Constant(Value::TrackedFixture(30)),
                }],
                Terminator::Fault(Fault::new("boom")),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![outer, middle, inner],
    })
    .expect("faulting calls are structurally valid");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined fault is not UB");

    assert_eq!(report.terminal, TerminalStatus::Faulted("boom".into()));
    assert_eq!(report.result, None);

    let drops = report
        .verification_events
        .iter()
        .filter_map(|event| match event.kind {
            VerificationEventKind::DropTrackedFixture { id, .. } => Some((event.activation.0, id)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(drops, vec![(3, 30), (2, 20), (1, 10)]);
}

#[test]
fn faulting_result_call_does_not_write_destination_or_follow_normal_continuation() {
    let (types, tracked) = tracked_types();
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![LocalDecl::new("result", tracked, false)],
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
                    vec![Statement::Read {
                        src: Place::local(LocalId(0)).into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "faulting_result".into(),
        parameters: Vec::new(),
        result: Some(tracked),
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Fault(Fault::new("result-fault")),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("result-bearing call with a faulting execution path is valid");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined fault is not UB");

    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("result-fault".into())
    );
    assert!(!report.verification_events.iter().any(|event| {
        event.activation.0 == 1
            && matches!(
                event.kind,
                VerificationEventKind::Write {
                    ref place,
                    kind: VerificationWriteKind::Init,
                } if *place == Place::local(LocalId(0))
            )
    }));
    assert!(!report.verification_events.iter().any(|event| {
        event.activation.0 == 1
            && matches!(
                event.kind,
                VerificationEventKind::Read(ref place) if *place == Place::local(LocalId(0))
            )
    }));
}

#[test]
fn repeated_no_result_calls_get_fresh_activation_and_storage_and_resume_normally() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("Ptr", i64_ty));

    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: None,
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let callee = Function {
        name: "callee".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("value", i64_ty, false),
                LocalDecl::new("pointer", pointer_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(7)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::AddressOf(Place::local(LocalId(0)).into()),
                    },
                    Statement::RawRead {
                        pointer: Place::local(LocalId(1)).into(),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("intra-activation raw pointer use is valid");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined execution");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    let addresses = report
        .verification_events
        .iter()
        .filter_map(|event| match &event.kind {
            VerificationEventKind::AddressOf { pointer, .. } => {
                Some((event.activation.0, pointer.target.instance))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(addresses.len(), 2);
    assert_eq!(addresses[0].0, 2);
    assert_eq!(addresses[1].0, 3);
    assert_ne!(addresses[0].1, addresses[1].1);

    let raw_reads = report
        .verification_events
        .iter()
        .filter(|event| matches!(event.kind, VerificationEventKind::RawRead { .. }))
        .count();
    assert_eq!(raw_reads, 2);
}

#[test]
fn intra_activation_raw_pointer_ub_remains_an_ub_boundary() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("Ptr", i64_ty));
    let function = Function {
        name: "entry".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("value", i64_ty, false),
                LocalDecl::new("pointer", pointer_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::I64(3)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::AddressOf(Place::local(LocalId(0)).into()),
                    },
                    Statement::Drop {
                        place: Place::local(LocalId(0)).into(),
                    },
                    Statement::RawRead {
                        pointer: Place::local(LocalId(1)).into(),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("unsafe precondition failure is an execution boundary, not invalid MIR");
    let error = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect_err("raw read from dead target is UB");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawReadTargetNotLive { .. }
    ));
}
