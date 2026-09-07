use runen_core_ir::{
    FunctionId as CoreFunctionId, LocalId, Operand, PlaceAccess, Statement as CoreStatement,
    Terminator, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn hir(source: &str) -> runen_hir::TypedCompilation {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
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

fn integer_lt_statements(function: &runen_core_ir::Function) -> Vec<&CoreStatement> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::IntegerLt { .. }))
        .collect()
}

fn integer_eq_count(function: &runen_core_ir::Function) -> usize {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::IntegerEq { .. }))
        .count()
}

fn returned_value_mut<'a>(
    compilation: &'a mut runen_hir::TypedCompilation,
    name: &str,
) -> &'a mut runen_hir::Value {
    compilation
        .functions
        .iter_mut()
        .find(|function| function.name == name)
        .and_then(|function| function.body.terminal_return.as_mut())
        .and_then(|returned| returned.value.as_mut())
        .unwrap_or_else(|| panic!("missing HIR return value for {name}"))
}

#[test]
fn integer_strict_order_lowers_to_exactly_one_typed_integer_lt_for_every_width() {
    for source_type in ["I8", "I16", "I32", "I64", "U8", "U16", "U32", "U64"] {
        let source = format!(
            "fn f(left: {source_type}, right: {source_type}) -> Bool {{ return left < right; }}"
        );
        let lowered = lower_source(&source);
        let f = function(lowered.as_program(), "f");
        let statements = integer_lt_statements(f);
        assert_eq!(statements.len(), 1, "{source_type} must emit one IntegerLt");
        assert_eq!(integer_eq_count(f), 0, "strict order must not refine through equality");
        let CoreStatement::IntegerLt {
            dst,
            operand_type,
            left,
            right,
        } = statements[0]
        else {
            unreachable!();
        };

        let expected_operand_type = f.body.locals[f.parameters[0].0 as usize].ty;
        assert_eq!(*operand_type, expected_operand_type);
        assert_eq!(
            f.body.locals[f.parameters[1].0 as usize].ty,
            expected_operand_type
        );
        assert_eq!(
            f.body.locals[dst.local.0 as usize].ty,
            f.result.expect("Bool result type")
        );
        assert!(moved_local(left).is_some());
        assert!(moved_local(right).is_some());
        assert_eq!(
            f.body
                .blocks
                .iter()
                .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
                .count(),
            0,
            "plain integer < needs no synthetic control-flow refinement"
        );
    }
}

#[test]
fn call_backed_ordering_lowers_left_then_right_once_before_integer_lt() {
    let lowered = lower_source(
        "fn left() -> I32 { return 1; } \
         fn right() -> I32 { return 2; } \
         fn f() -> Bool { return left() < right(); }",
    );
    let f = function(lowered.as_program(), "f");

    let Terminator::Call {
        function: first,
        target: after_left,
        ..
    } = f.body.blocks[0].terminator
    else {
        panic!("left operand call must execute first");
    };
    assert_eq!(first, CoreFunctionId(0));

    let Terminator::Call {
        function: second,
        target: after_right,
        ..
    } = f.body.blocks[after_left.0 as usize].terminator
    else {
        panic!("right operand call must follow the successful left call");
    };
    assert_eq!(second, CoreFunctionId(1));
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Call { .. }))
            .count(),
        2,
        "integer < must not re-lower either call operand"
    );
    assert_eq!(integer_lt_statements(f).len(), 1);
    assert!(
        f.body.blocks[after_right.0 as usize]
            .statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::IntegerLt { .. }))
    );
}

#[test]
fn if_and_while_branch_on_materialized_integer_lt_results_without_fusion() {
    let lowered = lower_source(
        "fn f(left: I32, right: I32) { \
             if left < right {} \
             while left < right { break; } \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let integer_lts = integer_lt_statements(f);
    assert_eq!(integer_lts.len(), 2);
    assert_eq!(integer_eq_count(f), 0);
    let result_locals = integer_lts
        .iter()
        .map(|statement| {
            let CoreStatement::IntegerLt { dst, .. } = statement else {
                unreachable!();
            };
            dst.local
        })
        .collect::<Vec<_>>();

    for result in result_locals {
        assert!(f.body.blocks.iter().any(|block| matches!(
            &block.terminator,
            Terminator::Branch { condition, .. } if moved_local(condition) == Some(result)
        )));
    }
}

#[test]
fn lowering_rejects_non_bool_integer_ordering_outer_type() {
    let mut compilation = hir("fn f(left: I32, right: I32) -> Bool { return left < right; }");
    returned_value_mut(&mut compilation, "f").ty = Type::Intrinsic(IntrinsicType::I64);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-ordering result type is not Bool"
        ))
    );
}

#[test]
fn lowering_rejects_unsupported_retained_integer_ordering_operand_type() {
    let mut compilation = hir("fn f(left: I32, right: I32) -> Bool { return left < right; }");
    let value = returned_value_mut(&mut compilation, "f");
    let ValueKind::IntegerLt { operand_type, .. } = &mut value.kind else {
        panic!("expected integer strict-order HIR");
    };
    *operand_type = Type::Intrinsic(IntrinsicType::F32);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-ordering operand type is not a fixed-width integer"
        ))
    );
}

#[test]
fn lowering_rejects_ordering_operand_type_disagreement_without_repair() {
    let mut compilation = hir("fn f(left: I32, right: I32) -> Bool { return left < right; }");
    let value = returned_value_mut(&mut compilation, "f");
    let ValueKind::IntegerLt {
        operand_type, right, ..
    } = &mut value.kind
    else {
        panic!("expected integer strict-order HIR");
    };
    assert_eq!(*operand_type, Type::Intrinsic(IntrinsicType::I32));
    right.ty = Type::Intrinsic(IntrinsicType::I64);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-ordering right operand type does not match retained operand type"
        ))
    );
}
