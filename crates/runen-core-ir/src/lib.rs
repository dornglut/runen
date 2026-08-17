#![forbid(unsafe_code)]
//! Typed semantic data structures for the Runen Core proving kernel.
//!
//! This crate deliberately contains no interpreter, backend, platform model,
//! or source-syntax concerns.

use std::fmt;

/// Stable-in-one-body identifier for a type definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub u32);

/// Stable-in-one-body identifier for a local storage slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalId(pub u32);

/// Stable-in-one-body identifier for a basic block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BasicBlockId(pub u32);

/// Scalar kinds supported by the A0 semantic kernel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarType {
    Bool,
    I64,
    /// A non-copy scalar with observable destruction, used by semantic tests.
    Tracked,
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
}

impl TypeDef {
    #[must_use]
    pub fn scalar(name: impl Into<String>, scalar: ScalarType) -> Self {
        Self {
            name: name.into(),
            kind: TypeKind::Scalar(scalar),
        }
    }

    #[must_use]
    pub fn structure(name: impl Into<String>, fields: Vec<Field>) -> Self {
        Self {
            name: name.into(),
            kind: TypeKind::Struct(fields),
        }
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

    /// A0 copyability is structural and compiler-known.
    #[must_use]
    pub fn is_copy(&self, ty: TypeId) -> bool {
        match self.get(ty).map(|def| &def.kind) {
            Some(TypeKind::Scalar(ScalarType::Bool | ScalarType::I64)) => true,
            Some(TypeKind::Scalar(ScalarType::Tracked)) | None => false,
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

    /// Checks whether a semantic value has the declared structural type.
    #[must_use]
    pub fn value_matches(&self, ty: TypeId, value: &Value) -> bool {
        let Some(def) = self.get(ty) else {
            return false;
        };

        match (&def.kind, value) {
            (TypeKind::Scalar(ScalarType::Bool), Value::Bool(_))
            | (TypeKind::Scalar(ScalarType::I64), Value::I64(_))
            | (TypeKind::Scalar(ScalarType::Tracked), Value::Tracked(_)) => true,
            (TypeKind::Struct(fields), Value::Struct(values)) => {
                fields.len() == values.len()
                    && fields
                        .iter()
                        .zip(values)
                        .all(|(field, value)| self.value_matches(field.ty, value))
            }
            _ => false,
        }
    }
}

/// Runtime-independent semantic value used by A0 MIR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    I64(i64),
    /// Identity whose destruction is visible in the reference trace.
    Tracked(u64),
    Struct(Vec<Value>),
}

/// Projection from a local place to a nested sub-place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Projection {
    Field(u32),
}

/// A typed MIR place: one local storage slot plus structural projections.
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

/// Source of an owned value for an initialization or assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operand {
    Constant(Value),
    /// Ownership transfer. The source becomes uninitialized/dead.
    Move(Place),
    /// Non-consuming owned duplication. Requires a copyable type.
    Copy(Place),
}

/// A0 Core MIR operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    /// First initialization of a place. Re-initialization uses `Assign`.
    Init { dst: Place, src: Operand },
    /// Non-consuming semantic observation of an initialized place.
    Read { src: Place },
    /// Mutable write/replacement/re-initialization, dropping any live old contents first.
    Assign { dst: Place, src: Operand },
    /// Explicit destruction of all currently live subobjects in the place.
    Drop { place: Place },
}

/// Defined fault reason for the A0 reference machine.
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

/// One executable Core body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    pub types: TypeTable,
    pub locals: Vec<LocalDecl>,
    pub entry: BasicBlockId,
    pub blocks: Vec<BasicBlock>,
}

impl Body {
    #[must_use]
    pub fn local(&self, id: LocalId) -> Option<&LocalDecl> {
        self.locals.get(id.0 as usize)
    }

    #[must_use]
    pub fn block(&self, id: BasicBlockId) -> Option<&BasicBlock> {
        self.blocks.get(id.0 as usize)
    }
}
