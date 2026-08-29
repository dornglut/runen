use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Field, Function, FunctionId, LocalDecl, LocalId, Operand,
    Place, Program, ReferenceAccess, ReferencePermission, ScalarType, Statement, Terminator,
    TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus, UndefinedBehaviorKind};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

#[test]
fn shared_reference_copy_keeps_authority_alive_after_original_carrier_drops() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let original = Place::local(LocalId(1));
    let copy = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("original", shared_i64, false),
                LocalDecl::new("copy", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(3)),
                    },
                    Statement::Init {
                        dst: original.clone(),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: target.clone(),
                        },
                    },
                    Statement::Init {
                        dst: copy.clone(),
                        src: Operand::Copy(original.clone().into()),
                    },
                    Statement::Drop {
                        place: original.into(),
                    },
                    Statement::ReferenceRead {
                        src: ReferenceAccess::new(copy.clone()),
                    },
                    Statement::Drop { place: copy.into() },
                ],
                Terminator::Return(Some(Operand::Move(target.into()))),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("Shared Copy carrier lifecycle is valid Core");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("safe reference execution is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(3)));
}

#[test]
fn aggregate_shared_reference_copy_recursively_tracks_carriers() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let holder_ty = types.push(TypeDef::structure(
        "Holder",
        vec![
            Field::new("reference", shared_i64),
            Field::new("value", i64_ty),
        ],
    ));
    let target = Place::local(LocalId(0));
    let original = Place::local(LocalId(1));
    let copy = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("original", holder_ty, false),
                LocalDecl::new("copy", holder_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(13)),
                    },
                    Statement::Init {
                        dst: original.clone().field(0),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: target.clone(),
                        },
                    },
                    Statement::Init {
                        dst: original.clone().field(1),
                        src: Operand::Constant(Value::I64(1)),
                    },
                    Statement::Init {
                        dst: copy.clone(),
                        src: Operand::Copy(original.clone().into()),
                    },
                    Statement::Drop {
                        place: original.into(),
                    },
                    Statement::ReferenceRead {
                        src: ReferenceAccess::new(copy.clone().field(0)),
                    },
                    Statement::Drop { place: copy.into() },
                ],
                Terminator::Return(Some(Operand::Move(target.into()))),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("copyable aggregates recursively duplicate Shared carriers");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("aggregate reference carrier execution is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(13)));
}

#[test]
fn reference_assign_evaluates_reference_copy_source_before_old_carrier_destruction() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let replace_shared = types.push(TypeDef::reference(
        "ReplaceSharedI64",
        shared_i64,
        ReferencePermission::ExclusiveReplace,
    ));
    let target = Place::local(LocalId(0));
    let slot = Place::local(LocalId(1));
    let outer = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("slot", shared_i64, true),
                LocalDecl::new("outer", replace_shared, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(17)),
                    },
                    Statement::Init {
                        dst: slot.clone(),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: target.clone(),
                        },
                    },
                    Statement::Init {
                        dst: outer.clone(),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::ExclusiveReplace,
                            place: slot.clone(),
                        },
                    },
                    Statement::ReferenceAssign {
                        dst: ReferenceAccess::new(outer.clone()),
                        src: Operand::ReferenceCopy(ReferenceAccess::new(outer.clone())),
                    },
                    Statement::Drop {
                        place: outer.into(),
                    },
                    Statement::ReferenceRead {
                        src: ReferenceAccess::new(slot.clone()),
                    },
                    Statement::Drop { place: slot.into() },
                ],
                Terminator::Return(Some(Operand::Move(target.into()))),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("ReferenceAssign source copy is evaluated while the old destination carrier is live");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("source-first reference replacement is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(17)));
}

#[test]
fn ending_shared_reborrow_restores_exclusive_replace_parent() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let exclusive_replace = types.push(TypeDef::reference(
        "ExclusiveReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let shared = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let parent = Place::local(LocalId(1));
    let child = Place::local(LocalId(2));
    let held = Place::local(LocalId(3));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, true),
                LocalDecl::new("parent", exclusive_replace, false),
                LocalDecl::new("child", shared, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(9)),
                    },
                    Statement::Init {
                        dst: parent.clone(),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::ExclusiveReplace,
                            place: target.clone(),
                        },
                    },
                    Statement::Init {
                        dst: child.clone(),
                        src: Operand::ReferenceReborrow {
                            permission: ReferencePermission::Shared,
                            src: ReferenceAccess::new(parent.clone()),
                        },
                    },
                    Statement::ReferenceRead {
                        src: ReferenceAccess::new(child.clone()),
                    },
                    Statement::Drop {
                        place: child.into(),
                    },
                    Statement::Init {
                        dst: held.clone(),
                        src: Operand::ReferenceMove(ReferenceAccess::new(parent.clone())),
                    },
                    Statement::ReferenceAssign {
                        dst: ReferenceAccess::new(parent.clone()),
                        src: Operand::Move(held.into()),
                    },
                    Statement::Drop {
                        place: parent.into(),
                    },
                ],
                Terminator::Return(Some(Operand::Move(target.into()))),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("parent authority is restored after its child carrier ends");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("safe reborrow execution is defined");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(9)));
}

#[test]
fn raw_read_coexists_with_overlapping_shared_reference_authority() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let shared_ty = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let reference = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", pointer_ty, false),
                LocalDecl::new("reference", shared_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(19)),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(target.clone().into()),
                    },
                    Statement::Init {
                        dst: reference.clone(),
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: target.clone(),
                        },
                    },
                    Statement::RawRead {
                        pointer: pointer.into(),
                    },
                    Statement::Drop {
                        place: reference.into(),
                    },
                ],
                Terminator::Return(Some(Operand::Move(target.into()))),
            )],
        ),
    };

    let validated = validate_program(Program {
        types,
        functions: vec![function],
    })
    .expect("RawRead shared target compatibility permits overlapping Shared reference authority");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("RawRead is defined with overlapping Shared reference authority");

    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(19)));
}

#[test]
fn raw_read_reports_overlapping_exclusive_reference_authority() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let exclusive_ty = types.push(TypeDef::reference(
        "ExclusiveI64",
        i64_ty,
        ReferencePermission::Exclusive,
    ));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let reference = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", pointer_ty, false),
                LocalDecl::new("reference", exclusive_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(5)),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(target.clone().into()),
                    },
                    Statement::Init {
                        dst: reference,
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Exclusive,
                            place: target,
                        },
                    },
                    Statement::RawRead {
                        pointer: pointer.into(),
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
    .expect("raw UB is a defined validator no-continuation path");
    let ub = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect_err("RawRead conflicts with an overlapping Exclusive safe reference");

    assert!(matches!(
        ub.kind,
        UndefinedBehaviorKind::RawReadConflictsWithReferenceAuthority { .. }
    ));
}

#[test]
fn raw_move_reports_overlapping_shared_reference_authority() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let shared_ty = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let reference = Place::local(LocalId(2));
    let held = Place::local(LocalId(3));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", pointer_ty, false),
                LocalDecl::new("reference", shared_ty, false),
                LocalDecl::new("held", i64_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(23)),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(target.clone().into()),
                    },
                    Statement::Init {
                        dst: reference,
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: target,
                        },
                    },
                    Statement::Init {
                        dst: held,
                        src: Operand::RawMove(pointer.into()),
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
    .expect("raw UB remains a defined validator no-continuation path");
    let ub = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect_err("RawMove conflicts with any overlapping safe-reference authority");

    assert!(matches!(
        ub.kind,
        UndefinedBehaviorKind::RawMoveConflictsWithReferenceAuthority { .. }
    ));
}

#[test]
fn raw_assign_reports_overlapping_shared_reference_authority() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let pointer_ty = types.push(TypeDef::raw_pointer("RawI64", i64_ty));
    let shared_ty = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let target = Place::local(LocalId(0));
    let pointer = Place::local(LocalId(1));
    let reference = Place::local(LocalId(2));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        body: body(
            vec![
                LocalDecl::new("target", i64_ty, false),
                LocalDecl::new("pointer", pointer_ty, false),
                LocalDecl::new("reference", shared_ty, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: target.clone(),
                        src: Operand::Constant(Value::I64(29)),
                    },
                    Statement::Init {
                        dst: pointer.clone(),
                        src: Operand::AddressOf(target.clone().into()),
                    },
                    Statement::Init {
                        dst: reference,
                        src: Operand::ReferenceRoot {
                            permission: ReferencePermission::Shared,
                            place: target,
                        },
                    },
                    Statement::RawAssign {
                        pointer: pointer.into(),
                        src: Operand::Constant(Value::I64(31)),
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
    .expect("raw UB remains a defined validator no-continuation path");
    let ub = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect_err("RawAssign conflicts with any overlapping safe-reference authority");

    assert!(matches!(
        ub.kind,
        UndefinedBehaviorKind::RawAssignConflictsWithReferenceAuthority { .. }
    ));
}
