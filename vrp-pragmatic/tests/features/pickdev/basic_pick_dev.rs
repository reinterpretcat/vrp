use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_use_one_pickup_delivery_job_with_one_vehicle() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_pickup_delivery_job("job1", vec![1., 0.], vec![2., 0.])],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![create_default_vehicle("my_vehicle")],
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
                cost: 20.,
                distance: 4,
                duration: 6,
                times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 0 },
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
                        0,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity_with_tag(
                        "job1",
                        "pickup",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                        1,
                        "p1"
                    ),
                    create_stop_with_activity_with_tag(
                        "job1",
                        "delivery",
                        (2., 0.),
                        0,
                        ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                        2,
                        "d1"
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:06Z", "1970-01-01T00:00:06Z"),
                        4
                    )
                ],
                statistic: Statistic {
                    cost: 20.,
                    distance: 4,
                    duration: 6,
                    times: Timing { driving: 4, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}
