use super::create_transport_costs;
use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;
use std::sync::Arc;
use vrp_core::models::common::{Distance, Profile as CoreProfile, TimeWindow, Timestamp};
use vrp_core::models::problem::{Actor, ActorDetail, Vehicle};
use vrp_core::models::problem::{
    DriverIdDimension, OffHoursRateDimension, OvertimeRateDimension, RegularDurationDimension, RegularHoursDimension,
    TravelTime,
};
use vrp_core::models::problem::{JobIdDimension, VisitWindowKind, VisitWindowKindDimension, VisitWindowsDimension};
use vrp_core::models::solution::Route;

fn matrix(profile: Option<&str>, timestamp: Option<Float>, fill_value: i64, size: usize) -> Matrix {
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
    let coord_index = Arc::new(CoordIndex::new(&problem));

    let result = create_transport_costs(&problem, matrices, coord_index);

    assert_eq!(result.err(), Some(res_err.into()));
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
    let coord_index = Arc::new(CoordIndex::new(&problem));

    let transport = create_transport_costs(&problem, matrices, coord_index).unwrap();

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

#[test]
fn reads_driver_id_into_dimens() {
    let matrix = matrix(Some("car"), None, 1, 4);

    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (1., 1.))], relations: None, clustering: None },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_driver_id("my_vehicle", vec![10], "drv-1")],
            profiles: create_default_matrix_profiles(),
            resources: None,
        },
        objectives: None,
    };

    let problem = (problem, vec![matrix]).read_pragmatic().ok().unwrap();

    let vehicle = problem.fleet.vehicles.first().unwrap();
    assert_eq!(vehicle.dimens.get_driver_id(), Some(&"drv-1".to_string()));
}

#[test]
fn reads_overtime_rate_and_regular_duration_into_dimens() {
    let matrix = matrix(Some("car"), None, 1, 4);

    let vehicle = VehicleType {
        costs: VehicleCosts { overtime: Some(0.02), ..create_default_vehicle_costs() },
        shifts: vec![
            VehicleShift {
                start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                end: Some(ShiftEnd { earliest: None, latest: format_time(99.), location: (0., 0.).to_loc() }),
                regular_duration: Some(28800.0),
                ..create_default_vehicle_shift()
            },
            VehicleShift {
                start: ShiftStart { earliest: format_time(100.), latest: None, location: (0., 0.).to_loc() },
                end: Some(ShiftEnd { earliest: None, latest: format_time(200.), location: (0., 0.).to_loc() }),
                regular_duration: Some(21600.0),
                ..create_default_vehicle_shift()
            },
        ],
        ..create_default_vehicle_type()
    };

    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (1., 1.))], relations: None, clustering: None },
        fleet: Fleet { vehicles: vec![vehicle], profiles: create_default_matrix_profiles(), resources: None },
        objectives: None,
    };

    let problem = (problem, vec![matrix]).read_pragmatic().unwrap();

    let vehicles = &problem.fleet.vehicles;
    assert_eq!(vehicles.len(), 2);

    assert_eq!(vehicles[0].dimens.get_overtime_rate(), Some(&0.02));
    assert_eq!(vehicles[0].dimens.get_regular_duration(), Some(&28800.0));

    assert_eq!(vehicles[1].dimens.get_overtime_rate(), Some(&0.02));
    assert_eq!(vehicles[1].dimens.get_regular_duration(), Some(&21600.0));
}

#[test]
fn reads_visit_windows_and_the_job_tag() {
    let matrix = matrix(Some("car"), None, 1, 9);

    let vehicle = VehicleType {
        shifts: vec![VehicleShift {
            start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
            end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (0., 0.).to_loc() }),
            visit_windows: Some(VisitWindowsJson {
                recurring: Some(VisitWindowJson { earliest: format_time(100.), latest: format_time(400.) }),
                non_recurring: Some(VisitWindowJson { earliest: format_time(100.), latest: format_time(800.) }),
                overflow: Some(true),
            }),
            ..create_default_vehicle_shift()
        }],
        ..create_default_vehicle_type()
    };

    let problem = Problem {
        plan: Plan {
            jobs: vec![
                Job { visit_window: Some("recurring".to_string()), ..create_delivery_job("job1", (1., 1.)) },
                create_delivery_job("job2", (1., 0.)),
            ],
            relations: None,
            clustering: None,
        },
        fleet: Fleet { vehicles: vec![vehicle], profiles: create_default_matrix_profiles(), resources: None },
        objectives: None,
    };

    let problem = (problem, vec![matrix]).read_pragmatic().unwrap();

    let windows = problem.fleet.vehicles[0].dimens.get_visit_windows().expect("windows");
    assert_eq!(windows.recurring.as_ref().unwrap().earliest, 100.);
    assert_eq!(windows.recurring.as_ref().unwrap().latest, 400.);
    assert_eq!(windows.non_recurring.as_ref().unwrap().latest, 800.);
    assert!(windows.overflow);

    let kind_of = |id: &str| {
        problem
            .jobs
            .all()
            .iter()
            .find(|job| job.dimens().get_job_id().map(String::as_str) == Some(id))
            .and_then(|job| job.to_single().dimens.get_visit_window_kind().cloned())
    };
    assert_eq!(kind_of("job1"), Some(VisitWindowKind::Recurring));
    assert_eq!(kind_of("job2"), None);
}

#[test]
fn reads_off_hours_rate_and_regular_hours_into_dimens() {
    let matrix = matrix(Some("car"), None, 1, 4);

    let vehicle = VehicleType {
        costs: VehicleCosts { off_hours: Some(0.03), ..create_default_vehicle_costs() },
        shifts: vec![
            VehicleShift {
                start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                end: Some(ShiftEnd { earliest: None, latest: format_time(99.), location: (0., 0.).to_loc() }),
                regular_hours: Some(RegularHoursJson { earliest: format_time(10.), latest: format_time(90.) }),
                ..create_default_vehicle_shift()
            },
            VehicleShift {
                start: ShiftStart { earliest: format_time(100.), latest: None, location: (0., 0.).to_loc() },
                end: Some(ShiftEnd { earliest: None, latest: format_time(200.), location: (0., 0.).to_loc() }),
                ..create_default_vehicle_shift()
            },
        ],
        ..create_default_vehicle_type()
    };

    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", (1., 1.))], relations: None, clustering: None },
        fleet: Fleet { vehicles: vec![vehicle], profiles: create_default_matrix_profiles(), resources: None },
        objectives: None,
    };

    let problem = (problem, vec![matrix]).read_pragmatic().unwrap();

    let vehicles = &problem.fleet.vehicles;
    let hours = vehicles[0].dimens.get_regular_hours().copied().unwrap();
    assert_eq!((hours.earliest, hours.latest), (10., 90.));
    assert_eq!(vehicles[0].dimens.get_off_hours_rate(), Some(&0.03));
    assert!(vehicles[1].dimens.get_regular_hours().is_none());
    assert_eq!(vehicles[1].dimens.get_off_hours_rate(), Some(&0.03));
}
