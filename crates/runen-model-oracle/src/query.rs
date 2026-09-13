use crate::data::{
    BagValue, FieldKey, FixtureError, RelationValue, project_record_fields, relation_from_support,
};

/// Execute the accepted bounded record-field projection Model relation.
///
/// `retained_fields` is a verification-only finite carrier for the semantic
/// field-key set. Its order is not Model order, and duplicate keys are
/// idempotent set membership. Invalid fixture invocations are rejected through
/// [`FixtureError`]; those errors are not normative runtime query faults.
pub fn project_fields(
    input: &BagValue,
    retained_fields: &[FieldKey],
) -> Result<BagValue, FixtureError> {
    project_record_fields(input, retained_fields)
}

/// Execute the accepted `distinct : Bag<T> -> Relation<T>` Model relation.
///
/// The result is the input Bag's equivalence-class support. No representative,
/// insertion order, storage order, or multiplicity-one Bag is constructed.
pub fn distinct(input: &BagValue) -> RelationValue {
    relation_from_support(input)
}
