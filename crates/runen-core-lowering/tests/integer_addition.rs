use runen_core_ir::{
    FunctionId, LocalId, Operand, PlaceAccess, Statement as CoreStatement, Terminator,
    ValidatedProgram, Value as CoreValue,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};
use runen_reference::{Machine, TerminalStatus};
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

fn integer_add_statements(function: &runen_core_ir::Function) -> Vec<&CoreStatement> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::IntegerAdd { .. }))
        .collect()
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
fn integer_add_lowers_to_one_fresh_result_with_move_operands_and_no_cfg() {
    let lowered = lower_source("fn f(left: I8, right: I8) -> I8 { return left + right; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.blocks.len(), 1, "plain addition adds no Core block");
    let additions = integer_add_statements(f);
    assert_eq!(additions.len(), 1);
    let CoreStatement::IntegerAdd { dst, left, right } = additions[0] else {
        unreachable!();
    };
    assert!(dst.projections.is_empty());
    let left_local = moved_local(left).expect("IntegerAdd left operand must Move a temporary");
    let right_local = moved_local(right).expect("IntegerAdd right operand must Move a temporary");
    assert_ne!(left_local, right_local);
    assert_ne!(dst.local, left_local);
    assert_ne!(dst.local, right_local);
    assert!(f.body.locals[left_local.0 as usize].name.starts_with("$tmp"));
    assert!(f.body.locals[right_local.0 as usize].name.starts_with("$tmp"));
    assert!(f.body.locals[dst.local.0 as usize].name.starts_with("$tmp"));
    assert!(left_local.0 < right_local.0);
    assert!(right_local.0 < dst.local.0);

    let Terminator::Return(Some(returned)) = &f.body.blocks[0].terminator else {
        panic!("addition result must feed the function return");
    };
    assert_eq!(moved_local(returned), Some(dst.local));
    assert!(!matches!(f.body.blocks[0].terminator, Terminator::Branch { .. }));
}

#[test]
fn call_operands_lower_left_then_right_before_integer_add() {
    let lowered = lower_source(
        "fn left() -> I8 { return 1; } \
         fn right() -> I8 { return 2; } \
         fn f() -> I8 { return left() + right(); }",
    );
    let f = function(lowered.as_program(), "f");

    let Terminator::Call {
        destination: Some(left_destination),
        target: right_call_block,
        ..
    } = &f.body.blocks[0].terminator
    else {
        panic!("left producer must lower first");
    };
    assert!(left_destination.projections.is_empty());

    let Terminator::Call {
        destination: Some(right_destination),
        target: add_block,
        ..
    } = &f.body.blocks[right_call_block.0 as usize].terminator
    else {
        panic!("right producer must lower from the successful left continuation");
    };
    assert!(right_destination.projections.is_empty());

    let additions = f.body.blocks[add_block.0 as usize]
        .statements
        .iter()
        .filter(|statement| matches!(statement, CoreStatement::IntegerAdd { .. }))
        .collect::<Vec<_>>();
    assert_eq!(additions.len(), 1);
    let CoreStatement::IntegerAdd { dst, left, right } = additions[0] else {
        unreachable!();
    };
    assert_eq!(moved_local(left), Some(left_destination.local));
    assert_eq!(moved_local(right), Some(right_destination.local));
    assert!(right_destination.local.0 < dst.local.0);
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Call { .. }))
            .count(),
        2
    );
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        0,
        "integer addition adds no branch or join"
    );
}

#[test]
fn grouped_nested_additions_lower_one_core_add_per_represented_addition() {
    let lowered = lower_source(
        "fn left(a: I8, b: I8, c: I8) -> I8 { return (a + b) + c; } \
         fn right(a: I8, b: I8, c: I8) -> I8 { return a + (b + c); }",
    );

    for name in ["left", "right"] {
        let function = function(lowered.as_program(), name);
        assert_eq!(integer_add_statements(function).len(), 2);
        assert_eq!(function.body.blocks.len(), 1);
        assert!(
            function
                .body
                .blocks
                .iter()
                .all(|block| !matches!(block.terminator, Terminator::Branch { .. }))
        );
    }
}

#[test]
fn lowering_rejects_non_integer_retained_integer_add_result_fact() {
    let mut compilation = hir("fn f(left: I8, right: I8) -> I8 { return left + right; }");
    let value = compilation.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    value.ty = Type::Intrinsic(IntrinsicType::Bool);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-add result type is not a fixed-width integer"
        ))
    );
}

#[test]
fn lowering_rejects_integer_add_operand_type_facts_that_do_not_match_result() {
    let mut left_mismatch = hir("fn f(left: I8, right: I8) -> I8 { return left + right; }");
    let value = left_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::IntegerAdd { left, .. } = &mut value.kind else {
        panic!("expected integer-add HIR value");
    };
    left.ty = Type::Intrinsic(IntrinsicType::I16);
    assert_eq!(
        lower(&left_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-add left operand type does not match result type"
        ))
    );

    let mut right_mismatch = hir("fn f(left: I8, right: I8) -> I8 { return left + right; }");
    let value = right_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::IntegerAdd { right, .. } = &mut value.kind else {
        panic!("expected integer-add HIR value");
    };
    right.ty = Type::Intrinsic(IntrinsicType::I16);
    assert_eq!(
        lower(&right_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-add right operand type does not match result type"
        ))
    );
}

#[test]
fn source_to_hir_to_core_to_reference_proves_in_range_and_wrapping_results() {
    for (source, expected) in [
        ("fn f() -> I8 { return 40 + 2; }", CoreValue::I8(42)),
        (
            "fn f() -> I8 { return 127 + 1; }",
            CoreValue::I8(i8::MIN),
        ),
        ("fn f() -> U8 { return 255 + 1; }", CoreValue::U8(0)),
    ] {
        let report = execute_source(source, "f");
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
    }
}
