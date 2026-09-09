use runen_core_ir::{
    Function as CoreFunction, Operand, Place, PlaceAccess, ScalarType, Statement as CoreStatement,
    TypeKind,
};
use runen_core_lowering::lower;
use runen_hir::{ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::parse_source;

fn lower_source(source: &str) -> runen_core_ir::ValidatedProgram {
    let parsed = parse_source(source.as_bytes()).expect("valid UTF-8 test source");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    let hir = build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("accepted marker-bearing source must build typed HIR");
    lower(&hir).expect("accepted marker-bearing HIR must lower through existing Core refinement")
}

fn functions_named<'a>(program: &'a runen_core_ir::Program, name: &str) -> Vec<&'a CoreFunction> {
    program
        .functions
        .iter()
        .filter(|function| function.name == name)
        .collect()
}

fn scalar_parameter(
    program: &runen_core_ir::Program,
    function: &CoreFunction,
    slot: usize,
) -> Option<ScalarType> {
    let ty = function.parameter_type(slot)?;
    match &program.types.get(ty)?.kind {
        TypeKind::Scalar(scalar) => Some(*scalar),
        TypeKind::Struct(_) => None,
    }
}

fn direct(place: Place) -> PlaceAccess {
    PlaceAccess::Direct(place)
}

#[test]
fn marker_evidence_erases_before_existing_concrete_specialization() {
    let lowered = lower_source(
        "trait Marker; impl I64: Marker; \
         fn id[T: Marker](value: T) -> T { return value; } \
         fn root(value: I64) -> I64 { return id[I64](value); }",
    );
    let program = lowered.as_program();
    let ids = functions_named(program, "id");
    assert_eq!(ids.len(), 1);
    assert_eq!(scalar_parameter(program, ids[0], 0), Some(ScalarType::I64));
    assert_eq!(
        ids[0].body.blocks[0].statements[0],
        CoreStatement::Init {
            dst: Place::local(runen_core_ir::LocalId(1)),
            src: Operand::Move(direct(Place::local(runen_core_ir::LocalId(0)))),
        },
        "marker evidence must not reselect abstract generic Move as Copy"
    );
}

#[test]
fn distinct_marker_satisfaction_does_not_enlarge_specialization_identity() {
    let lowered = lower_source(
        "trait A; trait B; impl I64: A; impl I64: B; \
         fn id[T: A + B](value: T) -> T { return value; } \
         fn root(value: I64) -> I64 { return id[I64](value); }",
    );
    let program = lowered.as_program();
    assert_eq!(functions_named(program, "id").len(), 1);
    assert_eq!(
        functions_named(program, "id")[0]
            .result
            .and_then(|ty| program.types.get(ty))
            .map(|ty| &ty.kind),
        Some(&TypeKind::Scalar(ScalarType::I64))
    );
}
