use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, LocalDecl, LocalId, Operand, Place, ScalarType,
    Statement, Terminator, TypeDef, TypeTable, Value,
};
use runen_reference::{Machine, SemanticErrorKind, TraceEvent, WriteKind};

fn one_block(types: TypeTable, locals: Vec<LocalDecl>, statements: Vec<Statement>) -> Body {
    Body {
        types,
        locals,
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, Terminator::Return)],
    }
}

#[test]
fn partial_move_makes_whole_aggregate_unreadable_until_reinitialized() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::Tracked));
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
                src: Operand::Constant(Value::Struct(vec![Value::Tracked(10), Value::Tracked(20)])),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(root.clone().field(0)),
            },
            Statement::Read { src: root.clone() },
        ],
    );

    let error = Machine::new(body)
        .execute()
        .expect_err("a partially moved aggregate must not be readable as a whole");
    assert_eq!(error.kind, SemanticErrorKind::UseOfUninitialized(root));
}

#[test]
fn assign_reinitializes_storage_that_became_dead_after_move() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::Tracked));
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
                src: Operand::Constant(Value::Tracked(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(value.clone()),
            },
            Statement::Assign {
                dst: value.clone(),
                src: Operand::Constant(Value::Tracked(2)),
            },
            Statement::Read { src: value.clone() },
        ],
    );

    let report = Machine::new(body)
        .execute()
        .expect("assignment must be able to reinitialize mutable dead storage");

    assert!(report.trace.contains(&TraceEvent::Write {
        place: value,
        kind: WriteKind::Assign,
    }));
    let dropped_ids = report
        .trace
        .iter()
        .filter_map(|event| match event {
            TraceEvent::DropTracked { id, .. } => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(dropped_ids, vec![1, 2]);
}

#[test]
fn init_cannot_reinitialize_storage_that_became_dead_after_move() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::Tracked));
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
                src: Operand::Constant(Value::Tracked(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(value.clone()),
            },
            Statement::Init {
                dst: value.clone(),
                src: Operand::Constant(Value::Tracked(2)),
            },
        ],
    );

    let error = Machine::new(body)
        .execute()
        .expect_err("dead storage must be reinitialized with Assign, not Init");
    assert_eq!(
        error.kind,
        SemanticErrorKind::InitRequiresNeverInitialized(value)
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
                dst: value.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Read { src: value.clone() },
        ],
    );

    let report = Machine::new(body)
        .execute()
        .expect("mutable assignment may initialize never-initialized storage");
    assert!(report.trace.contains(&TraceEvent::Write {
        place: value,
        kind: WriteKind::Assign,
    }));
}

#[test]
fn explicit_drop_of_partial_aggregate_destroys_only_live_subobjects() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::Tracked));
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
                src: Operand::Constant(Value::Tracked(7)),
            },
            Statement::Drop {
                place: root.clone(),
            },
        ],
    );

    let report = Machine::new(body)
        .execute()
        .expect("a partial aggregate with a live subobject can be explicitly dropped");
    let drops = report
        .trace
        .iter()
        .filter_map(|event| match event {
            TraceEvent::DropTracked { place, id } => Some((place.clone(), *id)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(drops, vec![(left, 7)]);
}
