mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Field, LocalDecl, LocalId, MirValidationErrorKind,
    Operand, Place, Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value,
    validate_program,
};
use runen_reference::{ExecutionReport, TerminalStatus, VerificationEventKind};
use support::{event_kinds, machine, one_function_program};

fn one_block(types: TypeTable, locals: Vec<LocalDecl>, statements: Vec<Statement>) -> Program {
    one_function_program(
        types,
        Body {
            locals,
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(statements, Terminator::Return(None))],
        },
    )
}

fn defined_report(program: Program) -> ExecutionReport {
    machine(program)
        .execute()
        .expect("A0 value/place fixture must have defined execution")
}

#[test]
fn move_invalidates_source() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));

    let program = one_block(
        types,
        vec![
            LocalDecl::new("source", tracked, false),
            LocalDecl::new("target", tracked, false),
        ],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::TrackedFixture(7)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(Place::local(LocalId(0)).into()),
            },
            Statement::Read {
                src: Place::local(LocalId(0)).into(),
            },
        ],
    );

    let error = validate_program(program).expect_err("read after move must fail MIR validation");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(Place::local(LocalId(0)))
    );
}

#[test]
fn copy_preserves_source() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));

    let program = one_block(
        types,
        vec![
            LocalDecl::new("source", i64_ty, false),
            LocalDecl::new("target", i64_ty, false),
        ],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::I64(42)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(Place::local(LocalId(0)).into()),
            },
            Statement::Read {
                src: Place::local(LocalId(0)).into(),
            },
            Statement::Read {
                src: Place::local(LocalId(1)).into(),
            },
        ],
    );

    let report = defined_report(program);
    assert_eq!(report.terminal, TerminalStatus::Returned);
    let events = event_kinds(&report.verification_events);
    assert!(events.contains(&VerificationEventKind::Copy(Place::local(LocalId(0)))));
    assert!(events.contains(&VerificationEventKind::Read(Place::local(LocalId(0)))));
}

#[test]
fn partial_move_keeps_disjoint_field_live_and_drops_only_remaining_value() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", tracked), Field::new("right", tracked)],
    ));

    let left = Place::local(LocalId(0)).field(0);
    let right = Place::local(LocalId(0)).field(1);

    let program = one_block(
        types,
        vec![
            LocalDecl::new("pair", pair, false),
            LocalDecl::new("taken", tracked, false),
        ],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::Struct(vec![
                    Value::TrackedFixture(10),
                    Value::TrackedFixture(20),
                ])),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(left.clone().into()),
            },
            Statement::Read {
                src: right.clone().into(),
            },
        ],
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);
    let drops: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            VerificationEventKind::DropTrackedFixture { place, id } => Some((place.clone(), *id)),
            _ => None,
        })
        .collect();

    // Locals drop in reverse declaration order: `taken` (id 10), then the
    // still-live `pair.right` (id 20). Moved `pair.left` is not dropped twice.
    assert_eq!(drops, vec![(Place::local(LocalId(1)), 10), (right, 20)]);
}

#[test]
fn explicit_drop_is_not_repeated_at_scope_cleanup() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));

    let program = one_block(
        types,
        vec![LocalDecl::new("value", tracked, false)],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::TrackedFixture(99)),
            },
            Statement::Drop {
                place: Place::local(LocalId(0)).into(),
            },
        ],
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);
    let drops = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                VerificationEventKind::DropTrackedFixture { id: 99, .. }
            )
        })
        .count();
    assert_eq!(drops, 1);
}

#[test]
fn fault_unwinds_live_locals_once_in_reverse_declaration_order() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));

    let program = one_function_program(
        types,
        Body {
            locals: vec![
                LocalDecl::new("first", tracked, false),
                LocalDecl::new("second", tracked, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::Constant(Value::TrackedFixture(1)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::Constant(Value::TrackedFixture(2)),
                    },
                ],
                Terminator::Fault(Fault::new("TEST_FAULT")),
            )],
        },
    );

    let report = defined_report(program);
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("TEST_FAULT".into())
    );

    let events = event_kinds(&report.verification_events);
    let ids: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            VerificationEventKind::DropTrackedFixture { id, .. } => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, vec![2, 1]);
}

#[test]
fn fault_cleanup_uses_the_current_partial_destruction_domain() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", tracked), Field::new("right", tracked)],
    ));
    let root = Place::local(LocalId(0));
    let left = root.clone().field(0);
    let right = root.clone().field(1);
    let taken = Place::local(LocalId(1));

    let program = one_function_program(
        types,
        Body {
            locals: vec![
                LocalDecl::new("pair", pair, false),
                LocalDecl::new("taken", tracked, false),
            ],
            loans: Vec::new(),
            entry: BasicBlockId(0),
            blocks: vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: root,
                        src: Operand::Constant(Value::Struct(vec![
                            Value::TrackedFixture(10),
                            Value::TrackedFixture(20),
                        ])),
                    },
                    Statement::Init {
                        dst: taken.clone(),
                        src: Operand::Move(left.into()),
                    },
                    Statement::Drop {
                        place: right.clone().into(),
                    },
                ],
                Terminator::Fault(Fault::new("PARTIAL_FAULT")),
            )],
        },
    );

    let report = defined_report(program);
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("PARTIAL_FAULT".into())
    );

    let events = event_kinds(&report.verification_events);
    let drops = events
        .iter()
        .filter_map(|event| match event {
            VerificationEventKind::DropTrackedFixture { place, id } => Some((place.clone(), *id)),
            _ => None,
        })
        .collect::<Vec<_>>();

    // `pair.right` is destroyed explicitly before the fault. Fault cleanup then
    // destroys `taken` and skips both Dead fields of `pair`.
    assert_eq!(drops, vec![(right, 20), (taken, 10)]);
}

#[test]
fn fields_can_be_first_initialized_independently_before_whole_value_is_read() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", i64_ty), Field::new("right", i64_ty)],
    ));
    let root = Place::local(LocalId(0));

    let program = one_block(
        types,
        vec![LocalDecl::new("pair", pair, false)],
        vec![
            Statement::Init {
                dst: root.clone().field(0),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: root.clone().field(1),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Read { src: root.into() },
        ],
    );

    defined_report(program);
}

#[test]
fn struct_fields_drop_in_reverse_declaration_order() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let pair = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", tracked), Field::new("right", tracked)],
    ));

    let program = one_block(
        types,
        vec![LocalDecl::new("pair", pair, false)],
        vec![Statement::Init {
            dst: Place::local(LocalId(0)),
            src: Operand::Constant(Value::Struct(vec![
                Value::TrackedFixture(1),
                Value::TrackedFixture(2),
            ])),
        }],
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);
    let ids: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            VerificationEventKind::DropTrackedFixture { id, .. } => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, vec![2, 1]);
}

#[test]
fn copy_of_noncopy_value_is_rejected_before_execution() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));

    let program = one_block(
        types,
        vec![
            LocalDecl::new("source", tracked, false),
            LocalDecl::new("target", tracked, false),
        ],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::TrackedFixture(5)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(Place::local(LocalId(0)).into()),
            },
        ],
    );

    let error =
        validate_program(program).expect_err("TrackedFixture values are not copyable in A0");
    assert_eq!(error.kind, MirValidationErrorKind::CopyOfNonCopy(tracked));
}
