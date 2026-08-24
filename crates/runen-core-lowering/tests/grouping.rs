use runen_core_ir::{Statement as CoreStatement, Terminator, ValidatedProgram};
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

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a runen_core_ir::Function {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn branch_count(function: &runen_core_ir::Function) -> usize {
    function
        .body
        .blocks
        .iter()
        .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
        .count()
}

fn init_count(function: &runen_core_ir::Function) -> usize {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::Init { .. }))
        .count()
}

fn call_count(function: &runen_core_ir::Function) -> usize {
    function
        .body
        .blocks
        .iter()
        .filter(|block| matches!(block.terminator, Terminator::Call { .. }))
        .count()
}

fn terminator_kinds(function: &runen_core_ir::Function) -> Vec<&'static str> {
    function
        .body
        .blocks
        .iter()
        .map(|block| match block.terminator {
            Terminator::Return(_) => "return",
            Terminator::Goto(_) => "goto",
            Terminator::Branch { .. } => "branch",
            Terminator::Call { .. } => "call",
            Terminator::Fault(_) => "fault",
        })
        .collect()
}

#[test]
fn simple_grouping_adds_no_core_blocks_locals_statements_or_control_operations() {
    let plain = lower_source("fn f(flag: Bool) -> Bool { return flag; }");
    let grouped = lower_source("fn f(flag: Bool) -> Bool { return (((flag))); }");
    let plain = function(plain.as_program(), "f");
    let grouped = function(grouped.as_program(), "f");

    assert_eq!(grouped.body.locals.len(), plain.body.locals.len());
    assert_eq!(grouped.body.blocks.len(), plain.body.blocks.len());
    assert_eq!(branch_count(grouped), branch_count(plain));
    assert_eq!(call_count(grouped), call_count(plain));
    assert_eq!(init_count(grouped), init_count(plain));
    assert_eq!(terminator_kinds(grouped), terminator_kinds(plain));
}

#[test]
fn grouped_call_lowers_exactly_once_through_the_existing_call_relation() {
    let plain = lower_source(
        "fn ready() -> Bool { return true; } fn f() -> Bool { return ready(); }",
    );
    let grouped = lower_source(
        "fn ready() -> Bool { return true; } fn f() -> Bool { return (((ready()))); }",
    );
    let plain = function(plain.as_program(), "f");
    let grouped = function(grouped.as_program(), "f");

    assert_eq!(call_count(plain), 1);
    assert_eq!(call_count(grouped), 1);
    assert_eq!(grouped.body.locals.len(), plain.body.locals.len());
    assert_eq!(grouped.body.blocks.len(), plain.body.blocks.len());
    assert_eq!(terminator_kinds(grouped), terminator_kinds(plain));
}

#[test]
fn grouped_equality_uses_the_existing_equality_cfg_without_extra_grouping_structure() {
    let plain = lower_source("fn f(a: Bool, b: Bool) -> Bool { return a == b; }");
    let grouped = lower_source("fn f(a: Bool, b: Bool) -> Bool { return (a == b); }");
    let plain = function(plain.as_program(), "f");
    let grouped = function(grouped.as_program(), "f");

    assert_eq!(branch_count(plain), 3);
    assert_eq!(branch_count(grouped), 3);
    assert_eq!(grouped.body.locals.len(), plain.body.locals.len());
    assert_eq!(grouped.body.blocks.len(), plain.body.blocks.len());
    assert_eq!(init_count(grouped), init_count(plain));
    assert_eq!(terminator_kinds(grouped), terminator_kinds(plain));
}

#[test]
fn grouped_nested_operator_source_lowers_through_existing_operator_relations() {
    let lowered = lower_source(
        r#"
fn negated(a: Bool, b: Bool) -> Bool { return !(a == b); }
fn left(a: Bool, b: Bool, c: Bool) -> Bool { return (a == b) == c; }
fn right(a: Bool, b: Bool, c: Bool) -> Bool { return a == (b != c); }
"#,
    );

    assert_eq!(branch_count(function(lowered.as_program(), "negated")), 4);
    assert_eq!(branch_count(function(lowered.as_program(), "left")), 6);
    assert_eq!(branch_count(function(lowered.as_program(), "right")), 6);
}
