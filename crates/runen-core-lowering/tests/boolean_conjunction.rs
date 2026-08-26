use runen_core_ir::{
    FunctionId, LocalId, Operand, PlaceAccess, Statement as CoreStatement, Terminator,
    ValidatedProgram, Value as CoreValue,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};
use runen_reference::{Machine, ObservedValue, TerminalStatus};
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

fn moved_local(operand: &Operand) -> Option<LocalId> {
    let Operand::Move(PlaceAccess::Direct(place)) = operand else {
        return None;
    };
    place.projections.is_empty().then_some(place.local)
}

fn execute_source(source: &str, entry_name: &str) -> runen_reference::ExecutionReport {
    let lowered = lower_source(source);
    let entry_index = lowered
        .as_program()
        .functions
        .iter()
        .position(|function| function.name == entry_name)
        .unwrap_or_else(|| panic!("missing Core entry function {entry_name}"));
    let entry = FunctionId(u32::try_from(entry_index).expect("test function index fits u32"));
    Machine::new(lowered, entry)
        .expect("test entry has no parameters")
        .execute()
        .expect("safe lowered execution is defined")
}

#[test]
fn conjunction_refines_to_move_branch_false_init_true_rhs_and_one_join() {
    let lowered = lower_source("fn f(left: Bool, right: Bool) -> Bool { return left && right; }");
    let f = function(lowered.as_program(), "f");

    let Terminator::Branch {
        condition,
        true_target,
        false_target,
    } = &f.body.blocks[0].terminator
    else {
        panic!("conjunction entry must branch on left");
    };
    let left_local = moved_local(condition).expect("conjunction Branch must Move left temporary");
    assert!(
        f.body.locals[left_local.0 as usize]
            .name
            .starts_with("$tmp")
    );

    let false_block = &f.body.blocks[false_target.0 as usize];
    assert_eq!(false_block.statements.len(), 1);
    let CoreStatement::Init {
        dst: false_dst,
        src,
    } = &false_block.statements[0]
    else {
        panic!("false path must initialize conjunction result");
    };
    assert!(false_dst.projections.is_empty());
    assert_eq!(src, &Operand::Constant(CoreValue::Bool(false)));
    let Terminator::Goto(false_join) = false_block.terminator else {
        panic!("false path must jump directly to conjunction join");
    };

    let true_block = &f.body.blocks[true_target.0 as usize];
    assert_eq!(true_block.statements.len(), 2);
    let CoreStatement::Init {
        dst: right_dst,
        src: right_src,
    } = &true_block.statements[0]
    else {
        panic!("true path must lower right producer first");
    };
    assert!(right_dst.projections.is_empty());
    assert!(matches!(right_src, Operand::Copy(_)));
    let CoreStatement::Init { dst: true_dst, src } = &true_block.statements[1] else {
        panic!("true path must initialize conjunction result from RHS");
    };
    assert_eq!(true_dst.local, false_dst.local);
    assert_eq!(moved_local(src), Some(right_dst.local));
    let Terminator::Goto(true_join) = true_block.terminator else {
        panic!("true path must jump to conjunction join");
    };
    assert_eq!(true_join, false_join);

    let join = &f.body.blocks[true_join.0 as usize];
    let Terminator::Return(Some(returned)) = &join.terminator else {
        panic!("following return must lower from conjunction join");
    };
    assert_eq!(moved_local(returned), Some(false_dst.local));
}

#[test]
fn call_backed_rhs_is_lowered_only_from_true_branch_continuation() {
    let lowered = lower_source(
        "fn rhs() -> Bool { return true; } fn f(left: Bool) -> Bool { return left && rhs(); }",
    );
    let f = function(lowered.as_program(), "f");
    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[0].terminator
    else {
        panic!("entry must branch on conjunction left");
    };

    assert!(matches!(
        f.body.blocks[true_target.0 as usize].terminator,
        Terminator::Call { .. }
    ));
    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Goto(_)
    ));
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Call { .. }))
            .count(),
        1,
        "RHS call is represented on exactly the true path"
    );
}

#[test]
fn false_left_runtime_skips_faulting_rhs_but_true_left_reaches_same_fault() {
    let false_report = execute_source(
        "fn boom() -> Bool { fault; } fn f() -> Bool { return false && boom(); }",
        "f",
    );
    assert_eq!(false_report.terminal, TerminalStatus::Returned);
    assert_eq!(false_report.result, Some(ObservedValue::Bool(false)));

    let true_report = execute_source(
        "fn boom() -> Bool { fault; } fn f() -> Bool { return true && boom(); }",
        "f",
    );
    assert_eq!(
        true_report.terminal,
        TerminalStatus::Faulted("source.explicit".to_owned())
    );
    assert_eq!(true_report.result, None);
}

#[test]
fn conjunction_truth_values_execute_through_existing_core_machine() {
    for (source, expected) in [
        ("fn f() -> Bool { return false && false; }", false),
        ("fn f() -> Bool { return false && true; }", false),
        ("fn f() -> Bool { return true && false; }", false),
        ("fn f() -> Bool { return true && true; }", true),
    ] {
        let report = execute_source(source, "f");
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(ObservedValue::Bool(expected)));
    }
}

#[test]
fn nested_conjunctions_preserve_current_block_and_build_nested_cfg() {
    let lowered = lower_source("fn f(a: Bool, b: Bool, c: Bool) -> Bool { return a && (b && c); }");
    let f = function(lowered.as_program(), "f");
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        2,
        "one Branch is emitted per represented conjunction"
    );
    assert!(
        f.body
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Return(Some(_))))
    );
}

#[test]
fn conjunction_uses_only_existing_core_init_and_control_flow_operations() {
    let lowered = lower_source("fn f(left: Bool, right: Bool) -> Bool { return left && right; }");
    let f = function(lowered.as_program(), "f");

    assert!(
        f.body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .all(|statement| matches!(statement, CoreStatement::Init { .. }))
    );
    assert!(f.body.blocks.iter().all(|block| matches!(
        block.terminator,
        Terminator::Branch { .. } | Terminator::Goto(_) | Terminator::Return(_)
    )));
}

#[test]
fn lowering_rejects_malformed_retained_conjunction_type_facts() {
    let mut result_mismatch = hir("fn f(a: Bool, b: Bool) -> Bool { return a && b; }");
    let value = result_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    value.ty = Type::Intrinsic(IntrinsicType::I64);
    assert_eq!(
        lower(&result_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Boolean-conjunction result type is not Bool"
        ))
    );

    let mut left_mismatch = hir("fn f(a: Bool, b: Bool) -> Bool { return a && b; }");
    let value = left_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::BooleanAnd { left, .. } = &mut value.kind else {
        panic!("expected conjunction HIR value");
    };
    left.ty = Type::Intrinsic(IntrinsicType::I64);
    assert_eq!(
        lower(&left_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Boolean-conjunction left operand type is not Bool"
        ))
    );

    let mut right_mismatch = hir("fn f(a: Bool, b: Bool) -> Bool { return a && b; }");
    let value = right_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::BooleanAnd { right, .. } = &mut value.kind else {
        panic!("expected conjunction HIR value");
    };
    right.ty = Type::Intrinsic(IntrinsicType::I64);
    assert_eq!(
        lower(&right_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Boolean-conjunction right operand type is not Bool"
        ))
    );
}

#[test]
fn conjunction_lowers_through_existing_generic_value_consumers() {
    let lowered = lower_source(
        "record Wrapper { value: Bool } \
         fn sink(value: Bool) {} \
         fn f(a: Bool, b: Bool) -> Bool { \
             let wrapper: Wrapper = Wrapper { value: a && b }; \
             let mut local: Bool = a && b; \
             local = a && b; \
             sink(a && b); \
             if a && b {} \
             while a && b { break; } \
             return a && b; \
         }",
    );
    let f = function(lowered.as_program(), "f");
    assert!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count()
            >= 9,
        "conjunctions plus existing if/while control flow all refine through CFG"
    );
    assert!(
        f.body
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Call { .. })),
        "conjunction result remains usable as a call argument"
    );
    assert!(
        f.body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .any(|statement| matches!(
                statement,
                CoreStatement::Init { dst, .. } if !dst.projections.is_empty()
            )),
        "conjunction result remains usable as a record-construction field"
    );
}
