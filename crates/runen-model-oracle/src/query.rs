use crate::data::{
    BagValue, FieldKey, FixtureError, RelationValue, Value, filter_record_field_equivalent,
    join_record_fields_equivalent, project_record_fields, relation_from_support,
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

/// Execute the accepted bounded record-field equivalence filter Model relation.
///
/// `field` and `value` are verification-only carriers for the accepted logical
/// field key and exact typed comparison value. The implementation filters the
/// Bag's private equivalence classes directly and never selects a record
/// representative. Invalid fixture invocations are not normative runtime query
/// faults.
pub fn filter_field_equivalent(
    input: &BagValue,
    field: FieldKey,
    value: &Value,
) -> Result<BagValue, FixtureError> {
    filter_record_field_equivalent(input, field, value)
}

/// Execute the accepted bounded disjoint-record field-equivalence join.
///
/// The field tokens are verification-only carriers for the two admitted join
/// keys. The implementation compares and merges private equivalence classes
/// directly; it does not select record representatives or expose tree order.
/// Invalid fixture invocations are not normative runtime query faults.
pub fn join_fields_equivalent(
    left: &BagValue,
    left_field: FieldKey,
    right: &BagValue,
    right_field: FieldKey,
) -> Result<BagValue, FixtureError> {
    join_record_fields_equivalent(left, left_field, right, right_field)
}

/// Execute the accepted `distinct : Bag<T> -> Relation<T>` Model relation.
///
/// The result is the input Bag's equivalence-class support. No representative,
/// insertion order, storage order, or multiplicity-one Bag is constructed.
pub fn distinct(input: &BagValue) -> RelationValue {
    relation_from_support(input)
}
