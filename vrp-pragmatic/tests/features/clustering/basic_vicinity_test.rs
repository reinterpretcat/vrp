use super::*;
use crate::format::problem::*;

// TODO
//  check two adjusted clusters
//    - different stop location
//    - same stop location (use limit by cluster size)
//  check different matrix/scale?
//  check with constraints (e.g. skills)

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

parameterized_test! {can_cluster_simple_jobs, (visiting, serving, stop2, stop3_schedule, statistic), {
    can_cluster_simple_jobs_impl(visiting, serving, StopData::new(stop2), stop3_schedule, statistic);
}}

can_cluster_simple_jobs! {
    case_01_continue: (
        VicinityVisitPolicy::Continue, VicinityServingPolicy::Original,
        (3., 3, 1, (3., 10.), vec![
          ActivityData::new(("job3", Some(3.), "delivery", Some((3., 4.)), Some((None, None)))),
          ActivityData::new(("job2", Some(2.), "delivery", Some((5., 6.)), Some((Some((1., 4., 5.)), None)))),
          ActivityData::new(("job1", Some(1.), "delivery", Some((7., 8.)), Some((Some((1., 6., 7.)), Some((2., 8., 10.)))))),
        ]),
        (17., 18.),
        (38., 10, 18, (10, 4, 4)),
    ),
    case_02_return: (
       VicinityVisitPolicy::Return, VicinityServingPolicy::Original,
        (3., 3, 1, (3., 12.), vec![
          ActivityData::new(("job3", Some(3.), "delivery", Some((3., 4.)), Some((None, None)))),
          ActivityData::new(("job2", Some(2.), "delivery", Some((5., 6.)), Some((Some((1., 4., 5.)), Some((1., 6., 7.)))))),
          ActivityData::new(("job1", Some(1.), "delivery", Some((9., 10.)), Some((Some((2., 7., 9.)), Some((2., 10., 12.)))))),
        ]),
        (19., 20.),
        (40., 10, 20, (10, 4, 6)),
    ),
    case_03_fixed: (
       VicinityVisitPolicy::Continue, VicinityServingPolicy::Fixed { value: 5. },
        (3., 3, 1, (3., 22.), vec![
          ActivityData::new(("job3", Some(3.), "delivery", Some((3., 8.)), Some((None, None)))),
          ActivityData::new(("job2", Some(2.), "delivery", Some((9., 14.)), Some((Some((1., 8., 9.)), None)))),
          ActivityData::new(("job1", Some(1.), "delivery", Some((15., 20.)), Some((Some((1., 14., 15.)), Some((2., 20., 22.)))))),
        ]),
        (29., 30.),
        (50., 10, 30, (10, 16, 4))
    ),
    case_04_multiplier: (
       VicinityVisitPolicy::Continue, VicinityServingPolicy::Multiplier { multiplier: 5. },
        (3., 3, 1, (3., 22.), vec![
          ActivityData::new(("job3", Some(3.), "delivery", Some((3., 8.)), Some((None, None)))),
          ActivityData::new(("job2", Some(2.), "delivery", Some((9., 14.)), Some((Some((1., 8., 9.)), None)))),
          ActivityData::new(("job1", Some(1.), "delivery", Some((15., 20.)), Some((Some((1., 14., 15.)), Some((2., 20., 22.)))))),
        ]),
        (29., 30.),
        (50., 10, 30, (10, 16, 4))
    ),
}

fn can_cluster_simple_jobs_impl(
    visiting: VicinityVisitPolicy,
    serving: VicinityServingPolicy,
    stop2: StopData,
    stop3_schedule: (f64, f64),
    statistic: (f64, i64, i64, (i64, i64, i64)),
) {
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
                    stop2.into(),
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
