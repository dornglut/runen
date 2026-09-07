use runen_core_ir::{
    Function as CoreFunction, Operand, PlaceAccess, Projection, Statement as CoreStatement,
    Terminator, TypeKind, ValidatedProgram, Value as CoreValue,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    IntrinsicType, LiteralValue, ModuleId, RecordPatternTestKind, RecordPatternTransientCleanup,
    SourceUnit, Statement, Type, build_typed_hir,
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

fn integer_lts(function: &CoreFunction) -> Vec<(usize, &CoreStatement)> {
    function
        .body
        .blocks
        .iter()
        .enumerate()
        .flat_map(|(block, body)| {
            body.statements
                .iter()
                .filter(|statement| matches!(statement, CoreStatement::IntegerLt { .. }))
                .map(move |statement| (block, statement))
        })
        .collect()
}

fn selection_mut<'a>(
    compilation: &'a mut runen_hir::TypedCompilation,
    function_name: &str,
) -> &'a mut Statement {
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
    assert_eq!(
        eqs.len(),
        2,
        "each retained integer test must emit one IntegerEq"
    );

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
    assert_eq!(
        direct_projection(first_left),
        Some(vec![Projection::Field(0)])
    );
    assert_eq!(
        direct_projection(second_left),
        Some(vec![Projection::Field(1)])
    );
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
    assert_eq!(
        first_mismatch, second_mismatch,
        "first mismatch must skip every later test"
    );

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
fn mixed_equality_and_strict_upper_bound_tests_preserve_source_order_and_short_circuit() {
    let lowered = lower_source(
        "record R { first: I8, second: U8, kept: U8 } \
         fn sink(value: U8) {} \
         fn f(root: R) { \
             if let R { first: 1, kept: kept, second: < 200 } = (root) { sink(kept); } else {} \
         }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    let eqs = integer_eqs(f);
    let lts = integer_lts(f);
    assert_eq!(eqs.len(), 1);
    assert_eq!(lts.len(), 1);

    let (eq_block, CoreStatement::IntegerEq { left: eq_left, right: eq_right, .. }) = eqs[0] else {
        unreachable!();
    };
    let (lt_block, CoreStatement::IntegerLt { operand_type, left: lt_left, right: lt_right, .. }) =
        lts[0]
    else {
        unreachable!();
    };
    assert_eq!(direct_projection(eq_left), Some(vec![Projection::Field(0)]));
    assert_eq!(eq_right, &Operand::Constant(CoreValue::I8(1)));
    assert_eq!(direct_projection(lt_left), Some(vec![Projection::Field(1)]));
    assert_eq!(lt_right, &Operand::Constant(CoreValue::U8(200)));

    let root_ty = f.body.locals[f.parameters[0].0 as usize].ty;
    let root_def = program.types.get(root_ty).expect("record parameter type");
    let TypeKind::Struct(fields) = &root_def.kind else {
        panic!("record parameter must lower to a Core struct");
    };
    assert_eq!(*operand_type, fields[1].ty);

    let Terminator::Branch {
        true_target: after_eq,
        false_target: eq_mismatch,
        ..
    } = f.body.blocks[eq_block].terminator
    else {
        panic!("equality test must branch");
    };
    assert_eq!(after_eq.0 as usize, lt_block);
    let Terminator::Branch {
        true_target: after_lt,
        false_target: lt_mismatch,
        ..
    } = f.body.blocks[lt_block].terminator
    else {
        panic!("strict-upper-bound test must branch");
    };
    assert_eq!(eq_mismatch, lt_mismatch, "first mismatch must skip the later strict-bound test");

    let kept_local = f
        .body
        .locals
        .iter()
        .position(|local| local.name == "kept")
        .expect("success binding local");
    assert!(f.body.blocks[after_lt.0 as usize].statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init { dst, .. } if dst.local.0 as usize == kept_local
    )));
    assert!(!f.body.blocks[eq_block]
        .statements
        .iter()
        .chain(f.body.blocks[lt_block].statements.iter())
        .any(|statement| matches!(
            statement,
            CoreStatement::Init { dst, .. } if dst.local.0 as usize == kept_local
        )));
}

#[test]
fn strict_upper_bounds_retain_exact_signed_and_unsigned_field_types() {
    let lowered = lower_source(
        "record R { signed: I8, unsigned: U8 } \
         fn f(root: R) { if let R { signed: < -1, unsigned: < 200 } = (root) {} }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    assert!(integer_eqs(f).is_empty());
    let lts = integer_lts(f);
    assert_eq!(lts.len(), 2);

    let root_ty = f.body.locals[f.parameters[0].0 as usize].ty;
    let root_def = program.types.get(root_ty).expect("record parameter type");
    let TypeKind::Struct(fields) = &root_def.kind else {
        panic!("record parameter must lower to a Core struct");
    };

    let (_, CoreStatement::IntegerLt {
        operand_type: signed_type,
        left: signed_left,
        right: signed_right,
        ..
    }) = lts[0]
    else {
        unreachable!();
    };
    let (_, CoreStatement::IntegerLt {
        operand_type: unsigned_type,
        left: unsigned_left,
        right: unsigned_right,
        ..
    }) = lts[1]
    else {
        unreachable!();
    };
    assert_eq!(*signed_type, fields[0].ty);
    assert_eq!(*unsigned_type, fields[1].ty);
    assert_eq!(direct_projection(signed_left), Some(vec![Projection::Field(0)]));
    assert_eq!(direct_projection(unsigned_left), Some(vec![Projection::Field(1)]));
    assert_eq!(signed_right, &Operand::Constant(CoreValue::I8(-1)));
    assert_eq!(unsigned_right, &Operand::Constant(CoreValue::U8(200)));
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
    assert_eq!(
        calls.len(),
        1,
        "producer-backed scrutinee must be evaluated exactly once"
    );
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
fn strict_upper_bound_producer_is_evaluated_once_with_unchanged_cleanup_topology() {
    let lowered = lower_source(
        "record Ticket {} record R { tag: I8, ticket: Ticket, tail: U8 } \
         fn make() -> R { return R { tag: 1, ticket: Ticket {}, tail: 2 }; } \
         fn f() { \
             if let R { tag: < 5, ticket: moved, tail: copied } = (make()) { fault; } else { fault; } \
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
    assert_eq!(calls.len(), 1);
    let source = calls[0].local;
    assert!(integer_eqs(f).is_empty());
    let lts = integer_lts(f);
    assert_eq!(lts.len(), 1);
    let (test_block, CoreStatement::IntegerLt { left, right, .. }) = lts[0] else {
        unreachable!();
    };
    assert_eq!(direct_projection(left), Some(vec![Projection::Field(0)]));
    assert_eq!(right, &Operand::Constant(CoreValue::I8(5)));

    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[test_block].terminator
    else {
        panic!("strict-upper-bound test must branch to success or mismatch");
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
    assert_eq!(mismatch_drops, [Vec::<Projection>::new()]);
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
    assert_eq!(success_drops, [vec![Projection::Field(2)], vec![Projection::Field(0)]]);
}

#[test]
fn record_construction_scrutinee_is_materialized_once_before_testing() {
    let lowered = lower_source(
        "record Ticket {} record R { tag: I8, ticket: Ticket, tail: U8 } \
         fn f() { \
             if let R { tag: 1, ticket: moved, tail: copied } = \
                 (R { tag: 1, ticket: Ticket {}, tail: 2 }) { fault; } else { fault; } \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let eqs = integer_eqs(f);
    assert_eq!(eqs.len(), 1);
    let (test_block, CoreStatement::IntegerEq { left, .. }) = eqs[0] else {
        unreachable!();
    };
    let Operand::Copy(PlaceAccess::Direct(tested)) = left else {
        panic!("integer test must copy a projected construction result");
    };
    assert_eq!(tested.projections, [Projection::Field(0)]);
    let source = tested.local;

    let construction_writes = f
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| {
            matches!(
                statement,
                CoreStatement::Init { dst, .. } if dst.local == source
            )
        })
        .count();
    assert_eq!(
        construction_writes, 3,
        "bounded record-construction scrutinee must be materialized exactly once"
    );
    assert!(
        f.body.blocks[test_block]
            .statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::IntegerEq { .. })),
        "testing must consume the single materialized construction result"
    );
}

#[test]
fn field_value_receiver_cleanup_precedes_the_distinct_pattern_transient() {
    let lowered = lower_source(
        "record Ticket {} \
         record R { tag: I8, ticket: Ticket } \
         record Outer { inner: R, tail: U8 } \
         fn make() -> Outer { \
             return Outer { inner: R { tag: 1, ticket: Ticket {} }, tail: 9 }; \
         } \
         fn f() { \
             if let R { tag: 1, ticket: moved } = (make().inner) { fault; } else { fault; } \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let call_destination = f
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Terminator::Call {
                destination: Some(destination),
                ..
            } => Some(destination.local),
            _ => None,
        })
        .expect("field-value producer must retain one call receiver temporary");

    let eqs = integer_eqs(f);
    assert_eq!(eqs.len(), 1);
    let (test_block, CoreStatement::IntegerEq { left, .. }) = eqs[0] else {
        unreachable!();
    };
    let Operand::Copy(PlaceAccess::Direct(tested)) = left else {
        panic!("integer test must copy the selected R transient");
    };
    assert_ne!(tested.local, call_destination);

    let block = &f.body.blocks[test_block];
    let receiver_drop = block
        .statements
        .iter()
        .position(|statement| {
            matches!(
                statement,
                CoreStatement::Drop {
                    place: PlaceAccess::Direct(place),
                } if place.local == call_destination
                    && place.projections == [Projection::Field(1)]
            )
        })
        .expect("consumed field receiver must clean its remaining Outer frontier");
    let test = block
        .statements
        .iter()
        .position(|statement| matches!(statement, CoreStatement::IntegerEq { .. }))
        .expect("pattern test must be in the post-receiver-cleanup block");
    assert!(
        receiver_drop < test,
        "field-value receiver cleanup must complete before the pattern test/transient relation"
    );

    let Terminator::Branch { false_target, .. } = block.terminator else {
        panic!("integer pattern test must branch");
    };
    assert!(
        f.body.blocks[false_target.0 as usize]
            .statements
            .iter()
            .any(|statement| matches!(
                statement,
                CoreStatement::Drop {
                    place: PlaceAccess::Direct(place),
                } if place.local == tested.local && place.projections.is_empty()
            )),
        "pattern mismatch cleanup must be distinct from the already-completed receiver cleanup"
    );
}

#[test]
fn lowering_rejects_literal_value_that_disagrees_with_retained_test_type() {
    let mut compilation = hir("record R { value: I8 } fn f(root: R) { \
             if let R { value: 1 } = (root) {} else {} \
         }");
    let Statement::RefutableRecordSelection { tests, .. } = selection_mut(&mut compilation, "f")
    else {
        panic!("expected refutable record selection");
    };
    tests[0].value = LiteralValue::Bool(true);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "refutable record test value does not match retained test type"
        ))
    );
}

#[test]
fn lowering_rejects_test_path_type_disagreement_without_reconstructing_source_semantics() {
    let mut compilation = hir("record R { first: I8, second: U8 } fn f(root: R) { \
             if let R { first: 1, second: kept } = (root) {} else {} \
         }");
    let Statement::RefutableRecordSelection { tests, .. } = selection_mut(&mut compilation, "f")
    else {
        panic!("expected refutable record selection");
    };
    tests[0].ty = Type::Intrinsic(IntrinsicType::U8);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "refutable record test type does not match projected field type"
        ))
    );
}

#[test]
fn lowering_rejects_strict_upper_bound_kind_with_bool_retained_type() {
    let mut compilation = hir(
        "record R { flag: Bool } fn f(root: R) { if let R { flag: true } = (root) {} else {} }",
    );
    let Statement::RefutableRecordSelection { tests, .. } = selection_mut(&mut compilation, "f")
    else {
        panic!("expected refutable record selection");
    };
    tests[0].kind = RecordPatternTestKind::StrictUpperBound;

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "refutable record strict-upper-bound test type is not a fixed-width integer"
        ))
    );
}

#[test]
fn lowering_rejects_strict_upper_bound_value_that_disagrees_with_retained_type() {
    let mut compilation = hir(
        "record R { value: I8 } fn f(root: R) { if let R { value: < 3 } = (root) {} else {} }",
    );
    let Statement::RefutableRecordSelection { tests, .. } = selection_mut(&mut compilation, "f")
    else {
        panic!("expected refutable record selection");
    };
    tests[0].value = LiteralValue::Bool(true);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "refutable record test value does not match retained test type"
        ))
    );
}

#[test]
fn lowering_rejects_direct_root_with_impossible_pattern_transient_mismatch_cleanup() {
    let mut compilation = hir("record R { value: I8 } fn f(root: R) { \
             if let R { value: 1 } = (root) {} else {} \
         }");
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

#[test]
fn pattern_activity_is_reachable_only_after_successful_producer_call_continuation() {
    let lowered = lower_source(
        "record Ticket {} record R { tag: I8, ticket: Ticket, tail: U8 } \
         fn make() -> R { return R { tag: 1, ticket: Ticket {}, tail: 2 }; } \
         fn f() { \
             if let R { tag: 1, ticket: moved, tail: copied } = (make()) { fault; } else { fault; } \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let moved_local = f
        .body
        .locals
        .iter()
        .position(|local| local.name == "moved")
        .expect("success binding local");
    let copied_local = f
        .body
        .locals
        .iter()
        .position(|local| local.name == "copied")
        .expect("success binding local");

    let (producer_block, source, continuation) = f
        .body
        .blocks
        .iter()
        .enumerate()
        .find_map(|(block, body)| match &body.terminator {
            Terminator::Call {
                destination: Some(destination),
                target,
                ..
            } => Some((block, destination.local, *target)),
            _ => None,
        })
        .expect("producer-backed scrutinee must lower through one result-bearing call");
    let eqs = integer_eqs(f);
    assert_eq!(eqs.len(), 1);
    assert_eq!(
        continuation.0 as usize, eqs[0].0,
        "the first pattern test must begin only on the producer call's successful continuation"
    );

    assert!(
        !f.body.blocks[producer_block]
            .statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::IntegerEq { .. })),
        "producer evaluation must not perform pattern tests before the call returns"
    );
    assert!(
        !f.body.blocks[producer_block]
            .statements
            .iter()
            .any(|statement| matches!(
                statement,
                CoreStatement::Init { dst, .. }
                    if dst.local.0 as usize == moved_local || dst.local.0 as usize == copied_local
            )),
        "producer evaluation must not establish success bindings before the call returns"
    );
    assert!(
        !f.body.blocks[producer_block]
            .statements
            .iter()
            .any(|statement| matches!(
                statement,
                CoreStatement::Drop {
                    place: PlaceAccess::Direct(place),
                } if place.local == source
            )),
        "producer evaluation must not perform pattern-transient cleanup before the call returns"
    );
}

#[test]
fn success_body_starts_only_after_all_bindings_and_producer_cleanup() {
    let lowered = lower_source(
        "record Ticket {} record R { tag: I8, ticket: Ticket, count: U8 } \
         fn sink_ticket(value: Ticket) {} fn sink_count(value: U8) {} \
         fn make() -> R { return R { tag: 1, ticket: Ticket {}, count: 2 }; } \
         fn f() { \
             if let R { tag: 1, ticket: moved, count: copied } = (make()) { \
                 sink_ticket(moved); sink_count(copied); \
             } else { fault; } \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let moved_local = f
        .body
        .locals
        .iter()
        .position(|local| local.name == "moved")
        .expect("non-duplicable success binding local");
    let copied_local = f
        .body
        .locals
        .iter()
        .position(|local| local.name == "copied")
        .expect("duplicable success binding local");

    let eqs = integer_eqs(f);
    assert_eq!(eqs.len(), 1);
    let (test_block, CoreStatement::IntegerEq { left, .. }) = eqs[0] else {
        unreachable!();
    };
    let Operand::Copy(PlaceAccess::Direct(tested)) = left else {
        panic!("integer test must read the producer-backed pattern transient");
    };
    let source = tested.local;
    let Terminator::Branch { true_target, .. } = f.body.blocks[test_block].terminator else {
        panic!("integer pattern test must branch to the full-match path");
    };
    let success_entry = &f.body.blocks[true_target.0 as usize];

    let moved_init = success_entry
        .statements
        .iter()
        .position(|statement| {
            matches!(
                statement,
                CoreStatement::Init { dst, .. } if dst.local.0 as usize == moved_local
            )
        })
        .expect("full match must transfer the non-duplicable binding");
    let copied_init = success_entry
        .statements
        .iter()
        .position(|statement| {
            matches!(
                statement,
                CoreStatement::Init { dst, .. } if dst.local.0 as usize == copied_local
            )
        })
        .expect("full match must produce the duplicable binding");
    let source_drops = success_entry
        .statements
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| match statement {
            CoreStatement::Drop {
                place: PlaceAccess::Direct(place),
            } if place.local == source => Some((index, place.projections.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        source_drops
            .iter()
            .map(|(_, path)| path.clone())
            .collect::<Vec<_>>(),
        [vec![Projection::Field(2)], vec![Projection::Field(0)]],
        "producer success must clean exactly the retained post-binding frontier"
    );
    let first_cleanup = source_drops[0].0;
    assert!(moved_init < first_cleanup && copied_init < first_cleanup);
    assert!(matches!(
        success_entry.terminator,
        Terminator::Call {
            destination: None,
            ..
        }
    ));
}
