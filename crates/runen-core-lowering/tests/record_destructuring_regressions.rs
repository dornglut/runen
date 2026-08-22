use runen_core_ir::{
    Function as CoreFunction, LocalId, Operand, Place, PlaceAccess, Statement as CoreStatement,
    Terminator, ValidatedProgram,
};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR");
    lower(&hir).expect("accepted HIR must lower through canonical Core validation")
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

fn drops(function: &CoreFunction) -> Vec<PlaceAccess> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| match statement {
            CoreStatement::Drop { place } => Some(place.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn one_level_record_construction_producer_keeps_source_locals_before_temporaries() {
    let lowered = lower_source(
        "record Pair { left: I8, right: U8 } \
         fn f() { let Pair { right: r, left: l } = Pair { left: 1, right: 2 }; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["r", "l", "$tmp0", "$tmp1", "$tmp2"]
    );
    assert_eq!(drops(f), vec![direct(Place::local(LocalId(4)))]);
    assert!(f.body.blocks[0].statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            dst,
            src: Operand::Copy(PlaceAccess::Direct(source)),
        } if *dst == Place::local(LocalId(0)) && *source == Place::local(LocalId(4)).field(1)
    )));
    assert!(f.body.blocks[0].statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            dst,
            src: Operand::Copy(PlaceAccess::Direct(source)),
        } if *dst == Place::local(LocalId(1)) && *source == Place::local(LocalId(4)).field(0)
    )));
}

#[test]
fn one_level_direct_call_producer_reuses_exactly_its_result_temporary() {
    let lowered = lower_source(
        "record Pair { left: I8, right: U8 } \
         fn make() -> Pair { return Pair { left: 1, right: 2 }; } \
         fn f() { let Pair { left: l, right: r } = make(); }",
    );
    let f = function(lowered.as_program(), "f");

    let Terminator::Call {
        destination: Some(destination),
        target,
        ..
    } = &f.body.blocks[0].terminator
    else {
        panic!("expected producer call terminator");
    };
    assert!(destination.projections.is_empty());
    assert_eq!(
        f.body
            .locals
            .iter()
            .filter(|local| local.name.starts_with("$tmp"))
            .count(),
        1
    );

    let continuation = &f.body.blocks[target.0 as usize];
    let pattern_sources = continuation
        .statements
        .iter()
        .filter_map(|statement| match statement {
            CoreStatement::Init {
                src: Operand::Copy(PlaceAccess::Direct(place)),
                ..
            } if place.projections.len() == 1 => Some(place.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(pattern_sources.len(), 2);
    assert!(
        pattern_sources
            .iter()
            .all(|source| source.local == destination.local)
    );
    assert_eq!(drops(f), vec![direct(Place::local(destination.local))]);
}

#[test]
fn one_level_field_value_producer_moves_source_field_into_one_pattern_temporary() {
    let lowered = lower_source(
        "record Pair { left: I8, right: U8 } record Outer { pair: Pair } \
         fn f(root: Outer) { let Pair { left: l, right: r } = root.pair; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "l", "r", "$tmp0"]
    );
    assert_eq!(
        f.body.blocks[0].statements[0],
        CoreStatement::Init {
            dst: Place::local(LocalId(3)),
            src: Operand::Move(direct(Place::local(LocalId(0)).field(0))),
        }
    );
    assert!(f.body.blocks[0].statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::Copy(PlaceAccess::Direct(place)),
            ..
        } if place.local == LocalId(3) && place.projections == vec![runen_core_ir::Projection::Field(0)]
    )));
    assert_eq!(drops(f), vec![direct(Place::local(LocalId(3)))]);
}
