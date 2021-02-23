use super::*;
use crate::format_time;
use crate::helpers::*;

mod single {
    use super::*;
    use crate::format::solution::Tour as VehicleTour;
    use vrp_core::models::examples::create_example_problem;
    use RelationType::{Any, Sequence, Strict};

    fn create_relation(job_ids: Vec<&str>, relation_type: RelationType) -> Relation {
        Relation {
            type_field: relation_type,
            jobs: job_ids.iter().map(|id| id.to_string()).collect(),
            vehicle_id: "my_vehicle_1".to_string(),
            shift_index: None,
        }
    }

    fn create_relation_with_wrong_id(vehicle_id: &str) -> Relation {
        Relation {
            type_field: Sequence,
            jobs: vec!["job1".to_string()],
            vehicle_id: vehicle_id.to_string(),
            shift_index: None,
        }
    }

    fn create_relation_with_wrong_shift() -> Relation {
        Relation {
            type_field: Sequence,
            jobs: vec!["job1".to_string()],
            vehicle_id: "my_vehicle_1".to_string(),
            shift_index: Some(1),
        }
    }

    parameterized_test! {can_check_relations, (relations, expected_result), {
        can_check_relations_impl(relations, expected_result);
    }}

    can_check_relations! {
        case_sequence_01: (Some(vec![create_relation(vec!["departure", "job1", "job2"], Strict)]), Ok(())),
        case_sequence_02: (Some(vec![create_relation(vec!["job1", "job2"], Strict)]), Ok(())),
        case_sequence_03: (Some(vec![create_relation(vec!["job1", "job2"], Strict),
                                     create_relation(vec!["job4", "job5"], Strict)]), Ok(())),
        case_sequence_04: (Some(vec![create_relation(vec!["departure", "job1"], Strict),
                                     create_relation(vec!["job3", "reload"], Strict)]), Ok(())),
        case_sequence_05: (Some(vec![create_relation(vec!["departure", "job2", "job1"], Strict)]), Err(())),
        case_sequence_06: (Some(vec![create_relation(vec!["departure", "job1", "job1"], Strict)]), Err(())),
        case_sequence_07: (Some(vec![create_relation(vec!["departure", "job1", "job3"], Strict)]), Err(())),
        case_sequence_08: (Some(vec![create_relation(vec!["job1", "job2", "job7"], Strict)]), Err(())),

        case_flexible_01: (Some(vec![create_relation(vec!["departure", "job1", "job3"], Sequence)]), Ok(())),
        case_flexible_02: (Some(vec![create_relation(vec!["job1", "job3"], Sequence)]), Ok(())),
        case_flexible_03: (Some(vec![create_relation(vec!["departure", "job2", "job1"], Sequence)]), Err(())),

        case_tour_01:     (Some(vec![create_relation(vec!["departure", "job1", "job3"], Any)]), Ok(())),
        case_tour_02:     (Some(vec![create_relation(vec!["job1", "job2"], Any)]), Ok(())),
        case_tour_03:     (Some(vec![create_relation(vec!["job2", "job3"], Any)]), Ok(())),

        case_mixed_01:    (Some(vec![create_relation(vec!["departure", "job1"], Strict),
                                     create_relation(vec!["job3", "job4"], Sequence)]), Ok(())),

        case_wrong_vehicle_01: (Some(vec![create_relation_with_wrong_id("my_vehicle_2")]), Err(())),
        case_wrong_vehicle_02: (Some(vec![create_relation_with_wrong_id("my_vehicle_x")]), Err(())),
        case_wrong_vehicle_03: (Some(vec![create_relation_with_wrong_shift()]), Err(())),
    }

    fn can_check_relations_impl(relations: Option<Vec<Relation>>, expected_result: Result<(), ()>) {
        let problem = Problem {
            plan: Plan {
                jobs: vec![
                    create_delivery_job("job1", vec![1., 0.]),
                    create_delivery_job("job2", vec![2., 0.]),
                    create_pickup_job("job3", vec![3., 0.]),
                    create_delivery_job("job4", vec![4., 0.]),
                    create_pickup_job("job5", vec![5., 0.]),
                ],
                relations,
            },
            fleet: Fleet {
                vehicles: vec![VehicleType {
                    type_id: "my_vehicle".to_string(),
                    vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                    profile: "car".to_string(),
                    costs: create_default_vehicle_costs(),
                    shifts: vec![VehicleShift {
                        start: ShiftStart { earliest: format_time(0.), latest: None, location: vec![0., 0.].to_loc() },
                        end: Some(ShiftEnd {
                            earliest: None,
                            latest: format_time(1000.).to_string(),
                            location: vec![0., 0.].to_loc(),
                        }),
                        dispatch: None,
                        breaks: Some(vec![VehicleBreak {
                            time: VehicleBreakTime::TimeWindow(vec![format_time(0.), format_time(1000.)]),
                            duration: 2.0,
                            locations: None,
                            tag: None,
                        }]),
                        reloads: Some(vec![VehicleReload {
                            times: None,
                            location: vec![0., 0.].to_loc(),
                            duration: 2.0,
                            tag: None,
                        }]),
                    }],
                    capacity: vec![5],
                    skills: None,
                    limits: None,
                }],
                profiles: create_default_profiles(),
            },
            ..create_empty_problem()
        };
        let solution = Solution {
            statistic: Statistic {
                cost: 51.,
                distance: 16,
                duration: 25,
                times: Timing { driving: 16, serving: 9, waiting: 0, break_time: 2 },
            },
            tours: vec![
                VehicleTour {
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
                            0,
                        ),
                        create_stop_with_activity(
                            "job1",
                            "delivery",
                            (1., 0.),
                            1,
                            ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                            1,
                        ),
                        Stop {
                            location: vec![2., 0.].to_loc(),
                            time: Schedule {
                                arrival: "1970-01-01T00:00:03Z".to_string(),
                                departure: "1970-01-01T00:00:06Z".to_string(),
                            },
                            distance: 2,
                            load: vec![0],
                            activities: vec![
                                Activity {
                                    job_id: "job2".to_string(),
                                    activity_type: "delivery".to_string(),
                                    location: None,
                                    time: None,
                                    job_tag: None,
                                },
                                Activity {
                                    job_id: "break".to_string(),
                                    activity_type: "break".to_string(),
                                    location: None,
                                    time: None,
                                    job_tag: None,
                                },
                            ],
                        },
                        create_stop_with_activity(
                            "job3",
                            "pickup",
                            (3., 0.),
                            1,
                            ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                            3,
                        ),
                        create_stop_with_activity(
                            "reload",
                            "reload",
                            (0., 0.),
                            1,
                            ("1970-01-01T00:00:11Z", "1970-01-01T00:00:13Z"),
                            6,
                        ),
                        create_stop_with_activity(
                            "job4",
                            "delivery",
                            (4., 0.),
                            0,
                            ("1970-01-01T00:00:17Z", "1970-01-01T00:00:18Z"),
                            10,
                        ),
                        create_stop_with_activity(
                            "job5",
                            "pickup",
                            (5., 0.),
                            1,
                            ("1970-01-01T00:00:19Z", "1970-01-01T00:00:20Z"),
                            11,
                        ),
                        create_stop_with_activity(
                            "arrival",
                            "arrival",
                            (0., 0.),
                            0,
                            ("1970-01-01T00:00:25Z", "1970-01-01T00:00:25Z"),
                            16,
                        ),
                    ],
                    statistic: Statistic {
                        cost: 51.,
                        distance: 16,
                        duration: 25,
                        times: Timing { driving: 16, serving: 9, waiting: 0, break_time: 2 },
                    },
                },
                VehicleTour {
                    vehicle_id: "my_vehicle_2".to_string(),
                    type_id: "my_vehicle".to_string(),
                    shift_index: 0,
                    stops: vec![],
                    statistic: Default::default(),
                },
            ],
            ..create_empty_solution()
        };

        let result =
            check_relations(&CheckerContext::new(create_example_problem(), problem, None, solution)).map_err(|_| ());

        assert_eq!(result, expected_result);
    }
}
