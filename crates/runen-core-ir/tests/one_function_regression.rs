mod support {
    pub use ::runen_core_ir::{
        BasicBlockId, BorrowKind, Fault, Field, FunctionId, LoanDecl, LoanId, LocalDecl, LocalId,
        MirLocation, MirPoint, MirValidationError, MirValidationErrorKind, Operand, Place,
        PlaceAccess, Projection, ScalarType, Statement, StorageInstanceId, StorageRegion, TypeDef,
        TypeId, TypeKind, TypeTable, Value,
    };

    use ::runen_core_ir::{
        BasicBlock as ProgramBlock, Body as ProgramBody, Function, Program,
        Terminator as ProgramTerminator, ValidatedProgram, validate_program,
    };

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Terminator {
        Goto(BasicBlockId),
        Return,
        Fault(Fault),
    }

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

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Body {
        pub types: TypeTable,
        pub locals: Vec<LocalDecl>,
        pub loans: Vec<LoanDecl>,
        pub entry: BasicBlockId,
        pub blocks: Vec<BasicBlock>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ValidatedBody {
        body: Body,
        program: ValidatedProgram,
    }

    impl ValidatedBody {
        #[must_use]
        pub fn as_body(&self) -> &Body {
            &self.body
        }

        #[must_use]
        pub fn as_program(&self) -> &ValidatedProgram {
            &self.program
        }
    }

    pub fn validate_body(body: Body) -> Result<ValidatedBody, MirValidationError> {
        let program = validate_program(to_program(&body))?;
        Ok(ValidatedBody { body, program })
    }

    fn to_program(body: &Body) -> Program {
        Program {
            types: body.types.clone(),
            functions: vec![Function {
                name: "one_function_regression".into(),
                parameters: Vec::new(),
                result: None,
                body: ProgramBody {
                    locals: body.locals.clone(),
                    loans: body.loans.clone(),
                    entry: body.entry,
                    blocks: body
                        .blocks
                        .iter()
                        .map(|block| ProgramBlock {
                            statements: block.statements.clone(),
                            terminator: match &block.terminator {
                                Terminator::Goto(target) => ProgramTerminator::Goto(*target),
                                Terminator::Return => ProgramTerminator::Return(None),
                                Terminator::Fault(fault) => ProgramTerminator::Fault(fault.clone()),
                            },
                        })
                        .collect(),
                },
            }],
        }
    }
}

mod borrowing {
    use crate::support as runen_core_ir;
    include!("borrowing.rs");
}

mod borrowing_permissions {
    use crate::support as runen_core_ir;
    include!("borrowing_permissions.rs");
}

mod interior_mutability {
    use crate::support as runen_core_ir;
    include!("interior_mutability.rs");
}

mod interior_mutability_authority {
    use crate::support as runen_core_ir;
    include!("interior_mutability_authority.rs");
}

mod raw_pointer_assign {
    use crate::support as runen_core_ir;
    include!("raw_pointer_assign.rs");
}

mod raw_pointer_assign_overlap {
    use crate::support as runen_core_ir;
    include!("raw_pointer_assign_overlap.rs");
}

mod raw_pointer_move {
    use crate::support as runen_core_ir;
    include!("raw_pointer_move.rs");
}

mod raw_pointer_provenance {
    use crate::support as runen_core_ir;
    include!("raw_pointer_provenance.rs");
}

mod raw_pointer_provenance_authority {
    use crate::support as runen_core_ir;
    include!("raw_pointer_provenance_authority.rs");
}

mod raw_pointer_read {
    use crate::support as runen_core_ir;
    include!("raw_pointer_read.rs");
}

mod reborrowing {
    use crate::support as runen_core_ir;
    include!("reborrowing.rs");
}

mod validation {
    use crate::support as runen_core_ir;
    include!("validation.rs");
}
