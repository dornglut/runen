use std::collections::BTreeSet;

use runen_core_ir::{
    FunctionId, LocalId, Operand, PlaceAccess, Projection, ReferencePermission,
    SafeReferenceResultContract, ScalarType, Statement as CoreStatement, TypeId, TypeKind,
    ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    IntrinsicType, ModuleId, OwnedUse, ReferencePermission as HirReferencePermission,
    ReferenceReferent, SourceUnit, Statement as HirStatement, Type, ValueKind, build_typed_hir,
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
    lower(&hir(source)).expect("accepted safe-reference HIR must lower to validated Core")
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

fn reference_types(program: &runen_core_ir::Program) -> Vec<(TypeId, TypeId, ReferencePermission)> {
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

fn safe_reference(referent: ReferenceReferent, permission: HirReferencePermission) -> Type {
    Type::SafeReference {
        referent,
        permission,
    }
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
        .expect("safe lowered reference execution is defined")
}

#[test]
fn maps_only_used_shared_reference_types_once_per_hir_type() {
    let no_references = lower_source("fn f(x: I64) { let y: I64 = x; }");
    assert!(reference_types(no_references.as_program()).is_empty());

    let lowered = lower_source(
        "record copy Point { x: I64 }\
         fn f(value: I64, point: Point, scalar_ref: &I64, point_ref: &Point) {\
             let scalar_copy: &I64 = scalar_ref;\
             let point_copy: &Point = point_ref;\
         }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    let references = reference_types(program);
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
fn maps_replacement_reference_type_and_carrier_move_to_core_exclusive_replace() {
    let lowered = lower_source("fn f(r: &mut I64) { let moved: &mut I64 = r; }");
    let program = lowered.as_program();
    let f = function(program, "f");
    let r = f.parameters[0];
    let moved = local_named(f, "moved");
    let references = reference_types(program);

    assert_eq!(references.len(), 1);
    let (reference_ty, referent_ty, permission) = references[0];
    assert_eq!(permission, ReferencePermission::ExclusiveReplace);
    assert_eq!(f.body.locals[r.0 as usize].ty, reference_ty);
    assert_eq!(f.body.locals[moved.0 as usize].ty, reference_ty);
    assert_ne!(referent_ty, reference_ty);

    let statements = f
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .collect::<Vec<_>>();
    let carrier_temporary = statements
        .iter()
        .find_map(|statement| {
            let CoreStatement::Init {
                dst,
                src: Operand::Move(PlaceAccess::Direct(source)),
            } = statement
            else {
                return None;
            };
            (dst.projections.is_empty() && source.local == r && source.projections.is_empty())
                .then_some(dst.local)
        })
        .expect("replacement carrier is moved from the source binding into one value temporary");
    assert_ne!(carrier_temporary, moved);
    assert_eq!(f.body.locals[carrier_temporary.0 as usize].ty, reference_ty);
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            dst,
            src: Operand::Move(PlaceAccess::Direct(source)),
        } if dst.local == moved
            && dst.projections.is_empty()
            && source.local == carrier_temporary
            && source.projections.is_empty()
    )));
    assert!(!statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::Copy(PlaceAccess::Direct(source)),
            ..
        } if source.local == r && source.projections.is_empty()
    )));
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

    let references = reference_types(program);
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
fn lowers_shared_field_roots_to_exact_projected_reference_root_places() {
    let lowered = lower_source(
        "record copy Inner { value: I64 }\
         record copy Outer { inner: Inner, other: I64 }\
         fn f(root: Outer) {\
             let direct: &Inner = &root.inner;\
             let nested: &I64 = &root.inner.value;\
         }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    let root = f.parameters[0];
    let root_ty = f.body.locals[root.0 as usize].ty;
    let statements = f
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .collect::<Vec<_>>();
    let roots = statements
        .iter()
        .filter_map(|statement| {
            let CoreStatement::Init {
                src: Operand::ReferenceRoot { permission, place },
                ..
            } = statement
            else {
                return None;
            };
            (*permission == ReferencePermission::Shared && place.local == root).then_some(place)
        })
        .collect::<Vec<_>>();

    assert_eq!(roots.len(), 2);
    assert_eq!(roots[0].projections, vec![Projection::Field(0)]);
    assert_eq!(
        roots[1].projections,
        vec![Projection::Field(0), Projection::Field(0)]
    );

    let direct_ty = f.body.locals[local_named(f, "direct").0 as usize].ty;
    let nested_ty = f.body.locals[local_named(f, "nested").0 as usize].ty;
    let (direct_referent, direct_permission) = program
        .types
        .reference(direct_ty)
        .expect("direct projected root local has a Core reference type");
    let (nested_referent, nested_permission) = program
        .types
        .reference(nested_ty)
        .expect("nested projected root local has a Core reference type");
    assert_eq!(direct_permission, ReferencePermission::Shared);
    assert_eq!(nested_permission, ReferencePermission::Shared);
    assert_eq!(
        program.types.project_type(root_ty, &roots[0].projections),
        Some(direct_referent)
    );
    assert_eq!(
        program.types.project_type(root_ty, &roots[1].projections),
        Some(nested_referent)
    );

    assert!(!statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::Copy(PlaceAccess::Direct(place))
                | Operand::Move(PlaceAccess::Direct(place)),
            ..
        } if place.local == root && !place.projections.is_empty()
    )));
    assert!(!statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::ReferenceReborrow { .. } | Operand::AddressOf(_),
            ..
        }
    )));
}

#[test]
fn projected_shared_reference_root_executes_through_existing_reference_runtime() {
    let report = execute_source(
        "record copy Pair { left: I64, right: I64 }\
         fn entry() -> I64 {\
             let root: Pair = Pair { left: 83, right: 17 };\
             let r: &I64 = &root.left;\
             return *r;\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(83)));
}

#[test]
fn lowers_shared_field_relative_reborrows_to_exact_reference_access_projections() {
    let lowered = lower_source(
        "record copy Inner { value: I64 }\
         record copy Outer { inner: Inner, other: I64 }\
         fn f(parent: &Outer) {\
             let complete: &Outer = &*parent;\
             let direct: &Inner = &*parent.inner;\
             let nested: &I64 = &*parent.inner.value;\
         }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    let parent = f.parameters[0];
    let parent_ty = f.body.locals[parent.0 as usize].ty;
    let (parent_referent, parent_permission) = program
        .types
        .reference(parent_ty)
        .expect("field-relative reborrow parent has a Core reference type");
    assert_eq!(parent_permission, ReferencePermission::Shared);

    let statements = f
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .collect::<Vec<_>>();
    let reborrows = statements
        .iter()
        .filter_map(|statement| {
            let CoreStatement::Init {
                dst,
                src: Operand::ReferenceReborrow { permission, src },
            } = statement
            else {
                return None;
            };
            let PlaceAccess::Direct(reference_place) = &src.reference else {
                return None;
            };
            (reference_place.local == parent && reference_place.projections.is_empty())
                .then_some((dst.local, *permission, src))
        })
        .collect::<Vec<_>>();

    assert_eq!(reborrows.len(), 3);
    let expected = [
        Vec::<Projection>::new(),
        vec![Projection::Field(0)],
        vec![Projection::Field(0), Projection::Field(0)],
    ];
    for ((destination, permission, access), expected_projections) in
        reborrows.iter().zip(&expected)
    {
        assert_eq!(*permission, ReferencePermission::Shared);
        assert_eq!(&access.projections, expected_projections);
        let destination_ty = f.body.locals[destination.0 as usize].ty;
        let (child_referent, child_permission) = program
            .types
            .reference(destination_ty)
            .expect("reborrow temporary has a Core reference type");
        assert_eq!(child_permission, ReferencePermission::Shared);
        assert_eq!(
            program
                .types
                .project_type(parent_referent, &access.projections),
            Some(child_referent)
        );
    }

    assert!(!statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::ReferenceRoot { place, .. },
            ..
        } if place.local == parent
    )));
    assert!(!statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::Copy(PlaceAccess::Direct(place))
                | Operand::Move(PlaceAccess::Direct(place))
                | Operand::AddressOf(PlaceAccess::Direct(place)),
            ..
        } if place.local == parent
    )));
}

#[test]
fn projected_shared_child_executes_and_round_trips_through_identity_result() {
    let report = execute_source(
        "record Holder { left: I64, right: I64 }\
         fn id(r: &I64) -> &I64 { return r; }\
         fn entry() -> I64 {\
             let mut root: Holder = Holder { left: 83, right: 17 };\
             let parent: &mut Holder = &mut root;\
             let child: &I64 = &*parent.left;\
             let returned: &I64 = id(child);\
             return *returned;\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(83)));
}

#[test]
fn lowers_replacement_root_reborrow_move_and_assign_to_exact_core_operations() {
    let lowered = lower_source(
        "record Ticket { value: I64 }\
         fn f(seed: Ticket) {\
             let mut x: Ticket = seed;\
             let root: &mut Ticket = &mut x;\
             {\
                 let child: &mut Ticket = &mut *root;\
                 let moved: Ticket = *child;\
                 *child = moved;\
             }\
         }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    let x = local_named(f, "x");
    let root = local_named(f, "root");
    let child = local_named(f, "child");
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
        } if *permission == ReferencePermission::ExclusiveReplace
            && place.local == x
            && place.projections.is_empty()
    )));
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::ReferenceReborrow { permission, src },
            ..
        } if *permission == ReferencePermission::ExclusiveReplace
            && src.projections.is_empty()
            && matches!(
                &src.reference,
                PlaceAccess::Direct(place)
                    if place.local == root && place.projections.is_empty()
            )
    )));
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::ReferenceMove(access),
            ..
        } if access.projections.is_empty()
            && matches!(
                &access.reference,
                PlaceAccess::Direct(place)
                    if place.local == child && place.projections.is_empty()
            )
    )));
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::ReferenceAssign { dst, .. }
            if dst.projections.is_empty()
                && matches!(
                    &dst.reference,
                    PlaceAccess::Direct(place)
                        if place.local == child && place.projections.is_empty()
                )
    )));
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
fn replacement_parameter_move_restore_and_direct_temporary_execute() {
    let report = execute_source(
        "record Ticket { value: I64 }\
         fn replace(r: &mut Ticket) {\
             let old: Ticket = *r;\
             *r = Ticket { value: 73 };\
         }\
         fn entry() -> I64 {\
             let mut ticket: Ticket = Ticket { value: 11 };\
             replace(&mut ticket);\
             return ticket.value;\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(73)));
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
fn shared_reference_result_lowers_exact_type_and_origin_slot() {
    let lowered = lower_source("fn id(x: I64, r: &I64) -> &I64 { return r; }");
    let program = lowered.as_program();
    let id = function(program, "id");
    let result = id.result.expect("Shared-reference result type is retained");

    assert_eq!(
        id.safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 1 }
    );
    assert_eq!(result, id.body.locals[id.parameters[1].0 as usize].ty);
    assert_eq!(reference_types(program).len(), 1);
}

#[test]
fn shared_direct_child_result_maps_exactly_and_survives_parent_carrier_release() {
    let source = "fn child(r: &mut I64) -> &I64 { return &*r; }\
         fn entry() -> I64 {\
             let mut x: I64 = 71;\
             let returned: &I64 = child(&mut x);\
             return *returned;\
         }";
    let lowered = lower_source(source);
    let program = lowered.as_program();
    let child = function(program, "child");
    let result_ty = child
        .result
        .expect("Shared direct-child result type is retained");
    let parameter_ty = child.body.locals[child.parameters[0].0 as usize].ty;
    let (result_referent, result_permission) = program
        .types
        .reference(result_ty)
        .expect("Shared direct-child result lowers to a Core reference type");
    let (parameter_referent, parameter_permission) = program
        .types
        .reference(parameter_ty)
        .expect("direct-child origin parameter lowers to a Core reference type");

    assert_eq!(
        child.safe_reference_result_contract,
        SafeReferenceResultContract::SharedDirectChild { origin: 0 }
    );
    assert_eq!(result_permission, ReferencePermission::Shared);
    assert_eq!(parameter_permission, ReferencePermission::ExclusiveReplace);
    assert_eq!(result_referent, parameter_referent);

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(71)));
}

#[test]
fn shared_reference_result_round_trip_and_existing_carrier_coexist() {
    let report = execute_source(
        "fn id(r: &I64) -> &I64 { return r; }\
         fn entry() -> I64 {\
             let x: I64 = 47;\
             let original: &I64 = &x;\
             let returned: &I64 = id(original);\
             let a: I64 = *original;\
             let b: I64 = *returned;\
             return a + b;\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(94)));
}

#[test]
fn caller_created_shared_child_round_trips_through_identity_result() {
    let report = execute_source(
        "fn id(r: &I64) -> &I64 { return r; }\
         fn entry() -> I64 {\
             let mut x: I64 = 47;\
             let parent: &mut I64 = &mut x;\
             {\
                 let child: &I64 = &*parent;\
                 let returned: &I64 = id(child);\
                 return *returned;\
             }\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(47)));
}

#[test]
fn temporary_shared_argument_survives_as_result_until_lexical_cleanup() {
    let report = execute_source(
        "fn id(r: &I64) -> &I64 { return r; }\
         fn entry() -> I64 {\
             let mut x: I64 = 5;\
             {\
                 let returned: &I64 = id(&x);\
                 let observed: I64 = *returned;\
             }\
             x = 61;\
             return x;\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(61)));
}

#[test]
fn nested_shared_reference_result_forwarding_executes() {
    let report = execute_source(
        "fn inner(r: &I64) -> &I64 { return r; }\
         fn middle(r: &I64) -> &I64 { return inner(r); }\
         fn entry() -> I64 {\
             let x: I64 = 53;\
             let returned: &I64 = middle(&x);\
             return *returned;\
         }",
        "entry",
    );
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(53)));
}

#[test]
fn recursive_shared_reference_result_forwarding_lowers_without_execution() {
    let lowered = lower_source("fn recursive(r: &I64) -> &I64 { return recursive(r); }");
    let recursive = function(lowered.as_program(), "recursive");
    assert_eq!(
        recursive.safe_reference_result_contract,
        SafeReferenceResultContract::SharedIdentity { origin: 0 }
    );
    assert_eq!(
        recursive.result,
        Some(recursive.body.locals[recursive.parameters[0].0 as usize].ty)
    );
}

#[test]
fn contract_bearing_shared_reference_result_fault_has_no_program_result() {
    let report = execute_source(
        "fn fail(r: &I64) -> &I64 { fault; }\
         fn entry() -> I64 {\
             let x: I64 = 67;\
             let returned: &I64 = fail(&x);\
             return *returned;\
         }",
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
    field_reference.records[0].fields[0].ty = safe_reference(
        ReferenceReferent::Intrinsic(IntrinsicType::I64),
        HirReferencePermission::Shared,
    );
    assert_eq!(
        lower(&field_reference),
        Err(LoweringError::InvalidHirInvariant(
            "HIR record field contains a safe-reference type"
        ))
    );

    let mut consuming_shared = hir("fn f(r: &I64) { let s: &I64 = r; }");
    let HirStatement::Local { initializer, .. } =
        &mut consuming_shared.functions[0].body.statements[0]
    else {
        panic!("expected reference local");
    };
    let ValueKind::BindingUse { ownership, .. } = &mut initializer.kind else {
        panic!("expected reference binding use");
    };
    *ownership = OwnedUse::Consume;
    assert_eq!(
        lower(&consuming_shared),
        Err(LoweringError::InvalidHirInvariant(
            "Shared-reference binding use is not a source duplication"
        ))
    );

    let mut duplicating_replacement = hir("fn f(r: &mut I64) { let moved: &mut I64 = r; }");
    let HirStatement::Local { initializer, .. } =
        &mut duplicating_replacement.functions[0].body.statements[0]
    else {
        panic!("expected replacement-reference local");
    };
    let ValueKind::BindingUse { ownership, .. } = &mut initializer.kind else {
        panic!("expected replacement-reference binding use");
    };
    *ownership = OwnedUse::Duplicate;
    assert_eq!(
        lower(&duplicating_replacement),
        Err(LoweringError::InvalidHirInvariant(
            "replacement-reference binding use is not a source move"
        ))
    );

    let mut projected_type_mismatch = hir(
        "record copy Pair { left: I64 }\
         fn f(r: &Pair) { let child: &I64 = &*r.left; }",
    );
    let HirStatement::Local { initializer, .. } =
        &mut projected_type_mismatch.functions[0].body.statements[0]
    else {
        panic!("expected projected reference local");
    };
    initializer.ty = safe_reference(
        ReferenceReferent::Intrinsic(IntrinsicType::I32),
        HirReferencePermission::Shared,
    );
    assert_eq!(
        lower(&projected_type_mismatch),
        Err(LoweringError::InvalidHirInvariant(
            "reference-reborrow projected parent referent does not match child referent"
        ))
    );

    let mut projected_replacement = hir(
        "record copy Pair { left: I64 }\
         fn f(r: &Pair) { let child: &I64 = &*r.left; }",
    );
    let HirStatement::Local { initializer, .. } =
        &mut projected_replacement.functions[0].body.statements[0]
    else {
        panic!("expected projected reference local");
    };
    initializer.ty = safe_reference(
        ReferenceReferent::Intrinsic(IntrinsicType::I64),
        HirReferencePermission::ExclusiveReplace,
    );
    let ValueKind::ReferenceReborrow { permission, .. } = &mut initializer.kind else {
        panic!("expected projected reference reborrow");
    };
    *permission = HirReferencePermission::ExclusiveReplace;
    assert_eq!(
        lower(&projected_replacement),
        Err(LoweringError::InvalidHirInvariant(
            "projected reference-reborrow is not Shared"
        ))
    );
}
