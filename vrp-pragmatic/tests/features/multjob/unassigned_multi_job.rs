use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_unassign_multi_job_due_to_capacity() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job(
                "multi",
                vec![((2., 0.), 1., vec![2]), ((8., 0.), 1., vec![1])],
                vec![((6., 0.), 1., vec![3])],
            )],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic::default(),
            tours: vec![],
            unassigned: Some(vec![UnassignedJob {
                job_id: "multi".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "CAPACITY_CONSTRAINT".to_string(),
                    description: "does not fit into any vehicle due to capacity".to_string()
                }]
            }]),
            ..create_empty_solution()
        }
    );
}
