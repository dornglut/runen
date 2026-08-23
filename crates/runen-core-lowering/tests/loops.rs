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
fn simple_while_uses_distinct_header_body_and_false_continuation() {
    let lowered = lower_source("fn f(flag: Bool) { while flag {} }");
    let f = function(lowered.as_program(), "f");

    let Terminator::Goto(header) = f.body.blocks[0].terminator else {
        panic!("entry must transfer once to loop header");
    };
    let Terminator::Branch {
        condition,
        true_target,
        false_target,
    } = &f.body.blocks[header.0 as usize].terminator
    else {
        panic!("loop header must branch");
    };
    let condition_local = moved_local(condition).expect("Branch moves one Bool temporary");
    assert!(
        f.body.locals[condition_local.0 as usize]
            .name
            .starts_with("$tmp")
    );
    assert_ne!(true_target, false_target);
    assert_eq!(
        f.body.blocks[true_target.0 as usize].terminator,
        Terminator::Goto(header)
    );
    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Return(None)
    ));
}

#[test]
fn source_before_loop_executes_before_one_transfer_to_header() {
    let lowered = lower_source("fn f(flag: Bool) { let before: I64 = 1; while flag {} }");
    let f = function(lowered.as_program(), "f");
    let entry = &f.body.blocks[0];

    assert!(
        entry
            .statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::Init { .. }))
    );
    assert!(matches!(entry.terminator, Terminator::Goto(_)));
}

#[test]
fn direct_call_condition_branches_from_call_continuation_and_backedges_to_call_header() {
    let lowered = lower_source(
        "fn ready() -> Bool { return true; } \
         fn f() { while ready() {} }",
    );
    let f = function(lowered.as_program(), "f");

    let Terminator::Goto(header) = f.body.blocks[0].terminator else {
        panic!("entry must transfer to condition header");
    };
    let Terminator::Call { target, .. } = f.body.blocks[header.0 as usize].terminator else {
        panic!("condition header must execute the condition call");
    };
    let Terminator::Branch { true_target, .. } = f.body.blocks[target.0 as usize].terminator else {
        panic!("successful condition-call continuation must Branch");
    };
    assert_eq!(
        f.body.blocks[true_target.0 as usize].terminator,
        Terminator::Goto(header),
        "normal body backedge must reevaluate the call from its header"
    );
}

#[test]
fn body_normal_cleanup_precedes_backedge() {
    let lowered = lower_source(
        "record Box { value: I64 } \
         fn f(flag: Bool) { while flag { let child: Box = Box { value: 1 }; } }",
    );
    let f = function(lowered.as_program(), "f");
    let Terminator::Goto(header) = f.body.blocks[0].terminator else {
        panic!("entry Goto header");
    };
    let Terminator::Branch { true_target, .. } = f.body.blocks[header.0 as usize].terminator else {
        panic!("header Branch");
    };
    let body = &f.body.blocks[true_target.0 as usize];

    assert!(
        body.statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::Drop { .. })),
        "body-local cleanup must execute before the backedge"
    );
    assert_eq!(body.terminator, Terminator::Goto(header));
}

#[test]
fn no_normal_body_emits_no_synthetic_backedge_and_false_path_continues() {
    let lowered = lower_source(
        "fn sink(value: I64) {} \
         fn f(flag: Bool, value: I64) { while flag { fault; } sink(value); }",
    );
    let f = function(lowered.as_program(), "f");
    let Terminator::Goto(header) = f.body.blocks[0].terminator else {
        panic!("entry Goto header");
    };
    let Terminator::Branch {
        true_target,
        false_target,
        ..
    } = f.body.blocks[header.0 as usize].terminator
    else {
        panic!("header Branch");
    };

    assert!(matches!(
        f.body.blocks[true_target.0 as usize].terminator,
        Terminator::Fault(_)
    ));
    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Call { .. }
    ));
}

#[test]
fn repeated_body_storage_validates_through_vacant_init() {
    let lowered = lower_source(
        "record Box { value: I64 } \
         fn f(flag: Bool) { while flag { let child: Box = Box { value: 1 }; } }",
    );
    let f = function(lowered.as_program(), "f");

    assert!(f.body.locals.iter().any(|local| local.name == "child"));
    assert!(
        f.body
            .blocks
            .iter()
            .any(|block| matches!(block.terminator, Terminator::Goto(_)))
    );
}

#[test]
fn repeated_result_condition_temporary_validates_through_vacant_call_destination() {
    let lowered = lower_source(
        "fn ready() -> Bool { return true; } \
         fn f() { while ready() {} }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Call { .. }))
            .count(),
        1,
        "one static call site is revisited through the Core cycle"
    );
}

#[test]
fn following_source_is_lowered_only_on_false_continuation() {
    let lowered = lower_source(
        "fn sink(value: I64) {} \
         fn f(flag: Bool, value: I64) { while flag {} sink(value); }",
    );
    let f = function(lowered.as_program(), "f");
    let Terminator::Goto(header) = f.body.blocks[0].terminator else {
        panic!("entry Goto header");
    };
    let Terminator::Branch { false_target, .. } = f.body.blocks[header.0 as usize].terminator
    else {
        panic!("header Branch");
    };

    assert!(matches!(
        f.body.blocks[false_target.0 as usize].terminator,
        Terminator::Call { .. }
    ));
}

#[test]
fn nested_while_and_if_create_nested_existing_core_control_flow_only() {
    let lowered = lower_source(
        "fn f(a: Bool, b: Bool) { while a { if b { while a {} } else {} } }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        3
    );
}

#[test]
fn zero_leaf_body_local_needs_no_core_drop_before_backedge() {
    let lowered = lower_source(
        "record Empty {} \
         fn make() -> Empty { return Empty {}; } \
         fn f(flag: Bool) { while flag { let child: Empty = make(); } }",
    );
    let f = function(lowered.as_program(), "f");
    let Terminator::Goto(header) = f.body.blocks[0].terminator else {
        panic!("entry Goto header");
    };
    let Terminator::Branch { true_target, .. } = f.body.blocks[header.0 as usize].terminator else {
        panic!("header Branch");
    };
    let body = &f.body.blocks[true_target.0 as usize];

    assert!(
        !body
            .statements
            .iter()
            .any(|statement| matches!(statement, CoreStatement::Drop { .. }))
    );
    assert_eq!(body.terminator, Terminator::Goto(header));
}

#[test]
fn non_bool_retained_while_is_rejected_as_hir_invariant() {
    let mut compilation = hir("fn f(flag: Bool) { while flag {} }");
    let f = compilation
        .functions
        .iter_mut()
        .find(|function| function.name == "f")
        .expect("function f");
    let HirStatement::While { condition, .. } = &mut f.body.statements[0] else {
        panic!("expected HIR while");
    };
    condition.ty = Type::Intrinsic(IntrinsicType::I64);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "while condition type is not Bool"
        ))
    );
}
