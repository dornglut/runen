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

fn integer_eq_statements(function: &runen_core_ir::Function) -> Vec<&CoreStatement> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter(|statement| matches!(statement, CoreStatement::IntegerEq { .. }))
        .collect()
}

fn returned_value_mut(
    compilation: &mut runen_hir::TypedCompilation,
    name: &str,
) -> &mut runen_hir::Value {
    compilation
        .functions
        .iter_mut()
        .find(|function| function.name == name)
        .and_then(|function| function.body.terminal_return.as_mut())
        .and_then(|returned| returned.value.as_mut())
        .unwrap_or_else(|| panic!("missing HIR return value for {name}"))
}

#[test]
fn integer_equality_lowers_to_exactly_one_typed_integer_eq_for_every_width() {
    for source_type in ["I8", "I16", "I32", "I64", "U8", "U16", "U32", "U64"] {
        let source = format!(
            "fn f(left: {source_type}, right: {source_type}) -> Bool {{ return left == right; }}"
        );
        let lowered = lower_source(&source);
        let f = function(lowered.as_program(), "f");
        let statements = integer_eq_statements(f);
        assert_eq!(statements.len(), 1, "{source_type} must emit one IntegerEq");
        let CoreStatement::IntegerEq {
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
        assert_eq!(f.body.locals[f.parameters[1].0 as usize].ty, expected_operand_type);
        assert_eq!(f.body.locals[dst.local.0 as usize].ty, f.result.expect("Bool result type"));
        assert!(moved_local(left).is_some());
        assert!(moved_local(right).is_some());
        assert_eq!(
            f.body
                .blocks
                .iter()
                .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
                .count(),
            0,
            "integer == needs no Boolean refinement CFG"
        );
    }
}

#[test]
fn integer_inequality_uses_one_integer_eq_then_existing_boolean_negation_cfg() {
    let lowered = lower_source("fn f(left: I32, right: I32) -> Bool { return left != right; }");
    let f = function(lowered.as_program(), "f");
    let statements = integer_eq_statements(f);
    assert_eq!(statements.len(), 1);

    let CoreStatement::IntegerEq { dst, .. } = statements[0] else {
        unreachable!();
    };
    let equality_result = dst.local;
    let branch = f
        .body
        .blocks
        .iter()
        .find_map(|block| match &block.terminator {
            Terminator::Branch {
                condition,
                true_target,
                false_target,
            } => Some((condition, *true_target, *false_target)),
            _ => None,
        })
        .expect("integer != must negate the IntegerEq Bool result");
    assert_eq!(moved_local(branch.0), Some(equality_result));

    let true_block = &f.body.blocks[branch.1.0 as usize];
    let false_block = &f.body.blocks[branch.2.0 as usize];
    let [CoreStatement::Init {
        dst: true_dst,
        src: Operand::Constant(runen_core_ir::Value::Bool(false)),
    }] = true_block.statements.as_slice()
    else {
        panic!("true equality outcome must initialize inequality false");
    };
    let [CoreStatement::Init {
        dst: false_dst,
        src: Operand::Constant(runen_core_ir::Value::Bool(true)),
    }] = false_block.statements.as_slice()
    else {
        panic!("false equality outcome must initialize inequality true");
    };
    assert_eq!(true_dst.local, false_dst.local);
    assert!(matches!(true_block.terminator, Terminator::Goto(_)));
    assert!(matches!(false_block.terminator, Terminator::Goto(_)));
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
fn call_backed_inequality_lowers_left_then_right_once_before_integer_eq() {
    let lowered = lower_source(
        "fn left() -> I32 { return 1; } \
         fn right() -> I32 { return 2; } \
         fn f() -> Bool { return left() != right(); }",
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
        "integer != must not re-lower either call operand"
    );
    assert_eq!(integer_eq_statements(f).len(), 1);
    assert!(f.body.blocks[after_right.0 as usize]
        .statements
        .iter()
        .any(|statement| matches!(statement, CoreStatement::IntegerEq { .. })));
    assert!(matches!(
        f.body.blocks[after_right.0 as usize].terminator,
        Terminator::Branch { .. }
    ));
}

#[test]
fn boolean_equality_lowering_remains_cfg_only_and_emits_no_integer_eq() {
    let lowered = lower_source("fn f(left: Bool, right: Bool) -> Bool { return left == right; }");
    let f = function(lowered.as_program(), "f");
    assert!(integer_eq_statements(f).is_empty());
    assert_eq!(
        f.body
            .blocks
            .iter()
            .filter(|block| matches!(block.terminator, Terminator::Branch { .. }))
            .count(),
        3,
        "existing Boolean equality remains one left branch plus two right branches"
    );
}

#[test]
fn lowering_rejects_non_bool_integer_comparison_outer_type() {
    let mut compilation = hir("fn f(left: I32, right: I32) -> Bool { return left == right; }");
    returned_value_mut(&mut compilation, "f").ty = Type::Intrinsic(IntrinsicType::I64);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-equality result type is not Bool"
        ))
    );
}

#[test]
fn lowering_rejects_unsupported_retained_integer_comparison_operand_type() {
    let mut compilation = hir("fn f(left: I32, right: I32) -> Bool { return left != right; }");
    let value = returned_value_mut(&mut compilation, "f");
    let ValueKind::IntegerNe { operand_type, .. } = &mut value.kind else {
        panic!("expected integer inequality HIR");
    };
    *operand_type = Type::Intrinsic(IntrinsicType::F32);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-equality operand type is not a fixed-width integer"
        ))
    );
}

#[test]
fn lowering_rejects_operand_type_disagreement_without_repairing_from_core_shape() {
    let mut compilation = hir("fn f(left: I32, right: I32) -> Bool { return left == right; }");
    let value = returned_value_mut(&mut compilation, "f");
    let ValueKind::IntegerEq {
        operand_type, left, ..
    } = &mut value.kind
    else {
        panic!("expected integer equality HIR");
    };
    assert_eq!(*operand_type, Type::Intrinsic(IntrinsicType::I32));
    left.ty = Type::Intrinsic(IntrinsicType::I64);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "Integer-equality left operand type does not match retained operand type"
        ))
    );
}
