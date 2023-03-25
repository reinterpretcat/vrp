use crate::format::problem::*;
use crate::format::Location;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_serve_multi_job_and_delivery_with_reload() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("simple1", (1., 0.)),
                create_delivery_job("simple2", (3., 0.)),
                create_delivery_job("simple3", (7., 0.)),
                create_multi_job(
                    "multi",
                    vec![((2., 0.), 1., vec![1]), ((8., 0.), 1., vec![1])],
                    vec![((9., 0.), 1., vec![2])],
                ),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(100.), location: (10., 0.).to_loc() }),
                    dispatch: None,
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        location: (0., 0.).to_loc(),
                        duration: 2.0,
                        ..create_default_reload()
                    }]),
                }],
                capacity: vec![2],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic_and_iterations(problem, Some(vec![matrix]), 1000);

    // TODO STABILITY: investigate why cost is not stable: it can be 46 or 50
    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.tours[0].stops.len(), 9);
}

#[test]
fn can_properly_handle_load_without_capacity_violation() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_pickup_delivery_job_with_params(
                    "job1",
                    vec![2],
                    ((52., 0.), 10., vec![]),
                    ((1., 0.), 12., vec![]),
                ),
                create_pickup_job_with_demand("job2", (67., 0.), vec![2]),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                costs: VehicleCosts { fixed: Some(20.0), distance: 0.002, time: 0.003 },
                shifts: vec![VehicleShift {
                    reloads: Some(vec![
                        VehicleReload {
                            location: Location::Coordinate { lat: 0.0, lng: 0.0 },
                            duration: 2620.0,
                            ..create_default_reload()
                        },
                        VehicleReload {
                            location: Location::Coordinate { lat: 0.0, lng: 0.0 },
                            duration: 2874.0,
                            ..create_default_reload()
                        },
                    ]),
                    ..create_default_vehicle_shift()
                }],
                capacity: vec![2],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    solve_with_metaheuristic(problem, None);
}
