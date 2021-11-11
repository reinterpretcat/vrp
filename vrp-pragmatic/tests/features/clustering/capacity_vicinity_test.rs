use super::*;
use std::iter::once;

#[test]
fn can_mix_pickup_delivery_jobs() {
    let a = ActivityData::new;
    let stop2 = StopData::new((
        3.,
        3,
        2,
        0,
        (3., 10.),
        vec![
            a(("job3", Some(3.), "delivery", Some((3., 4.)), Some((None, None)))),
            a(("job2", Some(2.), "pickup", Some((5., 6.)), Some((Some((3., 1., 4., 5.)), None)))),
            a(("job1", Some(1.), "delivery", Some((7., 8.)), Some((Some((2., 1., 6., 7.)), Some((3., 2., 8., 10.)))))),
        ],
    ));
    let stop3_schedule = (17., 18.);
    let statistic = create_statistic((38., 10, 18, (10, 4, 4, 0)));

    let problem = create_test_problem(
        &[(1., "delivery"), (2., "pickup"), (3., "delivery"), (10., "delivery")],
        3,
        Clustering::Vicinity {
            profile: VehicleProfile { matrix: "car".to_string(), scale: None },
            threshold: VicinityThresholdPolicy {
                moving_duration: 3.,
                moving_distance: 3.,
                min_shared_time: None,
                smallest_time_window: None,
                max_jobs_per_cluster: None,
            },
            visiting: VicinityVisitPolicy::Continue,
            serving: VicinityServingPolicy::Original { parking: 0. },
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
                        3,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    stop2.into(),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (10., 0.),
                        1,
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

parameterized_test! {can_vary_cluster_size_based_on_capacity, (capacity, stops, unassigned, statistic), {
    let stops = stops.into_iter().map(StopData::new).collect();
    can_vary_cluster_size_based_on_capacity_impl(capacity, stops, unassigned, statistic);
}}

can_vary_cluster_size_based_on_capacity! {
    case_01: (
        4,
        vec![
          (4., 4, 0, 0, (4., 14.), vec![
            ActivityData::new(("job4", Some(4.), "delivery", Some((4., 5.)), Some((None, None)))),
            ActivityData::new(("job3", Some(3.), "delivery", Some((6., 7.)), Some((Some((4., 1., 5., 6.)), None)))),
            ActivityData::new(("job2", Some(2.), "delivery", Some((8., 9.)), Some((Some((3., 1., 7., 8.)), None)))),
            ActivityData::new(("job1", Some(1.), "delivery", Some((10., 11.)), Some((Some((2., 1., 9., 10.)), Some((4., 3., 11., 14.)))))),
          ])
        ],
        None,
        (28., 4, 14, (4, 4, 6, 0)),
    ),
    case_02: (
        3,
        vec![
          (4., 4, 0, 0, (4., 11.), vec![
            ActivityData::new(("job4", Some(4.), "delivery", Some((4., 5.)), Some((None, None)))),
            ActivityData::new(("job3", Some(3.), "delivery", Some((6., 7.)), Some((Some((4., 1., 5., 6.)), None)))),
            ActivityData::new(("job2", Some(2.), "delivery", Some((8., 9.)), Some((Some((3., 1., 7., 8.)), Some((4., 2., 9., 11.)))))),
          ])
        ],
        Some(vec!["job1"]),
        (25., 4, 11, (4, 3, 4, 0)),
    ),
}

fn can_vary_cluster_size_based_on_capacity_impl(
    capacity: i32,
    stops: Vec<StopData>,
    unassigned: Option<Vec<&str>>,
    statistic_data: (f64, i64, i64, (i64, i64, i64, i64)),
) {
    let statistic = create_statistic(statistic_data);
    let problem = create_test_problem(
        &[(1., "delivery"), (2., "delivery"), (3., "delivery"), (4., "delivery")],
        capacity,
        Clustering::Vicinity {
            profile: VehicleProfile { matrix: "car".to_string(), scale: None },
            threshold: VicinityThresholdPolicy {
                moving_duration: 5.,
                moving_distance: 5.,
                min_shared_time: None,
                smallest_time_window: None,
                max_jobs_per_cluster: None,
            },
            visiting: VicinityVisitPolicy::Continue,
            serving: VicinityServingPolicy::Original { parking: 0. },
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
                stops: once(create_stop_with_activity(
                    "departure",
                    "departure",
                    (0., 0.),
                    capacity,
                    ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                    0,
                ))
                .chain(stops.into_iter().map(StopData::into))
                .collect(),

                statistic,
            }],
            unassigned: unassigned.map(|job_ids| job_ids
                .iter()
                .map(|job_id| UnassignedJob {
                    job_id: job_id.to_string(),
                    reasons: vec![UnassignedJobReason {
                        code: "CAPACITY_CONSTRAINT".to_string(),
                        description: "does not fit into any vehicle due to capacity".to_string()
                    }]
                })
                .collect()),
            ..create_empty_solution()
        }
    );
}
