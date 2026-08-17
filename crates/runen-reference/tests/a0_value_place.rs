use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Fault, Field, LocalDecl, LocalId, MirValidationErrorKind,
    Operand, Place, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_body,
};
use runen_reference::{Machine, TerminalStatus, VerificationEvent};

fn one_block(types: TypeTable, locals: Vec<LocalDecl>, statements: Vec<Statement>) -> Body {
    Body {
        types,
        locals,
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, Terminator::Return)],
    }
}

fn machine(body: Body) -> Machine {
    Machine::new(validate_body(body).expect("A0 test MIR must pass validation"))
}

#[test]
fn move_invalidates_source() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));

    let body = one_block(
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
                src: Operand::Move(Place::local(LocalId(0))),
            },
            Statement::Read {
                src: Place::local(LocalId(0)),
            },
        ],
    );

    let error = validate_body(body).expect_err("read after move must fail MIR validation");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::UseOfUninitialized(Place::local(LocalId(0)))
    );
}

#[test]
fn copy_preserves_source() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));

    let body = one_block(
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
                src: Operand::Copy(Place::local(LocalId(0))),
            },
            Statement::Read {
                src: Place::local(LocalId(0)),
            },
            Statement::Read {
                src: Place::local(LocalId(1)),
            },
        ],
    );

    let report = machine(body).execute();
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Copy(Place::local(LocalId(0))))
    );
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Read(Place::local(LocalId(0))))
    );
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

    let body = one_block(
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
                src: Operand::Move(left.clone()),
            },
            Statement::Read { src: right.clone() },
        ],
    );

    let report = machine(body).execute();
    let drops: Vec<_> = report
        .verification_events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::DropTrackedFixture { place, id } => Some((place.clone(), *id)),
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

    let body = one_block(
        types,
        vec![LocalDecl::new("value", tracked, false)],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::TrackedFixture(99)),
            },
            Statement::Drop {
                place: Place::local(LocalId(0)),
            },
        ],
    );

    let report = machine(body).execute();
    let drops = report
        .verification_events
        .iter()
        .filter(|event| matches!(event, VerificationEvent::DropTrackedFixture { id: 99, .. }))
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

    let body = Body {
        types,
        locals: vec![
            LocalDecl::new("first", tracked, false),
            LocalDecl::new("second", tracked, false),
        ],
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
    };

    let report = machine(body).execute();
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("TEST_FAULT".into())
    );

    let ids: Vec<_> = report
        .verification_events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::DropTrackedFixture { id, .. } => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, vec![2, 1]);
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

    let body = one_block(
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
            Statement::Read { src: root },
        ],
    );

    let _ = machine(body).execute();
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

    let body = one_block(
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

    let report = machine(body).execute();
    let ids: Vec<_> = report
        .verification_events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::DropTrackedFixture { id, .. } => Some(*id),
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

    let body = one_block(
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
                src: Operand::Copy(Place::local(LocalId(0))),
            },
        ],
    );

    let error = validate_body(body).expect_err("TrackedFixture values are not copyable in A0");
    assert_eq!(error.kind, MirValidationErrorKind::CopyOfNonCopy(tracked));
}
