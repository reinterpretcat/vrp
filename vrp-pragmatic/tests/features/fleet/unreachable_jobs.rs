use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_vehicle_with_open_end() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![1., 0.])], relations: Option::None },
        fleet: Fleet { vehicles: vec![create_default_vehicle_type()], profiles: create_default_profiles() },
        ..create_empty_problem()
    };
    let matrix = Matrix {
        profile: "car".to_owned(),
        timestamp: None,
        travel_times: vec![0, 1, 1, 0],
        distances: vec![0, 1, 1, 0],
        error_codes: Some(vec![0, 1, 1, 1]),
    };

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
                reasons: vec![UnassignedJobReason { code: 100, description: "location unreachable".to_string() }]
            }],
            extras: None,
        }
    );
}
