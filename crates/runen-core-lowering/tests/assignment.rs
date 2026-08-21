use runen_core_ir::{
    BasicBlockId, LocalId, Operand, Place, PlaceAccess, Statement as CoreStatement, Terminator,
    ValidatedProgram,
};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR");
    lower(&hir).expect("accepted assignment HIR must lower to valid Core")
}

fn direct(place: Place) -> PlaceAccess {
    PlaceAccess::Direct(place)
}

#[test]
fn maps_only_mutable_source_locals_to_mutable_core_locals() {
    let lowered = lower_source(
        "fn f(p: I64) { let immutable: I64 = p; let mut mutable: I64 = p; mutable = p; }",
    );
    let function = &lowered.as_program().functions[0];

    let p = function.body.locals.iter().find(|local| local.name == "p").unwrap();
    let immutable = function
        .body
        .locals
        .iter()
        .find(|local| local.name == "immutable")
        .unwrap();
    let mutable = function
        .body
        .locals
        .iter()
        .find(|local| local.name == "mutable")
        .unwrap();
    assert!(!p.mutable, "parameters remain immutable");
    assert!(!immutable.mutable, "plain let remains immutable");
    assert!(mutable.mutable, "let mut must reach Core mutability");
    assert!(
        function
            .body
            .locals
            .iter()
            .filter(|local| local.name.starts_with("$tmp"))
            .all(|local| !local.mutable),
        "compiler temporaries remain immutable"
    );
}

#[test]
fn materializes_rhs_before_emitting_core_assign() {
    let lowered = lower_source("fn f(p: I64) { let mut x: I64 = p; x = p; }");
    let function = &lowered.as_program().functions[0];
    let x = function
        .body
        .locals
        .iter()
        .position(|local| local.name == "x")
        .map(|index| LocalId(index as u32))
        .unwrap();
    let statements = &function.body.blocks[0].statements;

    let assignment_index = statements
        .iter()
        .position(|statement| matches!(statement, CoreStatement::Assign { dst, .. } if *dst == direct(Place::local(x))))
        .expect("assignment must lower to Core Assign");
    assert!(assignment_index > 0);

    let CoreStatement::Assign {
        src: Operand::Move(PlaceAccess::Direct(source)),
        ..
    } = &statements[assignment_index]
    else {
        panic!("assignment source must move from a materialized temporary");
    };
    let source_local = source.local;
    assert!(
        function.body.locals[source_local.0 as usize]
            .name
            .starts_with("$tmp")
    );
    assert!(
        statements[..assignment_index].iter().any(|statement| {
            matches!(
                statement,
                CoreStatement::Init { dst, .. } if dst.local == source_local
            )
        }),
        "the RHS temporary must be initialized before assignment"
    );
}

#[test]
fn nonduplicable_self_assignment_moves_to_temporary_then_assigns_back() {
    let lowered = lower_source(
        "record Ticket {} fn f(input: Ticket) -> Ticket { let mut x: Ticket = input; x = x; return x; }",
    );
    let function = lowered
        .as_program()
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();
    let x = function
        .body
        .locals
        .iter()
        .position(|local| local.name == "x")
        .map(|index| LocalId(index as u32))
        .unwrap();
    let statements = &function.body.blocks[0].statements;

    let assign_index = statements
        .iter()
        .position(|statement| matches!(statement, CoreStatement::Assign { dst, .. } if *dst == direct(Place::local(x))))
        .expect("self-assignment must emit Core Assign");
    let CoreStatement::Assign {
        src: Operand::Move(PlaceAccess::Direct(temp)),
        ..
    } = &statements[assign_index]
    else {
        panic!("self-assignment must move the produced temporary");
    };
    assert_eq!(
        statements[assign_index - 1],
        CoreStatement::Init {
            dst: temp.clone(),
            src: Operand::Move(direct(Place::local(x))),
        },
        "non-duplicable RHS must consume x before assigning the temporary back"
    );
}

#[test]
fn result_call_completes_before_assignment_in_the_continuation_block() {
    let lowered = lower_source(
        "record Ticket {} fn id(v: Ticket) -> Ticket { return v; } fn f(input: Ticket) -> Ticket { let mut x: Ticket = input; x = id(x); return x; }",
    );
    let program = lowered.as_program();
    let function = program
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();
    let x = function
        .body
        .locals
        .iter()
        .position(|local| local.name == "x")
        .map(|index| LocalId(index as u32))
        .unwrap();

    let Terminator::Call {
        destination: Some(destination),
        target: BasicBlockId(1),
        ..
    } = &function.body.blocks[0].terminator
    else {
        panic!("assignment RHS result call must terminate the producing block");
    };
    assert_eq!(
        function.body.blocks[1].statements[0],
        CoreStatement::Assign {
            dst: direct(Place::local(x)),
            src: Operand::Move(direct(destination.clone())),
        },
        "target replacement must begin only in the successful call continuation"
    );
}

#[test]
fn assignment_target_uses_binding_map_not_hir_numeric_identity() {
    let lowered = lower_source(
        "fn before(a: I64) {} fn f(p: I64) { let mut x: I64 = p; x = p; }",
    );
    let function = lowered
        .as_program()
        .functions
        .iter()
        .find(|function| function.name == "f")
        .unwrap();

    assert_eq!(function.body.locals[0].name, "p");
    assert_eq!(function.body.locals[1].name, "x");
    assert!(function.body.blocks[0].statements.iter().any(|statement| {
        matches!(
            statement,
            CoreStatement::Assign { dst, .. } if *dst == direct(Place::local(LocalId(1)))
        )
    }));
}
