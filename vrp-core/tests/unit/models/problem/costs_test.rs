use super::*;
use crate::helpers::models::solution::{create_empty_route_ctx, test_actor_with_profile};

fn create_matrix_data(
    profile: Profile,
    timestamp: Option<Timestamp>,
    duration: (Duration, usize),
    distance: (Distance, usize),
) -> MatrixData {
    MatrixData {
        index: profile.index,
        timestamp,
        durations: vec![duration.0; duration.1],
        distances: vec![distance.0; distance.1],
    }
}

#[test]
fn can_detect_dimensions_mismatch() {
    assert_eq!(
        create_matrix_transport_cost(vec![
            create_matrix_data(Profile::default(), Some(0.), (0., 2), (0., 2)),
            create_matrix_data(Profile::default(), Some(1.), (0., 1), (0., 2)),
        ])
        .err(),
        Some("distance and duration collections have different length".to_string())
    );
}

#[test]
fn can_return_error_when_mixing_timestamps() {
    let p0 = Profile::default();
    let p1 = Profile::new(1, None);

    assert_eq!(
        TimeAwareMatrixTransportCost::new(vec![create_matrix_data(Profile::default(), None, (0., 1), (0., 1))], 1,)
            .err(),
        Some("time-aware routing requires all matrices to have timestamp".to_string())
    );

    assert_eq!(
        TimeAwareMatrixTransportCost::new(
            vec![
                create_matrix_data(p0.clone(), Some(0.), (0., 1), (0., 1)),
                create_matrix_data(p0.clone(), None, (0., 1), (0., 1))
            ],
            1,
        )
        .err(),
        Some("time-aware routing requires all matrices to have timestamp".to_string())
    );

    assert_eq!(
        TimeAwareMatrixTransportCost::new(vec![create_matrix_data(p0.clone(), Some(0.), (0., 1), (0., 1))], 1,).err(),
        Some("should not use time aware matrix routing with single matrix".to_string())
    );

    assert_eq!(
        TimeAwareMatrixTransportCost::new(
            vec![
                create_matrix_data(p0.clone(), Some(0.), (1., 1), (1., 1)), //
                create_matrix_data(p0, Some(1.), (1., 1), (1., 1)),         //
                create_matrix_data(p1, Some(0.), (1., 1), (1., 1)),         //
            ],
            1,
        )
        .err(),
        Some("should not use time aware matrix routing with single matrix".to_string())
    );
}

#[test]
fn can_interpolate_durations() {
    let route0 = Route { actor: test_actor_with_profile(0), tour: Default::default() };
    let route1 = Route { actor: test_actor_with_profile(1), tour: Default::default() };
    let p0 = route0.actor.vehicle.profile.clone();
    let p1 = route1.actor.vehicle.profile.clone();

    let costs = TimeAwareMatrixTransportCost::new(
        vec![
            create_matrix_data(p0.clone(), Some(0.), (100., 2), (1., 2)),
            create_matrix_data(p0.clone(), Some(10.), (200., 2), (1., 2)),
            create_matrix_data(p1.clone(), Some(0.), (300., 2), (5., 2)),
            create_matrix_data(p1.clone(), Some(10.), (400., 2), (5., 2)),
        ],
        2,
    )
    .unwrap();

    for &(timestamp, duration) in &[(0., 100.), (10., 200.), (15., 200.), (3., 130.), (5., 150.), (7., 170.)] {
        assert_eq!(costs.duration(&route0, 0, 1, TravelTime::Departure(timestamp)), duration);
    }

    for &(timestamp, duration) in &[(0., 300.), (10., 400.), (15., 400.), (3., 330.), (5., 350.), (7., 370.)] {
        assert_eq!(costs.duration(&route1, 0, 1, TravelTime::Departure(timestamp)), duration);
    }

    assert_eq!(costs.distance(&route0, 0, 1, TravelTime::Departure(0.)), 1.);
    assert_eq!(costs.distance(&route1, 0, 1, TravelTime::Departure(0.)), 5.);

    assert_eq!(costs.distance_approx(&p0, 0, 1), 1.);
    assert_eq!(costs.distance_approx(&p1, 0, 1), 5.);
}

parameterized_test! {can_search_for_reserved_time, (times, tests), {
    can_search_for_reserved_time_impl(times, tests);
}}

can_search_for_reserved_time! {
    case01: (vec![(5., 10.), (20., 30.)],
        vec![((6., 6.), Some(0)), ((2., 6.), Some(0)), ((10., 11.), None),((2., 5.), None),
             ((10., 21.), Some(1)), ((25., 27.), Some(1)), ((29., 31.), Some(1)),
             ((0., 3.), None), ((31., 33.), None)]),
    case02: (vec![(0., 10.), (5., 15.)], vec![]),
}

fn can_search_for_reserved_time_impl(times: Vec<(f64, f64)>, tests: Vec<((f64, f64), Option<usize>)>) {
    let route_ctx = create_empty_route_ctx();
    let reserved_times = vec![(
        route_ctx.route().actor.clone(),
        times.iter().cloned().map(|(start, end)| TimeSpan::Window(TimeWindow::new(start, end))).collect::<Vec<_>>(),
    )]
    .into_iter()
    .collect();

    let reserved_time_func = create_reserved_times_func(reserved_times);

    if let Ok(reserved_time_func) = reserved_time_func {
        tests.iter().for_each(|((s, e), expected)| {
            let interval = TimeWindow::new(*s, *e);
            let expected = expected.and_then(|idx| times.get(idx)).map(|(s, e)| TimeWindow::new(*s, *e));

            let result = reserved_time_func.deref()(route_ctx.route(), &interval);

            assert_eq!(result, expected);
        });
    } else {
        assert!(tests.is_empty())
    }
}

mod objective {
    use super::*;
    use crate::construction::heuristics::{InsertionContext, MoveContext};
    use crate::helpers::models::domain::create_empty_insertion_context;
    use crate::models::{Feature, FeatureBuilder, FeatureObjective, GoalContext};
    use rosomaxa::prelude::compare_floats;

    struct TestObjective {
        index: usize,
    }

    impl Objective for TestObjective {
        type Solution = InsertionContext;

        fn total_order(&self, a: &Self::Solution, b: &Self::Solution) -> Ordering {
            let a = self.fitness(a);
            let b = self.fitness(b);

            compare_floats(a, b)
        }

        fn distance(&self, a: &Self::Solution, b: &Self::Solution) -> f64 {
            (self.fitness(a) - self.fitness(b)).abs()
        }

        fn fitness(&self, solution: &Self::Solution) -> f64 {
            solution
                .solution
                .state
                .get(&1)
                .and_then(|any| any.downcast_ref::<Vec<f64>>())
                .and_then(|data| data.get(self.index))
                .cloned()
                .unwrap()
        }
    }

    impl FeatureObjective for TestObjective {
        fn estimate(&self, _: &MoveContext<'_>) -> Cost {
            Cost::default()
        }
    }

    fn create_objective_feature(index: usize) -> Feature {
        FeatureBuilder::default()
            .with_name(format!("test_{index}").as_str())
            .with_objective(TestObjective { index })
            .build()
            .unwrap()
    }

    fn create_individual(data: Vec<f64>) -> InsertionContext {
        let mut individual = create_empty_insertion_context();
        individual.solution.state.insert(1, Arc::new(data));

        individual
    }

    parameterized_test! {can_use_total_order_with_hierarchy, (data_a, data_b, expected), {
        can_use_total_order_with_hierarchy_impl(data_a, data_b, expected);
    }}

    can_use_total_order_with_hierarchy! {
        case01: (vec![0., 1., 2.], vec![0., 1., 2.], Ordering::Equal),
        case02: (vec![1., 1., 2.], vec![0., 1., 2.], Ordering::Greater),
        case03: (vec![0., 1., 2.], vec![1., 1., 2.], Ordering::Less),
        case04: (vec![0., 1., 2.], vec![0., 2., 2.], Ordering::Less),
        case05: (vec![0., 2., 2.], vec![1., 0., 0.], Ordering::Less),
    }

    fn can_use_total_order_with_hierarchy_impl(data_a: Vec<f64>, data_b: Vec<f64>, expected: Ordering) {
        let objective_map = &[vec!["test_0".to_string()], vec!["test_1".to_string()], vec!["test_2".to_string()]];
        let goal = GoalContext::new(
            &[create_objective_feature(0), create_objective_feature(1), create_objective_feature(2)],
            objective_map,
            objective_map,
        )
        .unwrap();

        let a = create_individual(data_a);
        let b = create_individual(data_b);

        let result = goal.total_order(&a, &b);

        assert_eq!(result, expected);
    }

    parameterized_test! {can_use_total_order_with_multi, (data_a, data_b, case, expected), {
        can_use_total_order_with_multi_impl(data_a, data_b, case, expected);
    }}

    can_use_total_order_with_multi! {
        case01: (vec![0., 1., 2.], vec![0., 1., 2.], true, Ordering::Equal),
        case02: (vec![1., 0., 2.], vec![0., 1., 2.], true, Ordering::Equal),
        case03: (vec![1., 0., 2.], vec![0., 1., 0.], true, Ordering::Greater),
        case04: (vec![1., 0., 2.], vec![0., 1., 3.], true, Ordering::Less),
        case05: (vec![1., 1., 2.], vec![0., 1., 2.], true, Ordering::Greater),
        case06: (vec![0., 0., 2.], vec![0., 1., 2.], true, Ordering::Less),
        case07: (vec![1., 0., 2.], vec![0., 1., 2.], true, Ordering::Equal),

        case08: (vec![0., 1., 2.], vec![0., 1., 2.], false, Ordering::Equal),
        case09: (vec![0., 1., 0.], vec![0., 1., 1.], false, Ordering::Less),
        case10: (vec![1., 0., 0.], vec![0., 1., 1.], false, Ordering::Greater),
        case11: (vec![0., 1., 1.], vec![1., 0., 0.], false, Ordering::Less),
        case12: (vec![0., 1., 0.], vec![0., 0., 1.], false, Ordering::Equal),
    }

    fn can_use_total_order_with_multi_impl(data_a: Vec<f64>, data_b: Vec<f64>, case: bool, expected: Ordering) {
        let features = &[create_objective_feature(0), create_objective_feature(1), create_objective_feature(2)];
        let objective_map = if case {
            vec![vec!["test_0".to_string(), "test_1".to_string()], vec!["test_2".to_string()]]
        } else {
            vec![vec!["test_0".to_string()], vec!["test_1".to_string(), "test_2".to_string()]]
        };

        let goal = GoalContext::new(features, objective_map.as_slice(), objective_map.as_slice()).unwrap();

        let a = create_individual(data_a);
        let b = create_individual(data_b);

        let result = goal.total_order(&a, &b);

        assert_eq!(result, expected);
    }
}
