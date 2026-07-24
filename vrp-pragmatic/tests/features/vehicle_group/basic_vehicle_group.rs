use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;
use std::collections::HashSet;

/// Builds a delivery job at a fixed location, tagged with a vehicle group, restricted to a
/// given time window (so it can only be reached during one specific shift).
fn create_test_job(id: &str, times: (f64, f64), vehicle_group: &str) -> Job {
    Job {
        deliveries: Some(vec![JobTask {
            places: vec![JobPlace {
                location: (1., 0.).to_loc(),
                duration: 10.,
                times: Some(vec![vec![format_time(times.0), format_time(times.1)]]),
                tag: None,
            }],
            demand: Some(vec![1]),
            order: None,
            due_date: None,
        }]),
        vehicle_group: Some(vehicle_group.to_string()),
        ..create_job(id)
    }
}

fn create_second_day_shift(location: (f64, f64)) -> VehicleShift {
    VehicleShift {
        start: ShiftStart { earliest: format_time(86400.), latest: None, location: location.to_loc() },
        end: Some(ShiftEnd { earliest: None, latest: format_time(87400.), location: location.to_loc() }),
        ..create_default_vehicle_shift_with_locations(location, location)
    }
}

#[test]
fn can_bind_vehicle_group_jobs_to_same_vehicle_across_shifts() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_test_job("job_day1", (100., 900.), "sub-1"),
                create_test_job("job_day2", (86500., 87300.), "sub-1"),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                // Vehicle able to serve both days: two shifts, one per day, at the depot.
                VehicleType {
                    type_id: "multi_shift".to_string(),
                    vehicle_ids: vec!["multi_shift_1".to_string()],
                    shifts: vec![
                        create_default_vehicle_shift_with_locations((0., 0.), (0., 0.)),
                        create_second_day_shift((0., 0.)),
                    ],
                    ..create_default_vehicle_type()
                },
                // A second vehicle parked right on top of the jobs' location: without the
                // vehicle group constraint, it is the cheapest way to serve the day-1 job alone.
                VehicleType {
                    type_id: "single_shift".to_string(),
                    vehicle_ids: vec!["single_shift_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((1., 0.), (1., 0.))],
                    ..create_default_vehicle_type()
                },
            ],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let matrix = create_matrix_from_problem(&problem);
    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none(), "both vehicleGroup jobs must be assigned, not left unassigned");

    let vehicle_ids = solution
        .tours
        .iter()
        .filter(|tour| {
            tour.stops
                .iter()
                .flat_map(|stop| stop.activities().iter())
                .any(|activity| activity.job_id == "job_day1" || activity.job_id == "job_day2")
        })
        .map(|tour| tour.vehicle_id.clone())
        .collect::<HashSet<_>>();

    assert_eq!(vehicle_ids.len(), 1, "both vehicleGroup jobs should be served by the same vehicle");

    let served_job_ids = solution
        .tours
        .iter()
        .flat_map(|tour| tour.stops.iter().flat_map(|stop| stop.activities().iter()))
        .map(|activity| activity.job_id.as_str())
        .filter(|id| *id == "job_day1" || *id == "job_day2")
        .collect::<HashSet<_>>();

    assert_eq!(served_job_ids.len(), 2, "both vehicleGroup jobs must actually be served, not just one");
}
