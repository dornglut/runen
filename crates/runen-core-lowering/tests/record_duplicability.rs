use runen_core_ir::{
    Function as CoreFunction, Operand, PlaceAccess, Program, Projection,
    Statement as CoreStatement, ValidatedProgram,
};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn lower_source(source: &str) -> ValidatedProgram {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR");
    lower(&hir).expect("accepted HIR must lower to valid Core")
}

fn function<'a>(program: &'a Program, name: &str) -> &'a CoreFunction {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn source_operands(function: &CoreFunction) -> Vec<&Operand> {
    function
        .body
        .blocks
        .iter()
        .flat_map(|block| &block.statements)
        .filter_map(|statement| match statement {
            CoreStatement::Init { src, .. } | CoreStatement::Assign { src, .. } => Some(src),
            _ => None,
        })
        .collect()
}

fn is_direct_root_copy(operand: &Operand, local: runen_core_ir::LocalId) -> bool {
    matches!(
        operand,
        Operand::Copy(PlaceAccess::Direct(place))
            if place.local == local && place.projections.is_empty()
    )
}

fn is_direct_root_move(operand: &Operand, local: runen_core_ir::LocalId) -> bool {
    matches!(
        operand,
        Operand::Move(PlaceAccess::Direct(place))
            if place.local == local && place.projections.is_empty()
    )
}

fn is_field_copy(
    operand: &Operand,
    local: runen_core_ir::LocalId,
    expected_fields: &[u32],
) -> bool {
    let Operand::Copy(PlaceAccess::Direct(place)) = operand else {
        return false;
    };
    if place.local != local {
        return false;
    }
    let fields = place
        .projections
        .iter()
        .map(|projection| match projection {
            Projection::Field(field) => *field,
        })
        .collect::<Vec<_>>();
    fields == expected_fields
}

#[test]
fn selected_record_whole_binding_lowers_to_core_copy() {
    let lowered = lower_source(
        "record copy Token { value: I8 } \
         fn f(value: Token) -> Token { return value; }",
    );
    let f = function(lowered.as_program(), "f");
    let parameter = f.parameters[0];

    assert!(
        source_operands(f)
            .into_iter()
            .any(|operand| is_direct_root_copy(operand, parameter))
    );
}

#[test]
fn unselected_structurally_copyable_record_still_lowers_source_use_as_move() {
    let lowered = lower_source(
        "record Token { value: I8 } \
         fn f(value: Token) -> Token { return value; }",
    );
    let f = function(lowered.as_program(), "f");
    let parameter = f.parameters[0];
    let operands = source_operands(f);

    assert!(
        operands
            .iter()
            .any(|operand| is_direct_root_move(operand, parameter))
    );
    assert!(
        !operands
            .iter()
            .any(|operand| is_direct_root_copy(operand, parameter))
    );
}

#[test]
fn selected_record_field_value_lowers_to_projected_core_copy() {
    let lowered = lower_source(
        "record copy Leaf { value: I8 } \
         record Holder { leaf: Leaf } \
         fn f(holder: Holder) -> Leaf { return holder.leaf; }",
    );
    let f = function(lowered.as_program(), "f");
    let parameter = f.parameters[0];

    assert!(
        source_operands(f)
            .into_iter()
            .any(|operand| is_field_copy(operand, parameter, &[0]))
    );
}

#[test]
fn selected_record_pattern_leaf_lowers_to_projected_core_copy() {
    let lowered = lower_source(
        "record copy Leaf { value: I8 } \
         record Token { value: I8 } \
         record Pair { leaf: Leaf, token: Token } \
         fn f(pair: Pair) -> Leaf { \
             let Pair { leaf: leaf, token: token } = pair; \
             return leaf; \
         }",
    );
    let f = function(lowered.as_program(), "f");
    let parameter = f.parameters[0];

    assert!(
        source_operands(f)
            .into_iter()
            .any(|operand| is_field_copy(operand, parameter, &[0]))
    );
}

#[test]
fn selected_zero_field_record_uses_existing_core_copy_operation() {
    let lowered = lower_source(
        "record copy Empty {} \
         fn f(value: Empty) -> Empty { return value; }",
    );
    let f = function(lowered.as_program(), "f");
    let parameter = f.parameters[0];

    assert!(
        source_operands(f)
            .into_iter()
            .any(|operand| is_direct_root_copy(operand, parameter))
    );
}
