use super::*;
use crate::helpers::models::common::DEFAULT_PROFILE;
use crate::helpers::models::problem::*;
use crate::models::problem::VehicleDetail;

struct OnlyDistanceCost {}

impl TransportCost for OnlyDistanceCost {
    fn duration(&self, _profile: Profile, _from: Location, _to: Location, _departure: Timestamp) -> Duration {
        0.
    }

    fn distance(&self, _profile: Profile, from: Location, to: Location, _departure: Timestamp) -> Distance {
        fake_routing(from, to)
    }
}

impl Default for OnlyDistanceCost {
    fn default() -> Self {
        Self {}
    }
}

struct ProfileAwareTransportCost {
    func: Box<dyn Fn(Profile, f64) -> f64 + Sync + Send>,
}

impl ProfileAwareTransportCost {
    pub fn new(func: Box<dyn Fn(Profile, f64) -> f64 + Sync + Send>) -> ProfileAwareTransportCost {
        ProfileAwareTransportCost { func }
    }
}

impl TransportCost for ProfileAwareTransportCost {
    fn duration(&self, _profile: Profile, _from: Location, _to: Location, _departure: Timestamp) -> Duration {
        0.
    }

    fn distance(&self, profile: Profile, from: Location, to: Location, _departure: Timestamp) -> Distance {
        (self.func)(profile, fake_routing(from, to))
    }
}

fn create_profile_aware_transport_cost() -> Arc<dyn TransportCost + Sync + Send> {
    Arc::new(ProfileAwareTransportCost::new(Box::new(|p, d| if p == 2 { 10.0 - d } else { d })))
}

fn create_only_distance_transport_cost() -> Arc<dyn TransportCost + Sync + Send> {
    Arc::new(OnlyDistanceCost::default())
}

fn create_costs() -> Costs {
    Costs { fixed: 10.0, per_distance: 1.0, per_driving_time: 1.0, per_waiting_time: 1.0, per_service_time: 1.0 }
}

#[test]
fn all_returns_all_jobs() {
    let jobs = vec![Job::Single(Arc::new(test_single())), Job::Single(Arc::new(test_single()))];

    assert_eq!(Jobs::new(&test_fleet(), jobs, &create_only_distance_transport_cost()).all().count(), 2)
}

parameterized_test! {calculates_proper_cost_between_single_jobs, (left, right, expected), {
    assert_eq!(get_cost_between_jobs(DEFAULT_PROFILE,
                                    &create_costs(),
                                    &create_only_distance_transport_cost(),
                                    &Job::Single(left),
                                    &Job::Single(right)),
              expected);
}}

calculates_proper_cost_between_single_jobs! {
    case1: (test_single_with_location(Some(0)), test_single_with_location(Some(10)), 10.0),
    case2: (test_single_with_location(Some(0)), test_single_with_location(None), 0.0),
    case3: (test_single_with_location(None), test_single_with_location(None), 0.0),
    case4: (test_single_with_location(Some(3)), test_single_with_locations(vec![Some(5), Some(2)]), 1.0),
    case5: (test_single_with_locations(vec![Some(2), Some(1)]), test_single_with_locations(vec![Some(10), Some(9)]), 7.0),
}

parameterized_test! {calculates_proper_cost_between_multi_jobs, (left, right, expected), {
    assert_eq!(get_cost_between_jobs(DEFAULT_PROFILE,
                                     &create_costs(),
                                     &create_only_distance_transport_cost(),
                                     &Job::Multi(left),
                                     &Job::Multi(right)),
               expected);
}}

calculates_proper_cost_between_multi_jobs! {
    case1: (test_multi_job_with_locations(vec![vec![Some(1)], vec![Some(2)]]), test_multi_job_with_locations(vec![vec![Some(8)], vec![Some(9)]]), 6.0),
    case2: (test_multi_job_with_locations(vec![vec![Some(1)], vec![Some(2)]]), test_multi_job_with_locations(vec![vec![None], vec![Some(9)]]), 0.0),
    case3: (test_multi_job_with_locations(vec![vec![None], vec![None]]), test_multi_job_with_locations(vec![vec![None], vec![Some(9)]]), 0.0),
    case4: (test_multi_job_with_locations(vec![vec![None], vec![None]]), test_multi_job_with_locations(vec![vec![None], vec![None]]), 0.0),
}

parameterized_test! {returns_proper_job_neighbours, (index, expected), {
    returns_proper_job_neighbours_impl(index, expected.iter().map(|s| s.to_string()).collect());
}}

returns_proper_job_neighbours! {
    case1: (0, vec!["s1", "s2", "s3", "s4"]),
    case2: (1, vec!["s0", "s2", "s3", "s4"]),
    case3: (2, vec!["s1", "s3", "s0", "s4"]),
    case4: (3, vec!["s2", "s4", "s1", "s0"]),
}

fn returns_proper_job_neighbours_impl(index: usize, expected: Vec<String>) {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            VehicleBuilder::default().id("v1").profile(1).details(vec![test_vehicle_detail()]).build(),
            VehicleBuilder::default().id("v2").profile(1).details(vec![test_vehicle_detail()]).build(),
        ])
        .build();
    let species = vec![
        SingleBuilder::default().id("s0").location(Some(0)).build_as_job_ref(),
        SingleBuilder::default().id("s1").location(Some(1)).build_as_job_ref(),
        SingleBuilder::default().id("s2").location(Some(2)).build_as_job_ref(),
        SingleBuilder::default().id("s3").location(Some(3)).build_as_job_ref(),
        SingleBuilder::default().id("s4").location(Some(4)).build_as_job_ref(),
    ];
    let jobs = Jobs::new(&fleet, species.clone(), &create_profile_aware_transport_cost());

    let result: Vec<String> = jobs
        .neighbors(1, species.get(index).unwrap(), 0.0, u32::max_value() as f64)
        .map(|j| get_job_id(&j).clone())
        .collect();

    assert_eq!(result, expected);
}

parameterized_test! {returns_proper_job_ranks, (index, profile, expected), {
    returns_proper_job_ranks_impl(index, profile, expected);
}}

returns_proper_job_ranks! {
    case1: (0, 1, 0.0),
    case2: (1, 1, 5.0),
    case3: (2, 1, 6.0),
    case4: (3, 1, 16.0),
    case5: (0, 3, 30.0),
    case6: (1, 3, 20.0),
    case7: (2, 3, 9.0),
    case8: (3, 3, 1.0),
}

fn returns_proper_job_ranks_impl(index: usize, profile: Profile, expected: Distance) {
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            VehicleBuilder::default()
                .id("v1_1")
                .profile(1)
                .details(vec![VehicleDetail {
                    start: Some(0),
                    end: Some(0),
                    time: Some(TimeWindow { start: 0.0, end: 0.0 }),
                }])
                .build(),
            VehicleBuilder::default()
                .id("v1_2")
                .profile(1)
                .details(vec![VehicleDetail {
                    start: Some(15),
                    end: Some(0),
                    time: Some(TimeWindow { start: 0.0, end: 0.0 }),
                }])
                .build(),
            VehicleBuilder::default()
                .id("v2_1")
                .profile(3)
                .details(vec![VehicleDetail {
                    start: Some(30),
                    end: Some(0),
                    time: Some(TimeWindow { start: 0.0, end: 0.0 }),
                }])
                .build(),
        ])
        .build();
    let species = vec![
        SingleBuilder::default().id("s0").location(Some(0)).build_as_job_ref(),
        SingleBuilder::default().id("s1").location(Some(10)).build_as_job_ref(),
        SingleBuilder::default().id("s2").location(Some(21)).build_as_job_ref(),
        SingleBuilder::default().id("s3").location(Some(31)).build_as_job_ref(),
    ];
    let jobs = Jobs::new(&fleet, species.clone(), &create_profile_aware_transport_cost());

    let result = jobs.rank(profile, species.get(index).unwrap());

    assert_eq!(result, expected);
}

#[test]
fn can_use_multi_job_bind_and_roots() {
    let job = test_multi_job_with_locations(vec![vec![Some(0)], vec![Some(1)]]);
    let jobs = vec![Job::Multi(job.clone())];

    let jobs = Jobs::new(&test_fleet(), jobs, &create_only_distance_transport_cost());
    let job = Job::Multi(Multi::roots(&job.jobs.first().unwrap()).unwrap());

    assert_eq!(jobs.neighbors(0, &job, 0.0, 100.0).count(), 0);
}
