use runen_core_ir::{
    FunctionId, LocalId, MirValidationErrorKind, Operand, PlaceAccess, Statement as CoreStatement,
    Terminator, ValidatedProgram, Value as CoreValue,
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
fn integer_complement_lowers_to_existing_integer_sub_with_same_type_minus_one_and_move() {
    let lowered = lower_source("fn f(value: I8) -> I8 { return ~value; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body.blocks.len(),
        1,
        "integer complement adds no Core CFG"
    );
    let subtractions = integer_sub_statements(f);
    assert_eq!(subtractions.len(), 1);
    let CoreStatement::IntegerSub { dst, left, right } = subtractions[0] else {
        unreachable!();
    };
    assert!(dst.projections.is_empty());
    assert_eq!(left, &Operand::Constant(CoreValue::I8(-1)));
    let operand_local =
        moved_local(right).expect("complement right operand must Move its temporary");
    assert_ne!(dst.local, operand_local);
    assert!(
        f.body.locals[operand_local.0 as usize]
            .name
            .starts_with("$tmp")
    );
    assert!(f.body.locals[dst.local.0 as usize].name.starts_with("$tmp"));
    assert!(operand_local.0 < dst.local.0);

    let Terminator::Return(Some(returned)) = &f.body.blocks[0].terminator else {
        panic!("integer-complement result must feed the function return");
    };
    assert_eq!(moved_local(returned), Some(dst.local));
}

#[test]
fn integer_complement_uses_exact_same_type_minus_one_value_for_all_eight_integer_types() {
    let lowered = lower_source(
        "fn i8_not(a: I8) -> I8 { return ~a; } \
         fn i16_not(a: I16) -> I16 { return ~a; } \
         fn i32_not(a: I32) -> I32 { return ~a; } \
         fn i64_not(a: I64) -> I64 { return ~a; } \
         fn u8_not(a: U8) -> U8 { return ~a; } \
         fn u16_not(a: U16) -> U16 { return ~a; } \
         fn u32_not(a: U32) -> U32 { return ~a; } \
         fn u64_not(a: U64) -> U64 { return ~a; }",
    );

    for (name, expected) in [
        ("i8_not", CoreValue::I8(-1)),
        ("i16_not", CoreValue::I16(-1)),
        ("i32_not", CoreValue::I32(-1)),
        ("i64_not", CoreValue::I64(-1)),
        ("u8_not", CoreValue::U8(255)),
        ("u16_not", CoreValue::U16(65_535)),
        ("u32_not", CoreValue::U32(4_294_967_295)),
        ("u64_not", CoreValue::U64(18_446_744_073_709_551_615)),
    ] {
        let function = function(lowered.as_program(), name);
        let subtractions = integer_sub_statements(function);
        assert_eq!(subtractions.len(), 1, "{name}");
        let CoreStatement::IntegerSub { left, .. } = subtractions[0] else {
            unreachable!();
        };
        assert_eq!(left, &Operand::Constant(expected), "{name}");
    }
}

#[test]
fn producer_completes_before_integer_complement_subtraction_is_emitted() {
    let lowered = lower_source(
        "fn produce() -> I8 { return 7; } \
         fn f() -> I8 { return ~produce(); }",
    );
    let f = function(lowered.as_program(), "f");

    let Terminator::Call {
        destination: Some(operand_destination),
        target: continuation,
        ..
    } = &f.body.blocks[0].terminator
    else {
        panic!("operand producer must execute before complement refinement");
    };

    assert!(
        f.body.blocks[0]
            .statements
            .iter()
            .all(|statement| !matches!(statement, CoreStatement::IntegerSub { .. })),
        "complement subtraction cannot exist before the operand call continuation"
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
    assert_eq!(left, &Operand::Constant(CoreValue::I8(-1)));
    assert_eq!(moved_local(right), Some(operand_destination.local));
    assert!(
        f.body
            .blocks
            .iter()
            .all(|block| !matches!(block.terminator, Terminator::Branch { .. })),
        "integer complement adds no branch or join"
    );
}

#[test]
fn lowering_rejects_non_integer_retained_integer_complement_result_fact() {
    let mut compilation = hir("fn f(value: I8) -> I8 { return ~value; }");
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
            "Integer-complement result type is not a fixed-width integer"
        ))
    );
}

#[test]
fn lowering_rejects_integer_complement_operand_type_fact_that_differs_from_result() {
    let mut compilation = hir("fn f(value: I8) -> I8 { return ~value; }");
    let value = compilation.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::IntegerComplement { operand } = &mut value.kind else {
        panic!("expected integer-complement HIR value");
    };
    operand.ty = Type::Intrinsic(IntrinsicType::I16);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-complement operand type does not match result type"
        ))
    );
}

#[test]
fn lowering_rejects_malformed_operand_source_local_type_mismatch_without_conversion() {
    let mut compilation = hir("fn f(value: I8) -> I8 { return ~value; }");
    compilation.functions[0].parameters[0].ty = Type::Intrinsic(IntrinsicType::I16);

    let Err(LoweringError::CoreValidation(error)) = lower(&compilation) else {
        panic!("mismatched operand source local must be rejected by Core validation");
    };
    assert!(matches!(
        error.kind,
        MirValidationErrorKind::TypeMismatch { .. }
    ));
}

#[test]
fn mixed_and_grouped_complement_lower_one_existing_subtraction_per_operation() {
    let lowered = lower_source(
        "fn nested() -> I8 { return ~~1; } \
         fn grouped() -> I8 { return ~(2 + 3); } \
         fn multiplied() -> I8 { return ~(2 + 3) * 4; } \
         fn additive() -> I8 { return 2 + ~(3 * 4); } \
         fn negated() -> I8 { return -~1; }",
    );

    assert_eq!(
        arithmetic_statement_count(function(lowered.as_program(), "nested")),
        2
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
    assert_eq!(
        arithmetic_statement_count(function(lowered.as_program(), "negated")),
        2
    );

    for name in ["nested", "grouped", "multiplied", "additive", "negated"] {
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
fn source_to_hir_to_core_to_reference_proves_total_plain_integer_complement() {
    for (source, expected) in [
        ("fn f() -> I8 { return ~0; }", ObservedValue::I8(-1)),
        ("fn f() -> I8 { return ~-1; }", ObservedValue::I8(0)),
        ("fn f() -> I8 { return ~127; }", ObservedValue::I8(-128)),
        ("fn f() -> I8 { return ~-128; }", ObservedValue::I8(127)),
        ("fn f() -> U8 { return ~0; }", ObservedValue::U8(255)),
        ("fn f() -> U8 { return ~255; }", ObservedValue::U8(0)),
        ("fn f() -> U8 { return ~1; }", ObservedValue::U8(254)),
        ("fn f() -> I8 { return ~~42; }", ObservedValue::I8(42)),
        ("fn f() -> U8 { return ~~200; }", ObservedValue::U8(200)),
        ("fn f() -> I8 { return ~(2 + 3) * 4; }", ObservedValue::I8(-24)),
        ("fn f() -> I8 { return 2 + ~(3 * 4); }", ObservedValue::I8(-11)),
        ("fn f() -> I8 { return -~1; }", ObservedValue::I8(2)),
    ] {
        let report = execute_source(source, "f");
        assert_eq!(report.terminal, TerminalStatus::Returned);
        assert_eq!(report.result, Some(expected), "{source}");
    }
}
