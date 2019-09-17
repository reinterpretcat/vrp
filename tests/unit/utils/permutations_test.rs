use super::*;
use crate::helpers::models::problem::test_multi_job_with_locations;

#[test]
fn can_generate_permutations() {
    let mut permutations = get_permutations(3);

    assert_eq!(permutations.next().unwrap(), vec![0, 1, 2]);
    assert_eq!(permutations.next().unwrap(), vec![1, 0, 2]);
    assert_eq!(permutations.next().unwrap(), vec![2, 0, 1]);
    assert_eq!(permutations.next().unwrap(), vec![0, 2, 1]);
    assert_eq!(permutations.next().unwrap(), vec![1, 2, 0]);
    assert_eq!(permutations.next().unwrap(), vec![2, 1, 0]);
    assert_eq!(permutations.next(), None);
}

//#[test]
//fn can_generate_job_permutations() {
//    let multi = if let Job::Multi(multi) =
//    test_multi_job_with_locations(vec![vec![Some(1)], vec![Some(2)], vec![Some(3)]])
//    {
//        multi
//    } else {
//        panic!()
//    };
//
//    let job_permutations = get_job_permutations(&multi);
//
//    assert_eq!(job_permutations.len(), 3);
//}
