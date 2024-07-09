use crate::format::problem::Objective::*;
use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_balance_duration() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_duration("job1", (1., 0.), 10.),
                create_delivery_job_with_duration("job2", (2., 0.), 10.),
                create_delivery_job_with_duration("job3", (3., 0.), 10.),
                create_delivery_job_with_duration("job4", (4., 0.), 10.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![3],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: Some(vec![MinimizeUnassigned { breaks: None }, BalanceDuration, MinimizeCost]),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution.tours.len(), 2);
    assert!(solution.tours.first().unwrap().statistic.duration < 30);
    assert!(solution.tours.last().unwrap().statistic.duration < 30);
}
