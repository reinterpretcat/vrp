use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_have_unassigned_due_to_missing_vehicle_skill() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_skills("job1", vec![1., 0.], vec!["unique_skill".to_string()])],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("vehicle_without_skill")],
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
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason { code: 1, description: "cannot serve required skill".to_string() }]
            }],
            extras: None,
        }
    );
}
