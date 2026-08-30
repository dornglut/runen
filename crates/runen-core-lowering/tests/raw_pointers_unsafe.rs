use std::collections::BTreeSet;

use runen_core_ir::{
    BasicBlockId, FunctionId, LocalId, Operand, Place, PlaceAccess, ScalarType,
    Statement as CoreStatement, Terminator, TypeId, TypeKind, ValidatedProgram,
};
use runen_core_lowering::{LoweringError, lower};
use runen_hir::{
    IntrinsicType, ModuleId, RawPointerPointee, SourceUnit, Type, TypedCompilation, build_typed_hir,
};
use runen_reference::{
    Machine, ObservedValue, TerminalStatus, VerificationEventKind, VerificationWriteKind,
};
use runen_syntax::{Parse, parse_source};

fn parse(source: &str) -> Parse {
    parse_source(source.as_bytes()).expect("valid UTF-8 test source")
}

fn hir(source: &str) -> TypedCompilation {
    let parsed = parse(source);
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
        .expect("test source must produce accepted HIR")
}

fn lower_source(source: &str) -> ValidatedProgram {
    lower(&hir(source)).expect("accepted raw-pointer HIR must lower to validated Core")
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

fn raw_pointer_types(program: &runen_core_ir::Program) -> Vec<(TypeId, TypeId)> {
    (0..program.types.len())
        .filter_map(|index| {
            let id = TypeId(u32::try_from(index).expect("test type index fits u32"));
            let definition = program.types.get(id).expect("enumerated Core type exists");
            match definition.kind {
                TypeKind::Scalar(ScalarType::RawPointer(pointee)) => Some((id, pointee)),
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
        .expect("source-proved raw-pointer execution is defined")
}

fn direct_local(access: &PlaceAccess) -> Option<LocalId> {
    let PlaceAccess::Direct(place) = access else {
        return None;
    };
    place.projections.is_empty().then_some(place.local)
}

#[test]
fn maps_each_used_raw_pointer_type_once_and_lowers_address_copy_and_retarget() {
    let lowered = lower_source(
        "record copy Point { x: I64 }\
         fn f(a: I64, b: I64, point: Point) {\
             let scalar: raw I64 = raw &a;\
             let copied: raw I64 = scalar;\
             let mut retarget: raw I64 = raw &a;\
             retarget = raw &b;\
             let record_ptr: raw Point = raw &point;\
         }",
    );
    let program = lowered.as_program();
    let f = function(program, "f");
    let raw_types = raw_pointer_types(program);
    assert_eq!(raw_types.len(), 2);

    let a = f.parameters[0];
    let b = f.parameters[1];
    let point = f.parameters[2];
    let scalar = local_named(f, "scalar");
    let copied = local_named(f, "copied");
    let retarget = local_named(f, "retarget");
    let record_ptr = local_named(f, "record_ptr");

    let expected_pointees = BTreeSet::from([
        f.body.locals[a.0 as usize].ty,
        f.body.locals[point.0 as usize].ty,
    ]);
    let actual_pointees = raw_types
        .iter()
        .map(|(_, pointee)| *pointee)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_pointees, expected_pointees);
    assert_eq!(f.body.locals[scalar.0 as usize].ty, f.body.locals[copied.0 as usize].ty);
    assert_eq!(f.body.locals[scalar.0 as usize].ty, f.body.locals[retarget.0 as usize].ty);
    assert_ne!(f.body.locals[scalar.0 as usize].ty, f.body.locals[record_ptr.0 as usize].ty);

    let statements = f
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .collect::<Vec<_>>();
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::AddressOf(PlaceAccess::Direct(place)),
            ..
        } if place.local == a && place.projections.is_empty()
    )));
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::Copy(PlaceAccess::Direct(place)),
            ..
        } if place.local == scalar && place.projections.is_empty()
    )));
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Assign { dst, .. }
            if direct_local(dst) == Some(retarget)
    )));
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::AddressOf(PlaceAccess::Direct(place)),
            ..
        } if place.local == b && place.projections.is_empty()
    )));
}

#[test]
fn raw_move_lowers_to_exact_operand_and_executes_defined() {
    let source = "fn entry() -> I64 {\
        let x: I64 = 41;\
        let p: raw I64 = raw &x;\
        let mut result: I64 = 0;\
        unsafe { result = raw move p; }\
        return result;\
    }";
    let lowered = lower_source(source);
    let entry = function(lowered.as_program(), "entry");
    let pointer = local_named(entry, "p");
    let statements = entry
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .collect::<Vec<_>>();
    assert!(statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            src: Operand::RawMove(PlaceAccess::Direct(place)),
            ..
        } if place.local == pointer && place.projections.is_empty()
    )));

    let report = execute_source(source, "entry");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(41)));
    assert!(report.verification_events.iter().any(|event| matches!(
        event.kind,
        VerificationEventKind::RawMove { .. }
    )));
}

#[test]
fn raw_assign_snapshots_pointer_before_rhs_call_and_writes_only_on_normal_continuation() {
    let lowered = lower_source(
        "fn produce() -> I64 { return 7; }\
         fn entry() -> I64 {\
             let x: I64 = 1;\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = produce(); }\
             return x;\
         }",
    );
    let entry = function(lowered.as_program(), "entry");
    let pointer = local_named(entry, "p");

    let (call_block_index, continuation) = entry
        .body
        .blocks
        .iter()
        .enumerate()
        .find_map(|(index, block)| match block.terminator {
            Terminator::Call {
                target,
                destination: Some(_),
                ..
            } => Some((index, target)),
            _ => None,
        })
        .expect("RawAssign RHS call must lower to a Core call terminator");
    let continuation_block = &entry.body.blocks[continuation.0 as usize];
    let raw_assign = continuation_block
        .statements
        .iter()
        .find_map(|statement| {
            let CoreStatement::RawAssign { pointer, .. } = statement else {
                return None;
            };
            direct_local(pointer)
        })
        .expect("normal call continuation must contain RawAssign");

    let call_block = &entry.body.blocks[call_block_index];
    assert!(call_block.statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Init {
            dst,
            src: Operand::Copy(PlaceAccess::Direct(place)),
        } if dst.local == raw_assign
            && dst.projections.is_empty()
            && place.local == pointer
            && place.projections.is_empty()
    )));
    assert!(continuation_block.statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::Drop { place }
            if direct_local(place) == Some(raw_assign)
    )));
}

#[test]
fn raw_assign_move_round_trip_and_unavailable_replacement_execute() {
    let round_trip = execute_source(
        "fn entry() -> I64 {\
             let x: I64 = 53;\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = raw move p; }\
             return x;\
         }",
        "entry",
    );
    assert_eq!(round_trip.terminal, TerminalStatus::Returned);
    assert_eq!(round_trip.result, Some(ObservedValue::I64(53)));
    assert!(round_trip.verification_events.iter().any(|event| matches!(
        event.kind,
        VerificationEventKind::Write {
            kind: VerificationWriteKind::RawAssign,
            ..
        }
    )));

    let unavailable = execute_source(
        "record Ticket { value: I64 }\
         fn take(value: Ticket) {}\
         fn entry() -> I64 {\
             let ticket: Ticket = Ticket { value: 61 };\
             let p: raw Ticket = raw &ticket;\
             take(ticket);\
             unsafe { raw assign p = Ticket { value: 67 }; }\
             return ticket.value;\
         }",
        "entry",
    );
    assert_eq!(unavailable.terminal, TerminalStatus::Returned);
    assert_eq!(unavailable.result, Some(ObservedValue::I64(67)));
}

#[test]
fn partial_raw_assign_and_temporary_shared_rhs_execute_without_runtime_ub() {
    let partial = execute_source(
        "record Ticket { value: I64 }\
         record Pair { left: Ticket, right: Ticket }\
         fn take(value: Ticket) {}\
         fn entry() -> I64 {\
             let pair: Pair = Pair {\
                 left: Ticket { value: 1 },\
                 right: Ticket { value: 2 }\
             };\
             let p: raw Pair = raw &pair;\
             take(pair.left);\
             unsafe {\
                 raw assign p = Pair {\
                     left: Ticket { value: 3 },\
                     right: Ticket { value: 71 }\
                 };\
             }\
             return pair.right.value;\
         }",
        "entry",
    );
    assert_eq!(partial.terminal, TerminalStatus::Returned);
    assert_eq!(partial.result, Some(ObservedValue::I64(71)));

    let temporary_shared = execute_source(
        "fn read(r: &I64) -> I64 { return *r; }\
         fn entry() -> I64 {\
             let x: I64 = 73;\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = read(&x); }\
             return x;\
         }",
        "entry",
    );
    assert_eq!(temporary_shared.terminal, TerminalStatus::Returned);
    assert_eq!(temporary_shared.result, Some(ObservedValue::I64(73)));
}

#[test]
fn defined_fault_rhs_performs_no_raw_assign_write() {
    let report = execute_source(
        "fn fail() -> I64 { fault; }\
         fn entry() -> I64 {\
             let x: I64 = 5;\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = fail(); }\
             return x;\
         }",
        "entry",
    );
    assert_eq!(
        report.terminal,
        TerminalStatus::Faulted("source.explicit".to_owned())
    );
    assert_eq!(report.result, None);
    assert!(report.verification_events.iter().all(|event| !matches!(
        event.kind,
        VerificationEventKind::Write {
            kind: VerificationWriteKind::RawAssign,
            ..
        }
    )));
}

#[test]
fn recursive_rhs_places_raw_assign_only_on_normal_call_continuation() {
    let lowered = lower_source(
        "fn recursive() -> I64 { return recursive(); }\
         fn entry() {\
             let x: I64 = 1;\
             let p: raw I64 = raw &x;\
             unsafe { raw assign p = recursive(); }\
         }",
    );
    let entry = function(lowered.as_program(), "entry");
    let pointer = local_named(entry, "p");
    let (call_block_index, continuation) = entry
        .body
        .blocks
        .iter()
        .enumerate()
        .find_map(|(index, block)| match block.terminator {
            Terminator::Call {
                target,
                destination: Some(_),
                ..
            } => Some((index, target)),
            _ => None,
        })
        .expect("recursive RHS must lower to a call with normal continuation");

    let continuation_block = &entry.body.blocks[continuation.0 as usize];
    assert!(continuation_block.statements.iter().any(|statement| matches!(
        statement,
        CoreStatement::RawAssign { .. }
    )));
    assert!(
        entry.body.blocks[..=call_block_index]
            .iter()
            .flat_map(|block| block.statements.iter())
            .all(|statement| !matches!(statement, CoreStatement::RawAssign { .. }))
    );
    assert!(entry.body.blocks[call_block_index]
        .statements
        .iter()
        .any(|statement| matches!(
            statement,
            CoreStatement::Init {
                src: Operand::Copy(PlaceAccess::Direct(place)),
                ..
            } if place.local == pointer && place.projections.is_empty()
        )));
}

#[test]
fn lowering_rejects_malformed_raw_pointer_interfaces_instead_of_widening_source() {
    let raw_i64 = Type::RawPointer(RawPointerPointee::Intrinsic(IntrinsicType::I64));

    let mut field = hir("record Holder { value: I64 } fn f(holder: Holder) {}");
    field.records[0].fields[0].ty = raw_i64;
    assert_eq!(
        lower(&field),
        Err(LoweringError::InvalidHirInvariant(
            "HIR record field contains a raw-pointer type"
        ))
    );

    let mut parameter = hir("fn f(x: I64) {}");
    parameter.functions[0].parameters[0].ty = raw_i64;
    assert_eq!(
        lower(&parameter),
        Err(LoweringError::InvalidHirInvariant(
            "HIR function parameter contains a raw-pointer type"
        ))
    );

    let mut result = hir("fn f() -> I64 { return 1; }");
    result.functions[0].result = Some(raw_i64);
    assert_eq!(
        lower(&result),
        Err(LoweringError::InvalidHirInvariant(
            "HIR function result contains a raw-pointer type"
        ))
    );
}
