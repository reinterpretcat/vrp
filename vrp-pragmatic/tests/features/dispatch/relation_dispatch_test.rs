use crate::format::problem::*;
use crate::format_time;
use crate::helpers::*;

#[test]
fn can_use_dispatch_in_relation() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
            ],
            relations: Some(vec![Relation {
                type_field: RelationType::Strict,
                jobs: to_strings(vec!["departure", "dispatch", "job1", "job2", "job3"]),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    dispatch: Some(vec![VehicleDispatch {
                        location: (7., 0.).to_loc(),
                        limits: vec![VehicleDispatchLimit { max: 1, start: format_time(7.), end: format_time(9.) }],
                        tag: None,
                    }]),
                    ..create_default_vehicle_shift()
                }],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 1);
    assert_eq!(
        get_ids_from_tour(solution.tours.first().unwrap()),
        vec![vec!["departure"], vec!["dispatch"], vec!["job1"], vec!["job2"], vec!["job3"], vec!["arrival"]]
    );
}
