use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_reloads_with_different_locations() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (10., 0.)),
                create_delivery_job("job2", (11., 0.)),
                create_delivery_job("job3", (20., 0.)),
                create_delivery_job("job4", (21., 0.)),
                create_delivery_job("job5", (30., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (32., 0.).to_loc() }),
                    dispatch: None,
                    breaks: None,
                    reloads: Some(vec![
                        VehicleReload {
                            times: None,
                            location: (12., 0.).to_loc(),
                            duration: 2.0,
                            tag: Some("close".to_string()),
                        },
                        VehicleReload {
                            times: None,
                            location: (33., 0.).to_loc(),
                            duration: 2.0,
                            tag: Some("far".to_string()),
                        },
                    ]),
                }],
                capacity: vec![2],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 1);
    let has_reload = get_ids_from_tour(solution.tours.first().unwrap())
        .into_iter()
        .flat_map(|stop| stop.into_iter())
        .any(|stop| stop == "reload");
    assert!(has_reload);
}
