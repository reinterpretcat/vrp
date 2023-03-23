use crate::format::problem::*;
use crate::helpers::*;

#[test]
fn can_split_into_two_tours_because_of_strict_times() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_times("job1", (10., 0.), vec![(70, 80)], 10.),
                create_delivery_job_with_times("job2", (20., 0.), vec![(50, 60)], 10.),
                create_delivery_job_with_times("job3", (30., 0.), vec![(0, 40), (100, 120)], 10.),
                create_delivery_job_with_times("job4", (40., 0.), vec![(0, 40)], 10.),
                create_delivery_job_with_times("job5", (50., 0.), vec![(50, 60)], 10.),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        objectives: create_min_jobs_cost_objective(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert!(solution.unassigned.is_none());
    assert_eq!(solution.tours.len(), 2);
}
