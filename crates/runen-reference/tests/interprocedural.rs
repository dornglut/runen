use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{
    Machine, ObservedValue, TerminalStatus, UndefinedBehaviorKind, VerificationEvent,
    VerificationEventKind, VerificationWriteKind,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
fn reused_vacant_result_destination_is_initialized_without_replacement_destruction() {
    let (types, tracked) = tracked_types();
    let result = Place::local(LocalId(0));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: None,
        shared_reference_result_origin: None,
        body: body(
            vec![LocalDecl::new("result", tracked, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(result.clone()),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Drop {
                        place: result.clone().into(),
                    }],
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: Vec::new(),
                        destination: Some(result.clone()),
                        target: BasicBlockId(2),
                    },
                ),
                BasicBlock::new(
                    vec![Statement::Read {
                        src: result.clone().into(),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "produce".into(),
        parameters: Vec::new(),
        result: Some(tracked),
        shared_reference_result_origin: None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::Constant(Value::TrackedFixture(7)))),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![caller, callee],
    })
    .expect("a Dead call result destination is vacant");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined repeated result transfer");

    let caller_events = report
        .verification_events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.activation.0 == 1)
        .collect::<Vec<_>>();
    let writes = caller_events
        .iter()
        .filter_map(|(index, event)| {
            matches!(
                event.kind,
                VerificationEventKind::Write {
                    ref place,
                    kind: VerificationWriteKind::Init,
                } if *place == result
            )
            .then_some(*index)
        })
        .collect::<Vec<_>>();
    let drops = caller_events
        .iter()
        .filter_map(|(index, event)| {
            matches!(
                event.kind,
                VerificationEventKind::DropTrackedFixture {
                    ref place,
                    id: 7,
                } if *place == result
            )
            .then_some(*index)
        })
        .collect::<Vec<_>>();

    assert_eq!(writes.len(), 2);
    assert_eq!(drops.len(), 2);
    assert!(writes[0] < drops[0]);
    assert!(drops[0] < writes[1]);
    assert!(writes[1] < drops[1]);
}

#[test]
fn defined_fault_propagates_through_two_suspended_callers_with_exact_cleanup() {
    let (types, tracked) = tracked_types();

    let calling_function = |name: &str, fixture: u64, callee: FunctionId| -> Function {
        Function {
            name: name.into(),
            parameters: Vec::new(),
            result: None,
            shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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
        shared_reference_result_origin: None,
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

fn branch(condition: Operand, true_target: u32, false_target: u32) -> Terminator {
    Terminator::Branch {
        condition,
        true_target: BasicBlockId(true_target),
        false_target: BasicBlockId(false_target),
    }
}

fn execute_constant_branch(condition: bool) -> runen_reference::ExecutionReport {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "branch_result".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        shared_reference_result_origin: None,
        body: body(
            Vec::new(),
            vec![
                BasicBlock::new(
                    Vec::new(),
                    branch(Operand::Constant(Value::Bool(condition)), 1, 2),
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(Value::I64(11)))),
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Constant(Value::I64(22)))),
                ),
            ],
        ),
    };
    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("valid constant Branch program");
    Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined Branch execution")
}

#[test]
fn runtime_branch_selects_exactly_the_concrete_bool_edge() {
    let true_report = execute_constant_branch(true);
    assert_eq!(true_report.terminal, TerminalStatus::Returned);
    assert_eq!(true_report.result, Some(ObservedValue::I64(11)));

    let false_report = execute_constant_branch(false);
    assert_eq!(false_report.terminal, TerminalStatus::Returned);
    assert_eq!(false_report.result, Some(ObservedValue::I64(22)));
}

fn execute_stored_bool_branch(copy: bool) -> runen_reference::ExecutionReport {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let condition = Place::local(LocalId(0));
    let operand = if copy {
        Operand::Copy(condition.clone().into())
    } else {
        Operand::Move(condition.clone().into())
    };
    let function = Function {
        name: "stored_condition".into(),
        parameters: Vec::new(),
        result: None,
        shared_reference_result_origin: None,
        body: body(
            vec![LocalDecl::new("condition", bool_ty, false)],
            vec![
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: condition,
                        src: Operand::Constant(Value::Bool(true)),
                    }],
                    branch(operand, 1, 2),
                ),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
                BasicBlock::new(Vec::new(), Terminator::Return(None)),
            ],
        ),
    };
    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("valid stored-Bool Branch program");
    Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined stored-Bool Branch execution")
}

#[test]
fn runtime_branch_move_and_copy_conditions_are_evaluated_exactly_once() {
    let moved = execute_stored_bool_branch(false);
    assert_eq!(
        moved
            .verification_events
            .iter()
            .filter(|event| matches!(event.kind, VerificationEventKind::Move(_)))
            .count(),
        1
    );
    assert_eq!(
        moved
            .verification_events
            .iter()
            .filter(|event| matches!(event.kind, VerificationEventKind::Copy(_)))
            .count(),
        0
    );

    let copied = execute_stored_bool_branch(true);
    assert_eq!(
        copied
            .verification_events
            .iter()
            .filter(|event| matches!(event.kind, VerificationEventKind::Copy(_)))
            .count(),
        1
    );
    assert_eq!(
        copied
            .verification_events
            .iter()
            .filter(|event| matches!(event.kind, VerificationEventKind::Move(_)))
            .count(),
        0
    );
}

#[test]
fn runtime_branch_condition_ub_reaches_neither_successor() {
    let mut types = TypeTable::new();
    let bool_ty = types.push(TypeDef::scalar("Bool", ScalarType::Bool));
    let bool_ptr = types.push(TypeDef::raw_pointer("BoolPtr", bool_ty));
    let tracked_ty = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let condition = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let marker = Place::local(LocalId(2));
    let function = Function {
        name: "ub_condition".into(),
        parameters: Vec::new(),
        result: None,
        shared_reference_result_origin: None,
        body: body(
            vec![
                LocalDecl::new("condition", bool_ty, false),
                LocalDecl::new("pointer", bool_ptr, false),
                LocalDecl::new("marker", tracked_ty, false),
            ],
            vec![
                BasicBlock::new(
                    vec![
                        Statement::Init {
                            dst: condition.clone(),
                            src: Operand::Constant(Value::Bool(true)),
                        },
                        Statement::Init {
                            dst: pointer.clone(),
                            src: Operand::AddressOf(condition.clone().into()),
                        },
                        Statement::Drop {
                            place: condition.into(),
                        },
                    ],
                    branch(Operand::RawMove(pointer.into()), 1, 2),
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: marker.clone(),
                        src: Operand::Constant(Value::TrackedFixture(1)),
                    }],
                    Terminator::Return(None),
                ),
                BasicBlock::new(
                    vec![Statement::Init {
                        dst: marker.clone(),
                        src: Operand::Constant(Value::TrackedFixture(2)),
                    }],
                    Terminator::Return(None),
                ),
            ],
        ),
    };
    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("unsafe condition failure is valid MIR with no defined Branch successor");
    let error = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect_err("RawMove through dead Bool target is UB");

    assert!(matches!(
        error.kind,
        UndefinedBehaviorKind::RawMoveTargetNotLive { .. }
    ));
    assert!(!error.verification_events.iter().any(|event| {
        matches!(
            event.kind,
            VerificationEventKind::Write {
                ref place,
                kind: VerificationWriteKind::Init,
            } if *place == marker
        )
    }));
}
