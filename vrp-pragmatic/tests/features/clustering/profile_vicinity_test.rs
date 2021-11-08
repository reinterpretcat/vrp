use super::*;

#[test]
fn can_use_scale_on_profile() {
    let capacity = 2;
    let a = ActivityData::new;
    let activities = vec![
        a(("job2", Some(2.), "delivery", Some((2., 3.)), Some((None, None)))),
        a(("job1", Some(1.), "delivery", Some((5., 6.)), Some((Some((2., 1., 3., 5.)), Some((2., 1., 6., 8.)))))),
    ];
    let stop2 = StopData::new((2., 2, 0, (2., 8.), activities));
    let statistic = create_statistic((20., 2, 8, (2, 2, 4)));
    let problem = create_test_problem(
        &[(1., "delivery"), (2., "delivery")],
        capacity,
        Clustering::Vicinity {
            profile: VehicleProfile { matrix: "car".to_string(), scale: Some(2.) },
            threshold: VicinityThresholdPolicy {
                moving_duration: 3.,
                moving_distance: 3.,
                min_shared_time: None,
                smallest_time_window: None,
                max_jobs_per_cluster: None,
            },
            visiting: VicinityVisitPolicy::Continue,
            serving: VicinityServingPolicy::Original,
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
                        capacity,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0,
                    ),
                    stop2.into(),
                ],
                statistic,
            }],
            ..create_empty_solution()
        }
    );
}
