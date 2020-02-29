use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_unassign_multi_job_due_to_capacity() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_multi_job(
                "multi",
                vec![((2., 0.), 1., vec![2]), ((8., 0.), 1., vec![1])],
                vec![((6., 0.), 1., vec![3])],
            )],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 0.,
                distance: 0,
                duration: 0,
                times: Timing { driving: 0, serving: 0, waiting: 0, break_time: 0 },
            },
            tours: vec![],
            unassigned: vec![UnassignedJob {
                job_id: "multi".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: 3,
                    description: "does not fit into any vehicle due to capacity".to_string()
                }]
            }],
            extras: None,
        }
    );
}
