use std::collections::BTreeSet;

use runen_core_ir::{
    FunctionId, LocalId, Operand, PlaceAccess, ReferencePermission, ScalarType,
    Statement as CoreStatement, TypeId, TypeKind, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    IntrinsicType, ModuleId, OwnedUse, ReferenceReferent, SourceUnit, Statement as HirStatement,
    Type, ValueKind, build_typed_hir,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus};
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
    lower(&hir(source)).expect("accepted Shared-reference HIR must lower to validated Core")
}

fn function<'a>(program: &'a runen_core_ir::Program, name: &str) -> &'a runen_core_ir::Function {
    program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing Core function {name}"))
}

fn local_named(function: &runen_core_ir::Function, name: &str) -> LocalId {
    let index = function
        .body
        .locals
        .iter()
        .position(|local| local.name == name)
        .unwrap_or_else(|| panic!("missing Core local {name}"));
    LocalId(u32::try_from(index).expect("test local index fits u32"))
}

fn shared_reference_types(
    program: &runen_core_ir::Program,
) -> Vec<(TypeId, TypeId, ReferencePermission)> {
    (0..program.types.len())
        .filter_map(|index| {
            let id = TypeId(u32::try_from(index).expect("test type index fits u32"));
            let definition = program.types.get(id).expect("enumerated Core type exists");
            match definition.kind {
                TypeKind::Scalar(ScalarType::Reference {
                    referent,
                    permission,
                }) => Some((id, referent, permission)),
                TypeKind::Scalar(_) | TypeKind::Struct(_) => None,
            }
        })
        .collect()
}

fn execute_source(source: &str, entry_name: &str) -> runen_reference::ExecutionReport {
    let lowered = lower_source(source);
    let entry_index = lowered
        .as_program()
        .functions
        .iter()
        .position(|function| function.name == entry_name)
        .unwrap_or_else(|| panic!("missing Core entry function {entry_name}"));
    let entry = FunctionId(u32::try_from(entry_index).expect("test function index fits u32"));
    Machine::new(lowered, entry)
        .expect("test entry has no parameters")
        .execute()
        .expect("safe lowered Shared-reference execution is defined")
}

#[test]
fn maps_only_used_shared_reference_types_once_per_hir_type() {
    let no_references = lower_source("fn f(x: I64) { let y: I64 = x; }");
    assert!(shared_reference_types(no_references.as_program()).is_empty());

    let lowered = lower_source(
        "record copy Point { x: I64 }\
         fn f(value: I64, point: Point, scalar_ref: &I64, point_ref: &Point) {\
             let scalar_copy: &I64 = scalar_ref;\
             let point_copy: &Point = point_ref;\
         }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    let references = shared_reference_types(program);
    assert_eq!(references.len(), 2);
    assert!(
        references
            .iter()
            .all(|(_, _, permission)| { *permission == ReferencePermission::Shared })
    );

    let expected_referents = BTreeSet::from([
        f.body.locals[f.parameters[0].0 as usize].ty,
        f.body.locals[f.parameters[1].0 as usize].ty,
    ]);
    let actual_referents = references
        .iter()
        .map(|(_, referent, _)| *referent)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_referents, expected_referents);

    let scalar_reference_ty = f.body.locals[f.parameters[2].0 as usize].ty;
    let point_reference_ty = f.body.locals[f.parameters[3].0 as usize].ty;
    assert_ne!(scalar_reference_ty, point_reference_ty);
    assert_eq!(local_named(f, "scalar_copy").0 as usize, 4);
    assert_eq!(
        f.body.locals[local_named(f, "scalar_copy").0 as usize].ty,
        scalar_reference_ty
    );
    assert_eq!(
        f.body.locals[local_named(f, "point_copy").0 as usize].ty,
        point_reference_ty
    );
}

#[test]
fn lowers_root_borrow_reference_duplication_and_dereference_to_exact_core_operations() {
    let lowered = lower_source(
        "fn f(x: I64, r: &I64) {\
             let formed: &I64 = &x;\
             let copied: &I64 = r;\
             let a: I64 = *formed;\
             let b: I64 = *copied;\
         }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    let x = f.parameters[0];
    let r = f.parameters[1];
    let formed = local_named(f, "formed");
    let copied = local_named(f, "copied");

    let references = shared_reference_types(program);
    assert_eq!(references.len(), 1);
    let (reference_ty, referent_ty, permission) = references[0];
    assert_eq!(permission, ReferencePermission::Shared);
    assert_eq!(referent_ty, f.body.locals[x.0 as usize].ty);
    assert_eq!(f.body.locals[r.0 as usize].ty, reference_ty);
    assert_eq!(f.body.locals[formed.0 as usize].ty, reference_ty);
    assert_eq!(f.body.locals[copied.0 as usize].ty, reference_ty);

    let statements = f
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .collect::<Vec<_>>();

    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::ReferenceRoot { permission, place },
            ..
        } if *permission == ReferencePermission::Shared
            && place.local == x
            && place.projections.is_empty()
    )));
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::Copy(PlaceAccess::Direct(place)),
            ..
        } if place.local == r && place.projections.is_empty()
    )));

    let dereferenced_sources = statements
        .iter()
        .filter_map(|statement| {
            let CoreStatement::Init {
                src: Operand::ReferenceCopy(access),
                ..
            } = statement
            else {
                return None;
            };
            if !access.projections.is_empty() {
                return None;
            }
            let PlaceAccess::Direct(place) = &access.reference else {
                return None;
            };
            place.projections.is_empty().then_some(place.local)
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(dereferenced_sources, BTreeSet::from([formed, copied]));
}

#[test]
fn nested_block_reference_local_is_discovered_mapped_cleaned_and_executable() {
    let report = execute_source(
        "fn entry() -> I64 {\
             let x: I64 = 7;\
             { let r: &I64 = &x; let y: I64 = *r; }\
             return x;\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(7)));
}

#[test]
fn completed_call_temporary_borrow_ends_before_following_assignment() {
    let report = execute_source(
        "fn read_ref(r: &I64) -> I64 { return *r; }\
         fn entry() -> I64 {\
             let mut x: I64 = 9;\
             x = read_ref(&x);\
             return x;\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(9)));
}

#[test]
fn nested_shared_reference_calls_resolve_suspended_ancestor_storage() {
    let report = execute_source(
        "fn leaf(r: &I64) -> I64 { return *r; }\
         fn middle(r: &I64) -> I64 { return leaf(r); }\
         fn entry() -> I64 { let x: I64 = 41; return middle(&x); }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(41)));
}

#[test]
fn defined_fault_cleans_transferred_and_duplicated_reference_carriers() {
    let report = execute_source(
        "fn fail(r: &I64) { let s: &I64 = r; let value: I64 = *s; fault; }\
         fn entry() { let x: I64 = 5; fail(&x); }",
        "entry",
    );
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("source.explicit".to_owned())
    );
    assert_eq!(report.result, None);
}

#[test]
fn lowering_rejects_malformed_reference_hir_instead_of_widening_the_slice() {
    let mut field_reference = hir("record Holder { value: I64 } fn f(holder: Holder) {}");
    field_reference.records[0].fields[0].ty =
        Type::SharedReference(ReferenceReferent::Intrinsic(IntrinsicType::I64));
    assert_eq!(
        lower(&field_reference),
        Err(LoweringError::InvalidHirInvariant(
            "HIR record field contains a Shared-reference type"
        ))
    );

    let mut consuming_reference = hir("fn f(r: &I64) { let s: &I64 = r; }");
    let HirStatement::Local { initializer, .. } =
        &mut consuming_reference.functions[0].body.statements[0]
    else {
        panic!("expected reference local");
    };
    let ValueKind::BindingUse { ownership, .. } = &mut initializer.kind else {
        panic!("expected reference binding use");
    };
    *ownership = OwnedUse::Consume;
    assert_eq!(
        lower(&consuming_reference),
        Err(LoweringError::InvalidHirInvariant(
            "Shared-reference binding use is not a source duplication"
        ))
    );
}
