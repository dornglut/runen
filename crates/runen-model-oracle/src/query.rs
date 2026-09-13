use crate::data::{BagValue, RelationValue, relation_from_support};

/// Execute the accepted `distinct : Bag<T> -> Relation<T>` Model relation.
///
/// The result is the input Bag's equivalence-class support. No representative,
/// insertion order, storage order, or multiplicity-one Bag is constructed.
pub fn distinct(input: &BagValue) -> RelationValue {
    relation_from_support(input)
}
