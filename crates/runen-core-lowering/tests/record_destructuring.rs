use runen_core_ir::{
    Function as CoreFunction, LocalId, Operand, Place, PlaceAccess, Projection,
    Statement as CoreStatement, Terminator, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    ModuleId, RecordPatternScrutinee, RecordPatternTransientCleanup, SourceUnit, Statement, Type,
    build_typed_hir,
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
fn pattern_bindings_are_predeclared_in_source_order_and_lower_without_temporaries() {
    let lowered = lower_source(
        "record Token {} record Mixed { first: I8, token: Token, last: U8 } \
         fn f(root: Mixed) { let Mixed { last: z, token: moved, first: a } = root; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "z", "moved", "a"]
    );
    assert_eq!(
        f.body.blocks[0].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(2))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Move(direct(Place::local(LocalId(0)).field(1))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(0))),
            },
        ]
    );
}

#[test]
fn projected_copy_and_move_follow_pattern_source_order_not_record_order() {
    let lowered = lower_source(
        "record Token { value: I8 } record Pair { left: Token, count: I8, right: Token } \
         fn f(root: Pair) { \
             let Pair { right: r, count: c, left: l } = root; \
         }",
    );
    let f = function(lowered.as_program(), "f");

    let projections = f.body.blocks[0]
        .statements
        .iter()
        .map(|statement| match statement {
            CoreStatement::Init {
                src:
                    Operand::Copy(PlaceAccess::Direct(place))
                    | Operand::Move(PlaceAccess::Direct(place)),
                ..
            } => place.projections.clone(),
            other => panic!("unexpected statement {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        projections,
        vec![
            vec![Projection::Field(2)],
            vec![Projection::Field(1)],
            vec![Projection::Field(0)],
        ]
    );
}

#[test]
fn producer_construction_keeps_source_locals_before_temporaries_and_drops_complete_all_dup_value() {
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
    assert_eq!(
        drops(f),
        vec![direct(Place::local(LocalId(4)))]
    );
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
fn mixed_producer_cleanup_drops_retained_direct_fields_in_hir_order() {
    let lowered = lower_source(
        "record Token { value: I8 } record Mixed { first: I8, token: Token, last: U8 } \
         fn f() { \
             let Mixed { token: moved, last: z, first: a } = \
                 Mixed { first: 1, token: Token { value: 2 }, last: 3 }; \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let cleanup = drops(f);
    assert_eq!(cleanup.len(), 2);

    let [PlaceAccess::Direct(first), PlaceAccess::Direct(second)] = cleanup.as_slice() else {
        panic!("expected direct transient cleanup places");
    };
    assert_eq!(first.local, second.local);
    assert_eq!(first.projections, vec![Projection::Field(2)]);
    assert_eq!(second.projections, vec![Projection::Field(0)]);
}

#[test]
fn all_transferred_producer_emits_no_transient_drop() {
    let lowered = lower_source(
        "record Token { value: I8 } record Pair { left: Token, right: Token } \
         fn f() { \
             let Pair { right: r, left: l } = \
                 Pair { left: Token { value: 1 }, right: Token { value: 2 } }; \
         }",
    );
    let f = function(lowered.as_program(), "f");
    assert!(drops(f).is_empty());

    let moved_fields = f.body.blocks[0]
        .statements
        .iter()
        .filter_map(|statement| match statement {
            CoreStatement::Init {
                src: Operand::Move(PlaceAccess::Direct(place)),
                ..
            } if place.projections.len() == 1 => Some(place.projections.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(moved_fields.contains(&vec![Projection::Field(1)]));
    assert!(moved_fields.contains(&vec![Projection::Field(0)]));
}

#[test]
fn direct_call_producer_uses_call_result_temporary_as_pattern_source() {
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
fn field_value_producer_moves_source_field_into_one_pattern_temporary() {
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
        } if place.local == LocalId(3) && place.projections == vec![Projection::Field(0)]
    )));
    assert_eq!(drops(f), vec![direct(Place::local(LocalId(3)))]);
}

#[test]
fn zero_field_direct_pattern_emits_no_local_and_no_core_statement() {
    let lowered = lower_source("record Empty {} fn f(root: Empty) { let Empty {} = root; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.locals.len(), 1);
    assert!(f.body.blocks[0].statements.is_empty());
}

#[test]
fn zero_field_producer_keeps_source_ownership_but_erases_lower_vacuous_drop() {
    let lowered = lower_source("record Empty {} fn f() { let Empty {} = Empty {}; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["$tmp0"]
    );
    assert!(drops(f).is_empty());
    assert_eq!(f.body.blocks[0].statements.len(), 1);
    assert!(matches!(
        f.body.blocks[0].statements[0],
        CoreStatement::Init {
            src: Operand::Constant(runen_core_ir::Value::Struct(ref fields)),
            ..
        } if fields.is_empty()
    ));
}

#[test]
fn zero_leaf_nonduplicable_field_still_lowers_as_projected_move() {
    let lowered = lower_source(
        "record Empty {} record Holder { empty: Empty } \
         fn f(root: Holder) { let Holder { empty: moved } = root; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body.blocks[0].statements,
        vec![CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(Place::local(LocalId(0)).field(0))),
        }]
    );
}

#[test]
fn nested_pattern_cleanup_follows_reverse_source_binding_order() {
    let lowered = lower_source(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { { let Pair { right: second, left: first } = root; } }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "second", "first"]
    );
    assert_eq!(
        f.body.blocks[0].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(1))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(0))),
            },
            CoreStatement::Drop {
                place: direct(Place::local(LocalId(2))),
            },
            CoreStatement::Drop {
                place: direct(Place::local(LocalId(1))),
            },
        ]
    );
}

#[test]
fn lowering_rejects_invalid_pattern_field_index() {
    let mut compilation = hir("record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: a, right: b } = root; }");
    let Statement::RecordDestructure { bindings, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    bindings[0].field = 99;

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "HIR structural path does not match lowered Core type shape"
        ))
    );
}

#[test]
fn lowering_rejects_pattern_record_root_identity_mismatch() {
    let mut compilation = hir("record A { value: I8 } record B { value: I8 } \
         fn f(root: A) { let A { value: extracted } = root; }");
    let other = compilation.records[1].id;
    let Statement::RecordDestructure { record, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    *record = other;

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "record destructuring root type does not match its record identity"
        ))
    );
}

#[test]
fn lowering_rejects_producer_record_identity_mismatch() {
    let mut compilation = hir(
        "record A { value: I8 } record B { value: I8 } \
         fn f() { let A { value: extracted } = A { value: 1 }; }",
    );
    let other = compilation.records[1].id;
    let Statement::RecordDestructure { record, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    *record = other;

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "producer-backed record destructuring value type does not match its record identity"
        ))
    );
}

#[test]
fn lowering_rejects_retained_pattern_binding_type_mismatch() {
    let mut compilation = hir("record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: a, right: b } = root; }");
    let Statement::RecordDestructure { bindings, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    bindings[0].ty = Type::Intrinsic(runen_hir::IntrinsicType::U8);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "record destructuring retained binding type does not match projected field type"
        ))
    );
}

#[test]
fn lowering_uses_retained_pattern_ownership_without_rederiving_source_duplicability() {
    let mut compilation = hir("record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: a, right: b } = root; }");
    let Statement::RecordDestructure { bindings, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    bindings[0].ownership = runen_hir::OwnedUse::Consume;

    let lowered = lower(&compilation)
        .expect("lowering must refine retained ownership without re-running source duplicability");
    let f = function(lowered.as_program(), "f");
    assert_eq!(
        f.body.blocks[0].statements[0],
        CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(Place::local(LocalId(0)).field(0))),
        }
    );
}

#[test]
fn lowering_uses_retained_transient_cleanup_without_rederiving_source_frontier() {
    let mut compilation = hir(
        "record Pair { left: I8, right: U8 } \
         fn f() { let Pair { left: a, right: b } = Pair { left: 1, right: 2 }; }",
    );
    let Statement::RecordDestructure { scrutinee, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    let RecordPatternScrutinee::Producer { cleanup, .. } = scrutinee else {
        panic!("expected producer-backed scrutinee");
    };
    *cleanup = RecordPatternTransientCleanup::DirectFields(vec![0, 1]);

    let lowered = lower(&compilation)
        .expect("lowering must consume retained transient cleanup without source re-analysis");
    let f = function(lowered.as_program(), "f");
    let cleanup = drops(f);
    let [PlaceAccess::Direct(first), PlaceAccess::Direct(second)] = cleanup.as_slice() else {
        panic!("expected retained projected cleanup");
    };
    assert_eq!(first.local, second.local);
    assert_eq!(first.projections, vec![Projection::Field(0)]);
    assert_eq!(second.projections, vec![Projection::Field(1)]);
}

#[test]
fn lowering_rejects_duplicate_retained_transient_cleanup_field() {
    let mut compilation = hir(
        "record Pair { left: I8, right: U8 } \
         fn f() { let Pair { left: a, right: b } = Pair { left: 1, right: 2 }; }",
    );
    let Statement::RecordDestructure { scrutinee, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    let RecordPatternScrutinee::Producer { cleanup, .. } = scrutinee else {
        panic!("expected producer-backed scrutinee");
    };
    *cleanup = RecordPatternTransientCleanup::DirectFields(vec![0, 0]);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "record pattern transient cleanup contains duplicate field identity"
        ))
    );
}
