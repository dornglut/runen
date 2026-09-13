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
        finite(Sign::Negative, 8, 1),  // -16
        finite(Sign::Negative, 8, 1),  // -16 from a distinct occurrence
        finite(Sign::Negative, 11, -1), // -5.5
    ]
}

fn sum_leaf(format: BinaryFormat, value: BinaryValueFixture) -> SumReductionResult {
    reduce_sum(format, &[value]).expect("valid submitted floating contribution")
}

#[test]
fn exact_sum_uses_exact_exec_occurrences_without_value_deduplication() {
    let format = tiny_format();
    let (reduction, produced) = root_fixture();
    let values = represented_values();

    assert_ne!(produced[0], produced[1]);
    assert_eq!(values[0], values[1]);

    let incorporated = [produced[2], produced[0], produced[1]];
    assert!(reduction.has_exact_contribution_coverage(&produced, &incorporated));

    let exact = reduce_sum(format, &values).expect("covered contributions are valid numeric inputs");
    let reordered = [values[2], values[0], values[1]];
    assert_eq!(
        reduce_sum(format, &reordered).expect("same covered inputs in another order"),
        exact
    );

    let deduplicated_equal_value = [values[0], values[2]];
    assert_ne!(
        reduce_sum(format, &deduplicated_equal_value)
            .expect("deduplicated comparison inputs remain numerically valid"),
        exact
    );

    let omitted = [produced[0], produced[1]];
    assert!(!reduction.has_exact_contribution_coverage(&produced, &omitted));

    let duplicated = [produced[0], produced[0], produced[2]];
    assert!(!reduction.has_exact_contribution_coverage(&produced, &duplicated));

    let invented = reduction
        .contribution(iteration(1), 99)
        .expect("participant may create another distinct occurrence");
    let with_invented = [produced[0], produced[1], invented];
    assert!(!reduction.has_exact_contribution_coverage(&produced, &with_invented));

    let substituted_producer = reduction
        .contribution(iteration(2), 11)
        .expect("second participant can use the same fixture token");
    let with_substituted_producer = [substituted_producer, produced[1], produced[2]];
    assert!(
        !reduction.has_exact_contribution_coverage(&produced, &with_substituted_producer)
    );

    // The rejected occurrence collections above are deliberately not passed to
    // the numeric oracle. Exec occurrence coverage is the admission boundary for
    // treating a numeric result as evidence about this reduction occurrence.
}

#[test]
fn fast_tree_candidates_vary_over_the_same_exactly_covered_exec_occurrences() {
    let format = tiny_format();
    let (reduction, produced) = root_fixture();
    let values = represented_values();

    let first_leaf_assignment = [produced[0], produced[1], produced[2]];
    let second_leaf_assignment = [produced[0], produced[2], produced[1]];
    assert!(reduction.has_exact_contribution_coverage(&produced, &first_leaf_assignment));
    assert!(reduction.has_exact_contribution_coverage(&produced, &second_leaf_assignment));

    let a = sum_leaf(format, values[0]);
    let b = sum_leaf(format, values[1]);
    let c = sum_leaf(format, values[2]);

    // Same full binary-tree shape, ((leaf0 + leaf1) + leaf2), with only the
    // exactly covered Exec contribution-to-leaf assignment permuted.
    let first_pair = add_standard_tree_node(format, a, b).expect("accepted tree node");
    let first_candidate =
        add_standard_tree_node(format, first_pair, c).expect("accepted tree candidate");

    let second_pair = add_standard_tree_node(format, a, c).expect("accepted tree node");
    let second_candidate =
        add_standard_tree_node(format, second_pair, b).expect("accepted tree candidate");

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
