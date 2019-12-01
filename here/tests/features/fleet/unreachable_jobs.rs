use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_vehicle_with_open_end() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![1., 0.])], relations: Option::None },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                places: create_default_open_vehicle_places(),
                capacity: vec![10],
                amount: 1,
                skills: None,
                limits: None,
            }],
        },
    };
    let matrix = Matrix {
        num_origins: 2,
        num_destinations: 2,
        travel_times: vec![0, 1, 1, 0],
        distances: vec![0, 1, 1, 0],
        error_codes: Some(vec![0, 1, 1, 1]),
    };

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
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
            extras: Extras { performance: vec![] },
        }
    );
}
