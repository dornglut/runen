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

fn float_sub_statements(function: &runen_core_ir::Function) -> Vec<&CoreStatement> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::FloatSub { .. }))
        .collect()
}

fn float_sub_contract(statement: &CoreStatement) -> CoreNumericContract {
    let CoreStatement::FloatSub { contract, .. } = statement else {
        panic!("expected Core FloatSub");
    };
    *contract
}

#[test]
fn float_sub_lowers_to_one_fresh_standard_result_with_move_operands_and_no_cfg() {
    let lowered = lower_source("fn f(left: F32, right: F32) -> F32 { return left - right; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.blocks.len(), 1, "plain FloatSub adds no Core block");
    let subtraction = float_sub_statements(f);
    assert_eq!(subtraction.len(), 1);
    assert_eq!(
        float_sub_contract(subtraction[0]),
        CoreNumericContract::Standard
    );
    assert!(
        f.body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .all(|statement| !matches!(statement, CoreStatement::IntegerSub { .. })),
        "floating source subtraction must remain distinct from Core IntegerSub"
    );

    let CoreStatement::FloatSub {
        dst, left, right, ..
    } = subtraction[0]
    else {
        unreachable!();
    };
    assert!(dst.projections.is_empty());
    let left_local = moved_local(left).expect("FloatSub left operand must Move a temporary");
    let right_local = moved_local(right).expect("FloatSub right operand must Move a temporary");
    assert_ne!(left_local, right_local);
    assert_ne!(dst.local, left_local);
    assert_ne!(dst.local, right_local);
    assert!(left_local.0 < right_local.0);
    assert!(right_local.0 < dst.local.0);

    let Terminator::Return(Some(returned)) = &f.body.blocks[0].terminator else {
        panic!("FloatSub result must feed the function return");
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
fn float_sub_numeric_contracts_lower_one_to_one_without_redefaulting() {
    let standard = lower_source("fn f(a: F32, b: F32) -> F32 { return a - b; }");
    assert_eq!(
        float_sub_contract(float_sub_statements(function(standard.as_program(), "f"))[0]),
        CoreNumericContract::Standard
    );

    let fast = lower_source("fn f(a: F32, b: F32) -> F32 { return @fast(a - b); }");
    assert_eq!(
        float_sub_contract(float_sub_statements(function(fast.as_program(), "f"))[0]),
        CoreNumericContract::Fast
    );

    let mut reproducible = hir("fn f(a: F32, b: F32) -> F32 { return a - b; }");
    let value = reproducible.functions[0]
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::FloatSub { contract, .. } = &mut value.kind else {
        panic!("expected FloatSub HIR value");
    };
    *contract = HirNumericContract::Reproducible;
    let reproducible = lower(&reproducible).expect("valid Reproducible FloatSub HIR must lower");
    assert_eq!(
        float_sub_contract(float_sub_statements(function(reproducible.as_program(), "f"))[0]),
        CoreNumericContract::Reproducible
    );
}

#[test]
fn mixed_float_add_and_sub_contracts_preserve_occurrence_locality() {
    let lowered = lower_source(
        "fn fast_sub_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a - (b + c)); } \
         fn fast_add_child(a: F32, b: F32, c: F32) -> F32 { return a - @fast(b + c); } \
         fn fast_add_root(a: F32, b: F32, c: F32) -> F32 { return @fast(a + (b - c)); } \
         fn fast_sub_child(a: F32, b: F32, c: F32) -> F32 { return a + @fast(b - c); }",
    );

    let fast_sub_root = function(lowered.as_program(), "fast_sub_root");
    assert!(
        fast_sub_root
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .any(|statement| {
                matches!(
                    statement,
                    CoreStatement::FloatAdd {
                        contract: CoreNumericContract::Standard,
                        ..
                    }
                )
            })
    );
    assert!(
        fast_sub_root
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .any(|statement| {
                matches!(
                    statement,
                    CoreStatement::FloatSub {
                        contract: CoreNumericContract::Fast,
                        ..
                    }
                )
            })
    );

    let fast_add_child = function(lowered.as_program(), "fast_add_child");
    assert!(
        fast_add_child
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .any(|statement| {
                matches!(
                    statement,
                    CoreStatement::FloatAdd {
                        contract: CoreNumericContract::Fast,
                        ..
                    }
                )
            })
    );
    assert!(
        fast_add_child
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .any(|statement| {
                matches!(
                    statement,
                    CoreStatement::FloatSub {
                        contract: CoreNumericContract::Standard,
                        ..
                    }
                )
            })
    );

    let fast_add_root = function(lowered.as_program(), "fast_add_root");
    assert!(
        fast_add_root
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .any(|statement| {
                matches!(
                    statement,
                    CoreStatement::FloatAdd {
                        contract: CoreNumericContract::Fast,
                        ..
                    }
                )
            })
    );
    assert!(
        fast_add_root
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .any(|statement| {
                matches!(
                    statement,
                    CoreStatement::FloatSub {
                        contract: CoreNumericContract::Standard,
                        ..
                    }
                )
            })
    );

    let fast_sub_child = function(lowered.as_program(), "fast_sub_child");
    assert!(
        fast_sub_child
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .any(|statement| {
                matches!(
                    statement,
                    CoreStatement::FloatAdd {
                        contract: CoreNumericContract::Standard,
                        ..
                    }
                )
            })
    );
    assert!(
        fast_sub_child
            .body
            .blocks
            .iter()
            .flat_map(|b| &b.statements)
            .any(|statement| {
                matches!(
                    statement,
                    CoreStatement::FloatSub {
                        contract: CoreNumericContract::Fast,
                        ..
                    }
                )
            })
    );
}

#[test]
fn call_operands_lower_complete_left_then_right_before_float_sub() {
    let lowered = lower_source(
        "fn left() -> F32 { return 3.0; } \
         fn right() -> F32 { return 1.0; } \
         fn f() -> F32 { return @fast(left() - right()); }",
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
        target: sub_block,
        ..
    } = &f.body.blocks[right_call_block.0 as usize].terminator
    else {
        panic!("right producer must lower from the successful left continuation");
    };

    let subtraction = f.body.blocks[sub_block.0 as usize]
        .statements
        .iter()
        .find(|statement| matches!(statement, CoreStatement::FloatSub { .. }))
        .expect("FloatSub after both calls");
    assert_eq!(float_sub_contract(subtraction), CoreNumericContract::Fast);
    let CoreStatement::FloatSub {
        dst, left, right, ..
    } = subtraction
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
        "FloatSub itself adds no CFG"
    );
}
