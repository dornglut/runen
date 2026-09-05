use runen_core_ir::{
    Function as CoreFunction, Operand, PlaceAccess, Projection, Statement as CoreStatement,
    Terminator, ValidatedProgram, Value as CoreValue,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    IntrinsicType, LiteralValue, ModuleId, RecordPatternTransientCleanup, SourceUnit, Statement, Type,
    build_typed_hir,
};
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
    lower(&hir(source)).expect("accepted HIR must lower through canonical Core validation")
}

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a CoreFunction {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn integer_eqs(function: &CoreFunction) -> Vec<(usize, &CoreStatement)> {
    function
        .body
        .blocks
        .iter()
        .enumerate()
        .flat_map(|(block, body)| {
            body.statements
                .iter()
                .filter(|statement| matches!(statement, CoreStatement::IntegerEq { .. }))
                .map(move |statement| (block, statement))
        })
        .collect()
}

fn selection_mut(
    compilation: &mut runen_hir::TypedCompilation,
    function_name: &str,
) -> &mut runen_hir::Statement {
    compilation
        .functions
        .iter_mut()
        .find(|function| function.name == function_name)
        .and_then(|function| function.body.statements.first_mut())
        .unwrap_or_else(|| panic!("missing selection statement for {function_name}"))
}

fn direct_projection(operand: &Operand) -> Option<Vec<Projection>> {
    let Operand::Copy(PlaceAccess::Direct(place)) = operand else {
        return None;
    };
    Some(place.projections.clone())
}

#[test]
fn integer_tests_short_circuit_in_source_test_order_before_interleaved_binding_transfer() {
    let lowered = lower_source(
        "record R { first: I8, second: I8, kept: U8 } \
         fn sink(value: U8) {} \
         fn f(root: R) { \
             if let R { first: 1, kept: kept, second: 2 } = (root) { sink(kept); } else {} \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let eqs = integer_eqs(f);
    assert_eq!(eqs.len(), 2, "each retained integer test must emit one IntegerEq");

    let (first_block, first_eq) = eqs[0];
    let (second_block, second_eq) = eqs[1];
    let CoreStatement::IntegerEq {
        operand_type: first_type,
        left: first_left,
        right: first_right,
        ..
    } = first_eq
    else {
        unreachable!();
    };
    let CoreStatement::IntegerEq {
        operand_type: second_type,
        left: second_left,
        right: second_right,
        ..
    } = second_eq
    else {
        unreachable!();
    };
    assert_eq!(first_type, second_type);
    assert_eq!(direct_projection(first_left), Some(vec![Projection::Field(0)]));
    assert_eq!(direct_projection(second_left), Some(vec![Projection::Field(1)]));
    assert_eq!(first_right, &Operand::Constant(CoreValue::I8(1)));
    assert_eq!(second_right, &Operand::Constant(CoreValue::I8(2)));

    let Terminator::Branch {
        true_target: after_first,
        false_target: first_mismatch,
        ..
    } = f.body.blocks[first_block].terminator
    else {
        panic!("first integer test must branch");
    };
    assert_eq!(after_first.0 as usize, second_block);

    let Terminator::Branch {
        true_target: after_second,
        false_target: second_mismatch,
        ..
    } = f.body.blocks[second_block].terminator
    else {
        panic!("second integer test must branch");
    };
    assert_eq!(first_mismatch, second_mismatch, "first mismatch must skip every later test");

    let kept_local = f
        .body
        .locals
        .iter()
        .position(|local| local.name == "kept")
        .expect("success binding local");
    assert!(
        f.body.blocks[after_second.0 as usize]
            .statements
            .iter()
            .any(|statement| matches!(
                statement,
                CoreStatement::Init { dst, .. } if dst.local.0 as usize == kept_local
            )),
        "binding transfer must occur only after all tests succeed"
    );
    assert!(
        !f.body.blocks[first_block]
            .statements
            .iter()
            .chain(f.body.blocks[second_block].statements.iter())
            .any(|statement| matches!(
                statement,
                CoreStatement::Init { dst, .. } if dst.local.0 as usize == kept_local
            )),
        "source-interleaved binding must not move ahead of retained tests"
    );
}

#[test]
fn boolean_literal_test_uses_branch_refinement_without_integer_eq() {
    let lowered = lower_source(
        "record R { flag: Bool, kept: U8 } \
         fn sink(value: U8) {} \
         fn f(root: R) { \
             if let R { flag: false, kept: kept } = (root) { sink(kept); } else {} \
         }",
    );
    let f = function(lowered.as_program(), "f");
    assert!(integer_eqs(f).is_empty());
    assert!(
        f.body
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Branch { .. })),
        "Bool literal testing must use existing Bool control-flow refinement"
    );
}

#[test]
fn producer_call_is_evaluated_once_and_selects_distinct_success_and_mismatch_cleanup() {
    let lowered = lower_source(
        "record Ticket {} record R { tag: I8, ticket: Ticket, tail: U8 } \
         fn make() -> R { return R { tag: 1, ticket: Ticket {}, tail: 2 }; } \
         fn f() { \
             if let R { tag: 1, ticket: moved, tail: copied } = (make()) { fault; } else { fault; } \
         }",
    );
    let f = function(lowered.as_program(), "f");

    let calls = f
        .body
        .blocks
        .iter()
        .filter_map(|block| match &block.terminator {
            Terminator::Call {
                destination: Some(destination),
                ..
            } => Some(destination.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 1, "producer-backed scrutinee must be evaluated exactly once");
    let source = calls[0].local;

    let eqs = integer_eqs(f);
    assert_eq!(eqs.len(), 1);
    let (test_block, _) = eqs[0];
    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[test_block].terminator
    else {
        panic!("retained integer test must branch to success or mismatch");
    };

    let mismatch_drops = f.body.blocks[false_target.0 as usize]
        .statements
        .iter()
        .filter_map(|statement| match statement {
            CoreStatement::Drop {
                place: PlaceAccess::Direct(place),
            } if place.local == source => Some(place.projections.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        mismatch_drops,
        [Vec::<Projection>::new()],
        "mismatch must clean the complete pattern transient exactly once"
    );

    let success_drops = f.body.blocks[true_target.0 as usize]
        .statements
        .iter()
        .filter_map(|statement| match statement {
            CoreStatement::Drop {
                place: PlaceAccess::Direct(place),
            } if place.local == source => Some(place.projections.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        success_drops,
        [vec![Projection::Field(2)], vec![Projection::Field(0)]],
        "success must clean only the retained post-binding frontier"
    );
}

#[test]
fn lowering_rejects_literal_value_that_disagrees_with_retained_test_type() {
    let mut compilation = hir(
        "record R { value: I8 } fn f(root: R) { \
             if let R { value: 1 } = (root) {} else {} \
         }",
    );
    let Statement::RefutableRecordSelection { tests, .. } = selection_mut(&mut compilation, "f")
    else {
        panic!("expected refutable record selection");
    };
    tests[0].value = LiteralValue::Bool(true);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "refutable record literal test value does not match retained test type"
        ))
    );
}

#[test]
fn lowering_rejects_test_path_type_disagreement_without_reconstructing_source_semantics() {
    let mut compilation = hir(
        "record R { first: I8, second: U8 } fn f(root: R) { \
             if let R { first: 1, second: kept } = (root) {} else {} \
         }",
    );
    let Statement::RefutableRecordSelection { tests, .. } = selection_mut(&mut compilation, "f")
    else {
        panic!("expected refutable record selection");
    };
    tests[0].ty = Type::Intrinsic(IntrinsicType::U8);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "refutable record literal test type does not match projected field type"
        ))
    );
}

#[test]
fn lowering_rejects_direct_root_with_impossible_pattern_transient_mismatch_cleanup() {
    let mut compilation = hir(
        "record R { value: I8 } fn f(root: R) { \
             if let R { value: 1 } = (root) {} else {} \
         }",
    );
    let Statement::RefutableRecordSelection {
        mismatch_cleanup, ..
    } = selection_mut(&mut compilation, "f")
    else {
        panic!("expected refutable record selection");
    };
    *mismatch_cleanup = Some(RecordPatternTransientCleanup {
        paths: vec![Vec::new()],
    });

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "direct-root refutable selection retains producer mismatch cleanup"
        ))
    );
}
