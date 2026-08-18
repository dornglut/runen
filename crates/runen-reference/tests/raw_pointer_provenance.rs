use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
    Operand, Place, PlaceAccess, Projection, ScalarType, Statement, Terminator, TypeDef, TypeTable,
    Value, validate_body,
};
use runen_reference::{Machine, RawPointerValue, VerificationEvent};

fn execute(
    types: TypeTable,
    locals: Vec<LocalDecl>,
    loans: Vec<LoanDecl>,
    statements: Vec<Statement>,
) -> Vec<VerificationEvent> {
    let body = Body {
        types,
        locals,
        loans,
        entry: BasicBlockId(0),
        blocks: vec![BasicBlock::new(statements, Terminator::Return)],
    };
    Machine::new(validate_body(body).expect("valid pointer-provenance fixture"))
        .execute()
        .verification_events
}

fn formed(events: &[VerificationEvent]) -> Vec<RawPointerValue> {
    events
        .iter()
        .filter_map(|event| match event {
            VerificationEvent::AddressOf { pointer, .. } => Some(pointer.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn distinct_local_storage_extents_have_distinct_storage_instances() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let events = execute(
        types,
        vec![
            LocalDecl::new("left", value_ty, false),
            LocalDecl::new("right", value_ty, false),
            LocalDecl::new("left_ptr", pointer_ty, false),
            LocalDecl::new("right_ptr", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::AddressOf(Place::local(LocalId(1)).into()),
            },
        ],
    );
    let pointers = formed(&events);

    assert_eq!(pointers.len(), 2);
    assert_ne!(pointers[0].target.instance, pointers[1].target.instance);
    assert_eq!(pointers[0].provenance, pointers[0].target.instance);
    assert_eq!(pointers[1].provenance, pointers[1].target.instance);
}

#[test]
fn address_of_field_records_structural_region_without_layout() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pair_ty = types.push(TypeDef::structure(
        "Pair",
        vec![Field::new("left", value_ty), Field::new("right", value_ty)],
    ));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let events = execute(
        types,
        vec![
            LocalDecl::new("pair", pair_ty, false),
            LocalDecl::new("pointer", pointer_ty, false),
        ],
        Vec::new(),
        vec![Statement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::AddressOf(Place::local(LocalId(0)).field(1).into()),
        }],
    );
    let pointers = formed(&events);

    assert_eq!(pointers.len(), 1);
    assert_eq!(pointers[0].target.projections, vec![Projection::Field(1)]);
    assert_eq!(pointers[0].provenance, pointers[0].target.instance);
}

#[test]
fn direct_and_loan_address_formation_share_underlying_storage_provenance() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let events = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("direct", pointer_ty, false),
            LocalDecl::new("loaned", pointer_ty, false),
        ],
        vec![LoanDecl::new("shared", value_ty)],
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: target.clone().into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(PlaceAccess::loan(LoanId(0))),
            },
            Statement::EndBorrow { loan: LoanId(0) },
        ],
    );
    let pointers = formed(&events);

    assert_eq!(pointers.len(), 2);
    assert_eq!(pointers[0], pointers[1]);
}

#[test]
fn ordinary_replacement_preserves_storage_identity_and_pointer_provenance() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let events = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, true),
            LocalDecl::new("before", pointer_ty, false),
            LocalDecl::new("after", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Assign {
                dst: target.clone().into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(target.into()),
            },
        ],
    );
    let pointers = formed(&events);

    assert_eq!(pointers.len(), 2);
    assert_eq!(pointers[0], pointers[1]);
}

#[test]
fn interior_replacement_preserves_storage_identity_and_pointer_provenance() {
    let mut types = TypeTable::new();
    let value_ty =
        types.push(TypeDef::scalar("InteriorI64", ScalarType::I64).with_interior_mutability());
    let pointer_ty = types.push(TypeDef::raw_pointer("interior_i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let events = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("before", pointer_ty, false),
            LocalDecl::new("after", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::InteriorAssign {
                dst: target.clone().into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(target.into()),
            },
        ],
    );
    let pointers = formed(&events);

    assert_eq!(pointers.len(), 2);
    assert_eq!(pointers[0], pointers[1]);
}

#[test]
fn move_and_drop_end_values_without_changing_storage_identity() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let target = Place::local(LocalId(0));
    let moved = Place::local(LocalId(1));
    let events = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, true),
            LocalDecl::new("moved", value_ty, false),
            LocalDecl::new("before", pointer_ty, false),
            LocalDecl::new("after_move", pointer_ty, false),
            LocalDecl::new("after_drop", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: target.clone(),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Init {
                dst: moved.clone(),
                src: Operand::Move(target.clone().into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::AddressOf(target.clone().into()),
            },
            Statement::Assign {
                dst: target.clone().into(),
                src: Operand::Constant(Value::I64(2)),
            },
            Statement::Drop {
                place: target.clone().into(),
            },
            Statement::Init {
                dst: Place::local(LocalId(4)),
                src: Operand::AddressOf(target.into()),
            },
        ],
    );
    let pointers = formed(&events);

    assert_eq!(pointers.len(), 3);
    assert_eq!(pointers[0], pointers[1]);
    assert_eq!(pointers[1], pointers[2]);
}

#[test]
fn pointer_copy_and_move_preserve_exact_target_and_provenance() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let first = Place::local(LocalId(1));
    let second = Place::local(LocalId(2));
    let events = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("first", pointer_ty, false),
            LocalDecl::new("second", pointer_ty, false),
            LocalDecl::new("third", pointer_ty, false),
        ],
        Vec::new(),
        vec![
            Statement::Init {
                dst: first.clone(),
                src: Operand::AddressOf(Place::local(LocalId(0)).into()),
            },
            Statement::Init {
                dst: second.clone(),
                src: Operand::Copy(first.into()),
            },
            Statement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::Move(second.into()),
            },
        ],
    );

    let formed = formed(&events);
    let copied = events.iter().find_map(|event| match event {
        VerificationEvent::RawPointerCopy { pointer, .. } => Some(pointer.clone()),
        _ => None,
    });
    let moved = events.iter().find_map(|event| match event {
        VerificationEvent::RawPointerMove { pointer, .. } => Some(pointer.clone()),
        _ => None,
    });

    assert_eq!(formed.len(), 1);
    assert_eq!(copied.as_ref(), Some(&formed[0]));
    assert_eq!(moved.as_ref(), Some(&formed[0]));
}

#[test]
fn ending_source_loan_does_not_mutate_previously_formed_pointer() {
    let mut types = TypeTable::new();
    let value_ty = types.push(TypeDef::scalar("i64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("i64_ptr", value_ty));
    let stored = Place::local(LocalId(1));
    let events = execute(
        types,
        vec![
            LocalDecl::new("target", value_ty, false),
            LocalDecl::new("stored", pointer_ty, false),
            LocalDecl::new("copied", pointer_ty, false),
        ],
        vec![LoanDecl::new("shared", value_ty)],
        vec![
            Statement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(Value::I64(1)),
            },
            Statement::Borrow {
                loan: LoanId(0),
                kind: BorrowKind::Shared,
                src: Place::local(LocalId(0)).into(),
            },
            Statement::Init {
                dst: stored.clone(),
                src: Operand::AddressOf(PlaceAccess::loan(LoanId(0))),
            },
            Statement::EndBorrow { loan: LoanId(0) },
            Statement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Copy(stored.into()),
            },
        ],
    );

    let formed = formed(&events);
    let copied = events.iter().find_map(|event| match event {
        VerificationEvent::RawPointerCopy { pointer, .. } => Some(pointer.clone()),
        _ => None,
    });

    assert_eq!(formed.len(), 1);
    assert_eq!(copied.as_ref(), Some(&formed[0]));
    assert!(
        events
            .iter()
            .any(|event| { matches!(event, VerificationEvent::BorrowEnd(LoanId(0))) })
    );
}
