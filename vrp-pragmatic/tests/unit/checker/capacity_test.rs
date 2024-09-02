use super::*;
use crate::format_time;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

parameterized_test! {can_check_load, (stop_loads, expected_result), {
    can_check_load_impl(stop_loads, expected_result);
}}

can_check_load! {
    case00: ( vec![1, 1, 3, 1, 2, 1, 0], Ok(())),

    case01: ( vec![1, 2, 3, 1, 2, 1, 0], Err(vec!["load mismatch at stop 1 in tour 'my_vehicle_1'".into()])),
    case02: ( vec![1, 1, 2, 1, 2, 1, 0], Err(vec!["load mismatch at stops 2, 3 in tour 'my_vehicle_1'".into()])),
    case03: ( vec![1, 1, 3, 2, 2, 1, 0], Err(vec!["load mismatch at stop 3 in tour 'my_vehicle_1'".into()])),
    case04: ( vec![1, 1, 3, 1, 1, 1, 0], Err(vec!["load mismatch at stop 4 in tour 'my_vehicle_1'".into()])),
    case05: ( vec![1, 1, 3, 1, 2, 2, 0], Err(vec!["load mismatch at stop 5 in tour 'my_vehicle_1'".into()])),

    case06_1: ( vec![10, 1, 3, 1, 2, 1, 0], Err(vec!["load exceeds capacity in tour 'my_vehicle_1'".into()])),
    case06_2: ( vec![1, 1, 30, 1, 2, 1, 0], Err(vec!["load exceeds capacity in tour 'my_vehicle_1'".into()])),
    case06_3: ( vec![1, 1, 3, 1, 20, 1, 0], Err(vec!["load exceeds capacity in tour 'my_vehicle_1'".into()])),
}

fn can_check_load_impl(stop_loads: Vec<i32>, expected_result: Result<(), Vec<GenericError>>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
                create_pickup_job("job4", (4., 0.)),
                create_pickup_delivery_job("job5", (1., 0.), (5., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: ShiftStart { earliest: format_time(0), latest: None, location: (0., 0.).to_loc() },
                    end: Some(ShiftEnd { earliest: None, latest: format_time(1000), location: (0., 0.).to_loc() }),
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        location: (0., 0.).to_loc(),
                        duration: 2,
                        ..create_default_reload()
                    }]),
                    recharges: None,
                }],
                capacity: vec![5],
                ..create_default_vehicle_type()
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
                        .schedule_stamp(0, 0)
                        .load(vec![*stop_loads.first().unwrap()])
                        .build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(3, 5)
                        .load(vec![*stop_loads.get(1).unwrap()])
                        .distance(1)
                        .activity(ActivityBuilder::delivery().job_id("job1").build())
                        .activity(ActivityBuilder::pickup().job_id("job5").tag("p1").build())
                        .build(),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(3, 5)
                        .load(vec![*stop_loads.get(2).unwrap()])
                        .distance(1)
                        .build_single("reload", "reload"),
                    StopBuilder::default()
                        .coordinate((2., 0.))
                        .schedule_stamp(7, 8)
                        .load(vec![*stop_loads.get(3).unwrap()])
                        .distance(3)
                        .activity(
                            ActivityBuilder::delivery().job_id("job2").coordinate((2., 0.)).time_stamp(8, 9).build(),
                        )
                        .activity(
                            ActivityBuilder::delivery().job_id("job3").coordinate((3., 0.)).time_stamp(9, 10).build(),
                        )
                        .build(),
                    StopBuilder::default()
                        .coordinate((4., 0.))
                        .schedule_stamp(11, 12)
                        .load(vec![*stop_loads.get(4).unwrap()])
                        .distance(5)
                        .build_single("job4", "pickup"),
                    StopBuilder::default()
                        .coordinate((5., 0.))
                        .schedule_stamp(13, 14)
                        .load(vec![*stop_loads.get(5).unwrap()])
                        .distance(6)
                        .build_single_tag("job5", "delivery", "d1"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(19, 19)
                        .load(vec![*stop_loads.get(6).unwrap()])
                        .distance(11)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(1).serving(1).build())
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_vehicle_load(&ctx);

    assert_eq!(result, expected_result);
}

#[test]
#[ignore]
fn can_check_load_when_departure_has_other_activity() {
    let problem = Problem {
        plan: Plan { jobs: vec![create_pickup_delivery_job("job1", (0., 0.), (1., 0.))], ..create_empty_plan() },
        fleet: Fleet { vehicles: vec![create_vehicle_with_capacity("my_vehicle", vec![2])], ..create_default_fleet() },
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(0, 1)
                        .load(vec![1])
                        .distance(0)
                        .activity(ActivityBuilder::default().job_id("departure").activity_type("departure").build())
                        .activity(ActivityBuilder::pickup().job_id("job1").tag("p1").build())
                        .build(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(2, 3)
                        .load(vec![0])
                        .distance(1)
                        .build_single_tag("job1", "delivery", "d1"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(4, 4)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(2).serving(2).build())
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_vehicle_load(&ctx);

    assert_eq!(result, Ok(()));
}

#[test]
fn can_check_resource_consumption() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", (1., 0.)),
                create_delivery_job("job2", (2., 0.)),
                create_delivery_job("job3", (3., 0.)),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    reloads: Some(vec![VehicleReload {
                        location: (4., 0.).to_loc(),
                        resource_id: Some("resource_1".to_string()),
                        ..create_default_reload()
                    }]),
                    ..create_default_open_vehicle_shift()
                }],
                ..create_vehicle_with_capacity("my_vehicle", vec![2])
            }],
            resources: Some(vec![VehicleResource::Reload { id: "resource_1".to_string(), capacity: vec![1] }]),
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0, 0).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(1, 2)
                        .load(vec![0])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((4., 0.))
                        .schedule_stamp(5, 7)
                        .load(vec![2])
                        .distance(4)
                        .build_single("reload", "reload"),
                    StopBuilder::default()
                        .coordinate((3., 0.))
                        .schedule_stamp(8, 9)
                        .load(vec![1])
                        .distance(5)
                        .build_single("job3", "delivery"),
                    StopBuilder::default()
                        .coordinate((2., 0.))
                        .schedule_stamp(10, 11)
                        .load(vec![0])
                        .distance(6)
                        .build_single("job2", "delivery"),
                ])
                .statistic(StatisticBuilder::default().driving(6).serving(5).build())
                .build(),
        )
        .build();
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_resource_consumption(&ctx);

    assert_eq!(
        result,
        Err("consumed more resource 'resource_1' than available: [2, 0, 0, 0, 0, 0, 0, 0] vs [1, 0, 0, 0, 0, 0, 0, 0]"
            .into())
    );
}
