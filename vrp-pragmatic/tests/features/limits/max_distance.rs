use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_limit_by_max_distance() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![100., 0.])], relations: Option::None },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                limits: Some(VehicleLimits { max_distance: Some(99.), shift_time: None }),
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = Matrix { travel_times: vec![1, 1, 1, 1], distances: vec![1, 100, 100, 1], error_codes: Option::None };

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
                reasons: vec![UnassignedJobReason {
                    code: 101,
                    description: "cannot be assigned due to max distance constraint of vehicle".to_string()
                }]
            }],
            extras: None,
        }
    );
}
