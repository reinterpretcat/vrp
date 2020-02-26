use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_even_dist_jobs() {
    let problem = Problem {
        id: "my_problem".to_string(),
        plan: Plan {
            jobs: vec![create_delivery_job("job1", vec![1., 0.]), create_delivery_job("job2", vec![1., 0.])],
            relations: None,
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![2],
                amount: 2,
                skills: None,
                limits: None,
            }],
            profiles: create_default_profiles(),
        },
        config: Some(Config {
            features: Some(Features {
                even_distribution: Some(EvenDistribution { enabled: true, extra_cost: Some(1000.0) }),
            }),
        }),
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_vehicle_agnostic(
        solution,
        Solution {
            problem_id: "my_problem".to_string(),
            statistic: Statistic {
                cost: 26.,
                distance: 2,
                duration: 4,
                times: Timing { driving: 2, serving: 2, waiting: 0, break_time: 0 },
            },
            tours: vec![
                Tour {
                    vehicle_id: "my_vehicle_1".to_string(),
                    type_id: "my_vehicle".to_string(),
                    shift_index: 0,
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            1,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                            0,
                        ),
                        create_stop_with_activity(
                            "job1",
                            "delivery",
                            (1., 0.),
                            0,
                            ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                            1,
                        ),
                    ],
                    statistic: Statistic {
                        cost: 13.,
                        distance: 1,
                        duration: 2,
                        times: Timing { driving: 1, serving: 1, waiting: 0, break_time: 0 },
                    },
                },
                Tour {
                    vehicle_id: "my_vehicle_2".to_string(),
                    type_id: "my_vehicle".to_string(),
                    shift_index: 0,
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            1,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                            0,
                        ),
                        create_stop_with_activity(
                            "job2",
                            "delivery",
                            (1., 0.),
                            0,
                            ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                            1,
                        ),
                    ],
                    statistic: Statistic {
                        cost: 13.,
                        distance: 1,
                        duration: 2,
                        times: Timing { driving: 1, serving: 1, waiting: 0, break_time: 0 },
                    },
                },
            ],
            unassigned: vec![],
            extras: None,
        },
    );
}
