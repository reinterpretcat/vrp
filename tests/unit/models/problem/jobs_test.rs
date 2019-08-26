use super::*;
use crate::helpers::models::common::DEFAULT_PROFILE;
use crate::helpers::models::costs::TestTransportCost;
use crate::helpers::models::problem::{
    test_multi_job_with_locations, test_single_job, test_single_job_with_location,
    test_single_job_with_locations,
};

#[test]
fn all_returns_all_jobs() {
    let fleet = Fleet::new(Default::default(), Default::default());
    let jobs = vec![test_single_job(), test_single_job()];
    assert_eq!(Jobs::new(&fleet, jobs).all().count(), 2)
}

parameterized_test! {calculates_proper_distance_between_single_jobs, (left, right, expected), {
    assert_eq!(get_distance_between_jobs(&String::from(DEFAULT_PROFILE), TestTransportCost{}, &left, &right), expected);
}}

calculates_proper_distance_between_single_jobs! {
    case1: (test_single_job_with_location(Some(0)), test_single_job_with_location(Some(10)), 10.0),
    case2: (test_single_job_with_location(Some(0)), test_single_job_with_location(None), 0.0),
    case3: (test_single_job_with_location(None), test_single_job_with_location(None), 0.0),
    case4: (test_single_job_with_location(Some(3)), test_single_job_with_locations(vec![Some(5), Some(2)]), 1.0),
    case5: (test_single_job_with_locations(vec![Some(2), Some(1)]), test_single_job_with_locations(vec![Some(10), Some(9)]), 7.0),
}

parameterized_test! {calculates_proper_distance_between_multi_jobs, (left, right, expected), {
    assert_eq!(get_distance_between_jobs(&String::from(DEFAULT_PROFILE), TestTransportCost{}, &left, &right), expected);
}}

calculates_proper_distance_between_multi_jobs! {
    case1: (test_multi_job_with_locations(vec![vec![Some(1)], vec![Some(2)]]), test_multi_job_with_locations(vec![vec![Some(8)], vec![Some(9)]]), 6.0),
    case2: (test_multi_job_with_locations(vec![vec![Some(1)], vec![Some(2)]]), test_multi_job_with_locations(vec![vec![None], vec![Some(9)]]), 0.0),
    case3: (test_multi_job_with_locations(vec![vec![None], vec![None]]), test_multi_job_with_locations(vec![vec![None], vec![Some(9)]]), 0.0),
    case4: (test_multi_job_with_locations(vec![vec![None], vec![None]]), test_multi_job_with_locations(vec![vec![None], vec![None]]), 0.0),
}
