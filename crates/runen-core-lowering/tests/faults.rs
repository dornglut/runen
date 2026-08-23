use runen_core_ir::{Statement as CoreStatement, Terminator, ValidatedProgram};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn hir(source: &str) -> runen_hir::TypedCompilation {
    let parsed = parse(source);
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR")
}

fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted HIR must lower to validated Core")
}

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a runen_core_ir::Function {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn fault_code(block: &runen_core_ir::BasicBlock) -> Option<&str> {
    let Terminator::Fault(fault) = &block.terminator else {
        return None;
    };
    Some(fault.code.as_str())
}

#[test]
fn root_fault_lowers_to_fault_without_return_operand_or_normal_successor() {
    let lowered = lower_source("fn f() { fault; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.blocks.len(), 1);
    assert!(matches!(f.body.blocks[0].terminator, Terminator::Fault(_)));
    assert!(!matches!(
        f.body.blocks[0].terminator,
        Terminator::Goto(_) | Terminator::Return(_)
    ));
}

#[test]
fn result_bearing_root_fault_lowers_without_result_value() {
    let lowered = lower_source("fn f() -> I64 { fault; }");
    let f = function(lowered.as_program(), "f");

    assert!(f.result.is_some());
    assert_eq!(f.body.blocks.len(), 1);
    assert!(matches!(f.body.blocks[0].terminator, Terminator::Fault(_)));
}

#[test]
fn all_source_explicit_fault_sites_use_one_stable_core_reason() {
    let lowered =
        lower_source("fn a() { fault; } fn b(flag: Bool) { if flag { fault; } else { fault; } }");
    let program = lowered.as_program();
    let mut codes = program
        .functions
        .iter()
        .flat_map(|function| &function.body.blocks)
        .filter_map(fault_code);
    let first = codes.next().expect("at least one lowered fault").to_owned();

    assert!(codes.all(|code| code == first));
}

#[test]
fn live_block_local_is_not_normally_dropped_before_fault() {
    let lowered = lower_source(
        "record Box { value: I64 } fn f() { let value: Box = Box { value: 1 }; fault; }",
    );
    let f = function(lowered.as_program(), "f");
    let fault = f
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Terminator::Fault(_)))
        .expect("faulting block");

    assert!(
        !fault
            .statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::Drop { .. })),
        "Core Fault termination, not source normal cleanup, owns live-value cleanup"
    );
}

#[test]
fn consumed_value_is_not_reintroduced_or_dropped_before_fault() {
    let lowered = lower_source(
        "record Ticket { value: I64 } \
         fn sink(value: Ticket) {} \
         fn f() { let ticket: Ticket = Ticket { value: 1 }; sink(ticket); fault; }",
    );
    let f = function(lowered.as_program(), "f");
    let fault = f
        .body
        .blocks
        .iter()
        .find(|block| matches!(block.terminator, Terminator::Fault(_)))
        .expect("fault continuation block");

    assert!(
        !fault
            .statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::Drop { .. }))
    );
}

#[test]
fn omitted_else_faulting_then_arm_keeps_one_direct_normal_path() {
    let lowered = lower_source("fn f(flag: Bool) { if flag { fault; } }");
    let f = function(lowered.as_program(), "f");
    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[0].terminator
    else {
        panic!("entry must branch");
    };

    assert!(matches!(
        f.body.blocks[true_target.0 as usize].terminator,
        Terminator::Fault(_)
    ));
    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Return(None)
    ));
}

#[test]
fn explicit_fault_normal_conditional_has_no_normal_edge_from_faulting_arm() {
    let lowered = lower_source("fn f(flag: Bool) { if flag { fault; } else {} }");
    let f = function(lowered.as_program(), "f");
    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[0].terminator
    else {
        panic!("entry must branch");
    };

    assert!(matches!(
        f.body.blocks[true_target.0 as usize].terminator,
        Terminator::Fault(_)
    ));
    assert!(!matches!(
        f.body.blocks[true_target.0 as usize].terminator,
        Terminator::Goto(_)
    ));
    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Return(None)
    ));
}

#[test]
fn two_faulting_arms_emit_no_synthetic_join_or_root_return() {
    let lowered = lower_source("fn f(flag: Bool) { if flag { fault; } else { fault; } }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        1
    );
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Fault(_)))
            .count(),
        2
    );
    assert!(!f.body.blocks.iter().any(|block| matches!(
        block.terminator,
        Terminator::Goto(_) | Terminator::Return(_)
    )));
}

#[test]
fn return_fault_zero_normal_conditional_has_only_represented_terminations() {
    let lowered = lower_source("fn f(flag: Bool) -> I64 { if flag { return 1; } else { fault; } }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Return(Some(_))))
            .count(),
        1
    );
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Fault(_)))
            .count(),
        1
    );
    assert!(!f.body.blocks.iter().any(|block| matches!(
        block.terminator,
        Terminator::Goto(_) | Terminator::Return(None)
    )));
}

#[test]
fn call_to_faulting_callee_still_has_existing_normal_call_target() {
    let lowered = lower_source("fn die() { fault; } fn f() { die(); let x: I64 = 1; }");
    let f = function(lowered.as_program(), "f");
    let Terminator::Call { target, .. } = f.body.blocks[0].terminator else {
        panic!("caller entry must retain normal Call target");
    };

    let continuation = &f.body.blocks[target.0 as usize];
    assert!(
        continuation
            .statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::Init { .. }))
    );
    assert!(matches!(continuation.terminator, Terminator::Return(None)));
}
