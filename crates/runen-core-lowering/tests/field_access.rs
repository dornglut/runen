use runen_core_ir::{
    Function as CoreFunction, LocalId, Operand, Place, PlaceAccess, Projection,
    Statement as CoreStatement, Terminator, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{ModuleId, OwnedUse, SourceUnit, ValueKind, build_typed_hir};
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
