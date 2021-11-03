use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_wait_for_job_start() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_skills(
                "job1",
                vec![1., 0.],
                all_of_skills(vec!["unique_skill".to_string()]),
            )],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![
                create_default_vehicle("vehicle_without_skill"),
                VehicleType {
                    type_id: "vehicle_with_skill".to_string(),
                    vehicle_ids: vec!["vehicle_with_skill_1".to_string()],
                    shifts: vec![create_default_vehicle_shift_with_locations((10., 0.), (10., 0.))],
                    skills: Some(vec!["unique_skill".to_string()]),
                    ..create_default_vehicle_type()
                },
            ],
            profiles: create_default_matrix_profiles(),
        },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 47.,
                distance: 18,
                duration: 19,
                times: Timing { driving: 18, serving: 1, ..Timing::default() },
            },
            tours: vec![Tour {
                vehicle_id: "vehicle_with_skill_1".to_string(),
                type_id: "vehicle_with_skill".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:09Z", "1970-01-01T00:00:10Z"),
                        9
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:19Z", "1970-01-01T00:00:19Z"),
                        18
                    )
                ],
                statistic: Statistic {
                    cost: 47.,
                    distance: 18,
                    duration: 19,
                    times: Timing { driving: 18, serving: 1, ..Timing::default() },
                },
            }],
            ..create_empty_solution()
        }
    );
}
