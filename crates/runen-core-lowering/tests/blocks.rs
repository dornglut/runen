use runen_core_ir::{
    Function as CoreFunction, LocalId, Place, PlaceAccess, Statement as CoreStatement, Terminator,
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
    lower(&hir).expect("accepted HIR must lower to validated Core")
}

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a CoreFunction {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn direct_drop_place(statement: &CoreStatement) -> Option<Place> {
    let CoreStatement::Drop {
        place: PlaceAccess::Direct(place),
    } = statement
    else {
        return None;
    };
    Some(place.clone())
}

fn direct_drop_local(statement: &CoreStatement) -> Option<LocalId> {
    let place = direct_drop_place(statement)?;
    place.projections.is_empty().then_some(place.local)
}

fn drop_places(function: &CoreFunction) -> Vec<Place> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter_map(direct_drop_place)
        .collect()
}

fn drop_sequence(function: &CoreFunction) -> Vec<LocalId> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .filter_map(direct_drop_local)
        .collect()
}

#[test]
fn preregisters_recursive_source_locals_in_lexical_order_before_temporaries() {
    let lowered = lower_source(
        "fn f(x: I64) { \
            let root: I64 = x; \
            { let child: I64 = root; { let inner: I64 = child; } } \
            let tail: I64 = x; \
        }",
    );
    let f = function(lowered.as_program(), "f");
    let names = f
        .body
        .locals
        .iter()
        .map(|local| local.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(&names[..5], &["x", "root", "child", "inner", "tail"]);
    assert!(names[5..].iter().all(|name| name.starts_with("$tmp")));
}

#[test]
fn emits_validated_inner_then_outer_normal_cleanup_without_root_cleanup() {
    let lowered = lower_source(
        "fn f(x: I64) { \
            let root: I64 = x; \
            { let child: I64 = root; { let inner: I64 = child; } } \
        }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(drop_sequence(f), vec![LocalId(3), LocalId(2)]);
}

#[test]
fn skips_normal_exit_drop_for_zero_field_record() {
    let lowered = lower_source(
        "record Empty {} \
         fn f() { { let child: Empty = Empty {}; } }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(drop_sequence(f).is_empty());
}

#[test]
fn skips_normal_exit_drop_for_recursively_zero_leaf_record() {
    let lowered = lower_source(
        "record Inner {} \
         record Outer { inner: Inner } \
         fn f() { { let child: Outer = Outer { inner: Inner {} }; } }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(drop_sequence(f).is_empty());
}

#[test]
fn emits_normal_exit_drop_for_scalar_bearing_record() {
    let lowered = lower_source(
        "record Box { value: I64 } \
         fn f() { { let child: Box = Box { value: 1 }; } }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(drop_sequence(f), vec![LocalId(0)]);
}

#[test]
fn emits_normal_exit_drop_for_mixed_zero_and_scalar_leaf_record() {
    let lowered = lower_source(
        "record Empty {} \
         record Mixed { empty: Empty, value: I64 } \
         fn f() { \
             { let child: Mixed = Mixed { empty: Empty {}, value: 1 }; } \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(drop_sequence(f), vec![LocalId(0)]);
}

#[test]
fn consumed_child_local_receives_no_normal_exit_drop() {
    let lowered = lower_source(
        "record Ticket {} \
         fn sink(value: Ticket) {} \
         fn f(value: Ticket) { { let child: Ticket = value; sink(child); } }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(
        !drop_sequence(f).contains(&LocalId(1)),
        "consumed child local must already be Dead at normal child exit"
    );
}

#[test]
fn partial_child_cleanup_emits_exact_projected_frontier_in_source_order() {
    let lowered = lower_source(
        "record Left { value: I8 } record Right { value: I8 } \
         record Pair { left: Left, right: Right, count: I8 } \
         fn sink(value: Left) {} \
         fn f() { \
             { \
                 let pair: Pair = Pair { \
                     left: Left { value: 1 }, \
                     right: Right { value: 2 }, \
                     count: 3 \
                 }; \
                 sink(pair.left); \
             } \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        drop_places(f),
        vec![Place::local(LocalId(0)).field(2), Place::local(LocalId(0)).field(1)]
    );
    assert!(
        drop_places(f)
            .iter()
            .all(|place| !(place.local == LocalId(0) && place.projections.is_empty())),
        "partial cleanup must not fall back to whole-record Drop"
    );
}

#[test]
fn zero_leaf_only_remaining_frontier_erases_without_invalid_whole_drop() {
    let lowered = lower_source(
        "record Empty {} record Payload { value: I8 } \
         record Mixed { empty: Empty, payload: Payload } \
         fn take(value: Payload) {} \
         fn f() { \
             { \
                 let mixed: Mixed = Mixed { \
                     empty: Empty {}, \
                     payload: Payload { value: 1 } \
                 }; \
                 take(mixed.payload); \
             } \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(
        drop_places(f).is_empty(),
        "the only source-owned remainder has an empty Core destruction domain"
    );
}

#[test]
fn separately_consumed_scalar_bearing_siblings_need_no_child_drop() {
    let lowered = lower_source(
        "record Left { value: I8 } record Right { value: I8 } \
         record Pair { left: Left, right: Right } \
         fn take_left(value: Left) {} fn take_right(value: Right) {} \
         fn f() { \
             { \
                 let pair: Pair = Pair { \
                     left: Left { value: 1 }, \
                     right: Right { value: 2 } \
                 }; \
                 take_left(pair.left); \
                 take_right(pair.right); \
             } \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(drop_places(f).is_empty());
}

#[test]
fn block_cleanup_after_direct_call_is_emitted_in_call_continuation() {
    let lowered = lower_source(
        "fn sink(value: I64) {} \
         fn f(value: I64) { { let child: I64 = value; sink(child); } }",
    );
    let f = function(lowered.as_program(), "f");

    let Terminator::Call { target, .. } = f.body.blocks[0].terminator else {
        panic!("nested call must terminate the first Core block");
    };
    let continuation = &f.body.blocks[target.0 as usize];
    assert!(
        f.body.blocks[0]
            .statements
            .iter()
            .all(|statement| direct_drop_local(statement).is_none())
    );
    assert!(
        continuation
            .statements
            .iter()
            .any(|statement| direct_drop_local(statement) == Some(LocalId(1)))
    );
}

#[test]
fn sibling_scopes_drop_only_their_distinct_core_locals() {
    let lowered = lower_source("fn f() { { let a: I64 = 1; } { let a: I64 = 2; } }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .take(2)
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "a"]
    );
    assert_eq!(drop_sequence(f), vec![LocalId(0), LocalId(1)]);
}

#[test]
fn future_source_local_precedes_temporaries_but_initializes_only_after_call_continuation() {
    let lowered = lower_source(
        "fn sink(value: I64) {} \
         fn f(value: I64) { \
             { let before: I64 = value; sink(before); let future: I64 = value; } \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let names = f
        .body
        .locals
        .iter()
        .map(|local| local.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(&names[..3], &["value", "before", "future"]);
    assert!(names[3..].iter().all(|name| name.starts_with("$tmp")));

    let Terminator::Call { target, .. } = f.body.blocks[0].terminator else {
        panic!("call must split Core control flow");
    };
    assert!(f.body.blocks[0].statements.iter().all(|statement| {
        !matches!(statement, CoreStatement::Init { dst, .. } if dst.local == LocalId(2))
    }));
    assert!(f.body.blocks[target.0 as usize].statements.iter().any(
        |statement| matches!(statement, CoreStatement::Init { dst, .. } if dst.local == LocalId(2))
    ));
}
