use runen_core_ir::{
    FunctionId, LocalId, Operand, PlaceAccess, Statement as CoreStatement, Terminator,
    ValidatedProgram,
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

fn integer_xor_statements(function: &runen_core_ir::Function) -> Vec<&CoreStatement> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::IntegerXor { .. }))
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
fn integer_xor_lowers_to_one_fresh_result_with_move_operands_and_no_rewrite_or_cfg() {
    let lowered = lower_source("fn f(left: I8, right: I8) -> I8 { return left ^ right; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.blocks.len(), 1, "plain XOR adds no Core block");
    let xors = integer_xor_statements(f);
    assert_eq!(xors.len(), 1);
    let CoreStatement::IntegerXor { dst, left, right } = xors[0] else {
        unreachable!();
    };
    assert!(dst.projections.is_empty());
    let left_local = moved_local(left).expect("IntegerXor left operand must Move a temporary");
    let right_local = moved_local(right).expect("IntegerXor right operand must Move a temporary");
    assert_ne!(left_local, right_local);
    assert_ne!(dst.local, left_local);
    assert_ne!(dst.local, right_local);
    assert!(left_local.0 < right_local.0);
    assert!(right_local.0 < dst.local.0);
    assert_eq!(
        f.body.locals[left_local.0 as usize].ty,
        f.body.locals[dst.local.0 as usize].ty
    );
    assert_eq!(
        f.body.locals[right_local.0 as usize].ty,
        f.body.locals[dst.local.0 as usize].ty
    );

    let arithmetic_rewrites = f.body.blocks[0]
        .statements
        .iter()
        .filter(|statement| {
            matches!(
                statement,
                CoreStatement::IntegerAdd { .. }
                    | CoreStatement::IntegerSub { .. }
                    | CoreStatement::IntegerMul { .. }
            )
        })
        .count();
    assert_eq!(arithmetic_rewrites, 0);
    assert!(!matches!(
        f.body.blocks[0].terminator,
        Terminator::Branch { .. }
    ));

    let Terminator::Return(Some(returned)) = &f.body.blocks[0].terminator else {
        panic!("XOR result must feed the function return");
    };
    assert_eq!(moved_local(returned), Some(dst.local));
}

#[test]
fn call_operands_lower_left_then_right_before_integer_xor_result_allocation() {
    let lowered = lower_source(
        "fn left() -> I8 { return 7; } \
         fn right() -> I8 { return 6; } \
         fn f() -> I8 { return left() ^ right(); }",
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
        target: xor_block,
        ..
    } = &f.body.blocks[right_call_block.0 as usize].terminator
    else {
        panic!("right producer must lower after the left continuation");
    };

    let xors = f.body.blocks[xor_block.0 as usize]
        .statements
        .iter()
        .filter(|statement| matches!(statement, CoreStatement::IntegerXor { .. }))
        .collect::<Vec<_>>();
    assert_eq!(xors.len(), 1);
    let CoreStatement::IntegerXor { dst, left, right } = xors[0] else {
        unreachable!();
    };
    assert_eq!(moved_local(left), Some(left_destination.local));
    assert_eq!(moved_local(right), Some(right_destination.local));
    assert!(left_destination.local.0 < right_destination.local.0);
    assert!(right_destination.local.0 < dst.local.0);
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        0,
        "integer XOR adds no branch or join"
    );
}

#[test]
fn grouped_nested_xor_emits_one_core_xor_per_source_xor_in_dependency_order() {
    let lowered = lower_source(
        "fn left(a: I8, b: I8, c: I8) -> I8 { return (a ^ b) ^ c; } \
         fn right(a: I8, b: I8, c: I8) -> I8 { return a ^ (b ^ c); }",
    );

    for name in ["left", "right"] {
        let function = function(lowered.as_program(), name);
        let xors = integer_xor_statements(function);
        assert_eq!(xors.len(), 2, "{name}");
        assert_eq!(function.body.blocks.len(), 1, "{name}");

        let CoreStatement::IntegerXor { dst: first_dst, .. } = xors[0] else {
            unreachable!();
        };
        let CoreStatement::IntegerXor {
            dst: second_dst,
            left,
            right,
        } = xors[1]
        else {
            unreachable!();
        };
        assert!(first_dst.local.0 < second_dst.local.0);
        assert!(
            moved_local(left) == Some(first_dst.local)
                || moved_local(right) == Some(first_dst.local),
            "outer XOR must consume the nested XOR result"
        );
    }
}

#[test]
fn lowering_rejects_malformed_integer_xor_retained_type_facts() {
    let mut non_integer = hir("fn f(left: I8, right: I8) -> I8 { return left ^ right; }");
    let value = non_integer.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    value.ty = Type::Intrinsic(IntrinsicType::Bool);
    assert_eq!(
        lower(&non_integer),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-XOR result type is not a fixed-width integer"
        ))
    );

    let mut left_mismatch = hir("fn f(left: I8, right: I8) -> I8 { return left ^ right; }");
    let value = left_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::IntegerXor { left, .. } = &mut value.kind else {
        panic!("expected integer-XOR HIR value");
    };
    left.ty = Type::Intrinsic(IntrinsicType::I16);
    assert_eq!(
        lower(&left_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-XOR left operand type does not match result type"
        ))
    );

    let mut right_mismatch = hir("fn f(left: I8, right: I8) -> I8 { return left ^ right; }");
    let value = right_mismatch.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::IntegerXor { right, .. } = &mut value.kind else {
        panic!("expected integer-XOR HIR value");
    };
    right.ty = Type::Intrinsic(IntrinsicType::I16);
    assert_eq!(
        lower(&right_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-XOR right operand type does not match result type"
        ))
    );
}

#[test]
fn source_to_hir_to_core_to_reference_proves_signed_unsigned_and_precedence_xor() {
    for (source, expected) in [
        ("fn f() -> I8 { return -5 ^ 3; }", ObservedValue::I8(-8)),
        (
            "fn f() -> I8 { return 127 ^ -1; }",
            ObservedValue::I8(i8::MIN),
        ),
        (
            "fn f() -> I8 { return -128 ^ -1; }",
            ObservedValue::I8(i8::MAX),
        ),
        ("fn f() -> U8 { return 255 ^ 15; }", ObservedValue::U8(240)),
        ("fn f() -> I8 { return 1 + 2 ^ 7; }", ObservedValue::I8(4)),
        ("fn f() -> I8 { return 7 ^ 1 + 2; }", ObservedValue::I8(4)),
        ("fn f() -> I8 { return (1 ^ 2) ^ 7; }", ObservedValue::I8(4)),
    ] {
        let report = execute_source(source, "f");
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
    }
}
