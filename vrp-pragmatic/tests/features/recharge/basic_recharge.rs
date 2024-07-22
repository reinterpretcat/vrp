use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_recharge_trivial_case() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (30., 0.)), create_delivery_job("job2", (70., 0.))],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    recharges: Some(VehicleRecharges {
                        max_distance: 55.,
                        stations: vec![JobPlace {
                            location: (50., 0.).to_loc(),
                            duration: 0.0,
                            times: None,
                            tag: None,
                        }],
                    }),
                    ..create_default_vehicle_shift_with_locations((0., 0.), (100., 0.))
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert!(!solution.tours.is_empty());
    assert_eq!(
        get_ids_from_tour(&solution.tours[0]),
        vec![vec!["departure"], vec!["job1"], vec!["recharge"], vec!["job2"], vec!["arrival"]]
    );
}

#[test]
fn can_still_skip_jobs_with_recharge() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (52.5577, 13.4783)),
                create_delivery_job("job2", (52.4838, 13.4319)),
                create_delivery_job("job3", (52.4656, 13.4485)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    end: Some(ShiftEnd {
                        earliest: None,
                        latest: format_time(3600. * 12.),
                        location: (52.5189, 13.4011).to_loc(),
                    }),
                    recharges: Some(VehicleRecharges {
                        max_distance: 10000.,
                        stations: vec![JobPlace {
                            location: (52.5459, 13.5058).to_loc(),
                            duration: 900.,
                            times: None,
                            tag: None,
                        }],
                    }),
                    ..create_default_vehicle_shift_with_locations((52.5189, 13.4011), (52.5189, 13.4011))
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let solution = solve_with_metaheuristic(problem, None);

    assert!(!solution.tours.is_empty());
    assert_eq!(solution.unassigned.iter().flatten().count(), 2);
}

#[test]
fn can_use_recharge_with_ten_jobs() {
    let problem = ApiProblem {
        plan: Plan {
            jobs: (1..=10).map(|idx| create_delivery_job(&format!("job{idx}"), ((idx as f64) * 10., 0.))).collect(),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    recharges: Some(VehicleRecharges {
                        max_distance: 55.,
                        stations: vec![VehicleRechargeStation {
                            location: (50., 0.).to_loc(),
                            duration: 0.0,
                            times: None,
                            tag: None,
                        }],
                    }),
                    ..create_default_open_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
            resources: None,
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(
        get_ids_from_tour(&solution.tours[0]),
        vec![
            vec!["departure"],
            vec!["job1"],
            vec!["job2"],
            vec!["job3"],
            vec!["job4"],
            vec!["recharge", "job5"],
            vec!["job6"],
            vec!["job7"],
            vec!["job8"],
            vec!["job9"],
            vec!["job10"]
        ]
    );
}
