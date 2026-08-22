use runen_core_ir::{
    Function as CoreFunction, LocalId, Operand, Place, PlaceAccess, Projection,
    Statement as CoreStatement, Terminator, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    FieldValueReceiver, IntrinsicType, LiteralValue, ModuleId, OwnedUse, SourceUnit, Type,
    ValueKind, build_typed_hir,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn hir(source: &str) -> runen_hir::TypedCompilation {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR")
}

fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted HIR must lower through canonical Core validation")
}

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a CoreFunction {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn direct(place: Place) -> PlaceAccess {
    PlaceAccess::Direct(place)
}

#[test]
fn duplicable_field_value_lowers_to_projected_copy() {
    let lowered =
        lower_source("record Box { value: I8 } fn read(root: Box) -> I8 { return root.value; }");
    let read = function(lowered.as_program(), "read");

    assert_eq!(
        read.body.blocks[0].statements,
        vec![CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Copy(direct(Place::local(LocalId(0)).field(0))),
        }]
    );
    assert_eq!(
        read.body.blocks[0].terminator,
        Terminator::Return(Some(Operand::Move(direct(Place::local(LocalId(1))))))
    );
}

#[test]
fn nested_duplicable_field_preserves_projection_order() {
    let lowered = lower_source(
        "record Inner { pad: U8, value: I8 } \
         record Outer { first: I8, inner: Inner } \
         fn read(root: Outer) -> I8 { return root.inner.value; }",
    );
    let read = function(lowered.as_program(), "read");

    let CoreStatement::Init {
        src: Operand::Copy(PlaceAccess::Direct(place)),
        ..
    } = &read.body.blocks[0].statements[0]
    else {
        panic!("duplicable field access must lower through projected Copy");
    };
    assert_eq!(place.local, LocalId(0));
    assert_eq!(
        place.projections,
        vec![Projection::Field(1), Projection::Field(1)]
    );
}

#[test]
fn nonduplicable_field_value_lowers_to_projected_move() {
    let lowered = lower_source(
        "record Inner { value: I8 } record Outer { inner: Inner } \
         fn take(root: Outer) -> Inner { return root.inner; }",
    );
    let take = function(lowered.as_program(), "take");

    assert_eq!(
        take.body.blocks[0].statements,
        vec![CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(Place::local(LocalId(0)).field(0))),
        }]
    );
}

#[test]
fn nested_nonduplicable_field_lowers_to_exact_projected_move() {
    let lowered = lower_source(
        "record Leaf { value: I8 } \
         record Inner { pad: I8, leaf: Leaf } \
         record Outer { first: I8, inner: Inner } \
         fn take(root: Outer) -> Leaf { return root.inner.leaf; }",
    );
    let take = function(lowered.as_program(), "take");

    let CoreStatement::Init {
        src: Operand::Move(PlaceAccess::Direct(place)),
        ..
    } = &take.body.blocks[0].statements[0]
    else {
        panic!("non-duplicable field access must lower through projected Move");
    };
    assert_eq!(place.local, LocalId(0));
    assert_eq!(
        place.projections,
        vec![Projection::Field(1), Projection::Field(1)]
    );
}

#[test]
fn field_copy_does_not_move_root_before_later_whole_binding_consumption() {
    let lowered = lower_source(
        "record Box { value: I8 } \
         fn f(root: Box) -> Box { let value: I8 = root.value; return root; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body.blocks[0].statements[0],
        CoreStatement::Init {
            dst: Place::local(LocalId(2)),
            src: Operand::Copy(direct(Place::local(LocalId(0)).field(0))),
        }
    );
    assert_eq!(
        f.body.blocks[0].statements[1],
        CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(Place::local(LocalId(2)))),
        }
    );
    assert_eq!(
        f.body.blocks[0].statements[2],
        CoreStatement::Init {
            dst: Place::local(LocalId(3)),
            src: Operand::Move(direct(Place::local(LocalId(0)))),
        }
    );
}

#[test]
fn producer_call_duplicate_field_work_is_only_in_normal_continuation_and_precedes_cleanup() {
    let lowered = lower_source(
        "record Box { value: I8, tail: I8 } \
         fn make() -> Box { return Box { value: 1, tail: 2 }; } \
         fn f() -> I8 { return make().value; }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(f.body.blocks[0].statements.is_empty());
    let Terminator::Call {
        destination: Some(destination),
        target,
        ..
    } = &f.body.blocks[0].terminator
    else {
        panic!("producer receiver must begin with direct call");
    };
    assert_eq!(*destination, Place::local(LocalId(0)));
    assert_eq!(*target, runen_core_ir::BasicBlockId(1));

    assert_eq!(
        f.body.blocks[1].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(0))),
            },
            CoreStatement::Drop {
                place: direct(Place::local(LocalId(0))),
            },
        ]
    );
    assert_eq!(
        f.body.blocks[1].terminator,
        Terminator::Return(Some(Operand::Move(direct(Place::local(LocalId(1))))))
    );
}

#[test]
fn producer_call_consuming_field_moves_result_then_cleans_only_remaining_frontier() {
    let lowered = lower_source(
        "record Token { value: I8 } record Box { token: Token, tail: I8 } \
         fn make() -> Box { return Box { token: Token { value: 1 }, tail: 2 }; } \
         fn f() -> Token { return make().token; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body.blocks[1].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Move(direct(Place::local(LocalId(0)).field(0))),
            },
            CoreStatement::Drop {
                place: direct(Place::local(LocalId(0)).field(1)),
            },
        ]
    );
    assert!(
        !f.body.blocks[1].statements.iter().any(|statement| matches!(
            statement,
            CoreStatement::Drop {
                place: PlaceAccess::Direct(place),
            } if place.projections == vec![Projection::Field(0)]
        ))
    );
}

#[test]
fn producer_zero_leaf_cleanup_is_retained_in_hir_but_erases_to_no_core_drop() {
    let compilation = hir(
        "record Token { value: I8 } record Empty {} record Box { token: Token, empty: Empty } \
         fn make() -> Box { return Box { token: Token { value: 1 }, empty: Empty {} }; } \
         fn f() -> Token { return make().token; }",
    );
    let f_hir = compilation
        .functions
        .iter()
        .find(|function| function.name == "f")
        .expect("HIR f");
    let returned = f_hir
        .body
        .terminal_return
        .as_ref()
        .and_then(|returned| returned.value.as_ref())
        .expect("returned value");
    let ValueKind::FieldValueUse { receiver, .. } = &returned.kind else {
        panic!("expected field-value use");
    };
    let FieldValueReceiver::Producer { cleanup, .. } = receiver else {
        panic!("expected producer receiver");
    };
    assert_eq!(cleanup.paths, vec![vec![1]]);

    let lowered = lower(&compilation).expect("zero-leaf cleanup must lower");
    let f = function(lowered.as_program(), "f");
    assert_eq!(f.body.blocks[1].statements.len(), 1);
    assert!(matches!(
        f.body.blocks[1].statements[0],
        CoreStatement::Init {
            src: Operand::Move(PlaceAccess::Direct(ref place)),
            ..
        } if place.projections == vec![Projection::Field(0)]
    ));
}

#[test]
fn construction_receiver_finishes_assembly_before_field_result_and_cleanup() {
    let lowered = lower_source(
        "record Box { value: I8, tail: I8 } \
         fn f() -> I8 { return Box { value: 1, tail: 2 }.value; }",
    );
    let f = function(lowered.as_program(), "f");
    let statements = &f.body.blocks[0].statements;

    assert_eq!(statements.len(), 6);
    assert!(matches!(
        statements[2],
        CoreStatement::Init {
            dst: Place { local: LocalId(2), ref projections },
            ..
        } if projections == &vec![Projection::Field(0)]
    ));
    assert!(matches!(
        statements[3],
        CoreStatement::Init {
            dst: Place { local: LocalId(2), ref projections },
            ..
        } if projections == &vec![Projection::Field(1)]
    ));
    assert_eq!(
        statements[4],
        CoreStatement::Init {
            dst: Place::local(LocalId(3)),
            src: Operand::Copy(direct(Place::local(LocalId(2)).field(0))),
        }
    );
    assert_eq!(
        statements[5],
        CoreStatement::Drop {
            place: direct(Place::local(LocalId(2))),
        }
    );
}

#[test]
fn producer_field_result_cleanup_precedes_assignment_and_call_transfer() {
    let lowered = lower_source(
        "record Box { value: I8 } \
         fn make() -> Box { return Box { value: 1 }; } fn sink(value: I8) {} \
         fn assign() { let mut target: I8 = 0; target = make().value; } \
         fn call() { sink(make().value); }",
    );

    let assign = function(lowered.as_program(), "assign");
    let assign_continuation = &assign.body.blocks[1];
    assert!(matches!(
        assign_continuation.statements.as_slice(),
        [
            CoreStatement::Init { .. },
            CoreStatement::Drop { .. },
            CoreStatement::Assign { .. }
        ]
    ));

    let call = function(lowered.as_program(), "call");
    let producer_continuation = &call.body.blocks[1];
    assert!(matches!(
        producer_continuation.statements.as_slice(),
        [CoreStatement::Init { .. }, CoreStatement::Drop { .. }]
    ));
    let Terminator::Call { arguments, .. } = &producer_continuation.terminator else {
        panic!("selected result must transfer to enclosing call");
    };
    assert_eq!(arguments.len(), 1);
}

#[test]
fn producer_field_composes_with_enclosing_construction_and_return() {
    lower_source(
        "record Box { value: I8 } record Holder { value: I8 } \
         fn make() -> Box { return Box { value: 1 }; } \
         fn f() -> Holder { return Holder { value: make().value }; }",
    );
}

#[test]
fn producer_bool_field_cleanup_completes_before_existing_branch() {
    let lowered = lower_source(
        "record Flag { ready: Bool } \
         fn make() -> Flag { return Flag { ready: true }; } \
         fn f() { if make().ready {} }",
    );
    let f = function(lowered.as_program(), "f");
    let continuation = &f.body.blocks[1];
    assert!(matches!(
        continuation.statements.as_slice(),
        [CoreStatement::Init { .. }, CoreStatement::Drop { .. }]
    ));
    let Terminator::Branch { condition, .. } = &continuation.terminator else {
        panic!("existing conditional lowering must branch on selected field result");
    };
    assert_eq!(condition, &Operand::Move(direct(Place::local(LocalId(1)))));
}

#[test]
fn producer_record_field_cleanup_precedes_pattern_leaf_production_and_pattern_cleanup() {
    let lowered = lower_source(
        "record Token { value: I8 } record Inner { token: Token, count: I8 } \
         record Outer { inner: Inner, pad: I8 } \
         fn make() -> Outer { return Outer { inner: Inner { token: Token { value: 1 }, count: 2 }, pad: 3 }; } \
         fn f() { let Inner { token: moved, count: copied } = make().inner; }",
    );
    let f = function(lowered.as_program(), "f");
    let continuation = &f.body.blocks[1];
    assert_eq!(
        continuation.statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::Move(direct(Place::local(LocalId(2)).field(0))),
            },
            CoreStatement::Drop {
                place: direct(Place::local(LocalId(2)).field(1)),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Move(direct(Place::local(LocalId(3)).field(0))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(direct(Place::local(LocalId(3)).field(1))),
            },
            CoreStatement::Drop {
                place: direct(Place::local(LocalId(3)).field(1)),
            },
        ]
    );
}

#[test]
fn disjoint_consuming_and_duplicating_field_uses_validate_together() {
    lower_source(
        "record Left { value: I8 } record Right { value: I8 } \
         record Pair { left: Left, right: Right, count: I8 } \
         fn take_left(value: Left) {} fn take_right(value: Right) {} \
         fn f(root: Pair) -> I8 { \
             take_left(root.left); \
             take_right(root.right); \
             return root.count; \
         }",
    );
}

#[test]
fn consuming_field_composes_with_call_transfer() {
    let lowered = lower_source(
        "record Token { value: I8 } record Holder { token: Token } \
         fn sink(value: Token) {} \
         fn f(root: Holder) { sink(root.token); }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body.blocks[0].statements,
        vec![CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(Place::local(LocalId(0)).field(0))),
        }]
    );
    let Terminator::Call { arguments, .. } = &f.body.blocks[0].terminator else {
        panic!("expected direct call");
    };
    assert_eq!(
        arguments,
        &[Operand::Move(direct(Place::local(LocalId(1))))]
    );
}

#[test]
fn assignment_rhs_consumes_target_field_before_existing_whole_assign() {
    let lowered = lower_source(
        "record Token { value: I8 } record Holder { token: Token, count: I8 } \
         fn f() -> Holder { \
             let mut holder: Holder = Holder { token: Token { value: 1 }, count: 2 }; \
             holder = Holder { token: holder.token, count: 3 }; \
             return holder; \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(
        f.body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|statement| matches!(
                statement,
                CoreStatement::Init {
                    src: Operand::Move(PlaceAccess::Direct(place)),
                    ..
                } if place.local == LocalId(0) && place.projections == vec![Projection::Field(0)]
            ))
    );
    assert!(
        f.body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|statement| matches!(
                statement,
                CoreStatement::Assign {
                    dst: PlaceAccess::Direct(place),
                    ..
                } if place.local == LocalId(0) && place.projections.is_empty()
            ))
    );
}

#[test]
fn consuming_field_composes_with_record_construction_and_return_cleanup() {
    lower_source(
        "record Token { value: I8 } record Pair { left: Token, right: Token } \
         record ResultBox { token: Token } \
         fn f(root: Pair) -> ResultBox { \
             return ResultBox { token: root.left }; \
         }",
    );
}

#[test]
fn lowering_rejects_empty_resolved_field_path_as_invalid_hir() {
    let mut compilation =
        hir("record Box { value: I8 } fn read(root: Box) -> I8 { return root.value; }");
    let value = compilation.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("returned field value");
    let ValueKind::FieldValueUse {
        fields, ownership, ..
    } = &mut value.kind
    else {
        panic!("expected field-value HIR");
    };
    assert_eq!(*ownership, OwnedUse::Duplicate);
    fields.clear();

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "field-value use has empty field path"
        ))
    );
}

#[test]
fn lowering_rejects_corrupted_producer_field_type_category_ownership_and_cleanup_invariants() {
    let mut wrong_ownership = hir("record Box { value: I8 } \
         fn make() -> Box { return Box { value: 1 }; } \
         fn f() -> I8 { return make().value; }");
    let wrong_ownership_value = wrong_ownership.functions[1]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("returned field value");
    let ValueKind::FieldValueUse { ownership, .. } = &mut wrong_ownership_value.kind else {
        panic!("expected field-value use");
    };
    *ownership = OwnedUse::Consume;
    assert_eq!(
        lower(&wrong_ownership),
        Err(LoweringError::InvalidHirInvariant(
            "field-value ownership does not match retained result duplicability"
        ))
    );

    let mut wrong_category = hir("record Box { value: I8 } \
         fn make() -> Box { return Box { value: 1 }; } \
         fn f() -> I8 { return make().value; }");
    let wrong_category_value = wrong_category.functions[1]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("returned field value");
    let ValueKind::FieldValueUse { receiver, .. } = &mut wrong_category_value.kind else {
        panic!("expected producer field value");
    };
    let FieldValueReceiver::Producer {
        value: producer, ..
    } = receiver
    else {
        panic!("expected producer receiver");
    };
    producer.kind = ValueKind::Literal(LiteralValue::I8(1));
    assert_eq!(
        lower(&wrong_category),
        Err(LoweringError::InvalidHirInvariant(
            "field-value producer receiver has unrepresented producer category"
        ))
    );

    let mut wrong_type = hir(
        "record Token { value: I8 } record Box { token: Token, tail: I8 } \
         fn make() -> Box { return Box { token: Token { value: 1 }, tail: 2 }; } \
         fn f() -> Token { return make().token; }",
    );
    let wrong_type_value = wrong_type.functions[1]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("returned field value");
    let ValueKind::FieldValueUse { ownership, .. } = &mut wrong_type_value.kind else {
        panic!("expected field-value use");
    };
    wrong_type_value.ty = Type::Intrinsic(IntrinsicType::I8);
    *ownership = OwnedUse::Duplicate;
    assert_eq!(
        lower(&wrong_type),
        Err(LoweringError::InvalidHirInvariant(
            "field-value retained result type does not match projected receiver field type"
        ))
    );

    let mut overlap = hir(
        "record Token { value: I8 } record Box { token: Token, tail: I8 } \
         fn make() -> Box { return Box { token: Token { value: 1 }, tail: 2 }; } \
         fn f() -> Token { return make().token; }",
    );
    let overlap_value = overlap.functions[1]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("returned field value");
    let ValueKind::FieldValueUse { receiver, .. } = &mut overlap_value.kind else {
        panic!("expected producer field value");
    };
    let FieldValueReceiver::Producer { cleanup, .. } = receiver else {
        panic!("expected producer receiver");
    };
    cleanup.paths = vec![vec![0]];
    assert_eq!(
        lower(&overlap),
        Err(LoweringError::InvalidHirInvariant(
            "consumed field receiver path overlaps retained cleanup frontier"
        ))
    );

    let mut incomplete_consume = hir(
        "record Token { value: I8 } record Box { token: Token, tail: I8 } \
         fn make() -> Box { return Box { token: Token { value: 1 }, tail: 2 }; } \
         fn f() -> Token { return make().token; }",
    );
    let consume_value = incomplete_consume.functions[1]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("returned field value");
    let ValueKind::FieldValueUse { receiver, .. } = &mut consume_value.kind else {
        panic!("expected producer field value");
    };
    let FieldValueReceiver::Producer { cleanup, .. } = receiver else {
        panic!("expected producer receiver");
    };
    cleanup.paths.clear();
    assert_eq!(
        lower(&incomplete_consume),
        Err(LoweringError::InvalidHirInvariant(
            "consuming field receiver cleanup does not match canonical remaining frontier"
        ))
    );

    let mut incomplete_duplicate = hir("record Box { value: I8, tail: I8 } \
         fn make() -> Box { return Box { value: 1, tail: 2 }; } \
         fn f() -> I8 { return make().value; }");
    let duplicate_value = incomplete_duplicate.functions[1]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("returned field value");
    let ValueKind::FieldValueUse { receiver, .. } = &mut duplicate_value.kind else {
        panic!("expected producer field value");
    };
    let FieldValueReceiver::Producer { cleanup, .. } = receiver else {
        panic!("expected producer receiver");
    };
    cleanup.paths = vec![vec![1]];
    assert_eq!(
        lower(&incomplete_duplicate),
        Err(LoweringError::InvalidHirInvariant(
            "duplicating field receiver does not retain complete receiver cleanup"
        ))
    );
}
