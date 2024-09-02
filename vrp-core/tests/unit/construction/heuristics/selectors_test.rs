use super::*;
use crate::helpers::models::solution::RouteContextBuilder;
use crate::helpers::solver::generate_matrix_routes_with_defaults;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::Cost;
use std::sync::Arc;

mod noise_checks {
    use super::*;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
    use crate::helpers::models::problem::TestSingleBuilder;

    fn make_success(cost: Cost) -> InsertionResult {
        InsertionResult::make_success(
            InsertionCost::new(&[cost]),
            TestSingleBuilder::default().id("job1").build_as_job_ref(),
            vec![],
            &RouteContextBuilder::default().build(),
        )
    }

    parameterized_test! {can_compare_insertion_result_with_noise, (left, right, reals, expected_result), {
        can_compare_insertion_result_with_noise_impl(left, right, reals, expected_result);
    }}

    can_compare_insertion_result_with_noise! {
        case_01: (make_success(10.), make_success(11.), vec![0.05, 1.2, 0.05, 1.],  Some(11.)),
        case_02: (make_success(11.), make_success(10.), vec![0.05, 0.8, 0.05, 1.],  Some(11.)),
        case_03: (make_success(11.), make_success(10.), vec![0.05, 1., 0.2],  Some(10.)),

        case_04: (InsertionResult::make_failure(), make_success(11.), vec![],  Some(11.)),
        case_05: (make_success(10.), InsertionResult::make_failure(), vec![],  Some(10.)),
        case_06: (InsertionResult::make_failure(), InsertionResult::make_failure(), vec![],  None),
    }

    fn can_compare_insertion_result_with_noise_impl(
        left: InsertionResult,
        right: InsertionResult,
        reals: Vec<Float>,
        expected_result: Option<Float>,
    ) {
        let noise_probability = 0.1;
        let noise_range = (0.9, 1.2);
        let random = Arc::new(FakeRandom::new(vec![2], reals));
        let noise = Noise::new_with_ratio(noise_probability, noise_range, random);
        let insertion_ctx = TestInsertionContextBuilder::default().build();

        let actual_result = NoiseResultSelector::new(noise).select_insertion(&insertion_ctx, left, right);

        match (actual_result, expected_result) {
            (InsertionResult::Success(success), Some(cost)) => assert_eq!(success.cost, InsertionCost::new(&[cost])),
            (InsertionResult::Failure(_), None) => {}
            _ => unreachable!(),
        }
    }
}

mod iterators {
    use super::*;

    #[test]
    fn can_get_size_hint_for_tour_legs() {
        let (_, solution) = generate_matrix_routes_with_defaults(5, 1, 1000., false);

        assert_eq!(solution.routes[0].tour.legs().skip(2).size_hint().0, 4);
    }
}

mod selections {
    use super::*;
    use crate::helpers::models::problem::TestSingleBuilder;

    parameterized_test! {can_use_stochastic_selection_mode, (skip, activities, expected_threshold), {
        can_use_stochastic_selection_mode_impl(skip, activities, expected_threshold);
    }}

    can_use_stochastic_selection_mode! {
        case_01: (0, 1000, 100),
        case_02: (991, 1000, 11),
    }

    fn can_use_stochastic_selection_mode_impl(skip: usize, activities: usize, expected_threshold: usize) {
        let target = 10;
        let selection_mode = LegSelection::Stochastic(Environment::default().random);
        let (_, solution) = generate_matrix_routes_with_defaults(activities, 1, 1000., false);
        let route_ctx = RouteContext::new_with_state(solution.routes.into_iter().next().unwrap(), Default::default());
        let mut counter = 0;

        let _ = selection_mode.sample_best(
            &route_ctx,
            &TestSingleBuilder::default().build_as_job_ref(),
            skip,
            -1,
            &mut |leg: Leg, _| {
                counter += 1;
                ControlFlow::Continue(leg.1 as i32)
            },
            |lhs: &i32, rhs: &i32| {
                match (*lhs % 2 == 0, *rhs % 2 == 0) {
                    (true, false) => return true,
                    (false, true) => return false,
                    _ => {}
                }
                match (*lhs, *rhs) {
                    (_, rhs) if rhs == target => false,
                    (lhs, _) if lhs == target => true,
                    (lhs, rhs) => (lhs - target).abs() < (rhs - target).abs(),
                }
            },
        );

        assert!(counter < expected_threshold);
    }
}

mod positions {
    use super::*;
    use crate::helpers::construction::heuristics::TestInsertionContextBuilder;
    use crate::helpers::models::problem::TestSingleBuilder;

    parameterized_test! {can_decide_how_to_fold, (jobs, routes, expected_result), {
        can_decide_how_to_fold_impl(jobs, routes, expected_result);
    }}

    can_decide_how_to_fold! {
        case01: (8, 2, true),
        case02: (2, 8, false),
        case03: (4, 4, false),
    }

    fn can_decide_how_to_fold_impl(jobs: usize, routes: usize, expected_result: bool) {
        let insertion_ctx = TestInsertionContextBuilder::default()
            .with_routes((0..routes).map(|_| RouteContextBuilder::default().build()).collect())
            .with_required((0..jobs).map(|_| TestSingleBuilder::default().build_as_job_ref()).collect())
            .build();

        assert_eq!(PositionInsertionEvaluator::is_fold_jobs(&insertion_ctx), expected_result);
    }
}
