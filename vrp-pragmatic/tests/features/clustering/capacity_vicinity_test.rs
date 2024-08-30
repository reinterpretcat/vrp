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
                duration: 3.,
                distance: 3.,
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
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(vec![
                        StopBuilder::default()
                            .coordinate((0., 0.))
                            .schedule_stamp(0., 0.)
                            .load(vec![3])
                            .build_departure(),
                        stop2.into(),
                        StopBuilder::default()
                            .coordinate((10., 0.))
                            .schedule_stamp(stop3_schedule.0, stop3_schedule.1)
                            .load(vec![1])
                            .distance(10)
                            .build_single("job4", "delivery"),
                    ])
                    .statistic(statistic)
                    .build()
            )
            .build()
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
    statistic_data: (Float, i64, i64, (i64, i64, i64, i64)),
) {
    let statistic = create_statistic(statistic_data);
    let problem = create_test_problem(
        &[(1., "delivery"), (2., "delivery"), (3., "delivery"), (4., "delivery")],
        capacity,
        Clustering::Vicinity {
            profile: VehicleProfile { matrix: "car".to_string(), scale: None },
            threshold: VicinityThresholdPolicy {
                duration: 5.,
                distance: 5.,
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
        SolutionBuilder::default()
            .tour(
                TourBuilder::default()
                    .stops(
                        once(
                            StopBuilder::default()
                                .coordinate((0., 0.))
                                .schedule_stamp(0., 0.)
                                .load(vec![capacity])
                                .build_departure(),
                        )
                        .chain(stops.into_iter().map(StopData::into))
                        .collect()
                    )
                    .statistic(statistic)
                    .build()
            )
            .unassigned(unassigned.map(|job_ids| {
                job_ids
                    .iter()
                    .map(|job_id| UnassignedJob {
                        job_id: job_id.to_string(),
                        reasons: vec![UnassignedJobReason {
                            code: "CAPACITY_CONSTRAINT".to_string(),
                            description: "does not fit into any vehicle due to capacity".to_string(),
                            details: Some(vec![UnassignedJobDetail {
                                vehicle_id: "my_vehicle_1".to_string(),
                                shift_index: 0,
                            }]),
                        }],
                    })
                    .collect()
            }))
            .build()
    );
}
