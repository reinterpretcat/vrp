use crate::format::problem::*;
use crate::format::solution::*;
use crate::helpers::*;

#[test]
fn can_create_solution() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![10., 0.])],
            relations: Option::None,
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("my_vehicle")], profiles: create_default_profiles() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 52.,
                distance: 20,
                duration: 22,
                times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
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
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (10., 0.),
                        1,
                        ("1970-01-01T00:00:10Z", "1970-01-01T00:00:11Z"),
                        10
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (5., 0.),
                        0,
                        ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z"),
                        15
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:22Z"),
                        20
                    )
                ],
                statistic: Statistic {
                    cost: 52.,
                    distance: 20,
                    duration: 22,
                    times: Timing { driving: 20, serving: 2, waiting: 0, break_time: 0 },
                },
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
fn can_merge_activities_in_one_stop() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![5., 0.]), create_delivery_job("job2", vec![5., 0.])],
            relations: Option::None,
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle("my_vehicle")], profiles: create_default_profiles() },
        ..create_empty_problem()
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_cheapest_insertion(problem, Some(vec![matrix]));

    assert_eq!(
        solution.statistic,
        Statistic {
            cost: 32.,
            distance: 10,
            duration: 12,
            times: Timing { driving: 10, serving: 2, waiting: 0, break_time: 0 },
        }
    );
    assert_eq!(solution.tours.len(), 1);
    assert_eq!(solution.tours.first().unwrap().stops.len(), 3);
    assert_eq!(solution.tours.first().unwrap().stops.get(1).unwrap().activities.len(), 2);
}
