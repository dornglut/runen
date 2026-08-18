/// Verification-only evidence for the complete contract required by the accepted
/// identity-bearing unordered reduction form.
///
/// These flags do not prove arbitrary operator implementations satisfy the
/// represented obligations and are not a source-language trait system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnorderedReductionEvidence {
    normal_and_closed: bool,
    result_only: bool,
    two_sided_identity: bool,
    associative: bool,
    commutative: bool,
}

impl UnorderedReductionEvidence {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            normal_and_closed: false,
            result_only: false,
            two_sided_identity: false,
            associative: false,
            commutative: false,
        }
    }

    #[must_use]
    pub const fn with_normal_and_closed(mut self) -> Self {
        self.normal_and_closed = true;
        self
    }

    #[must_use]
    pub const fn with_result_only(mut self) -> Self {
        self.result_only = true;
        self
    }

    #[must_use]
    pub const fn with_two_sided_identity(mut self) -> Self {
        self.two_sided_identity = true;
        self
    }

    #[must_use]
    pub const fn with_associativity(mut self) -> Self {
        self.associative = true;
        self
    }

    #[must_use]
    pub const fn with_commutativity(mut self) -> Self {
        self.commutative = true;
        self
    }

    /// Whether the represented evidence admits the accepted unordered reduction form.
    #[must_use]
    pub const fn permits_unordered_reduction(self) -> bool {
        self.normal_and_closed
            && self.result_only
            && self.two_sided_identity
            && self.associative
            && self.commutative
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

    required.iter().all(|required_id| {
        incorporated
            .iter()
            .filter(|incorporated_id| *incorporated_id == required_id)
            .count()
            == 1
    })
}

fn contains_duplicate(ids: &[ContributionId]) -> bool {
    ids.iter()
        .enumerate()
        .any(|(index, id)| ids[index + 1..].contains(id))
}
