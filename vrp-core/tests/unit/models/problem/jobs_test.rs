use super::*;
use crate::helpers::models::problem::*;
use crate::models::problem::{TravelTime, VehicleDetail, VehiclePlace};
use crate::models::solution::Route;

#[derive(Default)]
struct OnlyDistanceCost {}

impl TransportCost for OnlyDistanceCost {
    fn duration_approx(&self, _: &Profile, _: Location, _: Location) -> Duration {
        0.
    }

    fn distance_approx(&self, _: &Profile, from: Location, to: Location) -> Distance {
        fake_routing(from, to)
    }

    fn duration(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Duration {
        0.
    }

    fn distance(&self, _: &Route, from: Location, to: Location, _: TravelTime) -> Distance {
        fake_routing(from, to)
    }
}

struct ProfileAwareTransportCost {
    func: Box<dyn Fn(&Profile, f64) -> f64 + Sync + Send>,
}

impl ProfileAwareTransportCost {
    pub fn new(func: Box<dyn Fn(&Profile, f64) -> f64 + Sync + Send>) -> ProfileAwareTransportCost {
        ProfileAwareTransportCost { func }
    }
}

impl TransportCost for ProfileAwareTransportCost {
    fn duration_approx(&self, _: &Profile, _: Location, _: Location) -> Duration {
        0.
    }

    fn distance_approx(&self, profile: &Profile, from: Location, to: Location) -> Distance {
        (self.func)(profile, fake_routing(from, to))
    }

    fn duration(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Duration {
        0.
    }

    fn distance(&self, route: &Route, from: Location, to: Location, _: TravelTime) -> Distance {
        (self.func)(&route.actor.vehicle.profile, fake_routing(from, to))
    }
}

struct FixedTransportCost {
    duration_cost: f64,
    distance_cost: f64,
}

impl TransportCost for FixedTransportCost {
    fn duration_approx(&self, _: &Profile, _: Location, _: Location) -> Duration {
        self.duration_cost
    }

    fn distance_approx(&self, _: &Profile, _: Location, _: Location) -> Distance {
        self.distance_cost
    }

    fn duration(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Duration {
        self.duration_cost
    }

    fn distance(&self, _: &Route, _: Location, _: Location, _: TravelTime) -> Distance {
        self.distance_cost
    }
}

impl FixedTransportCost {
    pub fn new_shared(duration_cost: f64, distance_cost: f64) -> Arc<dyn TransportCost + Send + Sync> {
        Arc::new(Self { duration_cost, distance_cost })
    }
}

fn create_profile_aware_transport_cost() -> Arc<dyn TransportCost + Sync + Send> {
    Arc::new(ProfileAwareTransportCost::new(Box::new(|p, d| if p.index == 2 { 10.0 - d } else { d })))
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
    assert_eq!(get_cost_between_jobs(&Profile::default(),
                                    &create_costs(),
                                    create_only_distance_transport_cost().as_ref(),
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
    assert_eq!(get_cost_between_jobs(&Profile::default(),
                                     &create_costs(),
                                     create_only_distance_transport_cost().as_ref(),
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
    let p1 = Profile::new(1, None);
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            VehicleBuilder::default().id("v1").profile(p1.clone()).details(vec![test_vehicle_detail()]).build(),
            VehicleBuilder::default().id("v2").profile(p1.clone()).details(vec![test_vehicle_detail()]).build(),
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

    let result: Vec<String> =
        jobs.neighbors(&p1, species.get(index).unwrap(), 0.0).map(|(j, _)| get_job_id(j).clone()).collect();

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

fn returns_proper_job_ranks_impl(index: usize, profile_index: usize, expected: Distance) {
    let profile = Profile::new(profile_index, None);
    let p1 = Profile::new(1, None);
    let p3 = Profile::new(3, None);
    let create_vehicle_detail = |start_location: usize| VehicleDetail {
        start: Some(VehiclePlace { location: start_location, time: TimeInterval::default() }),
        end: Some(VehiclePlace { location: 0, time: TimeInterval::default() }),
    };
    let fleet = FleetBuilder::default()
        .add_driver(test_driver())
        .add_vehicles(vec![
            VehicleBuilder::default().id("v1_1").profile(p1.clone()).details(vec![create_vehicle_detail(0)]).build(),
            VehicleBuilder::default().id("v1_2").profile(p1).details(vec![create_vehicle_detail(15)]).build(),
            VehicleBuilder::default().id("v2_1").profile(p3).details(vec![create_vehicle_detail(30)]).build(),
        ])
        .build();
    let species = vec![
        SingleBuilder::default().id("s0").location(Some(0)).build_as_job_ref(),
        SingleBuilder::default().id("s1").location(Some(10)).build_as_job_ref(),
        SingleBuilder::default().id("s2").location(Some(21)).build_as_job_ref(),
        SingleBuilder::default().id("s3").location(Some(31)).build_as_job_ref(),
    ];
    let jobs = Jobs::new(&fleet, species.clone(), &create_profile_aware_transport_cost());

    let result = jobs.rank(&profile, species.get(index).unwrap());

    assert_eq!(result, expected);
}

#[test]
fn can_use_multi_job_bind_and_roots() {
    let job = test_multi_job_with_locations(vec![vec![Some(0)], vec![Some(1)]]);
    let jobs = vec![Job::Multi(job.clone())];

    let jobs = Jobs::new(&test_fleet(), jobs, &create_only_distance_transport_cost());
    let job = Job::Multi(Multi::roots(job.jobs.first().unwrap()).unwrap());

    assert_eq!(jobs.neighbors(&Profile::default(), &job, 0.0).count(), 0);
}

parameterized_test! {can_handle_negative_distances_durations, (duration_cost, distance_cost), {
    can_handle_negative_distances_durations_impl(FixedTransportCost::new_shared(duration_cost, distance_cost));
}}

can_handle_negative_distances_durations! {
    case01: (-1., 1.),
    case02: (1., -1.),
    case03: (-1., -1.),
    case04: (-1., 0.),
}

fn can_handle_negative_distances_durations_impl(transport_costs: Arc<dyn TransportCost + Send + Sync>) {
    let profile = Profile::default();
    let species = vec![
        SingleBuilder::default().id("s0").location(Some(0)).build_as_job_ref(),
        SingleBuilder::default().id("s1").location(Some(1)).build_as_job_ref(),
    ];

    let jobs = Jobs::new(&test_fleet(), species.clone(), &transport_costs);

    for job in &species {
        assert!(jobs
            .neighbors(&profile, job, 0.0)
            .all(|(_, cost)| { (*cost - UNREACHABLE_COST).abs() < std::f64::EPSILON }));
    }
}
