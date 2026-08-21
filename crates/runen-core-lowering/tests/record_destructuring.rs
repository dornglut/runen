use runen_core_ir::{
    Function as CoreFunction, LocalId, Operand, Place, PlaceAccess, Projection,
    Statement as CoreStatement, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{ModuleId, OwnedUse, SourceUnit, Statement, Type, build_typed_hir};
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

fn direct(place: Place) -> PlaceAccess {
    PlaceAccess::Direct(place)
}

#[test]
fn pattern_bindings_are_predeclared_in_source_order_and_lower_without_temporaries() {
    let lowered = lower_source(
        "record Token {} record Mixed { first: I8, token: Token, last: U8 } \
         fn f(root: Mixed) { let Mixed { last: z, token: moved, first: a } = root; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "z", "moved", "a"]
    );
    assert_eq!(
        f.body.blocks[0].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(2))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Move(direct(Place::local(LocalId(0)).field(1))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(3)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(0))),
            },
        ]
    );
}

#[test]
fn projected_copy_and_move_follow_pattern_source_order_not_record_order() {
    let lowered = lower_source(
        "record Token { value: I8 } record Pair { left: Token, count: I8, right: Token } \
         fn f(root: Pair) { \
             let Pair { right: r, count: c, left: l } = root; \
         }",
    );
    let f = function(lowered.as_program(), "f");

    let projections = f.body.blocks[0]
        .statements
        .iter()
        .map(|statement| match statement {
            CoreStatement::Init {
                src: Operand::Copy(PlaceAccess::Direct(place))
                    | Operand::Move(PlaceAccess::Direct(place)),
                ..
            } => place.projections.clone(),
            other => panic!("unexpected statement {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        projections,
        vec![
            vec![Projection::Field(2)],
            vec![Projection::Field(1)],
            vec![Projection::Field(0)],
        ]
    );
}

#[test]
fn zero_field_pattern_emits_no_local_and_no_core_statement() {
    let lowered = lower_source("record Empty {} fn f(root: Empty) { let Empty {} = root; }");
    let f = function(lowered.as_program(), "f");

    assert_eq!(f.body.locals.len(), 1);
    assert!(f.body.blocks[0].statements.is_empty());
}

#[test]
fn zero_leaf_nonduplicable_field_still_lowers_as_projected_move() {
    let lowered = lower_source(
        "record Empty {} record Holder { empty: Empty } \
         fn f(root: Holder) { let Holder { empty: moved } = root; }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body.blocks[0].statements,
        vec![CoreStatement::Init {
            dst: Place::local(LocalId(1)),
            src: Operand::Move(direct(Place::local(LocalId(0)).field(0))),
        }]
    );
}

#[test]
fn nested_pattern_cleanup_follows_reverse_source_binding_order() {
    let lowered = lower_source(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { { let Pair { right: second, left: first } = root; } }",
    );
    let f = function(lowered.as_program(), "f");

    assert_eq!(
        f.body
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "second", "first"]
    );
    assert_eq!(
        f.body.blocks[0].statements,
        vec![
            CoreStatement::Init {
                dst: Place::local(LocalId(1)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(1))),
            },
            CoreStatement::Init {
                dst: Place::local(LocalId(2)),
                src: Operand::Copy(direct(Place::local(LocalId(0)).field(0))),
            },
            CoreStatement::Drop {
                place: direct(Place::local(LocalId(2))),
            },
            CoreStatement::Drop {
                place: direct(Place::local(LocalId(1))),
            },
        ]
    );
}

#[test]
fn lowering_rejects_invalid_pattern_field_index() {
    let mut compilation = hir(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: a, right: b } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } = &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    bindings[0].field = 99;

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "HIR structural path does not match lowered Core type shape"
        ))
    );
}

#[test]
fn lowering_rejects_pattern_record_root_identity_mismatch() {
    let mut compilation = hir(
        "record A { value: I8 } record B { value: I8 } \
         fn f(root: A) { let A { value: extracted } = root; }",
    );
    let other = compilation.records[1].id;
    let Statement::RecordDestructure { record, .. } = &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    *record = other;

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "record destructuring root type does not match its record identity"
        ))
    );
}

#[test]
fn lowering_rejects_pattern_ownership_type_contradiction() {
    let mut compilation = hir(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: a, right: b } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } = &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    assert_eq!(bindings[0].ownership, OwnedUse::Duplicate);
    bindings[0].ownership = OwnedUse::Consume;

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "record destructuring ownership does not match retained field type"
        ))
    );
}

#[test]
fn lowering_rejects_retained_pattern_binding_type_mismatch() {
    let mut compilation = hir(
        "record Pair { left: I8, right: U8 } \
         fn f(root: Pair) { let Pair { left: a, right: b } = root; }",
    );
    let Statement::RecordDestructure { bindings, .. } = &mut compilation.functions[0].body.statements[0]
    else {
        panic!("expected pattern statement");
    };
    bindings[0].ty = Type::Intrinsic(runen_hir::IntrinsicType::U8);

    assert_eq!(
        lower(&compilation),
        Err(LoweringError::InvalidHirInvariant(
            "record destructuring retained binding type does not match projected field type"
        ))
    );
}
