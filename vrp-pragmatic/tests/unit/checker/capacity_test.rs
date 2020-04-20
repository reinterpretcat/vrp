use super::*;
use crate::format_time;
use crate::helpers::*;

parameterized_test! {can_check_load, (stop_loads, expected_result), {
    can_check_load_impl(stop_loads, expected_result);
}}

can_check_load! {
    case00: ( vec![1, 1, 3, 1, 2, 1, 0], Ok(())),

    case01: ( vec![1, 2, 3, 1, 2, 1, 0], Err("Load mismatch at stop 1 in tour 'my_vehicle_1'".to_owned())),
    case02: ( vec![1, 1, 2, 1, 2, 1, 0], Err("Load mismatch at stops 2, 3 in tour 'my_vehicle_1'".to_owned())),
    case03: ( vec![1, 1, 3, 2, 2, 1, 0], Err("Load mismatch at stop 3 in tour 'my_vehicle_1'".to_owned())),
    case04: ( vec![1, 1, 3, 1, 1, 1, 0], Err("Load mismatch at stop 4 in tour 'my_vehicle_1'".to_owned())),
    case05: ( vec![1, 1, 3, 1, 2, 2, 0], Err("Load mismatch at stop 5 in tour 'my_vehicle_1'".to_owned())),

    case06_1: ( vec![10, 1, 3, 1, 2, 1, 0], Err("Load exceeds capacity in tour 'my_vehicle_1'".to_owned())),
    case06_2: ( vec![1, 1, 30, 1, 2, 1, 0], Err("Load exceeds capacity in tour 'my_vehicle_1'".to_owned())),
    case06_3: ( vec![1, 1, 3, 1, 20, 1, 0], Err("Load exceeds capacity in tour 'my_vehicle_1'".to_owned())),
}

fn can_check_load_impl(stop_loads: Vec<i32>, expected_result: Result<(), String>) {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![3., 0.]),
                create_pickup_job("job4", vec![4., 0.]),
                create_pickup_delivery_job("job5", vec![1., 0.], vec![5., 0.]),
            ],
            relations: None,
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![VehicleShift {
                    start: VehiclePlace { time: format_time(0.), location: vec![0., 0.].to_loc() },
                    end: Some(VehiclePlace { time: format_time(1000.).to_string(), location: vec![0., 0.].to_loc() }),
                    breaks: None,
                    reloads: Some(vec![VehicleReload {
                        times: None,
                        location: vec![0., 0.].to_loc(),
                        duration: 2.0,
                        tag: None,
                    }]),
                }],
                capacity: vec![5],
                ..create_default_vehicle_type()
            }],
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let solution = Solution {
        statistic: Statistic {
            cost: 13.,
            distance: 1,
            duration: 2,
            times: Timing { driving: 1, serving: 1, waiting: 0, break_time: 0 },
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
                    *stop_loads.get(0).unwrap(),
                    ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    0,
                ),
                Stop {
                    location: vec![1., 0.].to_loc(),
                    time: Schedule {
                        arrival: "1970-01-01T00:00:03Z".to_string(),
                        departure: "1970-01-01T00:00:05Z".to_string(),
                    },
                    distance: 1,
                    load: vec![*stop_loads.get(1).unwrap()],
                    activities: vec![
                        Activity {
                            job_id: "job1".to_string(),
                            activity_type: "delivery".to_string(),
                            location: None,
                            time: None,
                            job_tag: None,
                        },
                        Activity {
                            job_id: "job5".to_string(),
                            activity_type: "pickup".to_string(),
                            location: None,
                            time: None,
                            job_tag: None,
                        },
                    ],
                },
                Stop {
                    location: vec![0., 0.].to_loc(),
                    time: Schedule {
                        arrival: "1970-01-01T00:00:03Z".to_string(),
                        departure: "1970-01-01T00:00:05Z".to_string(),
                    },
                    distance: 1,
                    load: vec![*stop_loads.get(2).unwrap()],
                    activities: vec![Activity {
                        job_id: "reload".to_string(),
                        activity_type: "reload".to_string(),
                        location: None,
                        time: None,
                        job_tag: None,
                    }],
                },
                Stop {
                    location: vec![2., 0.].to_loc(),
                    time: Schedule {
                        arrival: "1970-01-01T00:00:07Z".to_string(),
                        departure: "1970-01-01T00:00:08Z".to_string(),
                    },
                    distance: 3,
                    load: vec![*stop_loads.get(3).unwrap()],
                    activities: vec![
                        Activity {
                            job_id: "job2".to_string(),
                            activity_type: "delivery".to_string(),
                            location: Some(vec![2., 0.].to_loc()),
                            time: Some(Interval {
                                start: "1970-01-01T00:00:08Z".to_string(),
                                end: "1970-01-01T00:00:09Z".to_string(),
                            }),
                            job_tag: None,
                        },
                        Activity {
                            job_id: "job3".to_string(),
                            activity_type: "delivery".to_string(),
                            location: Some(vec![3., 0.].to_loc()),
                            time: Some(Interval {
                                start: "1970-01-01T00:00:09Z".to_string(),
                                end: "1970-01-01T00:00:10Z".to_string(),
                            }),
                            job_tag: None,
                        },
                    ],
                },
                create_stop_with_activity(
                    "job4",
                    "pickup",
                    (4., 0.),
                    *stop_loads.get(4).unwrap(),
                    ("1970-01-01T00:00:11Z", "1970-01-01T00:00:12Z"),
                    5,
                ),
                create_stop_with_activity(
                    "job5",
                    "delivery",
                    (5., 0.),
                    *stop_loads.get(5).unwrap(),
                    ("1970-01-01T00:00:13Z", "1970-01-01T00:00:14Z"),
                    6,
                ),
                create_stop_with_activity(
                    "arrival",
                    "arrival",
                    (0., 0.),
                    *stop_loads.get(6).unwrap(),
                    ("1970-01-01T00:00:19Z", "1970-01-01T00:00:19Z"),
                    11,
                ),
            ],
            statistic: Statistic {
                cost: 13.,
                distance: 1,
                duration: 2,
                times: Timing { driving: 1, serving: 1, waiting: 0, break_time: 0 },
            },
        }],
        unassigned: vec![],
        extras: None,
    };

    let result = check_vehicle_load(&CheckerContext::new(problem, None, solution));

    assert_eq!(result, expected_result);
}
