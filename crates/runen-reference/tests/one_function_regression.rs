mod core_support {
    pub use ::runen_core_ir::{
        BasicBlockId, BorrowKind, Fault, Field, FunctionId, LoanDecl, LoanId, LocalDecl, LocalId,
        MirValidationError, MirValidationErrorKind, Operand, Place, PlaceAccess, Projection,
        ScalarType, Statement, TypeDef, TypeId, TypeTable, Value,
    };

    use ::runen_core_ir::{
        BasicBlock as ProgramBlock, Body as ProgramBody, Function, Program,
        Terminator as ProgramTerminator, ValidatedProgram, validate_program,
    };

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Terminator {
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
        program: ValidatedProgram,
    }

    impl ValidatedBody {
        #[must_use]
        pub fn into_program(self) -> ValidatedProgram {
            self.program
        }
    }

    pub fn validate_body(body: Body) -> Result<ValidatedBody, MirValidationError> {
        let program = validate_program(to_program(&body))?;
        Ok(ValidatedBody { program })
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

mod reference_support {
    pub use ::runen_reference::{
        RawPointerValue, TerminalStatus, UndefinedBehaviorKind, VerificationWriteKind,
    };

    use crate::core_support::{FunctionId, ValidatedBody};
    use ::runen_reference::{Machine as ProgramMachine, VerificationEventKind as ProgramEventKind};

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum VerificationEvent {
        BorrowStart {
            loan: crate::core_support::LoanId,
            kind: crate::core_support::BorrowKind,
            place: crate::core_support::Place,
        },
        BorrowEnd(crate::core_support::LoanId),
        Read(crate::core_support::Place),
        RawRead {
            pointer: RawPointerValue,
            target: crate::core_support::Place,
        },
        RawMove {
            pointer: RawPointerValue,
            target: crate::core_support::Place,
        },
        Move(crate::core_support::Place),
        Copy(crate::core_support::Place),
        Write {
            place: crate::core_support::Place,
            kind: VerificationWriteKind,
        },
        AddressOf {
            place: crate::core_support::Place,
            pointer: RawPointerValue,
        },
        RawPointerMove {
            place: crate::core_support::Place,
            pointer: RawPointerValue,
        },
        RawPointerCopy {
            place: crate::core_support::Place,
            pointer: RawPointerValue,
        },
        DropTrackedFixture {
            place: crate::core_support::Place,
            id: u64,
        },
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ExecutionReport {
        pub terminal: TerminalStatus,
        pub verification_events: Vec<VerificationEvent>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct UndefinedBehavior {
        pub kind: UndefinedBehaviorKind,
        pub verification_events: Vec<VerificationEvent>,
    }

    pub struct Machine(ProgramMachine);

    impl Machine {
        #[must_use]
        pub fn new(body: ValidatedBody) -> Self {
            Self(
                ProgramMachine::new(body.into_program(), FunctionId(0))
                    .expect("one-function regression entry is zero-parameter"),
            )
        }

        pub fn execute(self) -> Result<ExecutionReport, UndefinedBehavior> {
            match self.0.execute() {
                Ok(report) => {
                    assert_eq!(
                        report.result, None,
                        "historical one-function regressions are resultless"
                    );
                    Ok(ExecutionReport {
                        terminal: report.terminal,
                        verification_events: report
                            .verification_events
                            .into_iter()
                            .map(|event| strip_event(event.kind))
                            .collect(),
                    })
                }
                Err(error) => Err(UndefinedBehavior {
                    kind: error.kind,
                    verification_events: error
                        .verification_events
                        .into_iter()
                        .map(|event| strip_event(event.kind))
                        .collect(),
                }),
            }
        }
    }

    fn strip_event(event: ProgramEventKind) -> VerificationEvent {
        match event {
            ProgramEventKind::BorrowStart { loan, kind, place } => {
                VerificationEvent::BorrowStart { loan, kind, place }
            }
            ProgramEventKind::BorrowEnd(loan) => VerificationEvent::BorrowEnd(loan),
            ProgramEventKind::Read(place) => VerificationEvent::Read(place),
            ProgramEventKind::RawRead { pointer, target } => {
                VerificationEvent::RawRead { pointer, target }
            }
            ProgramEventKind::RawMove { pointer, target } => {
                VerificationEvent::RawMove { pointer, target }
            }
            ProgramEventKind::Move(place) => VerificationEvent::Move(place),
            ProgramEventKind::Copy(place) => VerificationEvent::Copy(place),
            ProgramEventKind::Write { place, kind } => VerificationEvent::Write { place, kind },
            ProgramEventKind::AddressOf { place, pointer } => {
                VerificationEvent::AddressOf { place, pointer }
            }
            ProgramEventKind::RawPointerMove { place, pointer } => {
                VerificationEvent::RawPointerMove { place, pointer }
            }
            ProgramEventKind::RawPointerCopy { place, pointer } => {
                VerificationEvent::RawPointerCopy { place, pointer }
            }
            ProgramEventKind::DropTrackedFixture { place, id } => {
                VerificationEvent::DropTrackedFixture { place, id }
            }
        }
    }
}

macro_rules! regression_module {
    ($name:ident, $file:literal) => {
        mod $name {
            use crate::core_support as runen_core_ir;
            use crate::reference_support as runen_reference;
            include!($file);
        }
    };
}

regression_module!(a0_assign, "a0_assign.rs");
regression_module!(a0_state_transitions, "a0_state_transitions.rs");
regression_module!(a0_value_place, "a0_value_place.rs");
regression_module!(borrowing, "borrowing.rs");
regression_module!(borrowing_termination, "borrowing_termination.rs");
regression_module!(interior_mutability, "interior_mutability.rs");
regression_module!(raw_pointer_assign, "raw_pointer_assign.rs");
regression_module!(raw_pointer_assign_overlap, "raw_pointer_assign_overlap.rs");
regression_module!(raw_pointer_move, "raw_pointer_move.rs");
regression_module!(raw_pointer_provenance, "raw_pointer_provenance.rs");
regression_module!(raw_pointer_read, "raw_pointer_read.rs");
regression_module!(raw_pointer_read_alias, "raw_pointer_read_alias.rs");
regression_module!(reborrowing, "reborrowing.rs");
regression_module!(storage_identity, "storage_identity.rs");
