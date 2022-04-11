use crate::format::problem::*;
use crate::helpers::*;
use std::panic::catch_unwind;

#[test]
fn can_skip_constraints_check() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", (1., 0.)), create_delivery_job("job2", (2., 0.))],
            relations: Some(vec![Relation {
                type_field: RelationType::Any,
                jobs: to_strings(vec!["departure", "job1", "job2"]),
                vehicle_id: "my_vehicle_1".to_string(),
                shift_index: None,
            }]),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType { capacity: vec![1], ..create_default_vehicle_type() }],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let result = catch_unwind(|| solve_with_metaheuristic(problem, Some(vec![matrix])))
        .map_err(|err| err.downcast_ref::<String>().cloned());

    match result {
        Err(Some(err)) => {
            assert!(err.starts_with("check failed: 'load exceeds capacity in tour 'my_vehicle_1'"));
        }
        Err(None) => unreachable!("unknown panic message type"),
        Ok(_) => unreachable!("unexpected load or missing checker rule"),
    }
}
