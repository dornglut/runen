use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, LocalDecl, LocalId, MirValidationErrorKind, Operand,
    Place, ScalarType, Statement, Terminator, TypeDef, TypeTable, Value, validate_body,
};
use runen_reference::{Machine, VerificationEvent, VerificationWriteKind};

fn one_block(types: TypeTable, locals: Vec<LocalDecl>, statements: Vec<Statement>) -> Body {
    Body {
        types,
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, Terminator::Return)],
    }
}

fn machine(body: Body) -> Machine {
    Machine::new(validate_body(body).expect("A0 test MIR must pass validation"))
}

fn defined_report(body: Body) -> runen_reference::ExecutionReport {
    machine(body)
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

    let body = one_block(
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

    let error = validate_body(body)
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

    let body = one_block(
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

    let report = defined_report(body);

    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Write {
                place: value,
                kind: VerificationWriteKind::Assign,
            })
    );
    let dropped_ids = report
        .verification_events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::DropTrackedFixture { id, .. } => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(dropped_ids, vec![1, 2]);
}

#[test]
fn init_cannot_reinitialize_storage_that_became_dead_after_move() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));

    let body = one_block(
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
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::TrackedFixture(2)),
            },
        ],
    );

    let error =
        validate_body(body).expect_err("dead storage must be reinitialized with Assign, not Init");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InitRequiresNeverInitialized(value)
    );
}

#[test]
fn assign_can_initialize_never_initialized_mutable_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let value = Place::local(LocalId(0));

    let body = one_block(
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

    let report = defined_report(body);
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Write {
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

    let body = one_block(
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

    let report = defined_report(body);
    let dropped_ids = report
        .verification_events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::DropTrackedFixture { id, .. } => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(dropped_ids, vec![1, 3, 2]);
    assert!(
        report
            .verification_events
            .contains(&VerificationEvent::Write {
                place: root,
                kind: VerificationWriteKind::Assign,
            })
    );
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

    let body = one_block(
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

    let report = defined_report(body);
    let drops = report
        .verification_events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::DropTrackedFixture { place, id } => Some((place.clone(), *id)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(drops, vec![(left, 7)]);
}

#[test]
fn init_cannot_start_a_second_stored_value_lifetime_after_drop() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));

    let body = one_block(
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

    let error = validate_body(body).expect_err("Drop leaves Dead storage, not fresh storage");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InitRequiresNeverInitialized(value)
    );
}

#[test]
fn assign_starts_a_new_stored_value_lifetime_after_drop() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar(
        "TrackedFixture",
        ScalarType::TrackedFixture,
    ));
    let value = Place::local(LocalId(0));

    let body = one_block(
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

    let report = defined_report(body);
    assert_eq!(
        report.verification_events,
        vec![
            VerificationEvent::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEvent::DropTrackedFixture {
                place: value.clone(),
                id: 1,
            },
            VerificationEvent::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Assign,
            },
            VerificationEvent::Read(value.clone()),
            VerificationEvent::DropTrackedFixture {
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

    let body = one_block(
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

    let report = defined_report(body);
    assert_eq!(
        report.verification_events,
        vec![
            VerificationEvent::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Init,
            },
            VerificationEvent::Move(value.clone()),
            VerificationEvent::Write {
                place: value.clone(),
                kind: VerificationWriteKind::Assign,
            },
            VerificationEvent::DropTrackedFixture {
                place: value,
                id: 1,
            },
        ]
    );
}
