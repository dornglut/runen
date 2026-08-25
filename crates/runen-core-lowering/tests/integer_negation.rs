use runen_core_ir::{
    FunctionId, LocalId, Operand, PlaceAccess, Statement as CoreStatement, Terminator,
    ValidatedProgram, Value as CoreValue,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    DiagnosticKind, IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir,
};
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

fn integer_sub_statements(function: &runen_core_ir::Function) -> Vec<&CoreStatement> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::IntegerSub { .. }))
        .collect()
}

fn arithmetic_statement_count(function: &runen_core_ir::Function) -> usize {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| {
            matches!(
                statement,
                CoreStatement::IntegerAdd { .. }
                    | CoreStatement::IntegerSub { .. }
                    | CoreStatement::IntegerMul { .. }
            )
        })
        .count()
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
fn integer_negation_lowers_to_existing_integer_sub_with_zero_and_move() {
    let lowered = lower_source("fn f(value: I8) -> I8 { return -value; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.blocks.len(), 1, "integer negation adds no Core CFG");
    let subtractions = integer_sub_statements(f);
    assert_eq!(subtractions.len(), 1);
    let CoreStatement::IntegerSub { dst, left, right } = subtractions[0] else {
        unreachable!();
    };
    assert!(dst.projections.is_empty());
    assert_eq!(left, &Operand::Constant(CoreValue::I8(0)));
    let operand_local = moved_local(right).expect("negation right operand must Move its temporary");
    assert_ne!(dst.local, operand_local);
    assert!(
        f.body.locals[operand_local.0 as usize]
            .name
            .starts_with("$tmp")
    );
    assert!(f.body.locals[dst.local.0 as usize].name.starts_with("$tmp"));
    assert!(operand_local.0 < dst.local.0);

    let Terminator::Return(Some(returned)) = &f.body.blocks[0].terminator else {
        panic!("integer-negation result must feed the function return");
    };
    assert_eq!(moved_local(returned), Some(dst.local));
}

#[test]
fn integer_negation_uses_exact_same_kind_zero_for_all_eight_integer_types() {
    let lowered = lower_source(
        "fn i8_neg(a: I8) -> I8 { return -a; } \
         fn i16_neg(a: I16) -> I16 { return -a; } \
         fn i32_neg(a: I32) -> I32 { return -a; } \
         fn i64_neg(a: I64) -> I64 { return -a; } \
         fn u8_neg(a: U8) -> U8 { return -a; } \
         fn u16_neg(a: U16) -> U16 { return -a; } \
         fn u32_neg(a: U32) -> U32 { return -a; } \
         fn u64_neg(a: U64) -> U64 { return -a; }",
    );

    for (name, expected_zero) in [
        ("i8_neg", CoreValue::I8(0)),
        ("i16_neg", CoreValue::I16(0)),
        ("i32_neg", CoreValue::I32(0)),
        ("i64_neg", CoreValue::I64(0)),
        ("u8_neg", CoreValue::U8(0)),
        ("u16_neg", CoreValue::U16(0)),
        ("u32_neg", CoreValue::U32(0)),
        ("u64_neg", CoreValue::U64(0)),
    ] {
        let function = function(lowered.as_program(), name);
        let subtractions = integer_sub_statements(function);
        assert_eq!(subtractions.len(), 1, "{name}");
        let CoreStatement::IntegerSub { left, .. } = subtractions[0] else {
            unreachable!();
        };
        assert_eq!(left, &Operand::Constant(expected_zero), "{name}");
    }
}

#[test]
fn producer_completes_before_integer_sub_refinement_is_emitted() {
    let lowered = lower_source(
        "fn produce() -> I8 { return 7; } \
         fn f() -> I8 { return -produce(); }",
    );
    let f = function(lowered.as_program(), "f");

    let Terminator::Call {
        destination: Some(operand_destination),
        target: continuation,
        ..
    } = &f.body.blocks[0].terminator
    else {
        panic!("operand producer must execute before negation refinement");
    };

    assert!(
        f.body.blocks[0]
            .statements
            .iter()
            .all(|statement| !matches!(statement, CoreStatement::IntegerSub { .. })),
        "negation subtraction cannot exist before the operand call continuation"
    );

    let subtractions = f.body.blocks[continuation.0 as usize]
        .statements
        .iter()
        .filter(|statement| matches!(statement, CoreStatement::IntegerSub { .. }))
        .collect::<Vec<_>>();
    assert_eq!(subtractions.len(), 1);
    let CoreStatement::IntegerSub { left, right, .. } = subtractions[0] else {
        unreachable!();
    };
    assert_eq!(left, &Operand::Constant(CoreValue::I8(0)));
    assert_eq!(moved_local(right), Some(operand_destination.local));
    assert!(
        f.body
            .blocks
            .iter()
            .all(|block| !matches!(block.terminator, Terminator::Branch { .. })),
        "integer negation adds no branch or join"
    );
}

#[test]
fn lowering_rejects_non_integer_retained_integer_neg_result_fact() {
    let mut compilation = hir("fn f(value: I8) -> I8 { return -value; }");
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
            "Integer-neg result type is not a fixed-width integer"
        ))
    );
}

#[test]
fn lowering_rejects_integer_neg_operand_type_fact_that_differs_from_result() {
    let mut compilation = hir("fn f(value: I8) -> I8 { return -value; }");
    let value = compilation.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::IntegerNeg { operand } = &mut value.kind else {
        panic!("expected integer-neg HIR value");
    };
    operand.ty = Type::Intrinsic(IntrinsicType::I16);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-neg operand type does not match result type"
        ))
    );
}

#[test]
fn mixed_and_grouped_negation_lower_one_existing_arithmetic_statement_per_operation() {
    let lowered = lower_source(
        "fn nested() -> I8 { return --1; } \
         fn grouped() -> I8 { return -(2 + 3); } \
         fn multiplied() -> I8 { return -(2 + 3) * 4; } \
         fn additive() -> I8 { return 2 + -(3 * 4); }",
    );

    assert_eq!(
        arithmetic_statement_count(function(lowered.as_program(), "nested")),
        1
    );
    assert_eq!(
        arithmetic_statement_count(function(lowered.as_program(), "grouped")),
        2
    );
    assert_eq!(
        arithmetic_statement_count(function(lowered.as_program(), "multiplied")),
        3
    );
    assert_eq!(
        arithmetic_statement_count(function(lowered.as_program(), "additive")),
        3
    );

    for name in ["nested", "grouped", "multiplied", "additive"] {
        assert!(
            function(lowered.as_program(), name)
                .body
                .blocks
                .iter()
                .all(|block| !matches!(block.terminator, Terminator::Branch { .. })),
            "{name}"
        );
    }
}

#[test]
fn source_to_hir_to_core_to_reference_proves_total_plain_integer_negation() {
    for (source, expected) in [
        ("fn f() -> I8 { return -(1); }", CoreValue::I8(-1)),
        ("fn f() -> I8 { return -(-128); }", CoreValue::I8(-128)),
        ("fn f() -> U8 { return -(1); }", CoreValue::U8(255)),
        ("fn f() -> U8 { return -(255); }", CoreValue::U8(1)),
        ("fn f() -> I8 { return --1; }", CoreValue::I8(1)),
        ("fn f() -> I8 { return -(2 + 3) * 4; }", CoreValue::I8(-20)),
        ("fn f() -> I8 { return 2 + -(3 * 4); }", CoreValue::I8(-10)),
    ] {
        let report = execute_source(source, "f");
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected));
    }
}

#[test]
fn unsigned_signed_literal_distinction_survives_before_lowering() {
    let parsed = parse("fn bad() -> U8 { return -1; }");
    let errors = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect_err("negative signed literal must remain range-invalid for U8");
    assert!(errors.iter().any(|error| {
        error.kind
            == DiagnosticKind::IntegerLiteralOutOfRange {
                required: Type::Intrinsic(IntrinsicType::U8),
            }
    }));

    let report = execute_source("fn good() -> U8 { return -(1); }", "good");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(CoreValue::U8(255)));
}
