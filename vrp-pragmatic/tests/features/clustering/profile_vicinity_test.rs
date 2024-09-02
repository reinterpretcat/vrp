use super::*;

#[test]
fn can_use_scale_on_profile() {
    let capacity = 2;
    let a = ActivityData::new;
    let activities = vec![
        a(("job2", Some(2.), "delivery", Some((2, 3)), Some((None, None)))),
        a(("job1", Some(1.), "delivery", Some((5, 6)), Some((Some((2., 1, 3, 5)), Some((2., 1, 6, 8)))))),
    ];
    let stop2 = StopData::new((2., 2, 0, 0, (2, 8), activities));
    let statistic = create_statistic((20., 2, 8, (2, 2, 4, 0)));
    let problem = create_test_problem(
        &[(1., "delivery"), (2., "delivery")],
        capacity,
        Clustering::Vicinity {
            profile: VehicleProfile { matrix: "car".to_string(), scale: Some(2.) },
            threshold: VicinityThresholdPolicy {
                duration: 3,
                distance: 3,
                min_shared_time: None,
                smallest_time_window: None,
                max_jobs_per_cluster: None,
            },
            visiting: VicinityVisitPolicy::Continue,
            serving: VicinityServingPolicy::Original { parking: 0 },
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
                            .schedule_stamp(0, 0)
                            .load(vec![capacity])
                            .build_departure(),
                        stop2.into(),
                    ])
                    .statistic(statistic)
                    .build()
            )
            .build()
    );
}
