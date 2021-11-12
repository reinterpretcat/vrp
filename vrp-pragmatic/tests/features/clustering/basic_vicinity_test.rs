use super::*;
use std::iter::once;

parameterized_test! {can_cluster_simple_jobs, (visiting, serving, stop2, stop3_schedule, statistic), {
    can_cluster_simple_jobs_impl(visiting, serving, StopData::new(stop2), stop3_schedule, statistic);
}}

can_cluster_simple_jobs! {
    case_01_continue: (
        VicinityVisitPolicy::Continue, VicinityServingPolicy::Original { parking: 0. },
        (3., 3, 1, 0, (3., 10.), vec![
          ActivityData::new(("job3", Some(3.), "delivery", Some((3., 4.)), Some((None, None)))),
          ActivityData::new(("job2", Some(2.), "delivery", Some((5., 6.)), Some((Some((3., 1., 4., 5.)), None)))),
          ActivityData::new(("job1", Some(1.), "delivery", Some((7., 8.)), Some((Some((2., 1., 6., 7.)), Some((3., 2., 8., 10.)))))),
        ]),
        (17., 18.),
        (38., 10, 18, (10, 4, 4, 0)),
    ),
    case_02_return: (
       VicinityVisitPolicy::Return, VicinityServingPolicy::Original { parking: 0. },
        (3., 3, 1, 0, (3., 12.), vec![
          ActivityData::new(("job3", Some(3.), "delivery", Some((3., 4.)), Some((None, None)))),
          ActivityData::new(("job2", Some(2.), "delivery", Some((5., 6.)), Some((Some((3., 1., 4., 5.)), Some((3., 1., 6., 7.)))))),
          ActivityData::new(("job1", Some(1.), "delivery", Some((9., 10.)), Some((Some((3., 2., 7., 9.)), Some((3., 2., 10., 12.)))))),
        ]),
        (19., 20.),
        (40., 10, 20, (10, 4, 6, 0)),
    ),
    case_03_fixed: (
       VicinityVisitPolicy::Continue, VicinityServingPolicy::Fixed { value: 5., parking: 0. },
        (3., 3, 1, 0, (3., 22.), vec![
          ActivityData::new(("job3", Some(3.), "delivery", Some((3., 8.)), Some((None, None)))),
          ActivityData::new(("job2", Some(2.), "delivery", Some((9., 14.)), Some((Some((3., 1., 8., 9.)), None)))),
          ActivityData::new(("job1", Some(1.), "delivery", Some((15., 20.)), Some((Some((2., 1., 14., 15.)), Some((3., 2., 20., 22.)))))),
        ]),
        (29., 30.),
        (50., 10, 30, (10, 16, 4, 0))
    ),
    case_04_multiplier: (
       VicinityVisitPolicy::Continue, VicinityServingPolicy::Multiplier { multiplier: 5., parking: 0. },
        (3., 3, 1, 0, (3., 22.), vec![
          ActivityData::new(("job3", Some(3.), "delivery", Some((3., 8.)), Some((None, None)))),
          ActivityData::new(("job2", Some(2.), "delivery", Some((9., 14.)), Some((Some((3., 1., 8., 9.)), None)))),
          ActivityData::new(("job1", Some(1.), "delivery", Some((15., 20.)), Some((Some((2., 1., 14., 15.)), Some((3., 2., 20., 22.)))))),
        ]),
        (29., 30.),
        (50., 10, 30, (10, 16, 4, 0))
    ),
}

fn can_cluster_simple_jobs_impl(
    visiting: VicinityVisitPolicy,
    serving: VicinityServingPolicy,
    stop2: StopData,
    stop3_schedule: (f64, f64),
    statistic_data: (f64, i64, i64, (i64, i64, i64, i64)),
) {
    let statistic = create_statistic(statistic_data);
    let problem = create_test_problem(
        &[(1., "delivery"), (2., "delivery"), (3., "delivery"), (10., "delivery")],
        10,
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

parameterized_test! {can_handle_two_clusters, (job_locations, serving, ignore_job_ids, stops, statistic), {
    let stops = stops.into_iter().map(StopData::new).collect();
    can_handle_two_clusters_impl(job_locations, serving, ignore_job_ids, stops, statistic);
}}

can_handle_two_clusters! {
    case_01_diff_stops: (
        &[1., 2., 3., 4.], VicinityServingPolicy::Original { parking: 0. }, false,
        vec![
          (2., 2, 2, 0, (2., 6.), vec![
            ActivityData::new(("job2", Some(2.), "delivery", Some((2., 3.)), Some((None, None)))),
            ActivityData::new(("job1", Some(1.), "delivery", Some((4., 5.)), Some((Some((2., 1., 3., 4.)), Some((2., 1., 5., 6.)))))),
          ]),
          (4., 4, 0, 0, (8., 12.), vec![
            ActivityData::new(("job4", Some(4.), "delivery", Some((8., 9.)), Some((None, None)))),
            ActivityData::new(("job3", Some(3.), "delivery", Some((10., 11.)), Some((Some((4., 1., 9., 10.)), Some((4., 1., 11., 12.)))))),
          ])
        ],
        (26., 4, 12, (4, 4, 4, 0)),
    ),
    case_02_same_stops: (
        &[1., 1., 1., 1.], VicinityServingPolicy::Fixed { value: 2., parking: 0. }, true,
        vec![
          (1., 1, 0, 0, (1., 9.), vec![
            ActivityData::new(("x", Some(1.), "delivery", Some((1., 3.)), Some((None, None)))),
            ActivityData::new(("x", Some(1.), "delivery", Some((3., 5.)), Some((None, None)))),
            ActivityData::new(("x", Some(1.), "delivery", Some((5., 7.)), Some((None, None)))),
            ActivityData::new(("x", Some(1.), "delivery", Some((7., 9.)), Some((None, None)))),
          ])
        ],
        (20., 1, 9, (1, 8, 0, 0)),
    ),

    case_03_diff_stops_parking: (
        &[1., 2., 3., 4.], VicinityServingPolicy::Original { parking: 4. }, false,
        vec![
          (2., 2, 2, 4, (2., 10.), vec![
            ActivityData::new(("job2", Some(2.), "delivery", Some((6., 7.)), Some((None, None)))),
            ActivityData::new(("job1", Some(1.), "delivery", Some((8., 9.)), Some((Some((2., 1., 7., 8.)), Some((2., 1., 9., 10.)))))),
          ]),
          (4., 4, 0, 4, (12., 20.), vec![
            ActivityData::new(("job4", Some(4.), "delivery", Some((16., 17.)), Some((None, None)))),
            ActivityData::new(("job3", Some(3.), "delivery", Some((18., 19.)), Some((Some((4., 1., 17., 18.)), Some((4., 1., 19., 20.)))))),
          ])
        ],
        (34., 4, 20, (4, 4, 4, 8)),
    ),
    case_04_same_stops_parking: (
        &[1., 1., 1., 1.], VicinityServingPolicy::Fixed { value: 2., parking: 4. }, true,
        vec![
          (1., 1, 0, 4, (1., 17.), vec![
            ActivityData::new(("x", Some(1.), "delivery", Some((5., 7.)), Some((None, None)))),
            ActivityData::new(("x", Some(1.), "delivery", Some((7., 9.)), Some((None, None)))),
            ActivityData::new(("x", Some(1.), "delivery", Some((9., 15.)), Some((None, None)))),
            ActivityData::new(("x", Some(1.), "delivery", Some((15., 17.)), Some((None, None)))),
          ])
        ],
        (28., 1, 17, (1, 12, 0, 4)),
    ),
}

fn can_handle_two_clusters_impl(
    job_locations: &[f64],
    serving: VicinityServingPolicy,
    ignore_job_ids: bool,
    stops: Vec<StopData>,
    statistic_data: (f64, i64, i64, (i64, i64, i64, i64)),
) {
    let statistic = create_statistic(statistic_data);
    let problem = create_test_problem(
        job_locations.iter().map(|loc| (*loc, "delivery")).collect::<Vec<_>>().as_slice(),
        10,
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
            serving,
            filtering: None,
        },
    );
    let matrix = create_matrix_from_problem(&problem);

    let mut solution = solve_with_metaheuristic(problem, Some(vec![matrix]));

    if ignore_job_ids {
        // NOTE ignore job id comparison
        solution
            .tours
            .iter_mut()
            .flat_map(|tour| tour.stops.iter_mut().flat_map(|stop| stop.activities.iter_mut()))
            .for_each(|a| {
                if a.activity_type == "delivery" {
                    a.job_id = "x".to_string()
                }
            });
    }
    assert_eq!(
        solution,
        Solution {
            statistic: statistic.clone(),
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: once(create_stop_with_activity(
                    "departure",
                    "departure",
                    (0., 0.),
                    4,
                    ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    0,
                ))
                .chain(stops.into_iter().map(StopData::into))
                .collect(),
                statistic,
            }],
            ..create_empty_solution()
        }
    );
}
