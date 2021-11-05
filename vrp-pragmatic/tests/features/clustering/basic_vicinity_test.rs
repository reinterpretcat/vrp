use crate::format::problem::*;
use crate::format::solution::*;
use crate::format_time;
use crate::helpers::*;

// TODO
//  check different matrix/scale?
//  check with constraints (e.g. skills)
//  check two adjusted clusters
//    - different stop location
//    - same stop location (use limit by cluster size)

fn create_test_problem(job_locations: &[f64], clustering: Clustering) -> Problem {
    Problem {
        plan: Plan {
            jobs: job_locations
                .iter()
                .enumerate()
                .map(|(idx, &loc)| create_delivery_job(&format!("job{}", idx + 1), vec![loc, 0.]))
                .collect(),
            clustering: Some(clustering),
            ..create_empty_plan()
        },
        fleet: Fleet {
            vehicles: vec![VehicleType {
                shifts: vec![create_default_open_vehicle_shift()],
                ..create_default_vehicle_type()
            }],
            profiles: vec![MatrixProfile { name: "car".to_string(), speed: None }],
        },
        ..create_empty_problem()
    }
}

type ExpectedActivity = ((f64, f64), Option<(f64, f64, f64)>, Option<(f64, f64, f64)>);

parameterized_test! {can_cluster_simple_jobs, (visiting, serving, stop2_schedule, stop2_activity1, stop2_activity2, stop2_activity3, stop3_schedule, statistic), {
    can_cluster_simple_jobs_impl(visiting, serving, stop2_schedule, stop2_activity1, stop2_activity2, stop2_activity3, stop3_schedule, statistic);
}}

can_cluster_simple_jobs! {
    case_01_continue: (
        VicinityVisitPolicy::Continue, VicinityServingPolicy::Original,
        (3., 10.),
        ((3., 4.), None, None),
        ((5., 6.), Some((1., 4., 5.)), None),
        ((7., 8.), Some((1., 6., 7.)), Some((2., 8., 10.))),
        (17., 18.),
        (38., 10, 18, (10, 4, 4))
    ),
    case_02_return: (
        VicinityVisitPolicy::Return, VicinityServingPolicy::Original,
        (3., 12.),
        ((3., 4.), None, None),
        ((5., 6.), Some((1., 4., 5.)), Some((1., 6., 7.))),
        ((9., 10.), Some((2., 7., 9.)), Some((2., 10., 12.))),
        (19., 20.),
        (40., 10, 20, (10, 4, 6))
    ),

    case_03_fixed: (
        VicinityVisitPolicy::Continue, VicinityServingPolicy::Fixed { value: 5. },
        (3., 22.),
        ((3., 8.), None, None),
        ((9., 14.), Some((1., 8., 9.)), None),
        ((15., 20.), Some((1., 14., 15.)), Some((2., 20., 22.))),
        (29., 30.),
        (50., 10, 30, (10, 16, 4))
    ),

    case_04_multiplier: (
        VicinityVisitPolicy::Continue, VicinityServingPolicy::Multiplier { multiplier: 5. },
        (3., 22.),
        ((3., 8.), None, None),
        ((9., 14.), Some((1., 8., 9.)), None),
        ((15., 20.), Some((1., 14., 15.)), Some((2., 20., 22.))),
        (29., 30.),
        (50., 10, 30, (10, 16, 4))
    ),
}

fn can_cluster_simple_jobs_impl(
    visiting: VicinityVisitPolicy,
    serving: VicinityServingPolicy,
    stop2_schedule: (f64, f64),
    stop2_activity1: ExpectedActivity,
    stop2_activity2: ExpectedActivity,
    stop2_activity3: ExpectedActivity,
    stop3_schedule: (f64, f64),
    statistic: (f64, i64, i64, (i64, i64, i64)),
) {
    let convert_expected_commute_info = |commute: Option<(f64, f64, f64)>| {
        commute.map(|commute| CommuteInfo {
            distance: commute.0,
            time: Interval { start: format_time(commute.1), end: format_time(commute.2) },
        })
    };
    let statistic = Statistic {
        cost: statistic.0,
        distance: statistic.1,
        duration: statistic.2,
        times: Timing {
            driving: statistic.3 .0,
            serving: statistic.3 .1,
            commuting: statistic.3 .2,
            ..Timing::default()
        },
    };
    let problem = create_test_problem(
        &[1., 2., 3., 10.],
        Clustering::Vicinity {
            profile: VehicleProfile { matrix: "car".to_string(), scale: None },
            threshold: VicinityThresholdPolicy {
                moving_duration: 3.,
                moving_distance: 3.,
                min_shared_time: None,
                smallest_time_window: None,
                max_jobs_per_cluster: None,
            },
            visiting,
            serving,
            filtering: None,
        },
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(
        solution,
        Solution {
            statistic: statistic.clone(),
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        4,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    Stop {
                        location: vec![3., 0.].to_loc(),
                        time: Schedule {
                            arrival: format_time(stop2_schedule.0),
                            departure: format_time(stop2_schedule.1),
                        },
                        distance: 3,
                        load: vec![1],
                        activities: vec![
                            Activity {
                                job_id: "job3".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![3., 0.].to_loc()),
                                time: Some(Interval {
                                    start: format_time(stop2_activity1.0 .0),
                                    end: format_time(stop2_activity1.0 .1),
                                }),
                                job_tag: None,
                                commute: Some(Commute {
                                    forward: convert_expected_commute_info(stop2_activity1.1),
                                    backward: convert_expected_commute_info(stop2_activity1.2)
                                }),
                            },
                            Activity {
                                job_id: "job2".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![2., 0.].to_loc()),
                                time: Some(Interval {
                                    start: format_time(stop2_activity2.0 .0),
                                    end: format_time(stop2_activity2.0 .1),
                                }),
                                job_tag: None,
                                commute: Some(Commute {
                                    forward: convert_expected_commute_info(stop2_activity2.1),
                                    backward: convert_expected_commute_info(stop2_activity2.2)
                                }),
                            },
                            Activity {
                                job_id: "job1".to_string(),
                                activity_type: "delivery".to_string(),
                                location: Some(vec![1., 0.].to_loc()),
                                time: Some(Interval {
                                    start: format_time(stop2_activity3.0 .0),
                                    end: format_time(stop2_activity3.0 .1),
                                }),
                                job_tag: None,
                                commute: Some(Commute {
                                    forward: convert_expected_commute_info(stop2_activity3.1),
                                    backward: convert_expected_commute_info(stop2_activity3.2)
                                }),
                            },
                        ],
                    },
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (10., 0.),
                        0,
                        (&format_time(stop3_schedule.0), &format_time(stop3_schedule.1)),
                        10,
                    ),
                ],
                statistic,
            }],
            ..create_empty_solution()
        }
    );
}

#[test]
#[ignore]
fn can_handle_two_clusters() {
    let problem = create_test_problem(
        &[1., 2., 3., 4.],
        Clustering::Vicinity {
            profile: VehicleProfile { matrix: "car".to_string(), scale: None },
            threshold: VicinityThresholdPolicy {
                moving_duration: 5.,
                moving_distance: 5.,
                min_shared_time: None,
                smallest_time_window: None,
                max_jobs_per_cluster: Some(2),
            },
            visiting: VicinityVisitPolicy::Continue,
            serving: VicinityServingPolicy::Original,
            filtering: None,
        },
    );
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    assert_eq!(solution, create_empty_solution());
}
