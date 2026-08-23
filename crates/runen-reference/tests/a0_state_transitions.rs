mod support;

use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, LocalDecl, LocalId, MirValidationErrorKind, Operand,
    Place, Program, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{ExecutionReport, VerificationEventKind, VerificationWriteKind};
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
        .expect("A0 state-transition fixture must have defined execution")
}

#[test]
fn partial_move_makes_whole_aggregate_unreadable_until_reinitialized() {
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

    let program = one_block(
        types,
        vec![
            LocalDecl::new("pair", pair, false),
            LocalDecl::new("taken", tracked, false),
        ],
        vec![
            Statement::Init {
                dst: root.clone(),
                src: Operand::Constant(Value::Struct(vec![
                    Value::TrackedFixture(10),
                    Value::TrackedFixture(20),
                ])),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(root.clone().field(0).into()),
            },
            Statement::Read {
                src: root.clone().into(),
            },
        ],
    );

    let error = validate_program(program)
        .expect_err("a partially moved aggregate must be rejected before execution");
    assert_eq!(error.kind, MirValidationErrorKind::UseOfUninitialized(root));
}

#[test]
fn assign_reinitializes_storage_that_became_dead_after_move() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));

    let program = one_block(
        types,
        vec![
            LocalDecl::new("value", tracked, true),
            LocalDecl::new("taken", tracked, false),
        ],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(value.clone().into()),
            },
            Statement::Assign {
                dst: value.clone().into(),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
            Statement::Read {
                src: value.clone().into(),
            },
        ],
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);

    assert!(events.contains(&VerificationEventKind::Write {
        place: value,
        kind: VerificationWriteKind::Assign,
    }));
    let dropped_ids = events
        .iter()
        .filter_map(|event| match event {
            VerificationEventKind::DropTrackedFixture { id, .. } => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(dropped_ids, vec![1, 2]);
}

#[test]
fn init_reinitializes_storage_that_became_dead_after_move_without_replacement() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));
    let taken = Place::local(LocalId(1));

    let program = one_block(
        types,
        vec![
            LocalDecl::new("value", tracked, false),
            LocalDecl::new("taken", tracked, false),
        ],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Init {
                dst: taken.clone(),
                src: Operand::Move(value.clone().into()),
            },
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
            Statement::Read {
                src: value.clone().into(),
            },
        ],
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);
    assert_eq!(
        events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    VerificationEventKind::Write {
                        place,
                        kind: VerificationWriteKind::Init,
                    } if *place == value
                )
            })
            .count(),
        2
    );
    let dropped_ids = events
        .iter()
        .filter_map(|event| match event {
            VerificationEventKind::DropTrackedFixture { id, .. } => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(dropped_ids, vec![1, 2]);
}

#[test]
fn assign_can_initialize_never_initialized_mutable_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let program = one_block(
        types,
        vec![LocalDecl::new("value", i64_ty, true)],
        vec![
            Statement::Assign {
                dst: value.clone().into(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Read {
                src: value.clone().into(),
            },
        ],
    );

    let report = defined_report(program);
    assert!(
        event_kinds(&report.verification_events).contains(&VerificationEventKind::Write {
            place: value,
            kind: VerificationWriteKind::Assign,
        })
    );
}

#[test]
fn assign_replaces_partially_initialized_aggregate_and_drops_only_live_old_parts() {
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

    let program = one_block(
        types,
        vec![LocalDecl::new("pair", pair, true)],
        vec![
            Statement::Init {
                dst: root.clone().field(0),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Assign {
                dst: root.clone().into(),
                src: Operand::Constant(Value::Struct(vec![
                    Value::TrackedFixture(2),
                    Value::TrackedFixture(3),
                ])),
            },
            Statement::Read {
                src: root.clone().into(),
            },
        ],
    );

    let report = defined_report(program);
    let events = event_kinds(&report.verification_events);
    let dropped_ids = events
        .iter()
        .filter_map(|event| match event {
            VerificationEventKind::DropTrackedFixture { id, .. } => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(dropped_ids, vec![1, 3, 2]);
    assert!(events.contains(&VerificationEventKind::Write {
        place: root,
        kind: VerificationWriteKind::Assign,
    }));
}

#[test]
fn explicit_drop_of_partial_aggregate_destroys_only_live_subobjects() {
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

    let program = one_block(
        types,
        vec![LocalDecl::new("pair", pair, false)],
        vec![
            Statement::Init {
                dst: left.clone(),
                src: Operand::Constant(Value::TrackedFixture(7)),
            },
            Statement::Drop {
                place: root.clone().into(),
            },
        ],
    );

    let report = defined_report(program);
    let drops = event_kinds(&report.verification_events)
        .iter()
        .filter_map(|event| match event {
            VerificationEventKind::DropTrackedFixture { place, id } => Some((place.clone(), *id)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(drops, vec![(left, 7)]);
}

#[test]
fn init_starts_a_new_stored_value_lifetime_after_drop() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));

    let program = one_block(
        types,
        vec![LocalDecl::new("value", tracked, false)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Drop {
                place: value.clone().into(),
            },
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
        ],
    );

    let report = defined_report(program);
    assert_eq!(
        event_kinds(&report.verification_events),
        vec![
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::DropTrackedFixture {
                place: value.clone(),
                id: 1,
            },
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::DropTrackedFixture {
                place: value,
                id: 2,
            },
        ]
    );
}

#[test]
fn later_vacant_init_preserves_raw_pointer_storage_identity() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", i64_ty));
    let target = Place::local(LocalId(0));

    let program = one_block(
        types,
        vec![
            LocalDecl::new("target", i64_ty, false),
            LocalDecl::new("before", pointer_ty, false),
            LocalDecl::new("after", pointer_ty, false),
        ],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Drop {
                place: target.clone().into(),
            },
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(target.into()),
            },
        ],
    );

    let report = defined_report(program);
    let pointers = event_kinds(&report.verification_events)
        .into_iter()
        .filter_map(|event| match event {
            VerificationEventKind::AddressOf { pointer, .. } => Some(pointer),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(pointers.len(), 2);
    assert_eq!(pointers[0], pointers[1]);
}

#[test]
fn assign_starts_a_new_stored_value_lifetime_after_drop() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));

    let program = one_block(
        types,
        vec![LocalDecl::new("value", tracked, true)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Drop {
                place: value.clone().into(),
            },
            Statement::Assign {
                dst: value.clone().into(),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
            Statement::Read {
                src: value.clone().into(),
            },
        ],
    );

    let report = defined_report(program);
    assert_eq!(
        event_kinds(&report.verification_events),
        vec![
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::DropTrackedFixture {
                place: value.clone(),
                id: 1,
            },
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Assign,
            },
            VerificationEventKind::Read(value.clone()),
            VerificationEventKind::DropTrackedFixture {
                place: value,
                id: 2,
            },
        ]
    );
}

#[test]
fn self_move_assignment_uses_the_post_source_destruction_domain() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));

    let program = one_block(
        types,
        vec![LocalDecl::new("value", tracked, true)],
        vec![
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(1)),
            },
            Statement::Assign {
                dst: value.clone().into(),
                src: Operand::Move(value.clone().into()),
            },
        ],
    );

    let report = defined_report(program);
    assert_eq!(
        event_kinds(&report.verification_events),
        vec![
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEventKind::Move(value.clone()),
            VerificationEventKind::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Assign,
            },
            VerificationEventKind::DropTrackedFixture {
                place: value,
                id: 1,
            },
        ]
    );
}
