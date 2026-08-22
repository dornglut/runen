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
fn one_level_pattern_bindings_remain_predeclared_in_source_order_without_temporaries() {
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
fn recursive_direct_root_lowers_full_paths_in_depth_first_source_order_without_scrutinee_temp() {
    let lowered = lower_source(
        "record Token { value: I8 } \
         record Leaf { value: I8, token: Token } \
         record Inner { leaf: Leaf, count: U8 } \
         record Outer { tail: I8, inner: Inner } \
         fn f(root: Outer) { \
             let Outer { \
                 inner: Inner { count: count, leaf: Leaf { token: moved, value: value } }, \
                 tail: tail, \
             } = root; \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "count", "moved", "value", "tail"]
    );
    assert!(f.body.locals.iter().all(|local| !local.name.starts_with("$tmp")));

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
        [
            vec![Projection::Field(1), Projection::Field(1)],
            vec![
                Projection::Field(1),
                Projection::Field(0),
                Projection::Field(1),
            ],
            vec![
                Projection::Field(1),
                Projection::Field(0),
                Projection::Field(0),
            ],
            vec![Projection::Field(0)],
        ]
    );
    assert!(matches!(
        f.body.blocks[0].statements[1],
        CoreStatement::Init {
            src: Operand::Move(_),
            ..
        }
    ));
}

#[test]
fn producer_construction_keeps_source_locals_before_temporaries_and_drops_complete_all_dup_value() {
    let lowered = lower_source(
        "record Inner { left: I8, right: U8 } record Outer { inner: Inner, tail: I8 } \
         fn f() { \
             let Outer { inner: Inner { right: r, left: l }, tail: tail } = \
                 Outer { inner: Inner { left: 1, right: 2 }, tail: 3 }; \
         }",
    );
    let f = function(lowered.as_program(), "f");

    let names = f
        .body
        .locals
        .iter()
        .map(|local| local.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(&names[..3], ["r", "l", "tail"]);
    assert!(names[3..].iter().all(|name| name.starts_with("$tmp")));
    let cleanup = drops(f);
    assert_eq!(cleanup.len(), 1);
    let PlaceAccess::Direct(cleanup_root) = &cleanup[0] else {
        panic!("expected direct cleanup place");
    };
    assert!(cleanup_root.projections.is_empty());
}

#[test]
fn mixed_recursive_producer_cleanup_drops_retained_paths_in_canonical_hir_order() {
    let lowered = lower_source(
        "record Token { value: I8 } \
         record Inner { a: I8, token: Token, b: U8 } \
         record Outer { head: I8, inner: Inner, tail: U8 } \
         fn f() { \
             let Outer { \
                 inner: Inner { token: moved, a: a, b: b }, \
                 head: head, tail: tail, \
             } = Outer { \
                 head: 1, inner: Inner { a: 2, token: Token { value: 3 }, b: 4 }, tail: 5 \
             }; \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let cleanup = drops(f);
    assert_eq!(cleanup.len(), 4);
    let paths = cleanup
        .iter()
        .map(|place| match place {
            PlaceAccess::Direct(place) => place.projections.clone(),
            _ => panic!("expected direct cleanup place"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            vec![Projection::Field(2)],
            vec![Projection::Field(1), Projection::Field(2)],
            vec![Projection::Field(1), Projection::Field(0)],
            vec![Projection::Field(0)],
        ]
    );
}

#[test]
fn all_transferred_recursive_producer_emits_no_transient_drop() {
    let lowered = lower_source(
        "record A { value: I8 } record B { value: I8 } record C { value: I8 } \
         record Inner { b: B, c: C } record Outer { a: A, inner: Inner } \
         fn f() { \
             let Outer { inner: Inner { c: c, b: b }, a: a } = \
                 Outer { a: A { value: 1 }, inner: Inner { b: B { value: 2 }, c: C { value: 3 } } }; \
         }",
    );
    let f = function(lowered.as_program(), "f");
    assert!(drops(f).is_empty());

    let moved_paths = f.body.blocks[0]
        .statements
        .iter()
        .filter_map(|statement| match statement {
            CoreStatement::Init {
                src: Operand::Move(PlaceAccess::Direct(place)),
                ..
            } if !place.projections.is_empty() => Some(place.projections.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(moved_paths.iter().any(|path| path == &vec![Projection::Field(0)]));
    assert!(moved_paths.iter().any(|path| {
        path == &vec![Projection::Field(1), Projection::Field(0)]
    }));
    assert!(moved_paths.iter().any(|path| {
        path == &vec![Projection::Field(1), Projection::Field(1)]
    }));
}

#[test]
fn direct_call_producer_uses_existing_call_result_temporary_as_recursive_pattern_source() {
    let lowered = lower_source(
        "record Inner { left: I8, right: U8 } record Outer { inner: Inner, tail: I8 } \
         fn make() -> Outer { return Outer { inner: Inner { left: 1, right: 2 }, tail: 3 }; } \
         fn f() { let Outer { inner: Inner { left: l, right: r }, tail: tail } = make(); }",
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
            } if !place.projections.is_empty() => Some(place.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(pattern_sources.len(), 3);
    assert!(
        pattern_sources
            .iter()
            .all(|source| source.local == destination.local)
    );
    assert_eq!(drops(f), vec![direct(Place::local(destination.local))]);
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
}

#[test]
fn nested_zero_leaf_nonduplicable_leaf_still_lowers_as_full_projected_move() {
    let lowered = lower_source(
        "record Empty {} record Inner { empty: Empty } record Outer { inner: Inner } \
         fn f(root: Outer) { let Outer { inner: Inner { empty: moved } } = root; }",
    );
    let f = function(lowered.as_program(), "f");
    assert_eq!(
        f.body.blocks[0].statements,
        vec![CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(
                Place::local(LocalId(0)).field(0).field(0)
            )),
        }]
    );
}

#[test]
fn nested_block_cleanup_follows_reverse_depth_first_binding_order() {
    let lowered = lower_source(
        "record Inner { left: I8, right: U8 } record Outer { head: I8, inner: Inner } \
         fn f(root: Outer) { \
             { let Outer { inner: Inner { right: second, left: first }, head: head } = root; } \
         }",
    );
    let f = function(lowered.as_program(), "f");
    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "second", "first", "head"]
    );
    let cleanup = drops(f);
    assert_eq!(
        cleanup,
        [
            direct(Place::local(LocalId(3))),
            direct(Place::local(LocalId(2))),
            direct(Place::local(LocalId(1))),
        ]
    );
}

#[test]
fn lowering_rejects_invalid_recursive_pattern_path() {
    let mut compilation = hir(
        "record Inner { value: I8 } record Outer { inner: Inner } \
         fn f(root: Outer) { let Outer { inner: Inner { value: item } } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    bindings[0].fields = vec![0, 99];

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "HIR structural path does not match lowered Core type shape"
        ))
    );
}

#[test]
fn lowering_rejects_overlapping_retained_binding_paths() {
    let mut compilation = hir(
        "record Inner { value: I8 } record Outer { inner: Inner, other: I8 } \
         fn f(root: Outer) { let Outer { inner: Inner { value: item }, other: other } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    bindings[1].fields = vec![0];

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "record destructuring binding paths are not structurally disjoint"
        ))
    );
}

#[test]
fn lowering_rejects_pattern_record_root_identity_mismatch() {
    let mut compilation = hir(
        "record A { value: I8 } record B { value: I8 } \
         fn f(root: A) { let A { value: extracted } = root; }",
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
    let mut compilation = hir(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: a, right: b } = root; }",
    );
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
    let mut compilation = hir(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: a, right: b } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    bindings[0].ownership = runen_hir::OwnedUse::Consume;

    let lowered = lower(&compilation)
        .expect("lowering must refine retained ownership without source re-analysis");
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
fn lowering_uses_retained_recursive_transient_cleanup_without_rederiving_frontier() {
    let mut compilation = hir(
        "record Inner { left: I8, right: U8 } record Outer { inner: Inner, tail: I8 } \
         fn f() { \
             let Outer { inner: Inner { left: a, right: b }, tail: tail } = \
                 Outer { inner: Inner { left: 1, right: 2 }, tail: 3 }; \
         }",
    );
    let Statement::RecordDestructure { scrutinee, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    let RecordPatternScrutinee::Producer { cleanup, .. } = scrutinee else {
        panic!("expected producer-backed scrutinee");
    };
    *cleanup = RecordPatternTransientCleanup {
        paths: vec![vec![0, 1], vec![1]],
    };

    let lowered = lower(&compilation)
        .expect("lowering must consume retained cleanup without source re-analysis");
    let f = function(lowered.as_program(), "f");
    let paths = drops(f)
        .iter()
        .map(|place| match place {
            PlaceAccess::Direct(place) => place.projections.clone(),
            _ => panic!("expected direct cleanup place"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            vec![Projection::Field(0), Projection::Field(1)],
            vec![Projection::Field(1)],
        ]
    );
}

#[test]
fn lowering_rejects_overlapping_retained_transient_cleanup_paths() {
    let mut compilation = hir(
        "record Inner { left: I8, right: U8 } record Outer { inner: Inner, tail: I8 } \
         fn f() { \
             let Outer { inner: Inner { left: a, right: b }, tail: tail } = \
                 Outer { inner: Inner { left: 1, right: 2 }, tail: 3 }; \
         }",
    );
    let Statement::RecordDestructure { scrutinee, .. } =
        &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    let RecordPatternScrutinee::Producer { cleanup, .. } = scrutinee else {
        panic!("expected producer-backed scrutinee");
    };
    cleanup.paths = vec![vec![0], vec![0, 1]];

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "record pattern transient cleanup paths are not structurally disjoint"
        ))
    );
}
