use runen_core_ir::{
    BasicBlockId, Function as CoreFunction, LocalId, Operand, Place, PlaceAccess, Program,
    Statement as CoreStatement, Terminator, ValidatedProgram, Value as CoreValue,
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

fn function<'a>(program: &'a Program, name: &str) -> &'a CoreFunction {
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
fn reordered_fields_stage_all_producers_before_declaration_indexed_assembly() {
    let lowered = lower_source(
        "record Pair { left: I8, right: U64 } \
         fn make() -> Pair { return Pair { right: 2, left: 1 }; }",
    );
    let make = function(lowered.as_program(), "make");

    assert_eq!(
        make.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        vec!["$tmp0", "$tmp1", "$tmp2"]
    );
    assert_eq!(
        make.body.blocks[0].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(CoreValue::U64(2)),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Constant(CoreValue::I8(1)),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(2)).field(1),
                src: Operand::Move(direct(Place::local(LocalId(0)))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(2)).field(0),
                src: Operand::Move(direct(Place::local(LocalId(1)))),
            },
        ]
    );
    assert_eq!(
        make.body.blocks[0].terminator,
        Terminator::Return(Some(Operand::Move(direct(Place::local(LocalId(2))))))
    );
}

#[test]
fn nonduplicable_field_binding_uses_the_existing_ownership_move_path() {
    let lowered = lower_source(
        "record Token { marker: I8 } \
         record Wrap { item: Token } \
         fn wrap(token: Token) -> Wrap { return Wrap { item: token }; }",
    );
    let wrap = function(lowered.as_program(), "wrap");

    assert_eq!(
        wrap.body.blocks[0].statements[0],
        CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(Place::local(LocalId(0)))),
        }
    );
    assert_eq!(
        wrap.body.blocks[0].statements[1],
        CoreStatement::Init {
            dst: Place::local(LocalId(2)).field(0),
            src: Operand::Move(direct(Place::local(LocalId(1)))),
        }
    );
}

#[test]
fn nested_construction_completes_before_the_outer_consumer_transfer() {
    let lowered = lower_source(
        "record Inner { value: I8 } \
         record Outer { inner: Inner } \
         fn build() { let result: Outer = Outer { inner: Inner { value: 1 } }; }",
    );
    let build = function(lowered.as_program(), "build");

    assert_eq!(
        build
            .body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        vec!["result", "$tmp0", "$tmp1", "$tmp2"]
    );
    assert_eq!(
        build.body.blocks[0].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Constant(CoreValue::I8(1)),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(2)).field(0),
                src: Operand::Move(direct(Place::local(LocalId(1)))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(3)).field(0),
                src: Operand::Move(direct(Place::local(LocalId(2)))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Move(direct(Place::local(LocalId(3)))),
            },
        ]
    );
}

#[test]
fn later_call_reaches_assembly_only_in_its_success_continuation() {
    let lowered = lower_source(
        "record Triple { a: I8, b: I8, c: I8 } \
         fn produce() -> I8 { return 3; } \
         fn build() -> Triple { return Triple { a: 1, b: 2, c: produce() }; }",
    );
    let program = lowered.as_program();
    let build = function(program, "build");

    assert_eq!(
        build
            .body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        vec!["$tmp0", "$tmp1", "$tmp2", "$tmp3"]
    );
    assert_eq!(
        build.body.blocks[0].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(0)),
                src: Operand::Constant(CoreValue::I8(1)),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Constant(CoreValue::I8(2)),
            },
        ]
    );

    let Terminator::Call {
        function: target,
        arguments,
        destination: Some(destination),
        target: BasicBlockId(1),
    } = &build.body.blocks[0].terminator
    else {
        panic!("later field call must terminate the producer block");
    };
    assert!(arguments.is_empty());
    assert_eq!(*destination, Place::local(LocalId(2)));
    assert_eq!(program.function(*target).unwrap().name, "produce");

    assert_eq!(
        build.body.blocks[1].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(3)).field(0),
                src: Operand::Move(direct(Place::local(LocalId(0)))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(3)).field(1),
                src: Operand::Move(direct(Place::local(LocalId(1)))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(3)).field(2),
                src: Operand::Move(direct(Place::local(LocalId(2)))),
            },
        ]
    );
    assert_eq!(
        build.body.blocks[1].terminator,
        Terminator::Return(Some(Operand::Move(direct(Place::local(LocalId(3))))))
    );

    // If the call faults, its success continuation is never entered. The two earlier
    // producer temporaries are therefore the only live construction values: the call
    // destination and aggregate result remain never-initialized. Their declaration
    // order also leaves those earlier live field values eligible for reverse source-order
    // Core fault cleanup under the existing function-termination relation.
}

#[test]
fn empty_record_uses_the_existing_empty_struct_constant_and_validates() {
    let lowered = lower_source("record Empty {} fn make() -> Empty { return Empty {}; }");
    let make = function(lowered.as_program(), "make");

    assert_eq!(
        make.body.blocks[0].statements,
        vec![CoreStatement::Init {
            dst: Place::local(LocalId(0)),
            src: Operand::Constant(CoreValue::Struct(Vec::new())),
        }]
    );
    assert_eq!(
        make.body.blocks[0].terminator,
        Terminator::Return(Some(Operand::Move(direct(Place::local(LocalId(0))))))
    );
}
