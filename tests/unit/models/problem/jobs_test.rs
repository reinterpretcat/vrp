use super::*;
use crate::helpers::models::common::DEFAULT_PROFILE;
use crate::helpers::models::costs::{ProfileAwareTransportCost, TestTransportCost};
use crate::helpers::models::problem::*;
use crate::models::problem::VehicleDetail;
use std::borrow::Borrow;

fn create_profile_aware_transport_cost() -> ProfileAwareTransportCost {
    ProfileAwareTransportCost::new(Box::new(|p, d| if p == 2 { 10.0 - d } else { d }))
}

#[test]
fn all_returns_all_jobs() {
    let fleet = Fleet::new(Default::default(), Default::default());
    let jobs = vec![Arc::new(test_single_job()), Arc::new(test_single_job())];

    assert_eq!(
        Jobs::new(&fleet, jobs, &TestTransportCost {}).all().count(),
        2
    )
}

parameterized_test! {calculates_proper_distance_between_single_jobs, (left, right, expected), {
    assert_eq!(get_distance_between_jobs(DEFAULT_PROFILE, &TestTransportCost{}, &left, &right), expected);
}}

calculates_proper_distance_between_single_jobs! {
    case1: (test_single_job_with_location(Some(0)), test_single_job_with_location(Some(10)), 10.0),
    case2: (test_single_job_with_location(Some(0)), test_single_job_with_location(None), 0.0),
    case3: (test_single_job_with_location(None), test_single_job_with_location(None), 0.0),
    case4: (test_single_job_with_location(Some(3)), test_single_job_with_locations(vec![Some(5), Some(2)]), 1.0),
    case5: (test_single_job_with_locations(vec![Some(2), Some(1)]), test_single_job_with_locations(vec![Some(10), Some(9)]), 7.0),
}

parameterized_test! {calculates_proper_distance_between_multi_jobs, (left, right, expected), {
    assert_eq!(get_distance_between_jobs(DEFAULT_PROFILE, &TestTransportCost{}, &left, &right), expected);
}}

calculates_proper_distance_between_multi_jobs! {
    case1: (test_multi_job_with_locations(vec![vec![Some(1)], vec![Some(2)]]), test_multi_job_with_locations(vec![vec![Some(8)], vec![Some(9)]]), 6.0),
    case2: (test_multi_job_with_locations(vec![vec![Some(1)], vec![Some(2)]]), test_multi_job_with_locations(vec![vec![None], vec![Some(9)]]), 0.0),
    case3: (test_multi_job_with_locations(vec![vec![None], vec![None]]), test_multi_job_with_locations(vec![vec![None], vec![Some(9)]]), 0.0),
    case4: (test_multi_job_with_locations(vec![vec![None], vec![None]]), test_multi_job_with_locations(vec![vec![None], vec![None]]), 0.0),
}

fn returns_proper_job_neighbours_impl(index: usize, expected: Vec<String>) {
    let fleet = Fleet::new(
        vec![test_driver()],
        vec![
            VehicleBuilder::new()
                .id("v1")
                .profile(1)
                .details(vec![test_vehicle_detail()])
                .build(),
            VehicleBuilder::new()
                .id("v2")
                .profile(1)
                .details(vec![test_vehicle_detail()])
                .build(),
        ],
    );
    let species = vec![
        SingleBuilder::new()
            .id("s0")
            .location(Some(0))
            .build_as_job_ref(),
        SingleBuilder::new()
            .id("s1")
            .location(Some(1))
            .build_as_job_ref(),
        SingleBuilder::new()
            .id("s2")
            .location(Some(2))
            .build_as_job_ref(),
        SingleBuilder::new()
            .id("s3")
            .location(Some(3))
            .build_as_job_ref(),
        SingleBuilder::new()
            .id("s4")
            .location(Some(4))
            .build_as_job_ref(),
    ];
    let jobs = Jobs::new(
        &fleet,
        species.clone(),
        &create_profile_aware_transport_cost(),
    );

    let result: Vec<String> = jobs
        .neighbors(1, species.get(index).unwrap(), 0.0, u32::max_value() as f64)
        .into_iter()
        .map(|j| get_job_id(&j).clone())
        .collect();

    assert_eq!(result, expected);
}

parameterized_test! {returns_proper_job_neighbours, (index, expected), {
    returns_proper_job_neighbours_impl(index, expected.iter().map(|s| s.to_string()).collect());
}}

returns_proper_job_neighbours! {
    case1: (0, vec!["s1", "s2", "s3", "s4"]),
}
