use super::*;

#[test]
fn can_create_dedup_fn() {
    let objective = create_example_objective();
    let dedup_fn = create_dedup_fn::<VectorRosomaxaContext, _, _>(0.1);

    // test equal fitness
    let solution1 = VectorSolution { data: vec![1.0], weights: vec![1.0], fitness: -1.0 };
    let solution2 = VectorSolution { data: vec![1.0], weights: vec![1.0], fitness: -1.0 };
    assert!(dedup_fn(objective.as_ref(), &solution1, &solution2));

    // Test similar weights but different fitness
    let solution3 = VectorSolution { data: vec![1.05], weights: vec![1.05], fitness: -1.5 };
    assert!(dedup_fn(objective.as_ref(), &solution1, &solution3));

    // Test different weights
    let solution4 = VectorSolution { data: vec![2.0], weights: vec![2.0], fitness: -2.0 };
    assert!(!dedup_fn(objective.as_ref(), &solution1, &solution4));
}
