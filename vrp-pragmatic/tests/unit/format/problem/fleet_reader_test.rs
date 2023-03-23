use super::create_transport_costs;
use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::models::common::{Distance, Profile as CoreProfile, TimeWindow, Timestamp};
use vrp_core::models::problem::TravelTime;
use vrp_core::models::problem::{Actor, ActorDetail, Vehicle};
use vrp_core::models::solution::Route;

fn matrix(profile: Option<&str>, timestamp: Option<f64>, fill_value: i64, size: usize) -> Matrix {
    Matrix {
        profile: profile.map(|p| p.to_string()),
        timestamp: timestamp.map(format_time),
        travel_times: vec![fill_value; size],
        distances: vec![fill_value; size],
        error_codes: None,
    }
}

fn wrong_matrix(profile: Option<&str>, timestamp: Option<String>) -> Matrix {
    Matrix {
        profile: profile.map(|p| p.to_string()),
        timestamp,
        travel_times: vec![1; 4],
        distances: vec![2; 3],
        error_codes: None,
    }
}

fn create_problem(profiles: &[&str]) -> Problem {
    Problem {
        fleet: Fleet {
            profiles: profiles.iter().map(|p| MatrixProfile { name: p.to_string(), speed: None }).collect(),
            ..create_default_fleet()
        },
        ..create_empty_problem()
    }
}

parameterized_test! {can_create_transport_costs_negative_cases, (profiles, matrices, res_err), {
        can_create_transport_costs_negative_cases_impl(profiles, matrices, res_err);
}}

can_create_transport_costs_negative_cases! {
        case01: (
            &["car"],
            &[],
            "not enough routing matrices specified for fleet profiles defined: 1 must be less or equal to 0"
        ),
        case02: (
            &["car1", "car2"],
            &[matrix(None, None, 1, 4)],
            "not enough routing matrices specified for fleet profiles defined: 2 must be less or equal to 1"
        ),
        case03: (
            &["car1"],
            &[matrix(Some("car1"), None, 1, 4), matrix(Some("car2"), None, 2, 8)],
            "amount of fleet profiles does not match matrix profiles"
        ),
        case04: (
            &["car"],
            &[wrong_matrix(Some("car1"), None)],
            "distance and duration collections have different length"
        ),
        case05: (
            &["car1", "car2"],
            &[matrix(Some("car1"), None, 1, 4), matrix(Some("car2"), None, 2, 8)],
            "distance lengths don't match"
        ),
        case06: (
            &["car1"],
            &[matrix(Some("car1"), None, 1, 4), matrix(Some("car1"), None, 2, 4)],
            "duplicate profiles can be passed only for time aware routing"
        ),
        case07: (
            &["car1"],
            &[matrix(Some("car1"), None, 1, 4), matrix(Some("car1"), Some(0.), 2, 4)],
            "time-aware routing requires all matrices to have timestamp"
        ),
        case08: (
            &["car1", "car2"],
            &[matrix(Some("car1"), None, 1, 4), matrix(None, None, 2, 4)],
            "all matrices should have profile set or none of them"
        ),
        case09: (
            &["car1"],
            &[matrix(None, Some(0.), 1, 4)],
            "when timestamp is set, all matrices should have profile set"
        ),
        case10: (
            &["car1", "car2"],
            &[matrix(None, Some(0.), 1, 4), matrix(None, Some(0.), 2, 4)],
            "when timestamp is set, all matrices should have profile set"
        ),
}

fn can_create_transport_costs_negative_cases_impl(profiles: &[&str], matrices: &[Matrix], res_err: &str) {
    let problem = create_problem(profiles);

    let result = create_transport_costs(&problem, matrices);

    assert_eq!(result.err(), Some(res_err.to_string()));
}

parameterized_test! {can_create_transport_costs_positive_cases, (profiles, matrices, probes), {
        can_create_transport_costs_positive_cases_impl(profiles, matrices, probes);
}}

can_create_transport_costs_positive_cases! {
       case01: (
            &["car"],
            &[matrix(Some("car1"), None, 1, 4)],
            &[(0, 0., 1.)]
        ),
        case02: (
            &["car"],
            &[matrix(None, None, 1, 4)],
            &[(0, 0., 1.)]
        ),
        case03: (
            &["car1", "car2"],
            &[matrix(None, None, 1, 4), matrix(None, None, 2, 4)],
            &[(0, 0., 1.), (1, 0., 2.)]
        ),
        case04: (
            &["car1", "car2"],
            &[matrix(Some("car1"), None, 1, 4), matrix(Some("car2"), None, 2, 4)],
            &[(0, 0., 1.), (1, 0., 2.)]
        ),
        case05: (
            &["car1", "car2"],
            &[matrix(Some("car2"), None, 2, 4), matrix(Some("car1"), None, 1, 4)],
            &[(0, 0., 1.), (1, 0., 2.)]
        ),
        case06: (
            &["car"],
            &[matrix(Some("car"), Some(0.), 1, 4), matrix(Some("car"), Some(10.), 2, 4)],
            &[(0, 0., 1.), (0, 10., 2.)]
        ),
        case07: (
            &["car1", "car2"],
            &[matrix(Some("car1"), Some(0.), 1, 4),
              matrix(Some("car2"), Some(0.), 3, 4),
              matrix(Some("car1"), Some(10.), 2, 4),
              matrix(Some("car2"), Some(10.), 4, 4)],
            &[(0, 0., 1.), (0, 10., 2.), (1, 0., 3.), (1, 10., 4.)]
        ),
}

fn can_create_transport_costs_positive_cases_impl(
    profiles: &[&str],
    matrices: &[Matrix],
    probes: &[(usize, Timestamp, Distance)],
) {
    let problem = create_problem(profiles);

    let transport = create_transport_costs(&problem, matrices).unwrap();

    probes.iter().for_each(|&(profile_idx, timestamp, distance)| {
        let route = Route {
            actor: Arc::new(Actor {
                vehicle: Arc::new(Vehicle { profile: CoreProfile::new(profile_idx, None), ..test_vehicle("v1") }),
                driver: Arc::new(test_driver()),
                detail: ActorDetail { start: None, end: None, time: TimeWindow::new(0., 1.) },
            }),
            tour: Default::default(),
        };

        let result = transport.distance(&route, 0, 1, TravelTime::Departure(timestamp));
        assert_eq!(result, distance);
    });
}
