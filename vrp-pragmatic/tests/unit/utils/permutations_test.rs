use super::*;
use vrp_core::utils::DefaultRandom;

#[test]
fn can_generate_split_permutations() {
    let random = DefaultRandom::default();
    let job_permutations = get_split_permutations(5, 3, 12, &random);

    assert_eq!(job_permutations.len(), 12);
    job_permutations.iter().for_each(|permutation| {
        let left = *permutation.iter().take(3).max().unwrap();
        let right = *permutation.iter().skip(3).min().unwrap();

        assert_eq!(left, 2);
        assert_eq!(right, 3);
    });

    let job_permutations = get_split_permutations(3, 0, 10, &random);
    assert_eq!(job_permutations.len(), 6);

    let job_permutations = get_split_permutations(3, 3, 10, &random);
    assert_eq!(job_permutations.len(), 6);
}

#[test]
fn can_validate_permutations() {
    let random = Arc::new(DefaultRandom::default());
    let permutator = VariableJobPermutation::new(5, 3, 12, random.clone());

    assert!(permutator.validate(&vec![0, 1, 2, 3, 4]));
    assert!(permutator.validate(&vec![0, 2, 1, 3, 4]));
    assert!(permutator.validate(&vec![0, 2, 1, 4, 3]));

    assert!(!permutator.validate(&vec![]));
    assert!(!permutator.validate(&vec![0]));
    assert!(!permutator.validate(&vec![0, 3, 2, 1, 4]));
    assert!(!permutator.validate(&vec![0, 1, 3, 2, 4]));

    let permutator = VariableJobPermutation::new(3, 1, 3, random);
    assert!(permutator.validate(&vec![0, 1, 2]));
}

#[test]
fn can_generate_huge_sample_permutations() {
    for &end in &[3, 5, 10, 15, 20] {
        let permutations = generate_sample_permutations(0, end, 3, &DefaultRandom::default());
        assert_eq!(permutations.len(), 3);
    }
}
