/// Verification-only evidence that an unordered reduction operator contract
/// establishes the algebraic laws required by the accepted Exec reduction form.
///
/// These booleans do not prove arbitrary operator implementations satisfy the
/// laws and are not a source-language trait system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReductionLaws {
    two_sided_identity: bool,
    associative: bool,
    commutative: bool,
}

impl ReductionLaws {
    #[must_use]
    pub const fn new(
        two_sided_identity: bool,
        associative: bool,
        commutative: bool,
    ) -> Self {
        Self {
            two_sided_identity,
            associative,
            commutative,
        }
    }

    /// Whether the represented evidence admits the accepted unordered reduction form.
    #[must_use]
    pub const fn permits_unordered_reduction(self) -> bool {
        self.two_sided_identity && self.associative && self.commutative
    }
}

/// Verification-only identity token for one semantic reduction contribution.
///
/// The numeric representation carries no contribution order, worker identity,
/// lane identity, physical accumulator identity, or source-language meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContributionId(pub u32);

/// Checks that one realization incorporates exactly the required semantic
/// contributions once each, without using the slice order as semantic input.
///
/// Duplicate identifiers in `required` are rejected as invalid fixture input;
/// each required identifier denotes one distinct semantic contribution.
#[must_use]
pub fn has_exact_contribution_coverage(
    required: &[ContributionId],
    incorporated: &[ContributionId],
) -> bool {
    if required.len() != incorporated.len() || contains_duplicate(required) {
        return false;
    }

    required
        .iter()
        .all(|required_id| incorporated.iter().filter(|id| *id == required_id).count() == 1)
}

fn contains_duplicate(ids: &[ContributionId]) -> bool {
    ids.iter()
        .enumerate()
        .any(|(index, id)| ids[index + 1..].contains(id))
}
