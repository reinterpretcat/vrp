use crate::format::problem::*;
use crate::format::solution::*;
use crate::format::Location;
use crate::helpers::*;

#[test]
fn can_limit_by_area() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_delivery_job("job1", vec![10., 0.])], relations: Option::None },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                limits: Some(VehicleLimits {
                    max_distance: None,
                    shift_time: None,
                    allowed_areas: Some(vec![vec![
                        Location::new(-5., -5.),
                        Location::new(5., -5.),
                        Location::new(5., 5.),
                        Location::new(-5., 5.),
                    ]]),
                }),
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

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
            unassigned: Some(vec![UnassignedJob {
                job_id: "job1".to_string(),
                reasons: vec![UnassignedJobReason {
                    code: 106,
                    description: "cannot be assigned due to area constraint".to_string()
                }]
            }]),
            ..create_empty_solution()
        }
    );
}
