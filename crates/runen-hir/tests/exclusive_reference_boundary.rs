use runen_hir::{DiagnosticKind, ModuleId, SourceUnit, build_typed_hir};
use runen_syntax::parse_source;

fn compile(source: &str) -> Result<runen_hir::TypedCompilation, Vec<runen_hir::Diagnostic>> {
    let parsed = parse_source(source.as_bytes()).expect("valid UTF-8 test source");
    assert!(parsed.errors().is_empty(), "{:?}", parsed.errors());
    build_typed_hir(&[SourceUnit::new(ModuleId::new(1), &parsed, &[])])
}

fn has_diagnostic(errors: &[runen_hir::Diagnostic], kind: DiagnosticKind) -> bool {
    errors.iter().any(|error| error.kind == kind)
}

#[test]
fn replacement_root_rejects_overlapping_shared_authority() {
    let errors = compile(
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let shared: &I64 = &x;\
             let replacement: &mut I64 = &mut x;\
         }",
    )
    .expect_err("replacement root requires no overlapping safe authority");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::InvalidReplacementReferenceTarget),
        "missing overlapping-root rejection: {errors:?}"
    );
}

#[test]
fn shared_root_rejects_overlapping_replacement_authority() {
    let errors = compile(
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let replacement: &mut I64 = &mut x;\
             let shared: &I64 = &x;\
         }",
    )
    .expect_err("Shared root requires no overlapping exclusive safe authority");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::ReferencePermissionUnavailable),
        "missing Shared-root/exclusive-authority rejection: {errors:?}"
    );
}

#[test]
fn replacement_reference_carrier_cannot_be_used_twice() {
    let errors = compile(
        "fn f(r: &mut I64) {\
             let first: &mut I64 = r;\
             let second: &mut I64 = r;\
         }",
    )
    .expect_err("replacement-reference carrier transport is Move, never copy");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::UnavailableBinding),
        "second carrier use must observe the first Move: {errors:?}"
    );
}

#[test]
fn moved_replacement_carrier_cleanup_ends_its_authority_branch() {
    compile(
        "fn f(seed: I64) {\
             let mut x: I64 = seed;\
             let root: &mut I64 = &mut x;\
             { let moved: &mut I64 = root; }\
             x = 7;\
         }",
    )
    .expect("cleanup of the moved carrier must end its carrierless authority branch");
}

#[test]
fn explicit_return_requires_incoming_replacement_referent_restored() {
    let errors = compile(
        "record Ticket { value: I64 }\
         fn f(r: &mut Ticket) -> I64 {\
             let ticket: Ticket = *r;\
             return 1;\
         }",
    )
    .expect_err("normal Return may not leave the incoming external referent consumed");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::ReferenceRestorationRequired),
        "explicit Return must apply restoration after result effects: {errors:?}"
    );
}

#[test]
fn direct_binding_access_uses_the_canonical_safe_authority_requirements() {
    for source in [
        "fn duplicate(seed: I64) {\
             let mut x: I64 = seed;\
             let r: &mut I64 = &mut x;\
             let copied: I64 = x;\
         }",
        "record Ticket {}\
         fn consume(seed: Ticket) {\
             let mut x: Ticket = seed;\
             let r: &mut Ticket = &mut x;\
             let moved: Ticket = x;\
         }",
    ] {
        let errors = compile(source)
            .expect_err("direct binding access may not bypass a live replacement authority");
        assert!(
            has_diagnostic(&errors, DiagnosticKind::ReferencePermissionUnavailable),
            "missing direct binding/reference conflict: {errors:?}"
        );
    }
}

#[test]
fn direct_field_access_uses_the_canonical_safe_authority_requirements() {
    for source in [
        "record copy Pair { value: I64 }\
         fn duplicate(seed: Pair) {\
             let mut root: Pair = seed;\
             let r: &mut Pair = &mut root;\
             let value: I64 = root.value;\
         }",
        "record Token {}\
         record Box { token: Token }\
         fn consume(seed: Box) {\
             let mut root: Box = seed;\
             let r: &mut Box = &mut root;\
             let token: Token = root.token;\
         }",
    ] {
        let errors = compile(source)
            .expect_err("direct field production may not bypass a live replacement authority");
        assert!(
            has_diagnostic(&errors, DiagnosticKind::ReferencePermissionUnavailable),
            "missing direct field/reference conflict: {errors:?}"
        );
    }
}

#[test]
fn shared_field_root_observes_exact_structural_availability() {
    let consumed_ancestor = compile(
        "record Token {}\
         record Inner { token: Token, count: I64 }\
         record Outer { inner: Inner, other: I64 }\
         fn f(root: Outer) {\
             let moved: Inner = root.inner;\
             let r: &I64 = &root.inner.count;\
         }",
    )
    .expect_err("a consumed ancestor path makes its selected descendant unavailable");
    assert!(
        has_diagnostic(&consumed_ancestor, DiagnosticKind::UnavailableFieldValue),
        "missing consumed-ancestor field-root diagnostic: {consumed_ancestor:?}"
    );

    compile(
        "record Token {}\
         record Inner { token: Token, count: I64 }\
         record Outer { inner: Inner, other: I64 }\
         fn f(root: Outer) {\
             let moved: Inner = root.inner;\
             let r: &I64 = &root.other;\
         }",
    )
    .expect("a consumed disjoint sibling must not make the Shared field-root target unavailable");
}

#[test]
fn shared_field_authorities_use_ancestor_descendant_overlap_but_not_sibling_overlap() {
    compile(
        "record copy Inner { value: I64 }\
         record copy Outer { inner: Inner, other: I64 }\
         fn f(root: Outer) {\
             let ancestor: &Inner = &root.inner;\
             let descendant: &I64 = &root.inner.value;\
             let sibling: &I64 = &root.other;\
         }",
    )
    .expect("overlapping Shared ancestors/descendants and disjoint Shared siblings are compatible");
}

#[test]
fn shared_field_authority_blocks_whole_root_exclusive_operations() {
    let replacement = compile(
        "record copy Pair { left: I64, right: I64 }\
         fn f(seed: Pair) {\
             let mut root: Pair = seed;\
             let shared: &I64 = &root.left;\
             let replacement: &mut Pair = &mut root;\
         }",
    )
    .expect_err("whole-root replacement overlaps every descendant Shared authority");
    assert!(
        has_diagnostic(&replacement, DiagnosticKind::InvalidReplacementReferenceTarget),
        "missing projected-Shared/replacement conflict: {replacement:?}"
    );

    let assignment = compile(
        "record copy Pair { left: I64, right: I64 }\
         fn f(seed: Pair, replacement: Pair) {\
             let mut root: Pair = seed;\
             let shared: &I64 = &root.left;\
             root = replacement;\
         }",
    )
    .expect_err("whole-root assignment overlaps every descendant Shared authority");
    assert!(
        has_diagnostic(&assignment, DiagnosticKind::BorrowedAssignmentTarget),
        "missing projected-Shared/assignment conflict: {assignment:?}"
    );

    let raw_move = compile(
        "record copy Pair { left: I64, right: I64 }\
         fn f(seed: Pair) {\
             let mut root: Pair = seed;\
             let pointer: raw Pair = raw &root;\
             let shared: &I64 = &root.left;\
             unsafe { let moved: Pair = raw move pointer; }\
         }",
    )
    .expect_err("raw ownership move requires Exclusive compatibility with the whole root");
    assert!(
        has_diagnostic(&raw_move, DiagnosticKind::RawTargetSafeAuthorityConflict),
        "missing projected-Shared/raw-move conflict: {raw_move:?}"
    );
}

#[test]
fn raw_address_and_direct_field_access_use_exact_shared_field_overlap() {
    compile(
        "record copy Pair { left: I64, right: I64 }\
         fn f(root: Pair) {\
             let shared: &I64 = &root.left;\
             let pointer: raw Pair = raw &root;\
             let overlapping: I64 = root.left;\
             let disjoint: I64 = root.right;\
         }",
    )
    .expect("raw address and duplicating field uses remain Shared-compatible with projected authority");

    compile(
        "record Token {}\
         record Box { token: Token, count: I64 }\
         fn f(root: Box) {\
             let shared: &I64 = &root.count;\
             let moved: Token = root.token;\
         }",
    )
    .expect("consuming a disjoint sibling must not be blocked by a Shared field authority");
}

#[test]
fn direct_pattern_compatibility_is_applied_per_selected_leaf_path() {
    compile(
        "record Token {}\
         record Box { token: Token, count: I64 }\
         fn f(root: Box) {\
             let shared: &I64 = &root.count;\
             let Box { token: moved, count: copied } = root;\
         }",
    )
    .expect(
        "pattern consume on a disjoint leaf and Shared-compatible duplicate on the overlapping leaf must coexist",
    );
}

#[test]
fn direct_pattern_production_uses_the_canonical_safe_authority_requirements() {
    for source in [
        "record copy Pair { left: I64, right: I64 }\
         fn duplicate(seed: Pair) {\
             let mut root: Pair = seed;\
             let r: &mut Pair = &mut root;\
             let Pair { left: a, right: b } = root;\
         }",
        "record Token {}\
         record Box { token: Token, count: I64 }\
         fn consume(seed: Box) {\
             let mut root: Box = seed;\
             let r: &mut Box = &mut root;\
             let Box { token: moved, count: copied } = root;\
         }",
    ] {
        let errors = compile(source)
            .expect_err("direct pattern production may not bypass a live replacement authority");
        assert!(
            has_diagnostic(&errors, DiagnosticKind::ReferencePermissionUnavailable),
            "missing direct pattern/reference conflict: {errors:?}"
        );
    }
}

#[test]
fn raw_address_coexists_with_shared_authority_but_not_replacement_authority() {
    compile(
        "fn shared(seed: I64) {\
             let x: I64 = seed;\
             let shared: &I64 = &x;\
             let pointer: raw I64 = raw &x;\
         }",
    )
    .expect("raw address formation uses the Shared direct-compatibility requirement");

    let errors = compile(
        "fn exclusive(seed: I64) {\
             let mut x: I64 = seed;\
             let replacement: &mut I64 = &mut x;\
             let pointer: raw I64 = raw &x;\
         }",
    )
    .expect_err("raw address formation conflicts with a live replacement authority");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::RawTargetSafeAuthorityConflict),
        "missing raw-address/replacement-authority conflict: {errors:?}"
    );
}

#[test]
fn shared_child_restricts_parent_to_shared_relative_capability() {
    compile(
        "fn f(r: &mut I64) {\
             let child: &I64 = &*r;\
             let copied_from_parent: I64 = *r;\
             let copied_from_child: I64 = *child;\
         }",
    )
    .expect("a Shared child leaves its replacement parent with Shared relative capability");
}

#[test]
fn replacement_child_suspends_parent_reference_access() {
    let errors = compile(
        "fn f(r: &mut I64) {\
             let child: &mut I64 = &mut *r;\
             let copied_from_parent: I64 = *r;\
         }",
    )
    .expect_err("a live replacement child suspends parent reference-relative access");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::ReferencePermissionUnavailable),
        "missing suspended-parent authority diagnostic: {errors:?}"
    );
}

#[test]
fn scoped_child_cleanup_restores_parent_for_break_and_continue_transfers() {
    compile(
        "fn break_case(flag: Bool, r: &mut I64) {\
             while flag {\
                 let child: &mut I64 = &mut *r;\
                 break;\
             }\
             *r = 7;\
         }\
         fn continue_case(flag: Bool, r: &mut I64) {\
             while flag {\
                 let child: &mut I64 = &mut *r;\
                 continue;\
             }\
         }",
    )
    .expect("loop transfer cleanup must end scoped child carriers before exact-state comparison");
}

#[test]
fn ended_branch_and_loop_authorities_do_not_create_false_state_mismatches() {
    compile(
        "fn f(flag: Bool, seed: I64) {\
             let mut x: I64 = seed;\
             if flag { { let temporary: &mut I64 = &mut x; } } else {}\
             while flag { { let temporary: &mut I64 = &mut x; } }\
             x = 9;\
         }",
    )
    .expect("dead authority identities and allocation counters are not normalized live state");
}

#[test]
fn conditional_rejects_unequal_live_reference_authority_state() {
    let errors = compile(
        "fn f(flag: Bool, r: &mut I64) {\
             if flag { let moved: &mut I64 = r; } else {}\
         }",
    )
    .expect_err("two normal conditional outcomes require equal live reference authority state");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::ConditionalReferenceStateMismatch),
        "missing conditional reference-state mismatch: {errors:?}"
    );
}

#[test]
fn divergence_needs_no_synthetic_restoration_before_a_potentially_diverging_call() {
    compile(
        "record Ticket { value: I64 }\
         fn diverge() { diverge(); }\
         fn f(r: &mut Ticket) {\
             let ticket: Ticket = *r;\
             diverge();\
             *r = ticket;\
         }",
    )
    .expect(
        "the possible normal path restores after the call; divergence needs no pre-call repair",
    );
}

#[test]
fn replacement_rhs_may_move_the_referent_when_the_destination_carrier_survives() {
    compile(
        "record Ticket { value: I64 }\
         fn f(r: &mut Ticket) { *r = *r; }",
    )
    .expect("source-first replacement may Move the referent before committing through live r");
}

#[test]
fn cyclic_record_safe_referent_still_uses_canonical_cycle_diagnostic() {
    let errors = compile(
        "record Cycle { next: Cycle }\
         fn f(r: &mut Cycle) {}",
    )
    .expect_err("cyclic ordinary records remain rejected by the canonical containment validator");

    assert!(
        has_diagnostic(&errors, DiagnosticKind::RecordContainmentCycle),
        "safe-referent shape inspection must not pre-empt the containment-cycle diagnostic: {errors:?}"
    );
}
