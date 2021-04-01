use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_follow_priorities() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_priority("job1", vec![2., 0.], 3),
                create_delivery_job_with_priority("job2", vec![5., 0.], 2),
                create_delivery_job("job3", vec![7., 0.]),
            ],
            relations: Option::None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_vehicle_shift_with_locations((0., 0.), (10., 0.))],
                ..create_default_vehicle_type()
            }],
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
                cost: 53.,
                distance: 20,
                duration: 23,
                times: Timing { driving: 20, serving: 3, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        3,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (7., 0.),
                        2,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        7
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (5., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        9
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:14Z", "1970-01-01T00:00:15Z"),
                        12
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (10., 0.),
                        0,
                        ("1970-01-01T00:00:23Z", "1970-01-01T00:00:23Z"),
                        20
                    )
                ],
                statistic: Statistic {
                    cost: 53.,
                    distance: 20,
                    duration: 23,
                    times: Timing { driving: 20, serving: 3, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
