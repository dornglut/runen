mod support {
    pub use ::runen_core_ir::{
        BasicBlockId, BorrowKind, Field, LoanDecl, LoanId, LocalDecl, LocalId,
        MirValidationErrorKind, Operand, Place, PlaceAccess, Projection, ScalarType, Statement,
        TypeDef, TypeId, TypeTable, Value,
    };

    use ::runen_core_ir::{
        BasicBlock as ProgramBlock, Body as ProgramBody, Function, MirLocation, Program,
        Terminator as ProgramTerminator, validate_program,
    };

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MirPoint {
        pub block: BasicBlockId,
        pub statement: Option<usize>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MirValidationError {
        pub point: Option<MirPoint>,
        pub kind: MirValidationErrorKind,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Terminator {
        Goto(BasicBlockId),
        Return,
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
    }

    impl ValidatedBody {
        #[must_use]
        pub fn as_body(&self) -> &Body {
            &self.body
        }
    }

    pub fn validate_body(body: Body) -> Result<ValidatedBody, MirValidationError> {
        validate_program(to_program(&body)).map_err(adapt_error)?;
        Ok(ValidatedBody { body })
    }

    fn adapt_error(error: ::runen_core_ir::MirValidationError) -> MirValidationError {
        let point = match error.location {
            MirLocation::Point(point) => Some(MirPoint {
                block: point.block,
                statement: point.statement,
            }),
            MirLocation::Program | MirLocation::Function(_) => None,
        };
        MirValidationError {
            point,
            kind: error.kind,
        }
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
