use runen_core_ir::{
    LocalId, NumericContract as CoreNumericContract, Operand, PlaceAccess,
    Statement as CoreStatement, Terminator, ValidatedProgram,
};
use runen_core_lowering::lower;
use runen_hir::{
    ModuleId, NumericContract as HirNumericContract, SourceUnit, ValueKind, build_typed_hir,
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

fn float_div_statements(function: &runen_core_ir::Function) -> Vec<&CoreStatement> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::FloatDiv { .. }))
        .collect()
}

fn float_div_contract(statement: &CoreStatement) -> CoreNumericContract {
    let CoreStatement::FloatDiv { contract, .. } = statement else {
        panic!("expected Core FloatDiv");
    };
    *contract
}

#[test]
fn float_div_lowers_to_one_fresh_standard_result_with_move_operands_and_no_cfg() {
    let lowered = lower_source("fn f(left: F32, right: F32) -> F32 { return left / right; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.blocks.len(), 1, "plain FloatDiv adds no Core block");
    let divisions = float_div_statements(f);
    assert_eq!(divisions.len(), 1);
    assert_eq!(
        float_div_contract(divisions[0]),
        CoreNumericContract::Standard
    );
    assert!(
        f.body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .all(|statement| !matches!(statement, CoreStatement::FloatMul { .. })),
        "floating source division must remain distinct from Core FloatMul"
    );

    let CoreStatement::FloatDiv {
        dst, left, right, ..
    } = divisions[0]
    else {
        unreachable!();
    };
    assert!(dst.projections.is_empty());
    let left_local = moved_local(left).expect("FloatDiv left operand must Move a temporary");
    let right_local = moved_local(right).expect("FloatDiv right operand must Move a temporary");
    assert_ne!(left_local, right_local);
    assert_ne!(dst.local, left_local);
    assert_ne!(dst.local, right_local);
    assert!(left_local.0 < right_local.0);
    assert!(right_local.0 < dst.local.0);

    let Terminator::Return(Some(returned)) = &f.body.blocks[0].terminator else {
        panic!("FloatDiv result must feed the function return");
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
fn float_div_numeric_contracts_lower_one_to_one_without_redefaulting() {
    let standard = lower_source("fn f(a: F32, b: F32) -> F32 { return a / b; }");
    assert_eq!(
        float_div_contract(float_div_statements(function(standard.as_program(), "f"))[0]),
        CoreNumericContract::Standard
    );

    let fast = lower_source("fn f(a: F32, b: F32) -> F32 { return @fast(a / b); }");
    assert_eq!(
        float_div_contract(float_div_statements(function(fast.as_program(), "f"))[0]),
        CoreNumericContract::Fast
    );

    let mut reproducible = hir("fn f(a: F32, b: F32) -> F32 { return a / b; }");
    let value = reproducible.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::FloatDiv { contract, .. } = &mut value.kind else {
        panic!("expected FloatDiv HIR value");
    };
    *contract = HirNumericContract::Reproducible;
    let reproducible = lower(&reproducible).expect("valid Reproducible FloatDiv HIR must lower");
    assert_eq!(
        float_div_contract(float_div_statements(function(reproducible.as_program(), "f"))[0]),
        CoreNumericContract::Reproducible
    );
}

#[test]
fn nested_float_div_dataflow_preserves_operation_identity_and_occurrence_contracts() {
    let lowered =
        lower_source("fn f(a: F32, b: F32, c: F32) -> F32 { return @fast(a / @fast(b / c)); }");
    let f = function(lowered.as_program(), "f");
    let divisions = float_div_statements(f);
    assert_eq!(divisions.len(), 2);

    let [inner, outer] = divisions.as_slice() else {
        unreachable!();
    };
    assert_eq!(float_div_contract(inner), CoreNumericContract::Fast);
    assert_eq!(float_div_contract(outer), CoreNumericContract::Fast);
    let CoreStatement::FloatDiv { dst: inner_dst, .. } = inner else {
        unreachable!();
    };
    let CoreStatement::FloatDiv {
        dst: outer_dst,
        left: outer_left,
        right: outer_right,
        ..
    } = outer
    else {
        unreachable!();
    };
    assert_eq!(moved_local(outer_right), Some(inner_dst.local));
    assert_ne!(moved_local(outer_left), Some(inner_dst.local));
    assert_ne!(outer_dst.local, inner_dst.local);
}

#[test]
fn float_div_to_float_add_dataflow_preserves_consumed_result_and_contracts() {
    let lowered =
        lower_source("fn f(a: F32, b: F32, c: F32) -> F32 { return @fast(@fast(a / b) + c); }");
    let f = function(lowered.as_program(), "f");
    let statements = f
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .collect::<Vec<_>>();
    let division = statements
        .iter()
        .copied()
        .find(|statement| matches!(statement, CoreStatement::FloatDiv { .. }))
        .expect("nested FloatDiv");
    let addition = statements
        .iter()
        .copied()
        .find(|statement| matches!(statement, CoreStatement::FloatAdd { .. }))
        .expect("outer FloatAdd");

    assert_eq!(float_div_contract(division), CoreNumericContract::Fast);
    let CoreStatement::FloatDiv { dst: div_dst, .. } = division else {
        unreachable!();
    };
    let CoreStatement::FloatAdd {
        contract,
        left,
        dst: add_dst,
        ..
    } = addition
    else {
        unreachable!();
    };
    assert_eq!(*contract, CoreNumericContract::Fast);
    assert_eq!(moved_local(left), Some(div_dst.local));
    assert_ne!(add_dst.local, div_dst.local);
}

#[test]
fn call_operands_lower_complete_left_then_right_before_float_div() {
    let lowered = lower_source(
        "fn left() -> F32 { return 3.0; } \
         fn right() -> F32 { return 2.0; } \
         fn f() -> F32 { return @fast(left() / right()); }",
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

    let Terminator::Call {
        destination: Some(right_destination),
        target: div_block,
        ..
    } = &f.body.blocks[right_call_block.0 as usize].terminator
    else {
        panic!("right producer must lower from the successful left continuation");
    };

    let division = f.body.blocks[div_block.0 as usize]
        .statements
        .iter()
        .find(|statement| matches!(statement, CoreStatement::FloatDiv { .. }))
        .expect("FloatDiv after both calls");
    assert_eq!(float_div_contract(division), CoreNumericContract::Fast);
    let CoreStatement::FloatDiv {
        dst, left, right, ..
    } = division
    else {
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
        "FloatDiv itself adds no CFG"
    );
}
