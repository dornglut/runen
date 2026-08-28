#![forbid(unsafe_code)]
//! Executable reference semantics for validated Runen Core programs.

use runen_core_ir::{BinaryFloatValue, LoanId, StorageRegion};

/// Why a write occurred in verification instrumentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationWriteKind {
    Init,
    IntegerAdd,
    IntegerSub,
    IntegerMul,
    IntegerXor,
    IntegerOr,
    FloatAdd,
    FloatSub,
    FloatMul,
    Assign,
    InteriorAssign,
    RawAssign,
}

/// Reference-oracle observation of one binary-floating runtime value.
///
/// `NaNClass` records only membership in the semantic NaN value class. It does not
/// assign or expose a NaN member identity, sign, payload, quiet/signaling state,
/// physical encoding, or Runen equality relation. Derived structural equality is
/// verification-oracle equality of this observation representation only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservedBinaryFloatValue {
    Represented(BinaryFloatValue),
    NaNClass,
}

/// Reference-oracle observation of one returned runtime value.
///
/// This is deliberately separate from `runen_core_ir::Value`: Core `Value` is the
/// NaN-free fabricable constant carrier, while execution may produce a floating
/// NaN-class value. Structural equality on this verification representation does
/// not define Runen language equality, hashing, ABI identity, or physical layout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservedValue {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F16(ObservedBinaryFloatValue),
    F32(ObservedBinaryFloatValue),
    F64(ObservedBinaryFloatValue),
    /// Verification-only fixture identity whose destruction is visible in traces.
    TrackedFixture(u64),
    Struct(Vec<ObservedValue>),
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

mod floating;
mod interprocedural;
pub use interprocedural::{
    ActivationId, EntryError, ExecutionReport, Machine, TerminalStatus, UndefinedBehavior,
    VerificationEvent, VerificationEventKind,
};
