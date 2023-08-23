use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_use_recharge() {
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
                    ..create_default_vehicle_shift_with_locations((0., 0.), (100., 100.))
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        get_ids_from_tour(&solution.tours[0]),
        vec![vec!["departure"], vec!["job1"], vec!["recharge"], vec!["job2"], vec!["arrival"]]
    );
}
