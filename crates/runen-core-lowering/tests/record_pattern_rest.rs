use runen_core_ir::{
    Function as CoreFunction, Operand, PlaceAccess, Projection, Statement as CoreStatement,
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

fn drop_paths(function: &CoreFunction) -> Vec<Vec<Projection>> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| match statement {
            CoreStatement::Drop { place } => Some(match place {
                PlaceAccess::Direct(place) => place.projections.clone(),
                other => panic!("unexpected non-direct record-pattern cleanup {other:?}"),
            }),
            _ => None,
        })
        .collect()
}

#[test]
fn direct_root_rest_lowers_only_retained_explicit_path_without_temporary() {
    let lowered = lower_source(
        "record Token { value: I8 } \
         record Pair { selected: Token, omitted: Token } \
         fn f(root: Pair) { let Pair { selected: moved, .. } = root; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "moved"]
    );
    assert!(
        f.body
            .locals
            .iter()
            .all(|local| !local.name.starts_with("$tmp"))
    );
    assert_eq!(f.body.blocks[0].statements.len(), 1);
    match &f.body.blocks[0].statements[0] {
        CoreStatement::Init {
            src: Operand::Move(PlaceAccess::Direct(place)),
            ..
        } => assert_eq!(place.projections, [Projection::Field(0)]),
        other => panic!("unexpected direct-root rest lowering {other:?}"),
    }
}

#[test]
fn producer_rest_destroys_retained_omitted_and_duplicated_paths_in_hir_order() {
    let lowered = lower_source(
        "record Token { value: I8 } \
         record Pair { copied: I8, moved: Token, omitted: Token } \
         fn f() { \
             let Pair { copied: copied_value, moved: moved, .. } = \
                 Pair { copied: 1, moved: Token { value: 2 }, omitted: Token { value: 3 } }; \
         }",
    );
    let f = function(lowered.as_program(), "f");

    let names = f
        .body
        .locals
        .iter()
        .map(|local| local.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(&names[..2], ["copied_value", "moved"]);
    assert!(names[2..].iter().all(|name| name.starts_with("$tmp")));
    assert_eq!(
        drop_paths(f),
        [vec![Projection::Field(2)], vec![Projection::Field(0)]]
    );
}

#[test]
fn producer_rest_only_emits_whole_transient_cleanup_and_no_binding_local() {
    let lowered = lower_source(
        "record Token { value: I8 } \
         record Pair { left: Token, right: Token } \
         fn f() { \
             let Pair { .. } = Pair { left: Token { value: 1 }, right: Token { value: 2 } }; \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(!f.body.locals.is_empty());
    assert!(
        f.body
            .locals
            .iter()
            .all(|local| local.name.starts_with("$tmp"))
    );
    assert_eq!(drop_paths(f), [Vec::<Projection>::new()]);
}
