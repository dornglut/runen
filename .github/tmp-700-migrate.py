from __future__ import annotations

from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text()


def write(path: str, content: str) -> None:
    target = ROOT / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content)


def replace_once(path: str, old: str, new: str) -> None:
    content = read(path)
    count = content.count(old)
    if count != 1:
        raise RuntimeError(f"{path}: expected one replacement anchor, found {count}: {old[:80]!r}")
    write(path, content.replace(old, new, 1))


def migrate_program_literals() -> None:
    pattern = re.compile(r"(?:\b|::)Program\s*\{")
    changed = 0
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        text = path.read_text()
        lines = text.splitlines(keepends=True)
        output: list[str] = []
        file_changed = False
        for index, line in enumerate(lines):
            output.append(line)
            if not pattern.search(line):
                continue
            stripped = line.lstrip()
            if stripped.startswith("//") or "struct Program" in line:
                continue
            following = "".join(lines[index + 1 : index + 6])
            if re.search(r"^\s*persistent\s*:", following, re.MULTILINE):
                continue
            indent = line[: len(line) - len(stripped)] + "    "
            output.append(f"{indent}persistent: vec![],\n")
            file_changed = True
        if file_changed:
            path.write_text("".join(output))
            changed += 1
    if changed < 80:
        raise RuntimeError(f"expected broad Program fixture migration, changed only {changed} files")
    print(f"migrated Program literals in {changed} Rust files")


# Core program representation.
replace_once(
    "crates/runen-core-ir/src/interprocedural.rs",
    """use crate::{
    BasicBlockId, CallableInterface, Fault, FunctionId, LoanDecl, LocalDecl, LocalId, Operand,
    Place, SafeReferenceResultContract, Statement, TypeId, TypeTable,
};
""",
    """use crate::{
    BasicBlockId, CallableInterface, Fault, FunctionId, LoanDecl, LocalDecl, LocalId, Operand,
    PersistentDecl, PersistentId, Place, SafeReferenceResultContract, Statement, TypeId, TypeTable,
};
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural.rs",
    """pub struct Program {
    pub types: TypeTable,
    pub functions: Vec<Function>,
}

impl Program {
    #[must_use]
    pub fn function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(id.0 as usize)
    }
}
""",
    """pub struct Program {
    pub types: TypeTable,
    pub persistent: Vec<PersistentDecl>,
    pub functions: Vec<Function>,
}

impl Program {
    #[must_use]
    pub fn persistent(&self, id: PersistentId) -> Option<&PersistentDecl> {
        self.persistent.get(id.0 as usize)
    }

    #[must_use]
    pub fn function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(id.0 as usize)
    }
}
""",
)

# Core persistent operands encode the bounded operation set. Shared permission is
# represented by the root variant itself rather than a caller-supplied permission.
replace_once(
    "crates/runen-core-ir/src/common.rs",
    """pub enum Operand {
    Constant(Value),
    /// Forms one captureless callable value naming an existing same-program function entity.
""",
    """pub enum Operand {
    Constant(Value),
    /// Non-consuming read of one complete execution-persistent scalar declaration.
    PersistentRead(PersistentId),
    /// Forms one Shared safe-reference authority to a complete persistent scalar root.
    PersistentSharedRoot(PersistentId),
    /// Forms one captureless callable value naming an existing same-program function entity.
""",
)

# Program-level persistent validation and operand typing.
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """    BasicBlockId, BorrowKind, CallableInterface, FunctionId, LoanDecl, LoanId, LocalId, Operand,
    Place, PlaceAccess, Projection, ReferenceAccess, ReferencePermission,
""",
    """    BasicBlockId, BorrowKind, CallableInterface, FunctionId, LoanDecl, LoanId, LocalId, Operand,
    PersistentId, Place, PlaceAccess, Projection, ReferenceAccess, ReferencePermission,
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """pub enum MirValidationErrorKind {
    InvalidFunction(FunctionId),
    InvalidEntryBlock(BasicBlockId),
""",
    """pub enum MirValidationErrorKind {
    InvalidFunction(FunctionId),
    InvalidPersistent(PersistentId),
    InvalidPersistentType {
        persistent: PersistentId,
        ty: TypeId,
    },
    PersistentInitializerTypeMismatch {
        persistent: PersistentId,
        ty: TypeId,
    },
    InvalidEntryBlock(BasicBlockId),
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """pub fn validate_program(program: Program) -> Result<ValidatedProgram, MirValidationError> {
    validate_type_table(&program.types)?;

    for (index, function) in program.functions.iter().enumerate() {
""",
    """pub fn validate_program(program: Program) -> Result<ValidatedProgram, MirValidationError> {
    validate_type_table(&program.types)?;
    validate_persistent_declarations(&program)?;

    for (index, function) in program.functions.iter().enumerate() {
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """fn validate_type_table(types: &TypeTable) -> Result<(), MirValidationError> {
""",
    """fn validate_persistent_declarations(program: &Program) -> Result<(), MirValidationError> {
    for (index, declaration) in program.persistent.iter().enumerate() {
        let persistent = PersistentId(
            u32::try_from(index).expect("persistent declaration index exceeds u32::MAX"),
        );
        let Some(definition) = program.types.get(declaration.ty) else {
            return Err(program_error(MirValidationErrorKind::UnknownType(
                declaration.ty,
            )));
        };
        let admitted = !definition.interior_mutable
            && matches!(
                definition.kind,
                TypeKind::Scalar(
                    ScalarType::Bool
                        | ScalarType::I8
                        | ScalarType::I16
                        | ScalarType::I32
                        | ScalarType::I64
                        | ScalarType::U8
                        | ScalarType::U16
                        | ScalarType::U32
                        | ScalarType::U64
                        | ScalarType::F16
                        | ScalarType::F32
                        | ScalarType::F64
                )
            );
        if !admitted {
            return Err(program_error(MirValidationErrorKind::InvalidPersistentType {
                persistent,
                ty: declaration.ty,
            }));
        }
        if !program
            .types
            .value_matches(declaration.ty, &declaration.initial)
        {
            return Err(program_error(
                MirValidationErrorKind::PersistentInitializerTypeMismatch {
                    persistent,
                    ty: declaration.ty,
                },
            ));
        }
    }
    Ok(())
}

fn validate_type_table(types: &TypeTable) -> Result<(), MirValidationError> {
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        Operand::FunctionValue(function) => {
""",
    """        Operand::PersistentRead(persistent) => {
            let declaration = program.persistent(*persistent).ok_or_else(|| {
                point_error(point, MirValidationErrorKind::InvalidPersistent(*persistent))
            })?;
            require_type_match(declaration.ty, expected, point)
        }
        Operand::PersistentSharedRoot(persistent) => {
            let declaration = program.persistent(*persistent).ok_or_else(|| {
                point_error(point, MirValidationErrorKind::InvalidPersistent(*persistent))
            })?;
            let Some((referent, permission)) = types.reference(expected) else {
                return Err(point_error(
                    point,
                    MirValidationErrorKind::TypeMismatch { expected },
                ));
            };
            if referent == declaration.ty && permission == ReferencePermission::Shared {
                Ok(())
            } else {
                Err(point_error(
                    point,
                    MirValidationErrorKind::TypeMismatch { expected },
                ))
            }
        }
        Operand::FunctionValue(function) => {
""",
)

# Persistent regions participate in the abstract safe-reference state, but are
# never places/locals and never participate in raw-pointer state.
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """enum ValidationRegionRoot {
    Local(LocalId),
    External(ExternalRegionId),
}
""",
    """enum ValidationRegionRoot {
    Local(LocalId),
    Persistent(PersistentId),
    External(ExternalRegionId),
}
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """    fn external(id: ExternalRegionId) -> Self {
        Self {
            root: ValidationRegionRoot::External(id),
            projections: Vec::new(),
        }
    }

    fn projected(&self, projections: &[Projection]) -> Self {
""",
    """    fn persistent(id: PersistentId) -> Self {
        Self {
            root: ValidationRegionRoot::Persistent(id),
            projections: Vec::new(),
        }
    }

    fn external(id: ExternalRegionId) -> Self {
        Self {
            root: ValidationRegionRoot::External(id),
            projections: Vec::new(),
        }
    }

    fn projected(&self, projections: &[Projection]) -> Self {
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExternalRegionState {
    ty: TypeId,
    state: ObjectState,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ActiveLoan {
""",
    """#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PersistentRegionState {
    ty: TypeId,
    state: ObjectState,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ExternalRegionState {
    ty: TypeId,
    state: ObjectState,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ActiveLoan {
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """struct ValidationState {
    current: BasicBlockId,
    locals: Vec<ObjectState>,
    external_regions: Vec<ExternalRegionState>,
""",
    """struct ValidationState {
    current: BasicBlockId,
    locals: Vec<ObjectState>,
    persistent_regions: Vec<PersistentRegionState>,
    external_regions: Vec<ExternalRegionState>,
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        locals: body
            .locals
            .iter()
            .map(|local| ObjectState::uninitialized(types, local.ty))
            .collect(),
        external_regions: Vec::new(),
""",
    """        locals: body
            .locals
            .iter()
            .map(|local| ObjectState::uninitialized(types, local.ty))
            .collect(),
        persistent_regions: program
            .persistent
            .iter()
            .map(|declaration| PersistentRegionState {
                ty: declaration.ty,
                state: ObjectState::fully_live_unknown(types, declaration.ty),
            })
            .collect(),
        external_regions: Vec::new(),
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        Operand::FunctionValue(_) => Ok(DefinedStep::Continue(ValidationValue::Scalar(
            ValidationScalar::NonPointer,
        ))),
""",
    """        Operand::PersistentRead(persistent) => {
            let target = ValidationRegion::persistent(*persistent);
            require_region_fully_live(state, &target, point)?;
            debug_assert!(state.reference_authorities.iter().all(|active| {
                active.as_ref().is_none_or(|active| {
                    !active.target.overlaps(&target)
                        || active.permission.alias_kind() == BorrowKind::Shared
                })
            }));
            Ok(DefinedStep::Continue(ValidationValue::Scalar(
                ValidationScalar::NonPointer,
            )))
        }
        Operand::PersistentSharedRoot(persistent) => {
            let target = ValidationRegion::persistent(*persistent);
            require_region_fully_live(state, &target, point)?;
            debug_assert!(state.reference_authorities.iter().all(|active| {
                active.as_ref().is_none_or(|active| {
                    !active.target.overlaps(&target)
                        || active.permission.alias_kind() == BorrowKind::Shared
                })
            }));
            let authority = allocate_reference_authority(
                &mut state.reference_authorities,
                ActiveReferenceAuthority {
                    target,
                    permission: ReferencePermission::Shared,
                    parent: None,
                    carriers: 1,
                    is_result_origin: false,
                },
            );
            Ok(DefinedStep::Continue(ValidationValue::Scalar(
                ValidationScalar::Reference(authority),
            )))
        }
        Operand::FunctionValue(_) => Ok(DefinedStep::Continue(ValidationValue::Scalar(
            ValidationScalar::NonPointer,
        ))),
""",
)

# General reference-region helpers recognize persistent storage. Write/take paths
# are unreachable for the Shared-only persistent root category.
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        ValidationRegionRoot::External(id) => {
            let external = &mut state.external_regions[id.0 as usize];
            let object = projected_state_mut(&mut external.state, &region.projections);
            write_validation_value(types, ty, object, value);
        }
""",
    """        ValidationRegionRoot::Persistent(_) => {
            unreachable!("Shared-only persistent storage cannot be a write target")
        }
        ValidationRegionRoot::External(id) => {
            let external = &mut state.external_regions[id.0 as usize];
            let object = projected_state_mut(&mut external.state, &region.projections);
            write_validation_value(types, ty, object, value);
        }
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        ValidationRegionRoot::External(id) => {
            let external = &mut state.external_regions[id.0 as usize];
            take_validation_value(
                types,
                ty,
                projected_state_mut(&mut external.state, &region.projections),
            )
        }
""",
    """        ValidationRegionRoot::Persistent(_) => {
            unreachable!("Shared-only persistent storage cannot be an ownership-move target")
        }
        ValidationRegionRoot::External(id) => {
            let external = &mut state.external_regions[id.0 as usize];
            take_validation_value(
                types,
                ty,
                projected_state_mut(&mut external.state, &region.projections),
            )
        }
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        ValidationRegionRoot::External(id) => {
            let index = id.0 as usize;
            let (external_regions, authorities) =
                (&state.external_regions, &mut state.reference_authorities);
            clone_validation_value(
                types,
                ty,
                projected_state(&external_regions[index].state, &region.projections),
                authorities,
            )
        }
""",
    """        ValidationRegionRoot::Persistent(id) => {
            let index = id.0 as usize;
            let (persistent_regions, authorities) =
                (&state.persistent_regions, &mut state.reference_authorities);
            clone_validation_value(
                types,
                ty,
                projected_state(&persistent_regions[index].state, &region.projections),
                authorities,
            )
        }
        ValidationRegionRoot::External(id) => {
            let index = id.0 as usize;
            let (external_regions, authorities) =
                (&state.external_regions, &mut state.reference_authorities);
            clone_validation_value(
                types,
                ty,
                projected_state(&external_regions[index].state, &region.projections),
                authorities,
            )
        }
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        ValidationRegionRoot::External(id) => state.external_regions[id.0 as usize].ty,
""",
    """        ValidationRegionRoot::Persistent(id) => state.persistent_regions[id.0 as usize].ty,
        ValidationRegionRoot::External(id) => state.external_regions[id.0 as usize].ty,
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        ValidationRegionRoot::External(id) => projected_state(
            &state.external_regions[id.0 as usize].state,
            &region.projections,
        ),
""",
    """        ValidationRegionRoot::Persistent(id) => projected_state(
            &state.persistent_regions[id.0 as usize].state,
            &region.projections,
        ),
        ValidationRegionRoot::External(id) => projected_state(
            &state.external_regions[id.0 as usize].state,
            &region.projections,
        ),
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        ValidationRegionRoot::External(id) => projected_state_mut(
            &mut state.external_regions[id.0 as usize].state,
            &region.projections,
        ),
""",
    """        ValidationRegionRoot::Persistent(id) => projected_state_mut(
            &mut state.persistent_regions[id.0 as usize].state,
            &region.projections,
        ),
        ValidationRegionRoot::External(id) => projected_state_mut(
            &mut state.external_regions[id.0 as usize].state,
            &region.projections,
        ),
""",
)
replace_once(
    "crates/runen-core-ir/src/interprocedural_validation.rs",
    """        ValidationRegionRoot::External(id) => state.external_regions[id.0 as usize].ty,
    };
    is_interior_mutable_from_root(types, root_ty, &region.projections)
}
""",
    """        ValidationRegionRoot::Persistent(_) => return false,
        ValidationRegionRoot::External(id) => state.external_regions[id.0 as usize].ty,
    };
    is_interior_mutable_from_root(types, root_ty, &region.projections)
}
""",
)

# Existing Program literals need the new empty sequence until actual static lowering
# is connected in the next dependency layer.
migrate_program_literals()

# Focused Core conformance for declaration domain, reads, Shared roots, and origin policy.
write(
    "crates/runen-core-ir/tests/persistent_storage.rs",
    r'''use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, LocalDecl, LocalId, MirLocation,
    MirValidationErrorKind, Operand, PersistentDecl, PersistentId, Place, Program,
    ReferencePermission, SafeReferenceResultContract, ScalarType, Statement, Terminator, TypeDef,
    TypeId, TypeTable, Value, validate_program,
};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

fn no_result_function() -> Function {
    Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    }
}

#[test]
fn persistent_declarations_keep_program_identity_separate_from_initial_value() {
    let first = PersistentId(0);
    let second = PersistentId(1);
    assert_ne!(first, second);

    let declaration = PersistentDecl::new(TypeId(7), Value::I64(42));
    assert_eq!(declaration.ty, TypeId(7));
    assert_eq!(declaration.initial, Value::I64(42));
}

#[test]
fn persistent_read_is_typed_and_non_consuming() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("sum", i64_ty, false)],
            vec![BasicBlock::new(
                vec![Statement::IntegerAdd {
                    dst: Place::local(LocalId(0)),
                    left: Operand::PersistentRead(PersistentId(0)),
                    right: Operand::PersistentRead(PersistentId(0)),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };

    validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(21))],
        functions: vec![function],
    })
    .expect("repeated persistent scalar reads are valid and non-consuming");
}

#[test]
fn persistent_declaration_rejects_non_language_scalar_and_interior_mutability() {
    let mut types = TypeTable::new();
    let tracked = types.push(TypeDef::scalar("Tracked", ScalarType::TrackedFixture));
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(tracked, Value::TrackedFixture(1))],
        functions: Vec::new(),
    })
    .expect_err("verification-only scalar is outside persistent storage domain");
    assert_eq!(error.location, MirLocation::Program);
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidPersistentType {
            persistent: PersistentId(0),
            ty: tracked,
        }
    );

    let mut types = TypeTable::new();
    let interior = types.push(TypeDef::scalar("I64", ScalarType::I64).with_interior_mutability());
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(interior, Value::I64(1))],
        functions: Vec::new(),
    })
    .expect_err("persistent Shared roots must not expose interior mutation");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::InvalidPersistentType {
            persistent: PersistentId(0),
            ty: interior,
        }
    );
}

#[test]
fn persistent_initializer_must_match_exact_declared_type() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::Bool(true))],
        functions: Vec::new(),
    })
    .expect_err("persistent initializer must match its exact declaration type");
    assert_eq!(
        error.kind,
        MirValidationErrorKind::PersistentInitializerTypeMismatch {
            persistent: PersistentId(0),
            ty: i64_ty,
        }
    );
}

#[test]
fn persistent_root_is_shared_only_and_can_have_multiple_carriers() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![
                LocalDecl::new("left", shared_i64, false),
                LocalDecl::new("right", shared_i64, false),
            ],
            vec![BasicBlock::new(
                vec![
                    Statement::Init {
                        dst: Place::local(LocalId(0)),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    },
                    Statement::Init {
                        dst: Place::local(LocalId(1)),
                        src: Operand::PersistentSharedRoot(PersistentId(0)),
                    },
                    Statement::Drop {
                        place: Place::local(LocalId(1)).into(),
                    },
                    Statement::Drop {
                        place: Place::local(LocalId(0)).into(),
                    },
                ],
                Terminator::Return(None),
            )],
        ),
    };

    validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(7))],
        functions: vec![function],
    })
    .expect("multiple Shared persistent roots are compatible");

    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let replace_i64 = types.push(TypeDef::reference(
        "ReplaceI64",
        i64_ty,
        ReferencePermission::ExclusiveReplace,
    ));
    let function = Function {
        name: "invalid".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", replace_i64, false)],
            vec![BasicBlock::new(
                vec![Statement::Init {
                    dst: Place::local(LocalId(0)),
                    src: Operand::PersistentSharedRoot(PersistentId(0)),
                }],
                Terminator::Return(None),
            )],
        ),
    };
    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(7))],
        functions: vec![function],
    })
    .expect_err("persistent root has no replacement-capable representation");
    assert_eq!(error.kind, MirValidationErrorKind::TypeMismatch { expected: replace_i64 });
}

#[test]
fn fresh_persistent_root_cannot_satisfy_shared_identity_result_contract() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let function = Function {
        name: "wrong_origin".into(),
        parameters: vec![LocalId(0)],
        result: Some(shared_i64),
        safe_reference_result_contract: SafeReferenceResultContract::SharedIdentity { origin: 0 },
        body: body(
            vec![LocalDecl::new("origin", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::PersistentSharedRoot(PersistentId(0)))),
            )],
        ),
    };

    let error = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(7))],
        functions: vec![function],
    })
    .expect_err("fresh persistent root is not the parameter-origin Shared identity");
    assert_eq!(error.kind, MirValidationErrorKind::SharedIdentityResultMismatch);
}

#[test]
fn missing_persistent_identity_is_rejected_at_operand_site() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            Vec::new(),
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::PersistentRead(PersistentId(3)))),
            )],
        ),
    };
    let error = validate_program(Program {
        types,
        persistent: Vec::new(),
        functions: vec![function],
    })
    .expect_err("persistent operands must name an existing declaration");
    assert_eq!(error.kind, MirValidationErrorKind::InvalidPersistent(PersistentId(3)));
}

#[test]
fn empty_persistent_sequence_preserves_existing_program_shape() {
    let mut types = TypeTable::new();
    let _ = types.push(TypeDef::scalar("I64", ScalarType::I64));
    validate_program(Program {
        types,
        persistent: Vec::new(),
        functions: vec![no_result_function()],
    })
    .expect("existing programs remain valid with an explicit empty persistent sequence");
}
''',
)

# Reference machine: execution-owned persistent storage precedes frames and uses
# ordinary StorageRegion/reference authority machinery for Shared transport.
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """    BasicBlockId, BorrowKind, FunctionId, LoanId, LocalId, NumericContract, Operand, Place,
    PlaceAccess, Projection, ReferenceAccess, ReferencePermission, ScalarType, Statement,
""",
    """    BasicBlockId, BorrowKind, FunctionId, LoanId, LocalId, NumericContract, Operand,
    PersistentId, Place, PlaceAccess, Projection, ReferenceAccess, ReferencePermission, ScalarType,
    Statement,
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """struct LocalStorage {
    instance: StorageInstanceId,
    state: ObjectState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveLoan {
""",
    """struct LocalStorage {
    instance: StorageInstanceId,
    state: ObjectState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PersistentStorage {
    instance: StorageInstanceId,
    ty: TypeId,
    state: ObjectState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveLoan {
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """pub struct Machine {
    program: ValidatedProgram,
    frames: Vec<Frame>,
""",
    """pub struct Machine {
    program: ValidatedProgram,
    persistent: Vec<PersistentStorage>,
    frames: Vec<Frame>,
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """        let mut machine = Self {
            program,
            frames: Vec::new(),
            reference_authorities: Vec::new(),
            next_activation: 1,
            next_storage_instance: 1,
            next_reference_authority: 0,
            verification_events: Vec::new(),
        };
        let frame = machine.create_frame(entry, Vec::new(), None);
        machine.frames.push(frame);
        Ok(machine)
    }

    /// Execute until normal return, defined fault, or detected undefined behavior.
""",
    """        let mut next_storage_instance = 1_u64;
        let mut persistent = Vec::with_capacity(program.as_program().persistent.len());
        for declaration in &program.as_program().persistent {
            let instance = StorageInstanceId(next_storage_instance);
            next_storage_instance = next_storage_instance
                .checked_add(1)
                .expect("reference storage-instance identity exhausted");
            let mut state = ObjectState::uninitialized(&program.as_program().types, declaration.ty);
            write_value(
                &program.as_program().types,
                declaration.ty,
                &mut state,
                RuntimeValue::from_constant(&declaration.initial),
            );
            persistent.push(PersistentStorage {
                instance,
                ty: declaration.ty,
                state,
            });
        }

        let mut machine = Self {
            program,
            persistent,
            frames: Vec::new(),
            reference_authorities: Vec::new(),
            next_activation: 1,
            next_storage_instance,
            next_reference_authority: 0,
            verification_events: Vec::new(),
        };
        let frame = machine.create_frame(entry, Vec::new(), None);
        machine.frames.push(frame);
        Ok(machine)
    }

    /// Verification-only dynamic storage identity for one execution-persistent declaration.
    #[must_use]
    pub fn persistent_storage_instance(&self, id: PersistentId) -> Option<StorageInstanceId> {
        self.persistent.get(id.0 as usize).map(|storage| storage.instance)
    }

    /// Execute until normal return, defined fault, or detected undefined behavior.
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """                    } else {
                        return Ok(ExecutionReport {
                            terminal: TerminalStatus::Returned,
                            result: result.map(RuntimeValue::into_observed_value),
                            verification_events: self.verification_events,
                        });
                    }
""",
    """                    } else {
                        self.cleanup_persistent();
                        return Ok(ExecutionReport {
                            terminal: TerminalStatus::Returned,
                            result: result.map(RuntimeValue::into_observed_value),
                            verification_events: self.verification_events,
                        });
                    }
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """                    while !self.frames.is_empty() {
                        let index = self.frames.len() - 1;
                        self.cleanup_frame(index);
                        self.frames.pop();
                    }
                    return Ok(ExecutionReport {
""",
    """                    while !self.frames.is_empty() {
                        let index = self.frames.len() - 1;
                        self.cleanup_frame(index);
                        self.frames.pop();
                    }
                    self.cleanup_persistent();
                    return Ok(ExecutionReport {
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """            Operand::FunctionValue(function) => Ok(RuntimeValue::Function(*function)),
""",
    """            Operand::PersistentRead(persistent) => {
                let target = self.persistent_region(*persistent);
                debug_assert!(self.reference_authorities.iter().all(|active| {
                    active.as_ref().is_none_or(|active| {
                        !active.target.overlaps(&target)
                            || active.permission.alias_kind() == BorrowKind::Shared
                    })
                }));
                let index = persistent.0 as usize;
                let ty = self.persistent[index].ty;
                let types = &self.program.as_program().types;
                let (persistent, authorities) =
                    (&self.persistent, &mut self.reference_authorities);
                Ok(clone_value(types, ty, &persistent[index].state, authorities))
            }
            Operand::PersistentSharedRoot(persistent) => {
                let target = self.persistent_region(*persistent);
                let authority = self.allocate_reference_authority(
                    target.clone(),
                    ReferencePermission::Shared,
                    None,
                );
                Ok(RuntimeValue::SafeReference(SafeReferenceValue { target, authority }))
            }
            Operand::FunctionValue(function) => Ok(RuntimeValue::Function(*function)),
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """    fn reference_read(&self, actor_frame: usize, src: &ReferenceAccess) {
        let resolved = self.resolve_reference_access(actor_frame, src);
        let (target_frame, target_place) = self.frame_place_for_storage_region(&resolved.target);
        assert!(
            place_state(&self.frames[target_frame].locals, &target_place).fully_live(),
            "validated reference read reaches a fully-live referent"
        );
    }
""",
    """    fn reference_read(&self, actor_frame: usize, src: &ReferenceAccess) {
        let resolved = self.resolve_reference_access(actor_frame, src);
        assert!(
            self.storage_region_fully_live(&resolved.target),
            "validated reference read reaches a fully-live referent"
        );
    }
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """            Operand::ReferenceCopy(src) => {
                let resolved = self.resolve_reference_access(frame_index, src);
                let (target_frame, target_place) =
                    self.frame_place_for_storage_region(&resolved.target);
                let types = &self.program.as_program().types;
                let state = place_state(&self.frames[target_frame].locals, &target_place);
                Ok(clone_value(
                    types,
                    resolved.selected_ty,
                    state,
                    &mut self.reference_authorities,
                ))
            }
""",
    """            Operand::ReferenceCopy(src) => {
                let resolved = self.resolve_reference_access(frame_index, src);
                Ok(self.clone_storage_region_value(&resolved.target, resolved.selected_ty))
            }
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """    fn storage_region(&self, frame_index: usize, place: &Place) -> StorageRegion {
        let local = self.frames[frame_index]
            .locals
            .get(place.local.0 as usize)
            .expect("validated Core MIR references only known local storage");
        StorageRegion {
            instance: local.instance,
            projections: place.projections.clone(),
        }
    }

    /// Resolve a safe-reference target across every currently active frame.
""",
    """    fn storage_region(&self, frame_index: usize, place: &Place) -> StorageRegion {
        let local = self.frames[frame_index]
            .locals
            .get(place.local.0 as usize)
            .expect("validated Core MIR references only known local storage");
        StorageRegion {
            instance: local.instance,
            projections: place.projections.clone(),
        }
    }

    fn persistent_region(&self, id: PersistentId) -> StorageRegion {
        let storage = self
            .persistent
            .get(id.0 as usize)
            .expect("validated persistent operand names an execution-owned declaration");
        StorageRegion {
            instance: storage.instance,
            projections: Vec::new(),
        }
    }

    fn storage_region_fully_live(&self, region: &StorageRegion) -> bool {
        if let Some(storage) = self
            .persistent
            .iter()
            .find(|storage| storage.instance == region.instance)
        {
            debug_assert!(region.projections.is_empty());
            return storage.state.fully_live();
        }
        let (frame, place) = self.frame_place_for_storage_region(region);
        place_state(&self.frames[frame].locals, &place).fully_live()
    }

    fn clone_storage_region_value(&mut self, region: &StorageRegion, ty: TypeId) -> RuntimeValue {
        if let Some(index) = self
            .persistent
            .iter()
            .position(|storage| storage.instance == region.instance)
        {
            debug_assert!(region.projections.is_empty());
            debug_assert_eq!(self.persistent[index].ty, ty);
            let types = &self.program.as_program().types;
            let (persistent, authorities) =
                (&self.persistent, &mut self.reference_authorities);
            return clone_value(types, ty, &persistent[index].state, authorities);
        }
        let (frame, place) = self.frame_place_for_storage_region(region);
        let types = &self.program.as_program().types;
        let state = place_state(&self.frames[frame].locals, &place);
        clone_value(types, ty, state, &mut self.reference_authorities)
    }

    /// Resolve a safe-reference target across every currently active frame.
""",
)
replace_once(
    "crates/runen-reference/src/interprocedural.rs",
    """    fn cleanup_frame(&mut self, frame_index: usize) {
""",
    """    fn cleanup_persistent(&mut self) {
        for storage in &self.persistent {
            assert!(
                !self.reference_authorities.iter().any(|active| active
                    .as_ref()
                    .is_some_and(|active| active.target.instance == storage.instance)),
                "validated Core terminal cleanup cannot end persistent storage targeted by a surviving reference authority"
            );
        }
        for storage in &mut self.persistent {
            debug_assert!(matches!(storage.state, ObjectState::Leaf(_)));
            match &mut storage.state {
                ObjectState::Leaf(leaf @ LeafState::Live(_)) => *leaf = LeafState::Dead,
                ObjectState::Leaf(LeafState::NeverInitialized | LeafState::Dead) => {
                    unreachable!("validated persistent storage remains live until terminal cleanup")
                }
                ObjectState::Aggregate(_) => {
                    unreachable!("validated persistent storage is scalar")
                }
            }
        }
    }

    fn cleanup_frame(&mut self, frame_index: usize) {
""",
)

write(
    "crates/runen-reference/tests/persistent_storage.rs",
    r'''use runen_core_ir::{
    BasicBlock, BasicBlockId, Body, Function, FunctionId, LocalDecl, LocalId, Operand,
    PersistentDecl, PersistentId, Place, Program, ReferenceAccess, ReferencePermission,
    SafeReferenceResultContract, ScalarType, Terminator, TypeDef, TypeTable, Value, validate_program,
};
use runen_reference::{Machine, ObservedValue, TerminalStatus};

fn body(locals: Vec<LocalDecl>, blocks: Vec<BasicBlock>) -> Body {
    Body {
        locals,
        loans: Vec::new(),
        entry: BasicBlockId(0),
        blocks,
    }
}

#[test]
fn persistent_instances_are_distinct_from_each_other_and_from_frame_storage() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: None,
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("local", i64_ty, false)],
            vec![BasicBlock::new(Vec::new(), Terminator::Return(None))],
        ),
    };
    let validated = validate_program(Program {
        types,
        persistent: vec![
            PersistentDecl::new(i64_ty, Value::I64(1)),
            PersistentDecl::new(i64_ty, Value::I64(1)),
        ],
        functions: vec![function],
    })
    .expect("valid persistent program");
    let machine = Machine::new(validated, FunctionId(0)).expect("zero-parameter entry");
    let first = machine
        .persistent_storage_instance(PersistentId(0))
        .expect("first persistent instance");
    let second = machine
        .persistent_storage_instance(PersistentId(1))
        .expect("second persistent instance");
    assert_ne!(first, second);
}

#[test]
fn repeated_persistent_reads_are_non_consuming() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let function = Function {
        name: "main".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("sum", i64_ty, false)],
            vec![BasicBlock::new(
                vec![runen_core_ir::Statement::IntegerAdd {
                    dst: Place::local(LocalId(0)),
                    left: Operand::PersistentRead(PersistentId(0)),
                    right: Operand::PersistentRead(PersistentId(0)),
                }],
                Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
            )],
        ),
    };
    let validated = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(21))],
        functions: vec![function],
    })
    .expect("valid repeated persistent reads");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined persistent read execution");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(42)));
}

#[test]
fn persistent_shared_root_crosses_activation_and_reads_through_reference() {
    let mut types = TypeTable::new();
    let i64_ty = types.push(TypeDef::scalar("I64", ScalarType::I64));
    let shared_i64 = types.push(TypeDef::reference(
        "SharedI64",
        i64_ty,
        ReferencePermission::Shared,
    ));
    let caller = Function {
        name: "caller".into(),
        parameters: Vec::new(),
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("result", i64_ty, false)],
            vec![
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Call {
                        function: FunctionId(1),
                        arguments: vec![Operand::PersistentSharedRoot(PersistentId(0))],
                        destination: Some(Place::local(LocalId(0))),
                        target: BasicBlockId(1),
                    },
                ),
                BasicBlock::new(
                    Vec::new(),
                    Terminator::Return(Some(Operand::Move(Place::local(LocalId(0)).into()))),
                ),
            ],
        ),
    };
    let callee = Function {
        name: "read".into(),
        parameters: vec![LocalId(0)],
        result: Some(i64_ty),
        safe_reference_result_contract: SafeReferenceResultContract::None,
        body: body(
            vec![LocalDecl::new("reference", shared_i64, false)],
            vec![BasicBlock::new(
                Vec::new(),
                Terminator::Return(Some(Operand::ReferenceCopy(ReferenceAccess::new(
                    Place::local(LocalId(0)),
                )))),
            )],
        ),
    };
    let validated = validate_program(Program {
        types,
        persistent: vec![PersistentDecl::new(i64_ty, Value::I64(73))],
        functions: vec![caller, callee],
    })
    .expect("persistent Shared reference can cross a call boundary");
    let report = Machine::new(validated, FunctionId(0))
        .expect("zero-parameter entry")
        .execute()
        .expect("defined cross-activation persistent reference execution");
    assert_eq!(report.terminal, TerminalStatus::Returned);
    assert_eq!(report.result, Some(ObservedValue::I64(73)));
}
''',
)

print("#700 persistent substrate migration prepared")
