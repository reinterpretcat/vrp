use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_balance_max_load() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![3., 0.]),
                create_delivery_job("job4", vec![4., 0.]),
                create_delivery_job("job5", vec![5., 0.]),
                create_delivery_job("job6", vec![6., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![5],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        objectives: Some(Objectives {
            primary: vec![BalanceMaxLoad { threshold: None, tolerance: None }],
            secondary: Some(vec![MinimizeCost { goal: None, tolerance: None }]),
        }),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 2);
    assert_eq!(solution.tours.first().unwrap().stops.len(), 4);
    assert_eq!(solution.tours.last().unwrap().stops.len(), 4);
}
