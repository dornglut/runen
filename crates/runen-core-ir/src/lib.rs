#![forbid(unsafe_code)]
//! Typed semantic data structures for the Runen Core proving kernel.
//!
//! This crate deliberately contains no interpreter, backend, platform model,
//! or source-syntax concerns.

mod validation;

use std::fmt;

pub use validation::{
    MirPoint, MirValidationError, MirValidationErrorKind, ValidatedBody, validate_body,
};

/// Stable-in-one-body identifier for a type definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

/// Stable-in-one-body identifier for a local storage declaration.
///
/// A `LocalId` identifies MIR syntax. It is not the dynamic identity of one
/// execution's local storage extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalId(pub u32);

/// Execution-scoped semantic identity for one dynamic root storage extent.
///
/// The numeric field is an oracle/proving representation only. Runen programs do
/// not observe this number, and future storage owners need not derive it from a
/// `LocalId` or any physical address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StorageInstanceId(pub u64);

/// One structural region inside a dynamic root storage instance.
///
/// The projection path is semantic structure, not a byte offset or ABI layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StorageRegion {
    pub instance: StorageInstanceId,
    pub projections: Vec<Projection>,
}

/// Stable-in-one-body identifier for a semantic loan declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoanId(pub u32);

/// Stable-in-one-body identifier for a basic block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BasicBlockId(pub u32);

/// Scalar or leaf kinds represented by the Core proving kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarType {
    Bool,
    I64,
    /// Capability-neutral raw pointer to a pointee type.
    ///
    /// The current Core slice defines formation, ordinary value transport, and one
    /// non-consuming `RawRead`; broader pointer access remains outside this type.
    /// The pointee edge is semantic indirection rather than structural containment.
    RawPointer(TypeId),
    /// Verification-only non-copy scalar used to make destruction observable to tests.
    /// This is not a Runen language scalar primitive.
    TrackedFixture,
}

/// A field in a structural Core type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub ty: TypeId,
}

impl Field {
    #[must_use]
    pub fn new(name: impl Into<String>, ty: TypeId) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

/// A type definition known to the typed Core MIR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDef {
    pub name: String,
    pub kind: TypeKind,
    /// Whether storage whose structural path enters this type is an
    /// interior-mutable region for the dedicated Core interior-assignment operation.
    ///
    /// This is semantic proving-kernel metadata. It is independent of containing
    /// local assignment mutability and does not define source-language syntax.
    pub interior_mutable: bool,
}

impl TypeDef {
    #[must_use]
    pub fn scalar(name: impl Into<String>, scalar: ScalarType) -> Self {
        Self {
            name: name.into(),
            kind: TypeKind::Scalar(scalar),
            interior_mutable: false,
        }
    }

    #[must_use]
    pub fn raw_pointer(name: impl Into<String>, pointee: TypeId) -> Self {
        Self::scalar(name, ScalarType::RawPointer(pointee))
    }

    #[must_use]
    pub fn structure(name: impl Into<String>, fields: Vec<Field>) -> Self {
        Self {
            name: name.into(),
            kind: TypeKind::Struct(fields),
            interior_mutable: false,
        }
    }

    /// Marks storage of this type and its structural descendants as an
    /// interior-mutable region in the Core proving model.
    #[must_use]
    pub fn with_interior_mutability(mut self) -> Self {
        self.interior_mutable = true;
        self
    }
}

/// Structural shape of a Core type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Scalar(ScalarType),
    Struct(Vec<Field>),
}

/// Type definitions referenced by a body.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TypeTable {
    defs: Vec<TypeDef>,
}

impl TypeTable {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, def: TypeDef) -> TypeId {
        let id = TypeId(u32::try_from(self.defs.len()).expect("type table exceeds u32::MAX"));
        self.defs.push(def);
        id
    }

    #[must_use]
    pub fn get(&self, id: TypeId) -> Option<&TypeDef> {
        self.defs.get(id.0 as usize)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    #[must_use]
    pub fn raw_pointer_pointee(&self, ty: TypeId) -> Option<TypeId> {
        match &self.get(ty)?.kind {
            TypeKind::Scalar(ScalarType::RawPointer(pointee)) => Some(*pointee),
            TypeKind::Scalar(_) | TypeKind::Struct(_) => None,
        }
    }

    /// Copyability in the represented Core subset is structural and compiler-known.
    #[must_use]
    pub fn is_copy(&self, ty: TypeId) -> bool {
        match self.get(ty).map(|def| &def.kind) {
            Some(TypeKind::Scalar(
                ScalarType::Bool | ScalarType::I64 | ScalarType::RawPointer(_),
            )) => true,
            Some(TypeKind::Scalar(ScalarType::TrackedFixture)) | None => false,
            Some(TypeKind::Struct(fields)) => fields.iter().all(|field| self.is_copy(field.ty)),
        }
    }

    /// Resolves the type reached by a field-projection sequence.
    #[must_use]
    pub fn project_type(&self, root: TypeId, projections: &[Projection]) -> Option<TypeId> {
        let mut current = root;
        for projection in projections {
            match projection {
                Projection::Field(index) => {
                    let TypeKind::Struct(fields) = &self.get(current)?.kind else {
                        return None;
                    };
                    current = fields.get(*index as usize)?.ty;
                }
            }
        }
        Some(current)
    }

    /// Checks whether a MIR constant has the declared structural type.
    ///
    /// Non-null raw pointers are intentionally absent from [`Value`] and therefore
    /// cannot be fabricated as constants. They are runtime semantic values produced
    /// only by [`Operand::AddressOf`] in the current slice.
    #[must_use]
    pub fn value_matches(&self, ty: TypeId, value: &Value) -> bool {
        let Some(def) = self.get(ty) else {
            return false;
        };

        match (&def.kind, value) {
            (TypeKind::Scalar(ScalarType::Bool), Value::Bool(_))
            | (TypeKind::Scalar(ScalarType::I64), Value::I64(_))
            | (TypeKind::Scalar(ScalarType::TrackedFixture), Value::TrackedFixture(_)) => true,
            (TypeKind::Struct(fields), Value::Struct(values)) => {
                fields.len() == values.len()
                    && fields
                        .iter()
                        .zip(values)
                        .all(|(field, value)| self.value_matches(field.ty, value))
            }
            (TypeKind::Scalar(ScalarType::RawPointer(_)), _) => false,
            _ => false,
        }
    }
}

/// Constant value representation used by Core proving MIR and verification fixtures.
///
/// Dynamic raw-pointer values are deliberately not represented here because their
/// storage-instance provenance exists only during execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    I64(i64),
    /// Verification-only fixture identity whose destruction is visible in the oracle trace.
    /// This is not a Runen language value primitive.
    TrackedFixture(u64),
    Struct(Vec<Value>),
}

/// Projection from a local place or loan root to a nested sub-place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Projection {
    Field(u32),
}

/// A typed MIR place: one local storage declaration plus structural projections.
///
/// `Place` is static proving-MIR identity. Executing a place resolves it to a
/// [`StorageRegion`] within the current dynamic storage instance.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Place {
    pub local: LocalId,
    pub projections: Vec<Projection>,
}

impl Place {
    #[must_use]
    pub fn local(local: LocalId) -> Self {
        Self {
            local,
            projections: Vec::new(),
        }
    }

    #[must_use]
    pub fn field(mut self, index: u32) -> Self {
        self.projections.push(Projection::Field(index));
        self
    }

    #[must_use]
    pub fn with_projection(&self, projection: Projection) -> Self {
        let mut projected = self.clone();
        projected.projections.push(projection);
        projected
    }

    /// Whether two structural places denote overlapping storage.
    ///
    /// The current field-only place model overlaps exactly when both places have
    /// the same local declaration root and either projection sequence is a prefix
    /// of the other. Dynamic storage-instance identity is resolved during execution.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.local == other.local
            && (self.projections.starts_with(&other.projections)
                || other.projections.starts_with(&self.projections))
    }
}

impl fmt::Display for Place {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "_{}", self.local.0)?;
        for projection in &self.projections {
            match projection {
                Projection::Field(index) => write!(formatter, ".{index}")?,
            }
        }
        Ok(())
    }
}

/// Shared or exclusive semantic access permission for one root place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Exclusive,
}

/// Declaration for one stable MIR loan identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoanDecl {
    pub name: String,
    pub ty: TypeId,
}

impl LoanDecl {
    #[must_use]
    pub fn new(name: impl Into<String>, ty: TypeId) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

/// How a Core operation is authorized to reach storage.
///
/// `Direct` names storage through its local place. `Loan` names a place relative
/// to an active semantic loan root. This is proving-MIR access authority, not a
/// source-language reference value, raw-pointer value, or provenance identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlaceAccess {
    Direct(Place),
    Loan {
        loan: LoanId,
        projections: Vec<Projection>,
    },
}

impl PlaceAccess {
    #[must_use]
    pub fn direct(place: Place) -> Self {
        Self::Direct(place)
    }

    #[must_use]
    pub fn loan(loan: LoanId) -> Self {
        Self::Loan {
            loan,
            projections: Vec::new(),
        }
    }

    #[must_use]
    pub fn field(mut self, index: u32) -> Self {
        match &mut self {
            Self::Direct(place) => place.projections.push(Projection::Field(index)),
            Self::Loan { projections, .. } => projections.push(Projection::Field(index)),
        }
        self
    }
}

impl From<Place> for PlaceAccess {
    fn from(place: Place) -> Self {
        Self::Direct(place)
    }
}

/// Local storage declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalDecl {
    pub name: String,
    pub ty: TypeId,
    pub mutable: bool,
}

impl LocalDecl {
    #[must_use]
    pub fn new(name: impl Into<String>, ty: TypeId, mutable: bool) -> Self {
        Self {
            name: name.into(),
            ty,
            mutable,
        }
    }
}

/// Source of an owned runtime value for an initialization or assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Constant(Value),
    /// Ownership transfer. The source stored-value lifetime ends.
    Move(PlaceAccess),
    /// Non-consuming owned duplication. Requires a copyable type.
    Copy(PlaceAccess),
    /// Forms a non-null symbolic raw pointer to existing storage.
    ///
    /// Formation requires shared alias authorization for the selected storage but
    /// does not read the pointee value and therefore does not require it to be Live.
    AddressOf(PlaceAccess),
}

/// Core MIR operations represented by the current proving kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    /// First initialization of direct storage. Re-initialization uses `Assign`.
    Init { dst: Place, src: Operand },
    /// Begins a root borrow from direct access or a child borrow from loan access.
    Borrow {
        loan: LoanId,
        kind: BorrowKind,
        src: PlaceAccess,
    },
    /// Ends one active borrow interval. Active children must end first.
    EndBorrow { loan: LoanId },
    /// Non-consuming Core read. The current proving MIR discards the resulting value.
    ///
    /// Readability and its lack of ownership transfer are semantic. Recording the
    /// operation in reference-oracle instrumentation is verification-only.
    Read { src: PlaceAccess },
    /// Unsafe non-consuming read through a stored raw-pointer value.
    ///
    /// Language validation checks the pointer-value access itself. Concrete pointee
    /// liveness and active-loan compatibility are unsafe execution preconditions.
    /// The current proving MIR discards the read result and does not create a
    /// general dereferenced-place representation.
    RawRead { pointer: PlaceAccess },
    /// Ordinary mutable write/replacement/re-initialization.
    ///
    /// This requires exclusive alias authority and containing-local assignment
    /// mutability; interior mutability does not weaken those ordinary rules.
    Assign { dst: PlaceAccess, src: Operand },
    /// Interior-mutable write/replacement/re-initialization.
    ///
    /// This uses the ordinary replacement lifecycle but requires an explicit
    /// interior-mutable storage region and only shared alias authority. It does not
    /// grant ordinary assignment mutability or consuming loan authority.
    InteriorAssign { dst: PlaceAccess, src: Operand },
    /// Explicit destruction of all currently live subobjects in the accessed place.
    Drop { place: PlaceAccess },
}

/// Defined fault reason for the reference machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    pub code: String,
}

impl Fault {
    #[must_use]
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

/// End of a basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    Goto(BasicBlockId),
    Return,
    Fault(Fault),
}

/// Basic block in typed Core MIR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

impl BasicBlock {
    #[must_use]
    pub fn new(statements: Vec<Statement>, terminator: Terminator) -> Self {
        Self {
            statements,
            terminator,
        }
    }
}

/// One raw Core body before MIR validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    pub types: TypeTable,
    pub locals: Vec<LocalDecl>,
    pub loans: Vec<LoanDecl>,
    pub entry: BasicBlockId,
    pub blocks: Vec<BasicBlock>,
}

impl Body {
    #[must_use]
    pub fn local(&self, id: LocalId) -> Option<&LocalDecl> {
        self.locals.get(id.0 as usize)
    }

    #[must_use]
    pub fn loan(&self, id: LoanId) -> Option<&LoanDecl> {
        self.loans.get(id.0 as usize)
    }

    #[must_use]
    pub fn block(&self, id: BasicBlockId) -> Option<&BasicBlock> {
        self.blocks.get(id.0 as usize)
    }
}
