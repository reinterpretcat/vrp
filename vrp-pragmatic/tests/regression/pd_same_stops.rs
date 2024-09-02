use crate::format::problem::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::common::Duration;

/// Checks whether two p&d jobs can be assigned to two stops as they share the same locations.
/// The unusual part here is routing: it returns faster duration for the path with intermediate location.
#[test]
fn can_handle_two_pd_jobs_with_same_locations_and_unusual_routing() {
    let create_test_task_with_loc_ref = |index: usize, tag: &str| JobTask {
        places: vec![JobPlace {
            location: Location::Reference { index },
            duration: Duration::default(),
            times: None,
            tag: Some(tag.to_string()),
        }],
        demand: Some(vec![1]),
        order: None,
    };

    let problem = Problem {
        plan: Plan {
            jobs: vec![
                Job {
                    pickups: Some(vec![create_test_task_with_loc_ref(0, "p1")]),
                    deliveries: Some(vec![create_test_task_with_loc_ref(1, "d1")]),
                    ..create_job("job1")
                },
                Job {
                    pickups: Some(vec![create_test_task_with_loc_ref(0, "p2")]),
                    deliveries: Some(vec![create_test_task_with_loc_ref(1, "d2")]),
                    ..create_job("job2")
                },
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: VehicleCosts { fixed: None, distance: 0.0, time: 1.0 },
                shifts: vec![VehicleShift {
                    start: ShiftStart {
                        earliest: format_time(0),
                        latest: None,
                        location: Location::Reference { index: 2 },
                    },
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(7200),
                        location: Location::Reference { index: 2 },
                    }),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let matrices = vec![Matrix {
        profile: Some("car".to_string()),
        timestamp: None,
        travel_times: vec![0, 220, 2045, 152, 0, 2198, 2069, 2290, 0],
        distances: vec![0, 1612, 19774, 1155, 0, 20929, 20609, 22221, 0],
        error_codes: None,
    }];

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(matrices), 1000);

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.tours[0].stops.len(), 4);
}
