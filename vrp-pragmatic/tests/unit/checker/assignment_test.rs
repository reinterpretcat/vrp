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
            profiles: create_default_profiles(),
        },
        ..create_empty_problem()
    };
    let solution = Solution {
        statistic: Statistic::default(),
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
        ..create_empty_solution()
    };

    let result = check_vehicles(&CheckerContext::new(create_example_problem(), problem, None, solution));

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
        Err("Job served in multiple tours: 'job1'".to_string())
    ),
    case_03: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![("my_vehicle_1", 0, vec![("job1", "pickup")])],
        vec![],
        Err("Not all tasks served for 'job1', expected: 2, assigned: 1".to_string())
    ),
    case_04: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![("my_vehicle_1", 0, vec![("job1", "delivery"), ("job1", "pickup")])],
        vec![],
        Err("Found pickup after delivery for 'job1'".to_string())
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
        Err("Duplicated job ids in the list of unassigned jobs".to_string())
    ),
    case_07: (
        vec![("job1", vec!["pickup", "delivery"])],
        vec![],
        vec!["job2"],
        Err("Unknown job id in the list of unassigned jobs: 'job2'".to_string())
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
        Err("Job present as assigned and unassigned: 'job1'".to_string())
    ),
     case_10: (
        vec![("job1", vec!["pickup"])],
        vec![("my_vehicle_1", 0, vec![("job1", "pickup")])],
        vec![],
        Ok(())
    ),
}

fn check_jobs_impl(
    jobs: Vec<(&str, Vec<&str>)>,
    tours: Vec<(&str, usize, Vec<(&str, &str)>)>,
    unassigned: Vec<&str>,
    expected_result: Result<(), String>,
) {
    let create_tasks = |tgt: &str, tasks: &Vec<&str>| {
        (1..)
            .zip(tasks.iter())
            .filter(|(_, t)| **t == tgt)
            .map(|(idx, _)| JobTask {
                places: vec![],
                demand: if tgt != "service" { Some(vec![1]) } else { None },
                tag: Some(format!("{}{}", tgt, idx)),
            })
            .collect()
    };

    let create_stop = |stop: (&str, &str)| create_stop_with_activity(stop.0, stop.1, (0., 0.), 0, ("", ""), 0);

    let problem = Problem {
        plan: Plan {
            jobs: jobs
                .into_iter()
                .map(|(id, tasks)| Job {
                    id: id.to_string(),
                    pickups: Some(create_tasks("pickup", &tasks)),
                    deliveries: Some(create_tasks("delivery", &tasks)),
                    replacements: Some(create_tasks("replacement", &tasks)),
                    services: Some(create_tasks("service", &tasks)),
                    priority: None,
                    skills: None,
                })
                .collect(),
            relations: None,
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle_type()], profiles: create_default_profiles() },
        ..create_empty_problem()
    };
    let solution = Solution {
        statistic: Statistic::default(),
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
        ..create_empty_solution()
    };

    let result = check_jobs_presence(&CheckerContext::new(create_example_problem(), problem, None, solution));

    assert_eq!(result, expected_result);
}

#[test]
fn can_detect_time_window_violation() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![create_delivery_job_with_times("job1", vec![1., 0.], vec![(1, 2)], 1.)],
            relations: None,
        },
        fleet: Fleet { vehicles: vec![create_default_vehicle_type()], profiles: create_default_profiles() },
        ..create_empty_problem()
    };
    let solution = Solution {
        statistic: Statistic {
            cost: 15.,
            distance: 2,
            duration: 3,
            times: Timing { driving: 2, serving: 1, waiting: 0, break_time: 0 },
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
                    1,
                    ("1970-01-01T00:00:02Z", "1970-01-01T00:00:02Z"),
                    0,
                ),
                create_stop_with_activity(
                    "job1",
                    "delivery",
                    (1., 0.),
                    0,
                    ("1970-01-01T00:00:03Z", "1970-01-01T00:00:04Z"),
                    1,
                ),
                create_stop_with_activity(
                    "arrival",
                    "arrival",
                    (0., 0.),
                    0,
                    ("1970-01-01T00:00:05Z", "1970-01-01T00:00:05Z"),
                    2,
                ),
            ],
            statistic: Statistic {
                cost: 15.,
                distance: 2,
                duration: 3,
                times: Timing { driving: 2, serving: 1, waiting: 0, break_time: 0 },
            },
        }],
        ..create_empty_solution()
    };
    let core_problem = Arc::new(problem.clone().read_pragmatic().unwrap());

    let result = check_assignment(&CheckerContext::new(core_problem, problem, None, solution));

    assert_eq!(result, Err("cannot match activities to jobs: job1:<no tag>".to_owned()));
}
