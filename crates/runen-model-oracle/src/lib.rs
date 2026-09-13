#![forbid(unsafe_code)]
//! Verification-only executable oracle for accepted Runen Model relations.
//!
//! This package is not Runen source syntax, compiler Model IR, a planner,
//! storage engine, runtime, database system, incremental engine, or normative
//! semantic owner. Its finite tokens and deterministic internal ordering exist
//! only to make accepted Model contracts executable in conformance tests.

mod data;
mod observation;
mod query;

pub use data::{
    BagValue, FieldKey, FixtureError, FloatFormat, FloatValue, LogicalType, NaNRealizationId,
    RecordType, RelationValue, SequenceValue, Value, model_equivalent,
};
pub use observation::{ObservationFixtureError, ObservationId, ObservedBagDomain, StateDomainId};
pub use query::{
    bag_cardinality, distinct, filter_field_equivalent, group_by_fields, join_fields_equivalent,
    project_fields,
};
