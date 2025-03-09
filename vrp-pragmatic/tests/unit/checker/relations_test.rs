use super::*;
use crate::format_time;
use crate::helpers::*;

mod single {
    use super::*;
    use RelationType::{Any, Sequence, Strict};
    use vrp_core::models::examples::create_example_problem;

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
                    create_delivery_job("job1", (1., 0.)),
                    create_delivery_job("job2", (2., 0.)),
                    create_pickup_job("job3", (3., 0.)),
                    create_delivery_job("job4", (4., 0.)),
                    create_pickup_job("job5", (5., 0.)),
                ],
                relations,
                ..create_empty_plan()
            },
            fleet: Fleet {
                vehicles: vec![VehicleType {
                    type_id: "my_vehicle".to_string(),
                    vehicle_ids: vec!["my_vehicle_1".to_string(), "my_vehicle_2".to_string()],
                    profile: create_default_vehicle_profile(),
                    costs: create_default_vehicle_costs(),
                    shifts: vec![VehicleShift {
                        start: ShiftStart { earliest: format_time(0.), latest: None, location: (0., 0.).to_loc() },
                        end: Some(ShiftEnd { earliest: None, latest: format_time(1000.), location: (0., 0.).to_loc() }),
                        breaks: Some(vec![VehicleBreak::Optional {
                            time: VehicleOptionalBreakTime::TimeWindow(vec![format_time(0.), format_time(1000.)]),
                            places: vec![VehicleOptionalBreakPlace { duration: 2.0, location: None, tag: None }],
                            policy: None,
                        }]),
                        reloads: Some(vec![VehicleReload {
                            location: (0., 0.).to_loc(),
                            duration: 2.0,
                            ..create_default_reload()
                        }]),
                        recharges: None,
                    }],
                    capacity: vec![5],
                    skills: None,
                    limits: None,
                }],
                ..create_default_fleet()
            },
            ..create_empty_problem()
        };
        let solution = SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![2])
                            .build_departure(),
                        StopBuilder::default()
                            .coordinate((1., 0.))
                            .schedule_stamp(1., 2.)
                            .load(vec![1])
                            .distance(1)
                            .build_single("job1", "delivery"),
                        StopBuilder::default()
                            .coordinate((2., 0.))
                            .schedule_stamp(3., 6.)
                            .load(vec![0])
                            .distance(2)
                            .activity(ActivityBuilder::delivery().job_id("job2").build())
                            .activity(ActivityBuilder::break_type().job_id("break").build())
                            .build(),
                        StopBuilder::default()
                            .coordinate((3., 0.))
                            .schedule_stamp(7., 8.)
                            .load(vec![1])
                            .distance(3)
                            .build_single("job3", "pickup"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(11., 13.)
                            .load(vec![1])
                            .distance(6)
                            .build_single("reload", "reload"),
                        StopBuilder::default()
                            .coordinate((4., 0.))
                            .schedule_stamp(17., 18.)
                            .load(vec![0])
                            .distance(10)
                            .build_single("job4", "delivery"),
                        StopBuilder::default()
                            .coordinate((5., 0.))
                            .schedule_stamp(19., 20.)
                            .load(vec![1])
                            .distance(11)
                            .build_single("job5", "pickup"),
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(25., 2.)
                            .load(vec![0])
                            .distance(16)
                            .build_arrival(),
                    ])
                    .statistic(StatisticBuilder::default().driving(16).serving(9).break_time(2).build())
                    .build(),
            )
            .build();
        let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

        let result = check_relations(&ctx).map_err(|_| ());

        assert_eq!(result, expected_result);
    }
}
