use super::*;
use crate::algorithms::geometry::Point;
use crate::construction::features::{
    CapacityFeatureBuilder, JobGroupDimension, TransportFeatureBuilder, VehicleCapacityDimension, create_group_feature,
};
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::models::domain::get_customer_ids_from_routes;
use crate::helpers::models::problem::get_vehicle_id;
use crate::helpers::models::problem::{TestSingleBuilder, test_multi_with_id};
use crate::helpers::models::solution::{ActivityBuilder, RouteBuilder};
use crate::helpers::solver::*;
use crate::helpers::utils::create_test_environment_with_random;
use crate::helpers::utils::random::FakeRandom;
use crate::models::common::{Cost, Demand, Schedule, SingleDimLoad, TimeWindow};
use crate::models::problem::JobIdDimension;
use crate::models::solution::*;
use crate::models::{Feature, FeatureBuilder, FeatureObjective, FeatureState, ViolationCode};
use rosomaxa::prelude::Environment;
use std::iter::once;

fn create_insertion_success(insertion_ctx: &InsertionContext, insertion_data: (usize, &str, usize)) -> InsertionResult {
    let (route_idx, job_id, insertion_idx) = insertion_data;

    let actor = insertion_ctx.solution.routes.get(route_idx).unwrap().route().actor.clone();
    let job = get_jobs_by_ids(insertion_ctx, &[job_id]).first().cloned().unwrap();
    let activity = Activity {
        place: Place { idx: 0, location: 0, duration: 0.0, time: TimeWindow::new(0., 1.) },
        schedule: Schedule { arrival: 0., departure: 0. },
        job: Some(job.to_single().clone()),
        commute: None,
    };

    InsertionResult::Success(InsertionSuccess {
        cost: InsertionCost::default(),
        job,
        activities: vec![(activity, insertion_idx)],
        actor,
    })
}

fn create_insertion_ctx(
    matrix: (usize, usize),
    disallowed_pairs: Vec<(&str, &str)>,
    is_open_vrp: bool,
) -> InsertionContext {
    let (problem, solution) =
        generate_matrix_routes_with_disallow_list(matrix.0, matrix.1, is_open_vrp, disallowed_pairs);
    let environment = Arc::new(Environment::default());

    InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment)
}

fn create_default_selectors() -> (LegSelection, BestResultSelector) {
    let leg_selection = LegSelection::Stochastic(Environment::default().random);
    let result_selector = BestResultSelector::default();

    (leg_selection, result_selector)
}

fn create_capacity_constrained_context() -> InsertionContext {
    create_capacity_constrained_context_with(|_, _| {}, None)
}

fn create_capacity_constrained_context_with(
    modify_job: impl Fn(&str, &mut TestSingleBuilder),
    extra_feature: Option<Feature>,
) -> InsertionContext {
    let points =
        [(0., 0.), (18., -6.), (13., 18.), (-7., -19.), (10., -6.), (-14., 6.), (-12., 12.), (-18., -20.), (-2., 15.)];
    let distances =
        generate_matrix_distances_from_points(&points.iter().map(|&(x, y)| Point::new(x, y)).collect::<Vec<_>>())
            .into_iter()
            .map(Float::round)
            .collect::<Vec<_>>();
    let (mut problem, solution) = generate_matrix_routes(
        4,
        2,
        false,
        |_, _, _| TestGoalContextBuilder::default().build(),
        |id, location| {
            let mut builder = TestSingleBuilder::default();
            builder
                .id(id)
                .location(location.map(|location| location + 1))
                .demand(Demand { delivery: (SingleDimLoad::new(1), SingleDimLoad::default()), ..Demand::default() });
            modify_job(id, &mut builder);
            builder.build_shared()
        },
        |mut vehicle| {
            vehicle.dimens.set_vehicle_capacity(SingleDimLoad::new(4));
            vehicle.costs.fixed = 0.;
            vehicle.costs.per_distance = 1.;
            vehicle.costs.per_driving_time = 0.;
            vehicle.costs.per_waiting_time = 0.;
            vehicle.costs.per_service_time = 0.;
            vehicle
        },
        |_| (distances.clone(), distances.clone()),
    );
    let mut goal = TestGoalContextBuilder::empty()
        .add_feature(
            TransportFeatureBuilder::new("transport")
                .set_transport_cost(problem.transport.clone())
                .set_activity_cost(problem.activity.clone())
                .build_minimize_cost()
                .unwrap(),
        )
        .add_feature(
            CapacityFeatureBuilder::<SingleDimLoad>::new("capacity")
                .set_violation_code(ViolationCode(2))
                .build()
                .unwrap(),
        );
    if let Some(feature) = extra_feature {
        goal = goal.add_feature(feature);
    }
    problem.goal = Arc::new(goal.build());
    let mut insertion_ctx =
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), Arc::new(Environment::default()));
    rearrange_jobs_in_routes(&mut insertion_ctx, &[vec!["c1", "c0", "c3", "c2"], vec!["c6", "c4", "c5", "c7"]]);
    insertion_ctx
}

#[test]
fn can_exchange_jobs_in_full_capacity_routes_without_in_place_improvement() {
    let mut insertion_ctx = create_capacity_constrained_context();
    assert_eq!(insertion_ctx.get_total_cost(), Some(180.));

    try_exchange_jobs_in_routes(&mut insertion_ctx, (0, 1), &LegSelection::Exhaustive, &BestResultSelector::default());

    assert_eq!(insertion_ctx.get_total_cost(), Some(154.));
    assert!(insertion_ctx.solution.required.is_empty());
    assert!(insertion_ctx.solution.unassigned.is_empty());
    assert!(insertion_ctx.solution.routes.iter().all(|route| route.route().tour.job_count() == 4));
    assert_eq!(insertion_ctx.solution.registry.resources().available().count(), 0);
    assert_eq!(
        insertion_ctx.solution.routes.iter().flat_map(|route| route.route().tour.jobs()).collect::<HashSet<_>>().len(),
        8
    );
}

#[test]
fn checks_time_windows_after_ranking_positions() {
    let insertion_ctx = create_capacity_constrained_context_with(
        |id, builder| {
            if id == "c1" {
                builder.times(vec![TimeWindow::new(0., 25.)]);
            }
        },
        None,
    );
    let search_ctx: SearchContext = (&insertion_ctx, &LegSelection::Exhaustive, &BestResultSelector::default());
    let jobs = get_jobs_by_ids(&insertion_ctx, &["c1", "c6"]);
    let route_ctx = &insertion_ctx.solution.routes[1];
    let positions = find_top_positions(&search_ctx, route_ctx, &jobs[..1]);
    let removal = prepare_job_removal(&search_ctx, route_ctx, &jobs[1]);
    let position = removal.map_position(positions[0][0]).unwrap();
    let invalid = eval_job_insertion_in_route(
        &insertion_ctx,
        &get_evaluation_context(&search_ctx, &jobs[0]),
        &removal.route_ctx,
        InsertionPosition::Concrete(position),
        InsertionResult::make_failure(),
    );
    assert!(invalid.as_success().is_none(), "the cheapest position misses the time window");

    let result = find_best_result(&search_ctx, &removal, &jobs[0], &positions[0]);
    assert_eq!(result.as_success().unwrap().activities[0].1, 0);
}

#[test]
fn refreshes_shared_group_state_before_applying_exchange() {
    let insertion_ctx = create_capacity_constrained_context_with(
        |id, builder| {
            builder.dimens_mut().set_job_group(id.to_string());
        },
        Some(create_group_feature("groups", 8, ViolationCode(3)).unwrap()),
    );
    let proposals = (
        create_insertion_success(&insertion_ctx, (1, "c1", 3)),
        create_insertion_success(&insertion_ctx, (0, "c6", 3)),
    );
    let candidate =
        try_exchange_jobs(&insertion_ctx, proposals, &LegSelection::Exhaustive, &BestResultSelector::default())
            .expect("both groups have been removed from their old routes");

    assert_eq!(candidate.get_total_cost(), Some(154.));
    assert_eq!(insertion_ctx.get_total_cost(), Some(180.));
    assert!(candidate.solution.required.is_empty());
    assert!(candidate.solution.unassigned.is_empty());
    assert_eq!(candidate.solution.registry.resources().available().count(), 0);
}

#[test]
fn follows_surviving_insertion_anchor_after_solution_refresh() {
    struct RemoveConditionalActivity;

    impl FeatureState for RemoveConditionalActivity {
        fn accept_insertion(&self, _: &mut SolutionContext, _: usize, _: &Job) {}

        fn accept_route_state(&self, _: &mut RouteContext) {}

        fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
            // Model a conditional activity removed only while the exchanged job is temporarily absent.
            if !solution_ctx.required.iter().any(|job| job.dimens().get_job_id().is_some_and(|id| id == "c1")) {
                return;
            }

            let conditional = solution_ctx
                .routes
                .iter()
                .flat_map(|route| route.route().tour.jobs())
                .find(|job| job.dimens().get_job_id().is_some_and(|id| id == "c0"))
                .cloned();
            if let Some(conditional) = conditional {
                for route_ctx in &mut solution_ctx.routes {
                    if route_ctx.route().tour.contains(&conditional) {
                        route_ctx.route_mut().tour.remove(&conditional);
                    }
                }
                solution_ctx.ignored.push(conditional);
            }
        }
    }

    let insertion_ctx = create_capacity_constrained_context_with(
        |_, _| {},
        Some(FeatureBuilder::default().with_name("conditional").with_state(RemoveConditionalActivity).build().unwrap()),
    );
    let proposals = (
        create_insertion_success(&insertion_ctx, (1, "c1", 3)),
        create_insertion_success(&insertion_ctx, (0, "c6", 2)),
    );

    let candidate =
        try_exchange_jobs(&insertion_ctx, proposals, &LegSelection::Exhaustive, &BestResultSelector::default())
            .expect("the insertion after c3 remains feasible when c0 is removed");

    assert_eq!(get_customer_ids_from_routes(&candidate), vec![vec!["c3", "c6", "c2"], vec!["c4", "c5", "c7", "c1"]]);
    assert_eq!(
        get_customer_ids_from_routes(&insertion_ctx),
        vec![vec!["c1", "c0", "c3", "c2"], vec!["c6", "c4", "c5", "c7"]]
    );
    assert_eq!(candidate.solution.get_jobs_amount(), insertion_ctx.solution.get_jobs_amount());
    assert_eq!(candidate.solution.ignored.len(), 1);
    assert!(candidate.solution.required.is_empty());
    assert!(candidate.solution.unassigned.is_empty());
    assert_eq!(candidate.solution.registry.resources().available().count(), 0);
}

#[test]
fn maps_positions_after_removing_noncontiguous_multi_activities() {
    let insertion_ctx = create_insertion_ctx((5, 1), vec![], false);
    let multi = test_multi_with_id(
        "multi",
        vec![
            TestSingleBuilder::default().location(Some(1)).build_shared(),
            TestSingleBuilder::default().location(Some(3)).build_shared(),
        ],
    );
    let singles = get_jobs_by_ids(&insertion_ctx, &["c0", "c1", "c2"]);
    let jobs = [singles[0].to_single(), &multi.jobs[0], singles[1].to_single(), &multi.jobs[1], singles[2].to_single()];
    let route = RouteBuilder::default()
        .with_vehicle(&insertion_ctx.problem.fleet, "0")
        .add_activities(jobs.into_iter().map(|single| {
            ActivityBuilder::with_location(single.places[0].location.unwrap()).job(Some(single.clone())).build()
        }))
        .build();
    let mut route_ctx = RouteContext::new_with_state(route, RouteState::default());
    insertion_ctx.problem.goal.accept_route_state(&mut route_ctx);
    let search_ctx: SearchContext = (&insertion_ctx, &LegSelection::Exhaustive, &BestResultSelector::default());

    let removal = prepare_job_removal(&search_ctx, &route_ctx, &Job::Multi(multi));

    assert_eq!(removal.removed_indices, vec![2, 4]);
    assert_eq!(removal.route_ctx.route().tour.job_activity_count(), 3);
    assert_eq!(
        (0..6).map(|index| removal.map_position(index)).collect::<Vec<_>>(),
        vec![Some(0), None, None, None, None, Some(3)]
    );
}

#[test]
fn ranks_positions_with_configured_objective() {
    struct PreferEarlierPosition;
    impl FeatureObjective for PreferEarlierPosition {
        fn fitness(&self, _: &InsertionContext) -> Cost {
            0.
        }
        fn estimate(&self, move_ctx: &MoveContext<'_>) -> Cost {
            match move_ctx {
                MoveContext::Activity { activity_ctx, .. } => activity_ctx.index as Float,
                _ => 0.,
            }
        }
    }
    let mut insertion_ctx = create_capacity_constrained_context();
    Arc::get_mut(&mut insertion_ctx.problem).unwrap().goal = Arc::new(
        TestGoalContextBuilder::empty()
            .add_feature(
                FeatureBuilder::default()
                    .with_name("prefer_earlier")
                    .with_objective(PreferEarlierPosition)
                    .build()
                    .unwrap(),
            )
            .build(),
    );
    let search_ctx: SearchContext = (&insertion_ctx, &LegSelection::Exhaustive, &BestResultSelector::default());
    let jobs = get_jobs_by_ids(&insertion_ctx, &["c1"]);

    assert_eq!(find_top_positions(&search_ctx, &insertion_ctx.solution.routes[1], &jobs), vec![vec![0, 1, 2]]);
}

#[test]
fn respects_reached_quota_before_preprocessing() {
    struct ReachedQuota;
    impl Quota for ReachedQuota {
        fn is_reached(&self) -> bool {
            true
        }
    }
    let insertion_ctx = create_capacity_constrained_context();
    let (proposal, is_reached) = find_exchange_jobs_in_routes(
        &insertion_ctx,
        (0, 1),
        &LegSelection::Exhaustive,
        &BestResultSelector::default(),
        Some(&ReachedQuota),
    );
    assert!(proposal.is_none());
    assert!(is_reached);
}

parameterized_test! { can_use_exchange_swap_star, (jobs_order, expected), {
    can_use_exchange_swap_star_impl(jobs_order, expected);
}}

can_use_exchange_swap_star! {
    case_01: (
        vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"], vec!["c6", "c7", "c8"]],
        vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"], vec!["c6", "c7", "c8"]],
    ),
    case_02: (
        vec![vec!["c0", "c1", "c3"], vec!["c4", "c7", "c2"], vec!["c6", "c5", "c8"]],
        vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"], vec!["c6", "c7", "c8"]],
    ),
    case_03: (
        vec![vec!["c0", "c8", "c3"], vec!["c4", "c7", "c2"], vec!["c6", "c5", "c1"]],
        vec![vec!["c0", "c1", "c2"], vec!["c6", "c7", "c8"], vec!["c3", "c4", "c5"]],
    ),
}

fn can_use_exchange_swap_star_impl(jobs_order: Vec<Vec<&str>>, expected: Vec<Vec<&str>>) {
    let matrix = (3, 3);
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 9])));
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let mut insertion_ctx =
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());
    rearrange_jobs_in_routes(&mut insertion_ctx, jobs_order.as_slice());
    let vehicles = insertion_ctx
        .solution
        .routes
        .iter()
        .map(|route_ctx| get_vehicle_id(&route_ctx.route().actor.vehicle).clone())
        .collect::<Vec<_>>();
    assert_eq!(vehicles, vec!["0", "1", "2"]);

    let result = ExchangeSwapStar::new(environment.random.clone())
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx);

    match result {
        Some(result) => compare_with_ignore(get_customer_ids_from_routes(&result).as_slice(), expected.as_slice(), ""),
        None => assert_eq!(jobs_order, expected),
    }
}

#[test]
fn can_keep_locked_jobs_in_place() {
    let jobs_order = vec![vec!["c0", "c1", "c3"], vec!["c4", "c7", "c2"], vec!["c6", "c5", "c8"]];
    let locked_ids = vec!["c2", "c3"];
    let matrix = (3, 3);
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], vec![0.; 9])));
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let mut insertion_ctx = promote_to_locked(
        InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone()),
        locked_ids.as_slice(),
    );
    rearrange_jobs_in_routes(&mut insertion_ctx, jobs_order.as_slice());

    let insertion_ctx = ExchangeSwapStar::new(environment.random.clone())
        .explore(&create_default_refinement_ctx(insertion_ctx.problem.clone()), &insertion_ctx)
        .expect("cannot find new solution");

    let result_ids = get_customer_ids_from_routes(&insertion_ctx);
    assert!(result_ids[0].contains(&"c3".to_string()));
    assert!(!result_ids[0].contains(&"c2".to_string()));

    assert!(result_ids[1].contains(&"c2".to_string()));
    assert!(!result_ids[1].contains(&"c3".to_string()));

    assert!(!result_ids[2].contains(&"c2".to_string()));
    assert!(!result_ids[2].contains(&"c3".to_string()));
}

#[test]
fn can_exchange_jobs_in_routes() {
    let route_pair = (0, 1);
    let disallowed_pairs = vec![];
    let job_order = vec![vec!["c0", "c1", "c3"], vec!["c4", "c5", "c2"]];
    let expected_route_ids = vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"]];

    let matrix = (3, 2);
    let mut insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs, true);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order.as_slice());
    let (leg_selection, result_selector) = create_default_selectors();

    try_exchange_jobs_in_routes(&mut insertion_ctx, route_pair, &leg_selection, &result_selector);

    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), &expected_route_ids, "");
}

parameterized_test! { can_exchange_single_jobs, (outer_insertion, inner_insertion, disallowed_pairs, expected_route_ids), {
    can_exchange_single_jobs_impl(outer_insertion, inner_insertion, disallowed_pairs, expected_route_ids);
}}

can_exchange_single_jobs! {
    case_01: ((0, "c3", 0), (1, "c1", 0), vec![], vec![vec!["c3", "c0", "c2"], vec!["c1", "c4", "c5"]]),
    case_02: ((0, "c3", 1), (1, "c1", 0), vec![], vec![vec!["c0", "c3", "c2"], vec!["c1", "c4", "c5"]]),
    case_03: ((0, "c3", 2), (1, "c1", 0), vec![], vec![vec!["c0", "c2", "c3"], vec!["c1", "c4", "c5"]]),
    case_04: ((0, "c3", 0), (1, "c1", 1), vec![], vec![vec!["c3", "c0", "c2"], vec!["c4", "c1", "c5"]]),
    case_05: ((0, "c3", 0), (1, "c1", 2), vec![], vec![vec!["c3", "c0", "c2"], vec!["c4", "c5", "c1"]]),

    case_09: ((0, "c3", 0), (1, "c1", 0), vec![("cX", "c4")], vec![vec!["c0", "c1", "c2"], vec!["c3", "c4", "c5"]]),
}

fn can_exchange_single_jobs_impl(
    outer_insertion: (usize, &str, usize),
    inner_insertion: (usize, &str, usize),
    disallowed_pairs: Vec<(&str, &str)>,
    expected_route_ids: Vec<Vec<&str>>,
) {
    let matrix = (3, 2);
    let mut insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs, false);
    let (leg_selection, result_selector) = create_default_selectors();
    let insertion_pair = (
        create_insertion_success(&insertion_ctx, outer_insertion),
        create_insertion_success(&insertion_ctx, inner_insertion),
    );

    if let Some(candidate) = try_exchange_jobs(&insertion_ctx, insertion_pair, &leg_selection, &result_selector) {
        insertion_ctx = candidate;
    }

    compare_with_ignore(get_customer_ids_from_routes(&insertion_ctx).as_slice(), &expected_route_ids, "");
}

parameterized_test! { can_find_insertion_cost, (job_id, expected), {
    can_find_insertion_cost_impl(job_id, expected);
}}

can_find_insertion_cost! {
    case_01: ("c0", 0.),
    case_02: ("c1", 0.),
    case_03: ("c2", 4.),
}

fn can_find_insertion_cost_impl(job_id: &str, expected: Cost) {
    let matrix = (3, 1);
    let insertion_ctx = create_insertion_ctx(matrix, vec![], false);
    let (leg_selection, result_selector) = create_default_selectors();
    let search_ctx: SearchContext = (&insertion_ctx, &leg_selection, &result_selector);
    let job = get_jobs_by_ids(&insertion_ctx, &[job_id]).first().cloned().unwrap();
    let route_ctx = insertion_ctx.solution.routes.first().unwrap();

    let result = prepare_job_removal(&search_ctx, route_ctx, &job).original_cost;

    assert_eq!(result, InsertionCost::new(&[expected]));
}

parameterized_test! { can_find_in_place_result, (route_idx, insert_job, extract_job, disallowed_pairs, job_order, expected), {
    can_find_in_place_result_impl(route_idx, insert_job, extract_job, disallowed_pairs, job_order, expected);
}}

can_find_in_place_result! {
    case_01: (0, "c2", "c3", vec![], vec![vec!["c0", "c1", "c3"], vec!["c4", "c5", "c2"]], Some((2., 2))),
    case_02: (0, "c1", "c3", vec![], vec![vec!["c0", "c3", "c2"], vec!["c4", "c5", "c1"]], Some((0., 1))),
    case_03: (0, "c0", "c3", vec![], vec![vec!["c3", "c1", "c2"], vec!["c4", "c5", "c0"]], Some((0., 0))),
    case_04: (0, "c3", "c0", vec![], vec![vec!["c0", "c1", "c2"], vec!["c4", "c5", "c3"]], Some((8., 0))),
}

fn can_find_in_place_result_impl(
    route_idx: usize,
    insert_job: &str,
    extract_job: &str,
    disallowed_pairs: Vec<(&str, &str)>,
    job_order: Vec<Vec<&str>>,
    expected: Option<(Cost, usize)>,
) {
    let matrix = (3, 2);
    let mut insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs, true);
    rearrange_jobs_in_routes(&mut insertion_ctx, job_order.as_slice());
    let (leg_selection, result_selector) = create_default_selectors();
    let jobs_map = get_jobs_map_by_ids(&insertion_ctx);
    let search_ctx: SearchContext = (&insertion_ctx, &leg_selection, &result_selector);
    let route_ctx = insertion_ctx.solution.routes.get(route_idx).unwrap();
    let insert_job = jobs_map.get(insert_job).unwrap();
    let extract_job = jobs_map.get(extract_job).unwrap();
    let in_place_ctx = prepare_job_removal(&search_ctx, route_ctx, extract_job);

    let result = find_in_place_result(&search_ctx, &in_place_ctx, insert_job)
        .try_into()
        .ok()
        .map(|success: InsertionSuccess| (success.cost, success.activities.first().unwrap().1));

    assert_eq!(result, expected.map(|(cost, position)| (InsertionCost::new(&[cost]), position)));
}

parameterized_test! { can_find_top_positions, (job_id, disallowed_pairs, expected), {
    can_find_top_positions_impl(job_id, disallowed_pairs, expected);
}}

can_find_top_positions! {
    case_01: ("c5", vec![], vec![5, 4, 3]),
    case_02: ("c5", vec![("c3", "c4")], vec![5, 4, 3]),
    case_03: ("c5", vec![("cX", "cX")], vec![5, 4, 3]),
}

fn can_find_top_positions_impl(job_id: &str, disallowed_pairs: Vec<(&str, &str)>, expected: Vec<usize>) {
    let matrix = (5, 2);
    let insertion_ctx = create_insertion_ctx(matrix, disallowed_pairs, true);
    let (leg_selection, result_selector) = create_default_selectors();
    let search_ctx: SearchContext = (&insertion_ctx, &leg_selection, &result_selector);
    let job_ids = get_jobs_by_ids(&insertion_ctx, &[job_id]);
    let route_ctx = insertion_ctx.solution.routes.first().unwrap();

    let results =
        find_top_positions(&search_ctx, route_ctx, job_ids.as_slice()).iter().flatten().copied().collect::<Vec<_>>();

    assert_eq!(results, expected);
}

parameterized_test! { can_create_route_pairs, (route_pairs_threshold, is_proximity, expected_length), {
    can_create_route_pairs_impl(route_pairs_threshold, is_proximity, expected_length);
}}

can_create_route_pairs! {
    case_01: (9, true, 3),
    case_02: (9, false, 3),
    case_03: (2, true, 2),
    case_04: (2, false, 2),
}

fn can_create_route_pairs_impl(route_pairs_threshold: usize, is_proximity: bool, expected_length: usize) {
    let reals = once(i32::from(is_proximity)).chain([0; 32]).map(|value| value as Float).collect();
    let matrix = (3, 3);
    let environment = create_test_environment_with_random(Arc::new(FakeRandom::new(vec![], reals)));
    let (problem, solution) = generate_matrix_routes_with_defaults(matrix.0, matrix.1, true);
    let insertion_ctx = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);

    let pairs = create_route_pairs(&insertion_ctx, route_pairs_threshold);

    assert_eq!(pairs.len(), expected_length);
}
