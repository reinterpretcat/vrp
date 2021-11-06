use super::*;

// TODO
//  check different matrix/scale?
//  check with constraints
//  - skills?
//  - no capacity overload

#[test]
fn can_mix_pickup_delivery_jobs() {
    let stop2 = StopData::new((
        3.,
        3,
        2,
        (3., 10.),
        vec![
            ActivityData::new(("job3", Some(3.), "delivery", Some((3., 4.)), Some((None, None)))),
            ActivityData::new(("job2", Some(2.), "pickup", Some((5., 6.)), Some((Some((1., 4., 5.)), None)))),
            ActivityData::new((
                "job1",
                Some(1.),
                "delivery",
                Some((7., 8.)),
                Some((Some((1., 6., 7.)), Some((2., 8., 10.)))),
            )),
        ],
    ));
    let stop3_schedule = (17., 18.);
    let statistic = create_statistic((38., 10, 18, (10, 4, 4)));

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
