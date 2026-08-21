use runen_core_ir::{
    Function as CoreFunction, LocalId, Operand, Place, PlaceAccess, Projection, Statement as CoreStatement,
    Terminator, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{ModuleId, SourceUnit, ValueKind, build_typed_hir};
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
fn one_level_field_value_lowers_to_projected_copy() {
    let lowered = lower_source(
        "record Box { value: I8 } fn read(root: Box) -> I8 { return root.value; }",
    );
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
fn nested_field_path_preserves_projection_order() {
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
        panic!("field access must lower through direct projected Copy");
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
fn field_value_composes_with_call_transfer() {
    let lowered = lower_source(
        "record Box { value: I8 } \
         fn sink(value: I8) {} \
         fn f(root: Box) { sink(root.value); }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body.blocks[0].statements,
        vec![CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Copy(direct(Place::local(LocalId(0)).field(0))),
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
fn lowering_rejects_empty_resolved_field_path_as_invalid_hir() {
    let mut compilation = hir(
        "record Box { value: I8 } fn read(root: Box) -> I8 { return root.value; }",
    );
    let value = compilation.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("returned field value");
    let ValueKind::FieldValueUse { fields, .. } = &mut value.kind else {
        panic!("expected field-value HIR");
    };
    fields.clear();

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "field-value use has empty field path"
        ))
    );
}
