use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_limit_by_shift_time() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![100., 0.])], relations: Option::None },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                places: create_default_vehicle_places(),
                capacity: vec![10],
                amount: 1,
                skills: None,
                limits: Some(VehicleLimits { max_distance: None, shift_time: Some(99.) }),
                vehicle_break: None,
            }],
        },
    };
    let matrix = Matrix {
        num_origins: 2,
        num_destinations: 2,
        travel_times: vec![1, 100, 100, 1],
        distances: vec![1, 1, 1, 1],
        error_codes: Option::None,
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
                reasons: vec![UnassignedJobReason {
                    code: 102,
                    description: "cannot be assigned due to shift time constraint of vehicle".to_string()
                }]
            }],
            extras: Extras { performance: vec![] },
        }
    );
}
