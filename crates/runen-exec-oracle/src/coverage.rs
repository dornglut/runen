/// Mechanical verification helper for exact unordered coverage of distinct fixture identities.
///
/// Slice order is ignored. Duplicate required identities are rejected as invalid fixture input.
pub(crate) fn has_exact_unique_coverage<T: Eq>(required: &[T], observed: &[T]) -> bool {
    if required.len() != observed.len() || contains_duplicate(required) {
        return false;
    }

    required.iter().all(|required_item| {
        observed
            .iter()
            .filter(|observed_item| *observed_item == required_item)
            .count()
            == 1
    })
}

fn contains_duplicate<T: Eq>(items: &[T]) -> bool {
    items
        .iter()
        .enumerate()
        .any(|(index, item)| items[index + 1..].contains(item))
}
