use runen_core_ir::{
    BasicBlock, LocalId, Operand, PlaceAccess, Statement as CoreStatement, Terminator,
    ValidatedProgram, Value as CoreValue,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{IntrinsicType, ModuleId, SourceUnit, Type, ValueKind, build_typed_hir};
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

fn bool_init(block: &BasicBlock) -> (LocalId, bool) {
    assert_eq!(block.statements.len(), 1, "negation path has one Init");
    let CoreStatement::Init { dst, src } = &block.statements[0] else {
        panic!("negation path must initialize the result temporary");
    };
    assert!(dst.projections.is_empty());
    let Operand::Constant(CoreValue::Bool(value)) = src else {
        panic!("negation path must initialize from a semantic Bool constant");
    };
    (dst.local, *value)
}

#[test]
fn boolean_not_refines_to_move_branch_two_inits_and_one_join() {
    let lowered = lower_source("fn f(flag: Bool) -> Bool { return !flag; }");
    let f = function(lowered.as_program(), "f");

    let Terminator::Branch {
        condition,
        true_target,
        false_target,
    } = &f.body.blocks[0].terminator
    else {
        panic!("negation entry must terminate with Branch");
    };
    let operand =
        moved_local(condition).expect("Boolean-not Branch must Move its operand temporary");
    assert!(f.body.locals[operand.0 as usize].name.starts_with("$tmp"));

    let true_block = &f.body.blocks[true_target.0 as usize];
    let false_block = &f.body.blocks[false_target.0 as usize];
    let (true_result, true_value) = bool_init(true_block);
    let (false_result, false_value) = bool_init(false_block);
    assert_eq!(
        true_result, false_result,
        "both paths initialize one result local"
    );
    assert!(!true_value, "true operand path produces false");
    assert!(false_value, "false operand path produces true");

    let Terminator::Goto(true_join) = true_block.terminator else {
        panic!("true path must jump to the negation join");
    };
    let Terminator::Goto(false_join) = false_block.terminator else {
        panic!("false path must jump to the negation join");
    };
    assert_eq!(true_join, false_join, "both paths share one join");

    let join = &f.body.blocks[true_join.0 as usize];
    let Terminator::Return(Some(returned)) = &join.terminator else {
        panic!("following return must lower from the negation join");
    };
    assert_eq!(moved_local(returned), Some(true_result));
}

#[test]
fn call_backed_operand_branches_only_from_successful_call_continuation() {
    let lowered =
        lower_source("fn ready() -> Bool { return true; } fn f() -> Bool { return !ready(); }");
    let f = function(lowered.as_program(), "f");

    let Terminator::Call { target, .. } = f.body.blocks[0].terminator else {
        panic!("operand call must terminate the entry block");
    };
    assert!(matches!(
        f.body.blocks[target.0 as usize].terminator,
        Terminator::Branch { .. }
    ));
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Call { .. }))
            .count(),
        1,
        "Boolean-not must lower the call operand exactly once"
    );
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        1
    );
}

#[test]
fn nested_boolean_not_preserves_current_block_and_builds_nested_valid_cfg() {
    let lowered = lower_source("fn f(flag: Bool) -> Bool { return !!flag; }");
    let f = function(lowered.as_program(), "f");

    let branch_blocks = f
        .body
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| {
            matches!(block.terminator, Terminator::Branch { .. }).then_some(index)
        })
        .collect::<Vec<_>>();
    assert_eq!(branch_blocks.len(), 2);

    let inner_branch = branch_blocks[0];
    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[inner_branch].terminator
    else {
        unreachable!();
    };
    let Terminator::Goto(inner_join) = f.body.blocks[true_target.0 as usize].terminator else {
        panic!("inner true path must join");
    };
    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Goto(target) if target == inner_join
    ));
    assert!(
        matches!(
            f.body.blocks[inner_join.0 as usize].terminator,
            Terminator::Branch { .. }
        ),
        "outer negation must branch from the inner negation join"
    );
}

#[test]
fn following_source_lowers_from_negation_join() {
    let lowered = lower_source(
        "fn sink(value: Bool) {} fn f(flag: Bool) { let local: Bool = !flag; sink(local); }",
    );
    let f = function(lowered.as_program(), "f");
    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[0].terminator
    else {
        panic!("entry must branch for Boolean-not");
    };
    let Terminator::Goto(join) = f.body.blocks[true_target.0 as usize].terminator else {
        panic!("true path must join");
    };
    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Goto(target) if target == join
    ));
    assert!(matches!(
        f.body.blocks[join.0 as usize].terminator,
        Terminator::Call { .. }
    ));
    assert!(
        !f.body.blocks[join.0 as usize].statements.is_empty(),
        "local initialization after negation must be emitted in the join block"
    );
}

#[test]
fn lowering_rejects_non_bool_retained_outer_boolean_not_fact() {
    let mut compilation = hir("fn f(flag: Bool) -> Bool { return !flag; }");
    let f = compilation
        .functions
        .iter_mut()
        .find(|function| function.name == "f")
        .expect("function f");
    let value = f
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    value.ty = Type::Intrinsic(IntrinsicType::I64);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Boolean-not result type is not Bool"
        ))
    );
}

#[test]
fn lowering_rejects_non_bool_retained_boolean_not_operand_fact() {
    let mut compilation = hir("fn f(flag: Bool) -> Bool { return !flag; }");
    let f = compilation
        .functions
        .iter_mut()
        .find(|function| function.name == "f")
        .expect("function f");
    let value = f
        .body
        .terminal_return
        .as_mut()
        .and_then(|returned| returned.value.as_mut())
        .expect("return value");
    let ValueKind::BooleanNot { operand } = &mut value.kind else {
        panic!("expected Boolean-not value");
    };
    operand.ty = Type::Intrinsic(IntrinsicType::I64);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Boolean-not operand type is not Bool"
        ))
    );
}

#[test]
fn boolean_not_lowers_through_existing_generic_consumers() {
    let lowered = lower_source(
        "record Flags { value: Bool } \
         fn sink(value: Bool) {} \
         fn f(flag: Bool) -> Bool { \
             let mut local: Bool = !flag; \
             local = !local; \
             sink(!local); \
             let flags: Flags = Flags { value: !local }; \
             if !local {} \
             while !local { break; } \
             return !flags.value; \
         }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count()
            >= 7,
        "every Boolean-not plus if/while control flow must refine through existing CFG"
    );
    assert!(
        f.body
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Call { .. })),
        "call-argument consumption remains on the existing call path"
    );
}

#[test]
fn boolean_not_does_not_require_any_new_core_semantic_operation() {
    let lowered = lower_source("fn f(flag: Bool) -> Bool { return !flag; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        1
    );
    assert_eq!(
        f.body
            .blocks
            .iter()
            .flat_map(|block| &block.statements)
            .filter(|statement| matches!(statement, CoreStatement::Init { .. }))
            .count(),
        3,
        "operand materialization plus two path result Inits are sufficient"
    );
}
