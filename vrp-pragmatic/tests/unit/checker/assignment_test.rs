use super::*;
use crate::helpers::*;
use vrp_core::models::examples::create_example_problem;

parameterized_test! {check_vehicles, (known_ids, tours, expected_result), {
    check_vehicles_impl(known_ids, tours, expected_result);
}}

check_vehicles! {
    case_01: (vec!["vehicle_1"], vec![("vehicle_1", 0)], Ok(())),
    case_02: (vec!["vehicle_1"], vec![("vehicle_2", 0)], Err(())),
    case_03: (vec!["vehicle_1"], vec![("vehicle_1", 0), ("vehicle_1", 1)], Ok(())),
    case_04: (vec!["vehicle_1"], vec![("vehicle_1", 0), ("vehicle_1", 0)], Err(())),
}

fn check_vehicles_impl(known_ids: Vec<&str>, tours: Vec<(&str, usize)>, expected_result: Result<(), ()>) {
    let problem = Problem {
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: known_ids.into_iter().map(|id| id.to_string()).collect(),
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };
    let solution = Solution {
        tours: tours
            .into_iter()
            .map(|(id, shift_index)| Tour {
                vehicle_id: id.to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index,
                stops: vec![],
                statistic: Statistic::default(),
            })
            .collect(),
        ..SolutionBuilder::default().build()
    };
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_vehicles(&ctx);

    assert_eq!(result.map_err(|_| ()), expected_result);
}

parameterized_test! {check_jobs, (jobs, tours, unassigned, expected_result), {
    check_jobs_impl(jobs, tours, unassigned, expected_result);
}}

check_jobs! {
    case_01: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![("my_vehicle_1", 0, vec![("job1", "pickup"), ("job1", "delivery")])],
        vec![],
        Ok(())
    ),
    case_02: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![
            ("my_vehicle_1", 0, vec![("job1", "pickup")]),
            ("my_vehicle_2", 0, vec![("job1", "delivery")])
        ],
        vec![],
        Err("job served in multiple tours: 'job1'".into())
    ),
    case_03: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![("my_vehicle_1", 0, vec![("job1", "pickup")])],
        vec![],
        Err("not all tasks served for 'job1', expected: 2, assigned: 1".into())
    ),
    case_04: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![("my_vehicle_1", 0, vec![("job1", "delivery"), ("job1", "pickup")])],
        vec![],
        Err("found pickup after delivery for 'job1'".into())
    ),
    case_05: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![],
        vec!["job1"],
        Ok(())
    ),
    case_06: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![],
        vec!["job1", "job1"],
        Err("duplicated job ids in the list of unassigned jobs".into())
    ),
    case_07: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![],
        vec!["job2"],
        Err("unknown job id in the list of unassigned jobs: 'job2'".into())
    ),
    case_08: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![],
        vec!["job1", "vehicle_break"],
        Ok(())
    ),
    case_09: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![("my_vehicle_1", 0, vec![("job1", "pickup"), ("job1", "delivery")])],
        vec!["job1"],
        Err("job present as assigned and unassigned: 'job1'".into())
    ),
     case_10: (
        vec![("job1", vec!["pickup"])],
        vec![("my_vehicle_1", 0, vec![("job1", "pickup")])],
        vec![],
        Ok(())
    ),
}

#[allow(clippy::type_complexity)]
fn check_jobs_impl(
    jobs: Vec<(&str, Vec<&str>)>,
    tours: Vec<(&str, usize, Vec<(&str, &str)>)>,
    unassigned: Vec<&str>,
    expected_result: Result<(), GenericError>,
) {
    let create_tasks = |tgt: &str, tasks: &Vec<&str>| {
        (1..)
            .zip(tasks.iter())
            .filter(|(_, t)| **t == tgt)
            .map(|(idx, _)| JobTask {
                places: vec![JobPlace {
                    location: Location::Coordinate { lat: 0.0, lng: 0.0 },
                    duration: 0.0,
                    times: None,
                    tag: Some(format!("{tgt}{idx}")),
                }],
                demand: if tgt != "service" { Some(vec![1]) } else { None },
                order: None,
                due_date: None,
            })
            .collect()
    };

    let create_stop = |stop: (&str, &str)| StopBuilder::default().coordinate((0., 0.)).build_single(stop.0, stop.1);

    let problem = Problem {
        plan: Plan {
            jobs: jobs
                .into_iter()
                .map(|(id, tasks)| Job {
                    pickups: Some(create_tasks("pickup", &tasks)),
                    deliveries: Some(create_tasks("delivery", &tasks)),
                    replacements: Some(create_tasks("replacement", &tasks)),
                    services: Some(create_tasks("service", &tasks)),
                    ..create_job(id)
                })
                .collect(),
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let solution = Solution {
        tours: tours
            .into_iter()
            .map(|(id, shift_index, stops)| Tour {
                vehicle_id: id.to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index,
                stops: stops.into_iter().map(create_stop).collect(),
                statistic: Statistic::default(),
            })
            .collect(),
        unassigned: Some(
            unassigned.into_iter().map(|job| UnassignedJob { job_id: job.to_string(), reasons: vec![] }).collect(),
        ),
        ..SolutionBuilder::default().build()
    };
    let ctx = CheckerContext::new(create_example_problem(), problem, None, solution).unwrap();

    let result = check_jobs_presence(&ctx);

    assert_eq!(result, expected_result);
}

#[test]
fn can_detect_time_window_violation() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(1, 2)], 1.)],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(2., 2.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(3., 4.)
                        .load(vec![0])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(5., 5.)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(2).serving(1).build())
                .build(),
        )
        .build();
    let core_problem = Arc::new(problem.clone().read_pragmatic().unwrap());
    let ctx = CheckerContext::new(core_problem, problem, None, solution).unwrap();

    let result = check_assignment(&ctx);

    assert_eq!(result, Err(vec!["cannot match activities to jobs: job1:<no tag>".into()]));
}

#[test]
fn can_detect_job_duration_violation() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", (1., 0.), vec![(5, 10)], 1.)],
            ..create_empty_plan()
        },
        fleet: create_default_fleet(),
        ..create_empty_problem()
    };
    let solution = SolutionBuilder::default()
        .tour(
            TourBuilder::default()
                .stops(vec![
                    StopBuilder::default().coordinate((0., 0.)).schedule_stamp(2., 2.).load(vec![1]).build_departure(),
                    StopBuilder::default()
                        .coordinate((1., 0.))
                        .schedule_stamp(5., 7.)
                        .load(vec![0])
                        .distance(1)
                        .build_single("job1", "delivery"),
                    StopBuilder::default()
                        .coordinate((0., 0.))
                        .schedule_stamp(8., 8.)
                        .load(vec![0])
                        .distance(2)
                        .build_arrival(),
                ])
                .statistic(StatisticBuilder::default().driving(2).serving(2).waiting(2).build())
                .build(),
        )
        .build();
    let core_problem = Arc::new(problem.clone().read_pragmatic().unwrap());
    let ctx = CheckerContext::new(core_problem, problem, None, solution).unwrap();

    let result = check_assignment(&ctx);

    assert_eq!(result, Err(vec!["cannot match activities to jobs: job1:<no tag>".into()]));
}

#[test]
fn can_detect_group_violations() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job_with_group("job1", (1., 0.), "group1"),
                create_delivery_job_with_group("job2", (1., 0.), "group1"),
            ],
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                vehicle_ids: vec!["v1".to_string(), "v2".to_string()],
                ..create_default_vehicle_type()
            }],
            ..create_default_fleet()
        },
        ..create_empty_problem()
    };

    let create_tour = |vehicle_id: &str, job_id: &str| {
        TourBuilder::default()
            .vehicle_id(vehicle_id)
            .stops(vec![
                StopBuilder::default().coordinate((0., 0.)).schedule_stamp(0., 0.).load(vec![1]).build_departure(),
                StopBuilder::default()
                    .coordinate((1., 0.))
                    .schedule_stamp(1., 2.)
                    .load(vec![0])
                    .distance(1)
                    .build_single(job_id, "delivery"),
                StopBuilder::default()
                    .coordinate((0., 0.))
                    .schedule_stamp(3., 3.)
                    .load(vec![0])
                    .distance(2)
                    .build_arrival(),
            ])
            .statistic(StatisticBuilder::default().driving(2).serving(2).waiting(2).build())
            .build()
    };
    let solution = SolutionBuilder::default().tour(create_tour("v1", "job1")).tour(create_tour("v2", "job2")).build();
    let core_problem = Arc::new(problem.clone().read_pragmatic().unwrap());
    let ctx = CheckerContext::new(core_problem, problem, None, solution).unwrap();

    let result = check_groups(&ctx);

    assert_eq!(result, Err("job groups are not respected: 'group1'".into()));
}
