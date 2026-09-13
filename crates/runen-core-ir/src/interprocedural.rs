//! Canonical program-level Core MIR for the currently represented proving subset.

use crate::{
    BasicBlockId, CallableInterface, ExternalCallableId, Fault, FunctionId, LoanDecl, LocalDecl,
    LocalId, Operand, PersistentDecl, PersistentId, Place, SafeReferenceResultContract, Statement,
    TypeId, TypeTable,
};

/// End of one program-level Core basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    Goto(BasicBlockId),
    Branch {
        condition: Operand,
        true_target: BasicBlockId,
        false_target: BasicBlockId,
    },
    Call {
        function: FunctionId,
        arguments: Vec<Operand>,
        destination: Option<Place>,
        target: BasicBlockId,
    },
    ExternalCall {
        external: ExternalCallableId,
        arguments: Vec<Operand>,
        destination: Option<Place>,
        target: BasicBlockId,
    },
    /// Calls the function entity carried by `callee` under one exact callable type.
    IndirectCall {
        callable: TypeId,
        callee: Operand,
        arguments: Vec<Operand>,
        destination: Option<Place>,
        target: BasicBlockId,
    },
    Return(Option<Operand>),
    Fault(Fault),
}

/// Basic block in program-level Core MIR.
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
    pub safe_reference_result_contract: SafeReferenceResultContract,
    pub body: Body,
}

impl Function {
    /// Returns the canonical parameter-slot type derived from its designated local.
    #[must_use]
    pub fn parameter_type(&self, slot: usize) -> Option<TypeId> {
        let local = *self.parameters.get(slot)?;
        Some(self.body.local(local)?.ty)
    }

    /// Derives this function entity's exact representation-neutral callable interface.
    #[must_use]
    pub fn callable_interface(&self) -> Option<CallableInterface> {
        let parameters = self
            .parameters
            .iter()
            .map(|parameter| self.body.local(*parameter).map(|local| local.ty))
            .collect::<Option<Vec<_>>>()?;
        Some(CallableInterface {
            parameters,
            result: self.result,
            safe_reference_result_contract: self.safe_reference_result_contract,
        })
    }
}

/// One declaration-only external callable requirement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalCallableDecl {
    pub interface: CallableInterface,
}

impl ExternalCallableDecl {
    #[must_use]
    pub const fn new(interface: CallableInterface) -> Self {
        Self { interface }
    }
}

/// One finite Core program with one shared type-identity domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub types: TypeTable,
    pub persistent: Vec<PersistentDecl>,
    pub external_callables: Vec<ExternalCallableDecl>,
    pub functions: Vec<Function>,
}

impl Program {
    #[must_use]
    pub fn persistent(&self, id: PersistentId) -> Option<&PersistentDecl> {
        self.persistent.get(id.0 as usize)
    }

    #[must_use]
    pub fn external_callable(&self, id: ExternalCallableId) -> Option<&ExternalCallableDecl> {
        self.external_callables.get(id.0 as usize)
    }

    #[must_use]
    pub fn function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(id.0 as usize)
    }
}
