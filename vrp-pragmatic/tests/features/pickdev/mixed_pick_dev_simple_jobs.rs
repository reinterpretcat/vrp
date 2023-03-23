use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_use_one_pickup_delivery_and_two_deliveries_with_one_vehicle() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_pickup_delivery_job("job2", (2., 0.), (3., 0.)),
                create_delivery_job("job3", (4., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 30.,
                distance: 8,
                duration: 12,
                times: Timing { driving: 8, serving: 4, ..Timing::default() },
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
                        2,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity_with_tag(
                        "job2",
                        "pickup",
                        (2., 0.),
                        3,
                        ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                        2,
                        "p1"
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (4., 0.),
                        2,
                        ("1970-01-01T00:00:05Z", "1970-01-01T00:00:06Z"),
                        4
                    ),
                    create_stop_with_activity_with_tag(
                        "job2",
                        "delivery",
                        (3., 0.),
                        1,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        5,
                        "d1"
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        0,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        7
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:12Z"),
                        8
                    )
                ],
                statistic: Statistic {
                    cost: 30.,
                    distance: 8,
                    duration: 12,
                    times: Timing { driving: 8, serving: 4, ..Timing::default() },
                }
            }],
            ..create_empty_solution()
        }
    );
}
