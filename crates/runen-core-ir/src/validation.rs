use crate::{
    BasicBlockId, Body, LocalId, Operand, Place, Statement, Terminator, TypeId, TypeKind, TypeTable,
};

/// Location within Core MIR used by validation and reference-execution diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirPoint {
    pub block: BasicBlockId,
    pub statement: Option<usize>,
}

/// Statically decidable reasons that raw Core MIR cannot be admitted for execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirValidationErrorKind {
    InvalidEntryBlock(BasicBlockId),
    InvalidTargetBlock(BasicBlockId),
    InvalidLocal(LocalId),
    InvalidProjection(Place),
    UnknownType(TypeId),
    RecursiveType(TypeId),
    TypeMismatch { expected: TypeId },
    CopyOfNonCopy(TypeId),
    AssignToImmutable(LocalId),
}

/// Diagnostic produced while admitting raw Core MIR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirValidationError {
    pub point: Option<MirPoint>,
    pub kind: MirValidationErrorKind,
}

/// Core MIR that passed all statically decidable checks represented by this revision.
///
/// The wrapper is intentionally constructible only through [`validate_body`]. Exposing
/// the body read-only preserves the admission guarantee until the wrapper is consumed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedBody {
    body: Body,
}

impl ValidatedBody {
    #[must_use]
    pub fn as_body(&self) -> &Body {
        &self.body
    }

    #[must_use]
    pub fn into_body(self) -> Body {
        self.body
    }
}

/// Validates raw Core MIR before it may enter an executable semantic oracle.
///
/// This boundary is deliberately limited to structural and statically decidable
/// constraints of the currently represented MIR. Path-state legality remains an
/// execution-time proving concern until a later owning slice explicitly moves it.
pub fn validate_body(body: Body) -> Result<ValidatedBody, MirValidationError> {
    validate_type_table(&body.types)?;
    validate_local_declarations(&body)?;

    if body.block(body.entry).is_none() {
        return Err(MirValidationError {
            point: None,
            kind: MirValidationErrorKind::InvalidEntryBlock(body.entry),
        });
    }

    for (block_index, block) in body.blocks.iter().enumerate() {
        let block_id = BasicBlockId(
            u32::try_from(block_index).expect("basic block index exceeds u32::MAX"),
        );

        for (statement_index, statement) in block.statements.iter().enumerate() {
            let point = MirPoint {
                block: block_id,
                statement: Some(statement_index),
            };
            validate_statement(&body, statement, &point)?;
        }

        if let Terminator::Goto(target) = &block.terminator
            && body.block(*target).is_none()
        {
            return Err(MirValidationError {
                point: Some(MirPoint {
                    block: block_id,
                    statement: None,
                }),
                kind: MirValidationErrorKind::InvalidTargetBlock(*target),
            });
        }
    }

    Ok(ValidatedBody { body })
}

fn validate_type_table(types: &TypeTable) -> Result<(), MirValidationError> {
    // 0 = unseen, 1 = active DFS ancestor, 2 = fully validated.
    let mut marks = vec![0_u8; types.len()];

    for root_index in 0..types.len() {
        if marks[root_index] != 0 {
            continue;
        }

        let root = TypeId(u32::try_from(root_index).expect("type index exceeds u32::MAX"));
        marks[root_index] = 1;
        let mut stack = vec![(root, 0_usize)];

        while let Some((current, next_field)) = stack.pop() {
            let current_index = current.0 as usize;
            let Some(definition) = types.get(current) else {
                return Err(type_error(MirValidationErrorKind::UnknownType(current)));
            };

            match &definition.kind {
                TypeKind::Scalar(_) => {
                    marks[current_index] = 2;
                }
                TypeKind::Struct(fields) if next_field == fields.len() => {
                    marks[current_index] = 2;
                }
                TypeKind::Struct(fields) => {
                    stack.push((current, next_field + 1));
                    let child = fields[next_field].ty;
                    let child_index = child.0 as usize;

                    if child_index >= types.len() {
                        return Err(type_error(MirValidationErrorKind::UnknownType(child)));
                    }

                    match marks[child_index] {
                        0 => {
                            marks[child_index] = 1;
                            stack.push((child, 0));
                        }
                        1 => {
                            return Err(type_error(MirValidationErrorKind::RecursiveType(child)));
                        }
                        2 => {}
                        _ => unreachable!("type-validation mark outside declared state space"),
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_local_declarations(body: &Body) -> Result<(), MirValidationError> {
    for local in &body.locals {
        if body.types.get(local.ty).is_none() {
            return Err(type_error(MirValidationErrorKind::UnknownType(local.ty)));
        }
    }
    Ok(())
}

fn validate_statement(
    body: &Body,
    statement: &Statement,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match statement {
        Statement::Init { dst, src } => {
            let expected = place_type(body, dst, point)?;
            validate_operand(body, src, expected, point)
        }
        Statement::Read { src } => {
            place_type(body, src, point)?;
            Ok(())
        }
        Statement::Assign { dst, src } => {
            let local = body.local(dst.local).ok_or_else(|| MirValidationError {
                point: Some(point.clone()),
                kind: MirValidationErrorKind::InvalidLocal(dst.local),
            })?;
            if !local.mutable {
                return Err(MirValidationError {
                    point: Some(point.clone()),
                    kind: MirValidationErrorKind::AssignToImmutable(dst.local),
                });
            }
            let expected = place_type(body, dst, point)?;
            validate_operand(body, src, expected, point)
        }
        Statement::Drop { place } => {
            place_type(body, place, point)?;
            Ok(())
        }
    }
}

fn validate_operand(
    body: &Body,
    operand: &Operand,
    expected: TypeId,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    match operand {
        Operand::Constant(value) => {
            if body.types.value_matches(expected, value) {
                Ok(())
            } else {
                Err(MirValidationError {
                    point: Some(point.clone()),
                    kind: MirValidationErrorKind::TypeMismatch { expected },
                })
            }
        }
        Operand::Move(src) => {
            let actual = place_type(body, src, point)?;
            require_type_match(actual, expected, point)
        }
        Operand::Copy(src) => {
            let actual = place_type(body, src, point)?;
            require_type_match(actual, expected, point)?;
            if !body.types.is_copy(actual) {
                return Err(MirValidationError {
                    point: Some(point.clone()),
                    kind: MirValidationErrorKind::CopyOfNonCopy(actual),
                });
            }
            Ok(())
        }
    }
}

fn require_type_match(
    actual: TypeId,
    expected: TypeId,
    point: &MirPoint,
) -> Result<(), MirValidationError> {
    if actual == expected {
        Ok(())
    } else {
        Err(MirValidationError {
            point: Some(point.clone()),
            kind: MirValidationErrorKind::TypeMismatch { expected },
        })
    }
}

fn place_type(body: &Body, place: &Place, point: &MirPoint) -> Result<TypeId, MirValidationError> {
    let local = body.local(place.local).ok_or_else(|| MirValidationError {
        point: Some(point.clone()),
        kind: MirValidationErrorKind::InvalidLocal(place.local),
    })?;

    body.types
        .project_type(local.ty, &place.projections)
        .ok_or_else(|| MirValidationError {
            point: Some(point.clone()),
            kind: MirValidationErrorKind::InvalidProjection(place.clone()),
        })
}

fn type_error(kind: MirValidationErrorKind) -> MirValidationError {
    MirValidationError { point: None, kind }
}
