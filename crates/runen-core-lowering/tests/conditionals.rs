use runen_core_ir::{
    LocalId, Operand, PlaceAccess, Statement as CoreStatement, Terminator, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    IntrinsicType, ModuleId, SourceUnit, Statement as HirStatement, Type, build_typed_hir,
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

#[test]
fn branch_consumes_one_lowered_bool_condition_temporary() {
    let lowered = lower_source("fn f(flag: Bool) { if flag {} else {} }");
    let f = function(lowered.as_program(), "f");
    let Terminator::Branch {
        condition,
        true_target,
        false_target,
    } = &f.body.blocks[0].terminator
    else {
        panic!("entry must terminate with Branch");
    };

    let condition_local = moved_local(condition).expect("Branch must move the Bool temporary");
    assert!(
        f.body.locals[condition_local.0 as usize]
            .name
            .starts_with("$tmp")
    );
    assert_ne!(true_target, false_target);
    assert!(matches!(
        f.body.blocks[true_target.0 as usize].terminator,
        Terminator::Goto(_)
    ));
    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Goto(_)
    ));
}

#[test]
fn direct_call_condition_reaches_branch_through_one_call_continuation() {
    let lowered =
        lower_source("fn ready() -> Bool { return true; } fn f() { if ready() {} else {} }");
    let f = function(lowered.as_program(), "f");

    let Terminator::Call { target, .. } = f.body.blocks[0].terminator else {
        panic!("condition call must terminate entry exactly once");
    };
    let continuation = &f.body.blocks[target.0 as usize];
    assert!(matches!(continuation.terminator, Terminator::Branch { .. }));
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Call { .. }))
            .count(),
        1
    );
}

#[test]
fn explicit_arm_cleanup_precedes_each_normal_join_edge() {
    let lowered = lower_source(
        "record Box { value: I64 } \
         fn f(flag: Bool) { \
             if flag { let left: Box = Box { value: 1 }; } \
             else { let right: Box = Box { value: 2 }; } \
         }",
    );
    let f = function(lowered.as_program(), "f");

    let branch = f
        .body
        .blocks
        .iter()
        .find_map(|block| match block.terminator {
            Terminator::Branch {
                true_target,
                false_target,
                ..
            } => Some((true_target, false_target)),
            _ => None,
        })
        .expect("conditional Branch");

    for target in [branch.0, branch.1] {
        let arm = &f.body.blocks[target.0 as usize];
        assert!(
            arm.statements
                .iter()
                .any(|statement| matches!(statement, CoreStatement::Drop { .. })),
            "explicit arm cleanup must be emitted before its normal edge"
        );
        assert!(matches!(arm.terminator, Terminator::Goto(_)));
    }
}

#[test]
fn omitted_else_false_target_is_the_normal_join() {
    let lowered = lower_source("fn f(flag: Bool, value: I64) { if flag { let x: I64 = value; } }");
    let f = function(lowered.as_program(), "f");
    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[0].terminator
    else {
        panic!("entry must branch");
    };
    let Terminator::Goto(join) = f.body.blocks[true_target.0 as usize].terminator else {
        panic!("then arm must transfer to normal join");
    };
    assert_eq!(false_target, join);
}

#[test]
fn arm_locals_are_preregistered_before_temporaries_and_core_accepts_path_state_difference() {
    let lowered = lower_source(
        "fn f(flag: Bool) { \
             if flag { let same: I64 = 1; } else { let same: I64 = 2; } \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let names = f
        .body
        .locals
        .iter()
        .map(|local| local.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(&names[..3], &["flag", "same", "same"]);
    assert!(names[3..].iter().all(|name| name.starts_with("$tmp")));
}

#[test]
fn nested_conditionals_resume_subsequent_statement_from_outer_join() {
    let lowered = lower_source(
        "fn sink(value: I64) {} \
         fn f(a: Bool, b: Bool, value: I64) { \
             if a { if b { sink(value); } else {} } else {} \
             sink(value); \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        2
    );
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Call { .. }))
            .count(),
        2
    );
}

#[test]
fn non_bool_retained_conditional_is_rejected_as_hir_invariant() {
    let mut compilation = hir("fn f(flag: Bool) { if flag {} }");
    let f = compilation
        .functions
        .iter_mut()
        .find(|function| function.name == "f")
        .expect("function f");
    let HirStatement::If { condition, .. } = &mut f.body.statements[0] else {
        panic!("expected HIR conditional");
    };
    condition.ty = Type::Intrinsic(IntrinsicType::I64);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "conditional condition type is not Bool"
        ))
    );
}

#[test]
fn lowering_consumes_retained_source_join_fact_without_rederiving_it() {
    let lowered = lower_source(
        "record Left { value: I8 } record Right { value: I8 } \
         record Pair { left: Left, right: Right } \
         fn take_left(value: Left) {} \
         fn f(flag: Bool, pair: Pair) { \
             if flag { take_left(pair.left); } else { take_left(pair.left); } \
             let keep: Right = pair.right; \
         }",
    );
    let f = function(lowered.as_program(), "f");
    assert!(
        f.body
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Branch { .. }))
    );
}
