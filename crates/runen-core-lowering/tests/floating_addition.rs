use runen_core_ir::{
    BinaryFloatSign, BinaryFloatValue, FunctionId, LocalId, Operand, PlaceAccess,
    Statement as CoreStatement, Terminator, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};
use runen_reference::{
    Machine, ObservedBinaryFloatValue, ObservedValue, TerminalStatus,
};
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

fn float_add_statements(function: &runen_core_ir::Function) -> Vec<&CoreStatement> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::FloatAdd { .. }))
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

fn represented_normal(significand: u64, exponent: i16) -> ObservedBinaryFloatValue {
    ObservedBinaryFloatValue::Represented(BinaryFloatValue::Normal {
        sign: BinaryFloatSign::Positive,
        significand,
        exponent,
    })
}

#[test]
fn float_add_lowers_to_one_fresh_result_with_move_operands_and_no_cfg() {
    let lowered = lower_source("fn f(left: F32, right: F32) -> F32 { return left + right; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.blocks.len(), 1, "plain floating addition adds no Core block");
    let additions = float_add_statements(f);
    assert_eq!(additions.len(), 1);
    assert!(
        f.body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .all(|statement| !matches!(statement, CoreStatement::IntegerAdd { .. })),
        "floating source addition must not refine to Core IntegerAdd"
    );
    let CoreStatement::FloatAdd { dst, left, right } = additions[0] else {
        unreachable!();
    };
    assert!(dst.projections.is_empty());
    let left_local = moved_local(left).expect("FloatAdd left operand must Move a temporary");
    let right_local = moved_local(right).expect("FloatAdd right operand must Move a temporary");
    assert_ne!(left_local, right_local);
    assert_ne!(dst.local, left_local);
    assert_ne!(dst.local, right_local);
    assert!(f.body.locals[left_local.0 as usize].name.starts_with("$tmp"));
    assert!(f.body.locals[right_local.0 as usize].name.starts_with("$tmp"));
    assert!(f.body.locals[dst.local.0 as usize].name.starts_with("$tmp"));
    assert!(left_local.0 < right_local.0);
    assert!(right_local.0 < dst.local.0);

    let Terminator::Return(Some(returned)) = &f.body.blocks[0].terminator else {
        panic!("floating addition result must feed the function return");
    };
    assert_eq!(moved_local(returned), Some(dst.local));
    assert!(
        f.body
            .blocks
            .iter()
            .all(|block| !matches!(block.terminator, Terminator::Branch { .. }))
    );
}

#[test]
fn call_operands_lower_complete_left_then_right_before_float_add() {
    let lowered = lower_source(
        "fn left() -> F32 { return 1.0; } \
         fn right() -> F32 { return 2.0; } \
         fn f() -> F32 { return left() + right(); }",
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
        .filter(|statement| matches!(statement, CoreStatement::FloatAdd { .. }))
        .collect::<Vec<_>>();
    assert_eq!(additions.len(), 1);
    let CoreStatement::FloatAdd { dst, left, right } = additions[0] else {
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
        "FloatAdd itself adds no branch or join"
    );
}

#[test]
fn grouped_nested_float_additions_lower_one_core_add_per_hir_addition() {
    let lowered = lower_source(
        "fn left(a: F32, b: F32, c: F32) -> F32 { return (a + b) + c; } \
         fn right(a: F32, b: F32, c: F32) -> F32 { return a + (b + c); }",
    );

    for name in ["left", "right"] {
        let function = function(lowered.as_program(), name);
        assert_eq!(float_add_statements(function).len(), 2);
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
fn lowering_rejects_non_floating_retained_float_add_result_fact() {
    let mut compilation = hir("fn f(left: F32, right: F32) -> F32 { return left + right; }");
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
            "Float-add result type is not a represented floating type"
        ))
    );
}

#[test]
fn lowering_rejects_float_add_operand_type_facts_that_do_not_match_result() {
    let mut left_mismatch = hir("fn f(left: F32, right: F32) -> F32 { return left + right; }");
    let value = left_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::FloatAdd { left, .. } = &mut value.kind else {
        panic!("expected float-add HIR value");
    };
    left.ty = Type::Intrinsic(IntrinsicType::F16);
    assert_eq!(
        lower(&left_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Float-add left operand type does not match result type"
        ))
    );

    let mut right_mismatch = hir("fn f(left: F32, right: F32) -> F32 { return left + right; }");
    let value = right_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::FloatAdd { right, .. } = &mut value.kind else {
        panic!("expected float-add HIR value");
    };
    right.ty = Type::Intrinsic(IntrinsicType::F64);
    assert_eq!(
        lower(&right_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Float-add right operand type does not match result type"
        ))
    );
}

#[test]
fn source_to_hir_to_core_to_reference_executes_all_three_formats() {
    let cases = [
        (
            "fn f() -> F16 { return 1.0 + 1.0; }",
            ObservedValue::F16(represented_normal(1_u64 << 10, 1)),
        ),
        (
            "fn f() -> F32 { return 1.0 + 1.0; }",
            ObservedValue::F32(represented_normal(1_u64 << 23, 1)),
        ),
        (
            "fn f() -> F64 { return 1.0 + 1.0; }",
            ObservedValue::F64(represented_normal(1_u64 << 52, 1)),
        ),
    ];

    for (source, expected) in cases {
        let report = execute_source(source, "f");
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn source_reachable_f16_finite_addition_can_overflow_to_infinity() {
    let report = execute_source(
        "fn f() -> F16 { return 65504.0 + 65504.0; }",
        "f",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(
        report.result,
        Some(ObservedValue::F16(ObservedBinaryFloatValue::Represented(
            BinaryFloatValue::Infinity(BinaryFloatSign::Positive),
        )))
    );
}
