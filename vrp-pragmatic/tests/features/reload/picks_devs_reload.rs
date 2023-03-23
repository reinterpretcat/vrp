use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_vehicle_with_pickups_and_deliveries() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("d1", (1., 0.)),
                create_delivery_job("d2", (4., 0.)),
                create_delivery_job("d3", (10., 0.)),
                create_pickup_job("p1", (2., 0.)),
                create_pickup_job("p2", (5., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(100.), location: (6., 0.).to_loc() }),
                    dispatch: None,
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        location: (3., 0.).to_loc(),
                        duration: 2.0,
                        ..create_default_reload()
                    }]),
                }],
                capacity: vec![1],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 2000);

    assert_eq!(
        solution.tours,
        vec![Tour {
            vehicle_id: "my_vehicle_1".to_string(),
            type_id: "my_vehicle".to_string(),
            shift_index: 0,
            stops: vec![
                create_stop_with_activity(
                    "departure",
                    "departure",
                    (0., 0.),
                    1,
                    ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    0
                ),
                create_stop_with_activity(
                    "d1",
                    "delivery",
                    (1., 0.),
                    0,
                    ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                    1
                ),
                create_stop_with_activity(
                    "p1",
                    "pickup",
                    (2., 0.),
                    1,
                    ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                    2
                ),
                create_stop_with_activity(
                    "reload",
                    "reload",
                    (3., 0.),
                    1,
                    ("1970-01-01T00:00:05Z", "1970-01-01T00:00:07Z"),
                    3
                ),
                create_stop_with_activity(
                    "d2",
                    "delivery",
                    (4., 0.),
                    0,
                    ("1970-01-01T00:00:08Z", "1970-01-01T00:00:09Z"),
                    4
                ),
                create_stop_with_activity(
                    "p2",
                    "pickup",
                    (5., 0.),
                    1,
                    ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                    5
                ),
                create_stop_with_activity(
                    "arrival",
                    "arrival",
                    (6., 0.),
                    0,
                    ("1970-01-01T00:00:12Z", "1970-01-01T00:00:12Z"),
                    6
                ),
            ],
            statistic: Statistic {
                cost: 28.,
                distance: 6,
                duration: 12,
                times: Timing { driving: 6, serving: 6, ..Timing::default() },
            },
        }]
    );
    assert_eq!(
        solution.statistic,
        Statistic {
            cost: 28.,
            distance: 6,
            duration: 12,
            times: Timing { driving: 6, serving: 6, ..Timing::default() },
        }
    );
    assert!(solution.violations.is_none());

    // NOTE reason can be sometimes NO_REASON_FOUND or CAPACITY_CONSTRAINT
    let unassigned = solution.unassigned.expect("no unassigned");
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned.first().unwrap().job_id, "d3");
}
