#![forbid(unsafe_code)]
//! Resolved and typed HIR for the currently represented Runen source subset.
//!
//! This crate consumes accepted source semantics and `runen-syntax` trees. It
//! owns no normative language semantics and performs no Core lowering or runtime
//! execution.

mod build;

use std::collections::BTreeSet;

use runen_syntax::{Parse, SyntaxErrorKind, TextRange, user_identifier_key};

/// Caller-supplied opaque source-module identity for one compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(u32);

impl ModuleId {
    #[must_use]
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }
}

/// One normalized source-unit import-target binding supplied by compilation input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportTarget {
    alias: String,
    pub module: ModuleId,
}

impl ImportTarget {
    /// Construct one import target from a concrete `UserIdentifier` spelling.
    ///
    /// Returns `None` when `alias` is not an accepted concrete user identifier.
    #[must_use]
    pub fn new(alias: &str, module: ModuleId) -> Option<Self> {
        Some(Self {
            alias: user_identifier_key(alias)?,
            module,
        })
    }

    #[must_use]
    pub fn alias(&self) -> &str {
        &self.alias
    }
}

/// One directly supplied syntax unit, its explicit module assignment, and its
/// source-unit-local external import-target bindings.
#[derive(Debug, Clone, Copy)]
pub struct SourceUnit<'a> {
    pub module: ModuleId,
    pub parse: &'a Parse,
    pub imports: &'a [ImportTarget],
}

impl<'a> SourceUnit<'a> {
    #[must_use]
    pub const fn new(module: ModuleId, parse: &'a Parse, imports: &'a [ImportTarget]) -> Self {
        Self {
            module,
            parse,
            imports,
        }
    }
}

/// Opaque per-compilation record handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordId(pub(crate) usize);

/// Opaque per-compilation function handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FunctionId(pub(crate) usize);

/// Opaque per-compilation nominal marker-trait handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MarkerTraitId(pub(crate) usize);

/// Semantic identity of one ordered generic function type-parameter slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeParameterId {
    pub function: FunctionId,
    pub index: usize,
}

/// Opaque per-compilation parameter/local binding handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingId(pub(crate) usize);

/// Source-module accessibility retained through typed source resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accessibility {
    ModulePrivate,
    Exported,
}

/// Assignment-mutability classification retained for represented ordinary locals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentMutability {
    Immutable,
    Mutable,
}

impl AssignmentMutability {
    pub(crate) const fn is_mutable(self) -> bool {
        matches!(self, Self::Mutable)
    }
}

/// Intrinsic source type identities represented by the current source subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntrinsicType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F16,
    F32,
    F64,
}

/// Exact non-reference referent identity admitted by the bounded safe-reference HIR slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferenceReferent {
    Intrinsic(IntrinsicType),
    Record(RecordId),
}

impl ReferenceReferent {
    /// The ordinary source type denoted by this non-recursive referent identity.
    #[must_use]
    pub const fn ty(self) -> Type {
        match self {
            Self::Intrinsic(intrinsic) => Type::Intrinsic(intrinsic),
            Self::Record(record) => Type::Record(record),
        }
    }
}

/// Source safe-reference permission represented by this bounded HIR slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReferencePermission {
    Shared,
    ExclusiveReplace,
}

/// Bounded source-semantic contract for a scalar Shared-reference result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafeReferenceResultContract {
    None,
    SharedIdentity { origin: usize },
    SharedDirectChild { origin: usize },
}

/// Exact non-pointer pointee identity admitted by the first raw-pointer HIR slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RawPointerPointee {
    Intrinsic(IntrinsicType),
    Record(RecordId),
}

impl RawPointerPointee {
    /// The ordinary source type denoted by this non-recursive pointee identity.
    #[must_use]
    pub const fn ty(self) -> Type {
        match self {
            Self::Intrinsic(intrinsic) => Type::Intrinsic(intrinsic),
            Self::Record(record) => Type::Record(record),
        }
    }
}

/// Resolved represented source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    Intrinsic(IntrinsicType),
    Record(RecordId),
    /// One abstract first-slice generic type expression, identified by declaring
    /// function and ordered slot rather than lexical spelling.
    Parameter(TypeParameterId),
    SafeReference {
        referent: ReferenceReferent,
        permission: ReferencePermission,
    },
    RawPointer(RawPointerPointee),
}

/// One retained ordered generic function type-parameter declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParameter {
    pub id: TypeParameterId,
    pub name: String,
    pub requirements: BTreeSet<MarkerTraitId>,
    pub location: SourceLocation,
}

/// One resolved nominal method-free marker trait declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerTrait {
    pub id: MarkerTraitId,
    pub module: ModuleId,
    pub name: String,
    pub accessibility: Accessibility,
    pub location: SourceLocation,
}

/// Exact concrete target admitted by the first marker implementation relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MarkerImplementationTarget {
    Intrinsic(IntrinsicType),
    Record(RecordId),
}

impl MarkerImplementationTarget {
    #[must_use]
    pub const fn ty(self) -> Type {
        match self {
            Self::Intrinsic(intrinsic) => Type::Intrinsic(intrinsic),
            Self::Record(record) => Type::Record(record),
        }
    }
}

/// One coherent explicit marker implementation proposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerImplementation {
    pub trait_id: MarkerTraitId,
    pub target: MarkerImplementationTarget,
    pub location: SourceLocation,
}

/// Retained source-semantic owned-value duplicability for one nominal record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Duplicability {
    NonDuplicable,
    Duplicable,
}

/// Source location tied to the supplied-unit list for this compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub unit: usize,
    pub range: TextRange,
}

/// Resolved owned-value production consequence retained for lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedUse {
    Duplicate,
    Consume,
}

/// Semantic sign of one materialized binary-floating literal value.
///
/// This is representation-neutral source/HIR information, not a physical sign bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFloatSign {
    Positive,
    Negative,
}

/// Representation-neutral semantic binary-floating value retained after literal materialization.
///
/// `significand` and `exponent` use the accepted semantic format equations. No NaN
/// member is introduced because the represented decimal floating literal family has no NaN producer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFloatValue {
    Zero(BinaryFloatSign),
    Subnormal {
        sign: BinaryFloatSign,
        significand: u64,
    },
    Normal {
        sign: BinaryFloatSign,
        significand: u64,
        exponent: i16,
    },
    Infinity(BinaryFloatSign),
}

/// Exact typed scalar literal value represented by the current source subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralValue {
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    F16(BinaryFloatValue),
    F32(BinaryFloatValue),
    F64(BinaryFloatValue),
}

/// One resolved record field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub accessibility: Accessibility,
    pub location: SourceLocation,
}

/// One resolved nominal record declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: RecordId,
    pub module: ModuleId,
    pub name: String,
    pub accessibility: Accessibility,
    pub duplicability: Duplicability,
    pub fields: Vec<Field>,
    pub location: SourceLocation,
}

pub(crate) fn type_is_duplicable_in_records(ty: Type, records: &[Record]) -> bool {
    match ty {
        Type::Intrinsic(_) | Type::RawPointer(_) => true,
        Type::Parameter(_) => false,
        Type::SafeReference {
            permission: ReferencePermission::Shared,
            ..
        } => true,
        Type::SafeReference {
            permission: ReferencePermission::ExclusiveReplace,
            ..
        } => false,
        Type::Record(record) => records[record.0].duplicability == Duplicability::Duplicable,
    }
}

/// One resolved function parameter and body-local binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub binding: BindingId,
    pub name: String,
    pub ty: Type,
    pub location: SourceLocation,
}

/// One typed HIR value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub ty: Type,
    pub kind: ValueKind,
    pub location: SourceLocation,
}

/// One record-construction field producer, retaining constructor source order
/// while naming the resolved declaration-field index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFieldValue {
    pub field: usize,
    pub value: Value,
}

/// One resolved record-pattern leaf binding, retained in depth-first pattern source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPatternBinding {
    /// Complete resolved structural field path from the top pattern root.
    pub fields: Vec<usize>,
    /// Explicit retained refutable-test index for one source-selected same-path
    /// bind-and-test target. Ordinary and irrefutable bindings retain `None`.
    pub composite_test: Option<usize>,
    pub binding: BindingId,
    pub name: String,
    pub ty: Type,
    pub ownership: OwnedUse,
}

/// Exact accepted refutable record-pattern test relation retained for lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordPatternTestKind {
    Equality,
    StrictUpperBound,
}

/// One resolved refutable record-pattern test in depth-first source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPatternTest {
    /// Complete resolved structural field path from the top pattern root.
    pub fields: Vec<usize>,
    /// Accepted source-semantic test relation for this leaf.
    pub kind: RecordPatternTestKind,
    /// Exact scalar source type selected from the resolved field declaration.
    pub ty: Type,
    /// Materialized semantic equality literal or strict-upper-bound value under `ty`.
    pub value: LiteralValue,
}

/// Source-selected remaining cleanup paths for one producer-backed record-pattern transient.
///
/// Paths are retained in canonical structural cleanup order. An empty path denotes
/// the complete transient value; an empty path list means no transient ownership remains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPatternTransientCleanup {
    pub paths: Vec<Vec<usize>>,
}

/// Source-selected remaining cleanup paths for one producer-backed field receiver transient.
///
/// This is deliberately distinct from record-pattern transient cleanup. Paths are
/// retained in canonical structural cleanup order. An empty path denotes the
/// complete receiver value; an empty path list means no receiver ownership remains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldReceiverTransientCleanup {
    pub paths: Vec<Vec<usize>>,
}

/// Resolved receiver category for one represented field-value use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValueReceiver {
    /// Existing function-local binding root. The exact receiver type is retained
    /// so lowering does not need to re-run source type selection.
    Binding { binding: BindingId, ty: Type },
    /// One accepted producer whose successful result becomes a separate
    /// field-receiver transient before field selection and cleanup.
    Producer {
        value: Box<Value>,
        cleanup: FieldReceiverTransientCleanup,
    },
}

/// Resolved scrutinee category for the represented bounded record-pattern relations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordPatternScrutinee {
    /// Accepted bare binding root with direct per-leaf ownership semantics.
    DirectRoot(BindingId),
    /// Existing value producer whose successful result is the pattern transient.
    /// `cleanup` is the source-selected remaining frontier after successful binding production.
    Producer {
        value: Value,
        cleanup: RecordPatternTransientCleanup,
    },
}

/// Retained source-semantic relation for represented Boolean equality values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanEqualityRelation {
    Equal,
    NotEqual,
}

/// Numeric contract selected for one governed numeric operation occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericContract {
    Standard,
    Reproducible,
    Fast,
}

/// Resolved producer for one typed HIR value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueKind {
    Literal(LiteralValue),
    BooleanNot {
        operand: Box<Value>,
    },
    IntegerNeg {
        operand: Box<Value>,
    },
    IntegerComplement {
        operand: Box<Value>,
    },
    IntegerAdd {
        left: Box<Value>,
        right: Box<Value>,
    },
    FloatAdd {
        contract: NumericContract,
        left: Box<Value>,
        right: Box<Value>,
    },
    IntegerSub {
        left: Box<Value>,
        right: Box<Value>,
    },
    FloatSub {
        contract: NumericContract,
        left: Box<Value>,
        right: Box<Value>,
    },
    IntegerMul {
        left: Box<Value>,
        right: Box<Value>,
    },
    FloatMul {
        contract: NumericContract,
        left: Box<Value>,
        right: Box<Value>,
    },
    FloatDiv {
        contract: NumericContract,
        left: Box<Value>,
        right: Box<Value>,
    },
    IntegerXor {
        left: Box<Value>,
        right: Box<Value>,
    },
    IntegerOr {
        left: Box<Value>,
        right: Box<Value>,
    },
    BooleanEquality {
        relation: BooleanEqualityRelation,
        left: Box<Value>,
        right: Box<Value>,
    },
    IntegerEq {
        operand_type: Type,
        left: Box<Value>,
        right: Box<Value>,
    },
    IntegerNe {
        operand_type: Type,
        left: Box<Value>,
        right: Box<Value>,
    },
    IntegerLt {
        operand_type: Type,
        left: Box<Value>,
        right: Box<Value>,
    },
    BooleanAnd {
        left: Box<Value>,
        right: Box<Value>,
    },
    ReferenceRoot {
        target: BindingId,
        fields: Vec<usize>,
        permission: ReferencePermission,
    },
    ReferenceReborrow {
        reference: BindingId,
        fields: Vec<usize>,
        permission: ReferencePermission,
    },
    ReferenceDereference {
        reference: BindingId,
        ownership: OwnedUse,
    },
    RawAddressRoot {
        target: BindingId,
    },
    RawMove {
        pointer: BindingId,
    },
    BindingUse {
        binding: BindingId,
        ownership: OwnedUse,
    },
    DirectCall {
        function: FunctionId,
        type_arguments: Vec<Type>,
        arguments: Vec<Value>,
    },
    RecordConstruction {
        record: RecordId,
        fields: Vec<RecordFieldValue>,
    },
    FieldValueUse {
        receiver: FieldValueReceiver,
        fields: Vec<usize>,
        ownership: OwnedUse,
    },
}

/// One source-selected remaining-ownership cleanup path for a binding.
///
/// `fields` contains resolved declaration-field indices. An empty field path
/// denotes the complete binding value. This is resolved HIR structure, not a
/// source place/lvalue abstraction or physical layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupPath {
    pub binding: BindingId,
    pub fields: Vec<usize>,
}

/// One resolved nested lexical block and its validated source-continuation facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub terminal_return: Option<Return>,
    pub normal_cleanup: Vec<CleanupPath>,
    pub has_normal_continuation: bool,
    pub location: SourceLocation,
}

/// One resolved body statement represented by the current source subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Statement {
    Local {
        binding: BindingId,
        name: String,
        ty: Type,
        mutability: AssignmentMutability,
        initializer: Value,
        location: SourceLocation,
    },
    RecordDestructure {
        record: RecordId,
        scrutinee: RecordPatternScrutinee,
        bindings: Vec<RecordPatternBinding>,
        location: SourceLocation,
    },
    RefutableRecordSelection {
        record: RecordId,
        scrutinee: RecordPatternScrutinee,
        tests: Vec<RecordPatternTest>,
        bindings: Vec<RecordPatternBinding>,
        /// Producer-backed mismatch cleanup for the complete pattern transient.
        /// Direct-root selections retain `None` because no pattern transient exists.
        mismatch_cleanup: Option<RecordPatternTransientCleanup>,
        success_block: Block,
        mismatch_block: Option<Box<Block>>,
        location: SourceLocation,
    },
    Assignment {
        target: BindingId,
        fields: Vec<usize>,
        value: Value,
        location: SourceLocation,
    },
    ReferenceAssign {
        reference: BindingId,
        value: Value,
        location: SourceLocation,
    },
    RawAssign {
        pointer: BindingId,
        value: Value,
        location: SourceLocation,
    },
    Call {
        function: FunctionId,
        type_arguments: Vec<Type>,
        arguments: Vec<Value>,
        location: SourceLocation,
    },
    Fault {
        location: SourceLocation,
    },
    Break {
        cleanup: Vec<CleanupPath>,
        location: SourceLocation,
    },
    Continue {
        cleanup: Vec<CleanupPath>,
        location: SourceLocation,
    },
    Block(Block),
    If {
        condition: Value,
        then_block: Block,
        else_block: Option<Box<Block>>,
        location: SourceLocation,
    },
    While {
        condition: Value,
        body: Block,
        location: SourceLocation,
    },
}

/// Terminal represented return, when concrete source contains one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Return {
    pub value: Option<Value>,
    pub location: SourceLocation,
}

/// Resolved and validated root function body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    pub statements: Vec<Statement>,
    pub terminal_return: Option<Return>,
    pub has_normal_continuation: bool,
}

/// One resolved source function entity and its typed body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub id: FunctionId,
    pub module: ModuleId,
    pub name: String,
    pub accessibility: Accessibility,
    pub type_parameters: Vec<TypeParameter>,
    pub parameters: Vec<Parameter>,
    pub result: Option<Type>,
    pub safe_reference_result_contract: SafeReferenceResultContract,
    pub body: Body,
    pub location: SourceLocation,
}

/// One source module represented in this typed compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    pub id: ModuleId,
    pub records: Vec<RecordId>,
    pub functions: Vec<FunctionId>,
    pub marker_traits: Vec<MarkerTraitId>,
}

/// Complete resolved and typed HIR for a valid represented source compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedCompilation {
    pub modules: Vec<Module>,
    pub records: Vec<Record>,
    pub functions: Vec<Function>,
    pub marker_traits: Vec<MarkerTrait>,
    pub marker_implementations: Vec<MarkerImplementation>,
}

impl TypedCompilation {
    #[must_use]
    pub fn record(&self, id: RecordId) -> &Record {
        &self.records[id.0]
    }

    #[must_use]
    pub fn function(&self, id: FunctionId) -> &Function {
        &self.functions[id.0]
    }

    #[must_use]
    pub fn marker_trait(&self, id: MarkerTraitId) -> &MarkerTrait {
        &self.marker_traits[id.0]
    }

    /// Whether the represented source type has non-consuming owned-value duplication.
    /// Abstract generic parameters deliberately have no positive duplicability evidence.
    #[must_use]
    pub fn type_is_duplicable(&self, ty: Type) -> bool {
        type_is_duplicable_in_records(ty, &self.records)
    }
}

/// Structured source-validation diagnostic category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    SyntaxError(SyntaxErrorKind),
    DuplicateModuleBinding,
    DuplicateImportAlias,
    MissingImportTarget,
    DuplicateImportTarget,
    SelfImport,
    ImportDeclarationConflict,
    UnresolvedName,
    InaccessibleBinding,
    ExpectedRecordType,
    ExpectedMarkerTrait,
    PrivateTypeInExportedSignature,
    PrivateTypeInExportedField,
    PrivateMarkerTraitInExportedSignature,
    DuplicateRecordField,
    RecordContainmentCycle,
    InvalidRecordDuplicabilitySelection,
    SafeReferenceField,
    RawPointerField,
    RawPointerParameter,
    RawPointerResult,
    ReplacementReferenceResult,
    MissingSharedReferenceResultOrigin,
    AmbiguousSharedReferenceResultOrigin,
    SharedReferenceResultOriginMismatch,
    MutableSafeReferenceLocal,
    InvalidSafeReferenceReferent {
        referent: Type,
        permission: ReferencePermission,
    },
    InvalidRawPointerPointee {
        pointee: Type,
    },
    DuplicateTypeParameter,
    DuplicateMarkerRequirement,
    InvalidGenericTypeParameterPosition,
    MissingGenericTypeArguments,
    UnexpectedGenericTypeArguments,
    GenericTypeArgumentCount {
        expected: usize,
        found: usize,
    },
    InvalidMarkerImplementationTarget,
    DuplicateMarkerImplementation,
    UnsatisfiedMarkerRequirement {
        required: MarkerTraitId,
        argument: Type,
    },
    DuplicateParameter,
    LocalShadowing,
    ExpectedValueBinding,
    UnavailableBinding,
    ExpectedSafeReference,
    ExpectedRawPointer,
    InvalidReplacementReferenceTarget,
    ReferencePermissionUnavailable,
    ReferenceRestorationRequired,
    ReferenceReplacementUnavailable,
    RawPointerTargetExtentMismatch,
    UnsafeOperationOutsideUnsafeBlock,
    RawMoveTargetUnavailable,
    RawTargetSafeAuthorityConflict,
    BorrowedAssignmentTarget,
    ImmutableAssignmentTarget,
    ExpectedFunction,
    ArgumentCount {
        expected: usize,
        found: usize,
    },
    TypeMismatch {
        expected: Type,
        found: Type,
    },
    IntegerNegationRequiresInteger {
        required: Type,
    },
    IntegerComplementRequiresInteger {
        required: Type,
    },
    AdditionRequiresIntegerOrFloating {
        required: Type,
    },
    NumericContractSelectionRequiresGovernedFloatingOperation {
        required: Type,
    },
    SubtractionRequiresIntegerOrFloating {
        required: Type,
    },
    MultiplicationRequiresIntegerOrFloating {
        required: Type,
    },
    DivisionRequiresFloating {
        required: Type,
    },
    IntegerXorRequiresInteger {
        required: Type,
    },
    IntegerOrRequiresInteger {
        required: Type,
    },
    EqualityOperandsUnanchored,
    EqualityOperandTypeConflict {
        left: Type,
        right: Type,
    },
    EqualityRequiresBooleanOrInteger {
        operand_type: Type,
    },
    IntegerOrderingOperandsUnanchored,
    IntegerOrderingOperandTypeConflict {
        left: Type,
        right: Type,
    },
    IntegerOrderingRequiresInteger {
        operand_type: Type,
    },
    IntegerLiteralRequiresInteger {
        required: Type,
    },
    IntegerLiteralOutOfRange {
        required: Type,
    },
    FloatingLiteralRequiresFloating {
        required: Type,
    },
    DuplicateRecordInitializer,
    UnknownRecordField,
    MissingRecordInitializer,
    DuplicateRecordPatternField,
    MissingRecordPatternField,
    DuplicatePatternBinding,
    RefutableRecordPatternRequiresRefutableTest,
    ExpectedRecordForFieldAccess,
    InaccessibleRecordField,
    UnavailableFieldValue,
    NoResultCallUsedAsValue,
    ResultCallUsedAsStatement,
    BooleanConjunctionOwnershipMismatch,
    ConditionalOwnershipMismatch,
    ConditionalPointerOriginMismatch,
    ConditionalReferenceStateMismatch,
    LoopOwnershipMismatch,
    LoopPointerOriginMismatch,
    LoopReferenceStateMismatch,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    BreakOwnershipMismatch,
    ContinueOwnershipMismatch,
    BreakPointerOriginMismatch,
    ContinuePointerOriginMismatch,
    BreakReferenceStateMismatch,
    ContinueReferenceStateMismatch,
    UnreachableStatement,
    MissingResultReturn,
    ExpectedResultValue,
    UnexpectedResultValue,
}

/// One structured source-validation diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub location: SourceLocation,
}

/// Build resolved and typed HIR for the represented source subset.
///
/// Syntax-dirty inputs are rejected before semantic validation. Semantic errors
/// produce diagnostics and no partial HIR.
pub fn build_typed_hir(units: &[SourceUnit<'_>]) -> Result<TypedCompilation, Vec<Diagnostic>> {
    build::build(units)
}
