use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_use_vehicle_with_open_end() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![1., 0.])], ..create_empty_plan() },
        fleet: Fleet { vehicles: vec![create_default_vehicle_type()], profiles: create_default_matrix_profiles() },
        ..create_empty_problem()
    };
    let matrix = Matrix {
        profile: Some("car".to_owned()),
        timestamp: None,
        travel_times: vec![0, 1, 1, 0],
        distances: vec![0, 1, 1, 0],
        error_codes: Some(vec![0, 1, 1, 1]),
    };

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 0.,
                distance: 0,
                duration: 0,
                times: Timing { driving: 0, serving: 0, ..Timing::default() },
            },
            tours: vec![],
            unassigned: Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: "REACHABLE_CONSTRAINT".to_string(),
                    description: "location unreachable".to_string()
                }]
            }]),
            ..create_empty_solution()
        }
    );
}
