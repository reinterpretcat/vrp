use super::*;

#[test]
fn can_generate_permutations() {
    let mut permutations = get_permutations(0, 2);

    assert_eq!(permutations.next().unwrap(), vec![0, 1, 2]);
    assert_eq!(permutations.next().unwrap(), vec![1, 0, 2]);
    assert_eq!(permutations.next().unwrap(), vec![2, 0, 1]);
    assert_eq!(permutations.next().unwrap(), vec![0, 2, 1]);
    assert_eq!(permutations.next().unwrap(), vec![1, 2, 0]);
    assert_eq!(permutations.next().unwrap(), vec![2, 1, 0]);
    assert_eq!(permutations.next(), None);

    let mut permutations = get_permutations(3, 4);
    assert_eq!(permutations.next().unwrap(), vec![3, 4]);
    assert_eq!(permutations.next().unwrap(), vec![4, 3]);
    assert_eq!(permutations.next(), None);
}

#[test]
fn can_generate_split_permutations() {
    let job_permutations = get_split_permutations(5, 3, 12);

    assert_eq!(job_permutations.len(), 12);
    job_permutations.iter().for_each(|permutation| {
        let left = *permutation.iter().take(3).max().unwrap();
        let right = *permutation.iter().skip(3).min().unwrap();

        assert_eq!(left, 2);
        assert_eq!(right, 3);
    });
}
