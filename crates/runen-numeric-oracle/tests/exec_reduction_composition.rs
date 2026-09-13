use runen_exec_oracle::{
    EachId, IterationId, ReductionContribution, ReductionFixture, ReductionId,
};
use runen_numeric_oracle::{
    BinaryFormat, BinaryValueFixture, ExactDyadic, RoundedBinaryValue, Sign, SumReductionResult,
    add_standard_tree_node, reduce_sum,
};

fn tiny_format() -> BinaryFormat {
    BinaryFormat::new(4, -2, 5).expect("valid test format")
}

fn finite(sign: Sign, magnitude: u128, exponent: i32) -> BinaryValueFixture {
    BinaryValueFixture::Finite(ExactDyadic::from_parts(sign, magnitude, exponent))
}

fn iteration(token: u32) -> IterationId {
    IterationId::new(EachId::new(1), token)
}

fn root_fixture() -> (ReductionFixture, [ReductionContribution; 3]) {
    let each = EachId::new(1);
    let required = [iteration(1), iteration(2), iteration(3)];
    let reduction = ReductionFixture::root(ReductionId::new(1), each, &required)
        .expect("valid root reduction fixture");
    let contributions = [
        reduction
            .contribution(required[0], 11)
            .expect("first participant contributes"),
        reduction
            .contribution(required[1], 29)
            .expect("second participant contributes"),
        reduction
            .contribution(required[2], 47)
            .expect("third participant contributes"),
    ];

    (reduction, contributions)
}

fn represented_values() -> [BinaryValueFixture; 3] {
    [
        finite(Sign::Negative, 8, 1),   // -16
        finite(Sign::Negative, 8, 1),   // -16 from a distinct occurrence
        finite(Sign::Negative, 11, -1), // -5.5
    ]
}

fn admitted_values(
    reduction: &ReductionFixture,
    submitted: &[(ReductionContribution, BinaryValueFixture)],
    incorporated: &[ReductionContribution],
) -> Option<Vec<BinaryValueFixture>> {
    let produced = submitted
        .iter()
        .map(|(contribution, _)| *contribution)
        .collect::<Vec<_>>();
    if !reduction.has_exact_contribution_coverage(&produced, incorporated) {
        return None;
    }

    incorporated
        .iter()
        .map(|actual| {
            submitted
                .iter()
                .find_map(|(expected, value)| (*expected == *actual).then_some(*value))
        })
        .collect()
}

fn sum_leaf(format: BinaryFormat, value: BinaryValueFixture) -> SumReductionResult {
    reduce_sum(format, &[value]).expect("valid submitted floating contribution")
}

#[test]
fn exact_sum_uses_exact_exec_occurrences_without_value_deduplication() {
    let format = tiny_format();
    let (reduction, produced) = root_fixture();
    let values = represented_values();
    let submitted = [
        (produced[0], values[0]),
        (produced[1], values[1]),
        (produced[2], values[2]),
    ];

    assert_ne!(produced[0], produced[1]);
    assert_eq!(values[0], values[1]);

    let produced_values =
        admitted_values(&reduction, &submitted, &produced).expect("produced coverage is exact");
    let incorporated = [produced[2], produced[0], produced[1]];
    let incorporated_values = admitted_values(&reduction, &submitted, &incorporated)
        .expect("reordered occurrence coverage is exact");

    let exact = reduce_sum(format, &produced_values)
        .expect("covered contributions are valid numeric inputs");
    assert_eq!(
        reduce_sum(format, &incorporated_values).expect("same covered inputs in another order"),
        exact
    );

    let deduplicated_equal_value = [values[0], values[2]];
    assert_ne!(
        reduce_sum(format, &deduplicated_equal_value)
            .expect("deduplicated comparison inputs remain numerically valid"),
        exact
    );

    let omitted = [produced[0], produced[1]];
    assert!(admitted_values(&reduction, &submitted, &omitted).is_none());

    let duplicated = [produced[0], produced[0], produced[2]];
    assert!(admitted_values(&reduction, &submitted, &duplicated).is_none());

    let invented = reduction
        .contribution(iteration(1), 99)
        .expect("participant may create another distinct occurrence");
    let with_invented = [produced[0], produced[1], invented];
    assert!(admitted_values(&reduction, &submitted, &with_invented).is_none());

    let substituted_producer = reduction
        .contribution(iteration(2), 11)
        .expect("second participant can use the same fixture token");
    let with_substituted_producer = [substituted_producer, produced[1], produced[2]];
    assert!(admitted_values(&reduction, &submitted, &with_substituted_producer).is_none());

    // Rejected occurrence collections never reach the numeric oracle through
    // `admitted_values`. Exec occurrence coverage is the admission boundary for
    // treating a numeric result as evidence about this reduction occurrence.
}

#[test]
fn fast_tree_candidates_vary_over_the_same_exactly_covered_exec_occurrences() {
    let format = tiny_format();
    let (reduction, produced) = root_fixture();
    let values = represented_values();
    let submitted = [
        (produced[0], values[0]),
        (produced[1], values[1]),
        (produced[2], values[2]),
    ];

    let first_leaf_assignment = [produced[0], produced[1], produced[2]];
    let second_leaf_assignment = [produced[0], produced[2], produced[1]];
    let first_values = admitted_values(&reduction, &submitted, &first_leaf_assignment)
        .expect("first leaf assignment covers the exact Exec occurrences");
    let second_values = admitted_values(&reduction, &submitted, &second_leaf_assignment)
        .expect("second leaf assignment covers the exact Exec occurrences");

    // Same full binary-tree shape, ((leaf0 + leaf1) + leaf2), with only the
    // exactly covered Exec contribution-to-leaf assignment permuted.
    let first_pair = add_standard_tree_node(
        format,
        sum_leaf(format, first_values[0]),
        sum_leaf(format, first_values[1]),
    )
    .expect("accepted tree node");
    let first_candidate =
        add_standard_tree_node(format, first_pair, sum_leaf(format, first_values[2]))
            .expect("accepted tree candidate");

    let second_pair = add_standard_tree_node(
        format,
        sum_leaf(format, second_values[0]),
        sum_leaf(format, second_values[1]),
    )
    .expect("accepted tree node");
    let second_candidate =
        add_standard_tree_node(format, second_pair, sum_leaf(format, second_values[2]))
            .expect("accepted tree candidate");

    assert_eq!(
        first_candidate,
        SumReductionResult::Value(RoundedBinaryValue::Normal {
            sign: Sign::Negative,
            significand: 9,
            exponent: 5,
        })
    );
    assert_eq!(
        second_candidate,
        SumReductionResult::Value(RoundedBinaryValue::Normal {
            sign: Sign::Negative,
            significand: 10,
            exponent: 5,
        })
    );
    assert_ne!(first_candidate, second_candidate);

    // The tree choices are numeric-contract candidates over an Exec-selected
    // occurrence multiset. They are not source-visible trees, physical schedules,
    // host floating behavior, or evidence that floating addition is a generic
    // Exec unordered-reduction combine operator.
}
