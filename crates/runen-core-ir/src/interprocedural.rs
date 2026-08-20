//! Staged program-level Core MIR used while migrating the one-body proving kernel.
//!
//! This module is intentionally temporary on the feature branch. Its types become
//! the crate's canonical body/function/program representation when the legacy
//! one-body API is removed before acceptance.

use crate::{
    BasicBlockId, Fault, FunctionId, LoanDecl, LocalDecl, LocalId, Operand, Place, Statement,
    TypeId, TypeTable,
};

/// End of one program-level Core basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    Goto(BasicBlockId),
    Call {
        function: FunctionId,
        arguments: Vec<Operand>,
        destination: Option<Place>,
        target: BasicBlockId,
    },
    Return(Option<Operand>),
    Fault(Fault),
}

/// Basic block in staged program-level Core MIR.
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

/// One function body using the program-wide type domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
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
    pub fn loan(&self, id: crate::LoanId) -> Option<&LoanDecl> {
        self.loans.get(id.0 as usize)
    }

    #[must_use]
    pub fn block(&self, id: BasicBlockId) -> Option<&BasicBlock> {
        self.blocks.get(id.0 as usize)
    }
}

/// One represented Core function entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<LocalId>,
    pub result: Option<TypeId>,
    pub body: Body,
}

impl Function {
    /// Returns the canonical parameter-slot type derived from its designated local.
    #[must_use]
    pub fn parameter_type(&self, slot: usize) -> Option<TypeId> {
        let local = *self.parameters.get(slot)?;
        Some(self.body.local(local)?.ty)
    }
}

/// One finite Core program with one shared type-identity domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub types: TypeTable,
    pub functions: Vec<Function>,
}

impl Program {
    #[must_use]
    pub fn function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(id.0 as usize)
    }
}
