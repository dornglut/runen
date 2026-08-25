#![forbid(unsafe_code)]
//! Executable reference semantics for validated Runen Core programs.

use runen_core_ir::{LoanId, StorageRegion};

/// Why a write occurred in verification instrumentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationWriteKind {
    Init,
    IntegerAdd,
    IntegerSub,
    IntegerMul,
    IntegerXor,
    Assign,
    InteriorAssign,
    RawAssign,
}

/// Verification representation of one symbolic raw-pointer value.
///
/// `target` identifies the structural storage region selected when the pointer was
/// formed. The currently defined provenance root is represented once by
/// `target.instance`; the oracle does not materialize a duplicate provenance field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawPointerValue {
    pub target: StorageRegion,
}

/// Verification-only classification of detected undefined behavior.
///
/// These variants diagnose concrete unsafe-precondition violations in the reference
/// oracle. They are not Runen-observable outcomes and do not define a complete UB
/// taxonomy for the language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UndefinedBehaviorKind {
    RawReadTargetNotLive { target: StorageRegion },
    RawReadConflictsWithExclusiveLoan { target: StorageRegion, loan: LoanId },
    RawMoveTargetNotLive { target: StorageRegion },
    RawMoveConflictsWithLoan { target: StorageRegion, loan: LoanId },
    RawAssignConflictsWithLoan { target: StorageRegion, loan: LoanId },
}

mod interprocedural;
pub use interprocedural::{
    ActivationId, EntryError, ExecutionReport, Machine, TerminalStatus, UndefinedBehavior,
    VerificationEvent, VerificationEventKind,
};
