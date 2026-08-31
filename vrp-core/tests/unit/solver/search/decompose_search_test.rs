use super::*;
use crate::helpers::models::domain::TestGoalContextBuilder;
use crate::helpers::solver::generate_matrix_routes_with_defaults;
use crate::helpers::utils::random::FakeRandom;
use crate::models::{FeatureBuilder, FeatureObjective, FeatureState};
use crate::prelude::{Cost, Job};
use crate::solver::create_default_heuristic_operator;
use rosomaxa::prelude::*;

struct RouteCountKey;

struct RouteCountObjective;

impl FeatureObjective for RouteCountObjective {
    fn fitness(&self, solution: &InsertionContext) -> Cost {
        *solution
            .solution
            .state
            .get_value::<RouteCountKey, Cost>()
            .expect("decomposed solution state should be initialized")
    }

    fn estimate(&self, _: &MoveContext<'_>) -> Cost {
        Cost::default()
    }
}

struct RouteCountState;

impl FeatureState for RouteCountState {
    fn accept_insertion(&self, solution_ctx: &mut SolutionContext, _: usize, _: &Job) {
        self.accept_solution_state(solution_ctx);
    }

    fn accept_route_state(&self, _: &mut RouteContext) {}

    fn accept_solution_state(&self, solution_ctx: &mut SolutionContext) {
        solution_ctx.state.set_value::<RouteCountKey, Cost>(solution_ctx.routes.len() as Cost);
    }
}

struct NoopSearch;

impl HeuristicSearchOperator for NoopSearch {
    type Context = RefinementContext;
    type Objective = GoalContext;
    type Solution = InsertionContext;

    fn search(&self, _: &Self::Context, solution: &Self::Solution) -> Self::Solution {
        solution.deep_copy()
    }
}

fn assert_registry_consistency(insertion_ctx: &InsertionContext) {
    let all = insertion_ctx.solution.registry.resources().all().collect::<HashSet<_>>();
    let available = insertion_ctx.solution.registry.resources().available().collect::<HashSet<_>>();
    let used = insertion_ctx.solution.routes.iter().map(|route| route.route().actor.clone()).collect::<HashSet<_>>();

    assert_eq!(used.len(), insertion_ctx.solution.routes.len());
    assert!(used.is_disjoint(&available));
    assert_eq!(used.union(&available).cloned().collect::<HashSet<_>>(), all);
}

#[test]
fn can_decide_whether_to_retry() {
    assert!(should_retry(0, 2, true, &FakeRandom::new(vec![], vec![])));
    assert!(!should_retry(1, 2, true, &FakeRandom::new(vec![], vec![])));
    assert!(should_retry(0, 2, false, &FakeRandom::new(vec![], vec![0.1])));
    assert!(!should_retry(0, 2, false, &FakeRandom::new(vec![], vec![0.3])));
}

#[test]
fn can_sample_fallback_parts() {
    assert_eq!(sample_fallback_part_indices(3, &FakeRandom::new(vec![2], vec![])), (2, None));
    assert_eq!(sample_fallback_part_indices(4, &FakeRandom::new(vec![1, 1], vec![])), (1, Some(2)));
    assert_eq!(sample_fallback_part_indices(5, &FakeRandom::new(vec![3, 1], vec![])), (3, Some(1)));
}

#[test]
fn can_create_multiple_insertion_ctxs_without_unassigned() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());

    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 2)).unwrap();

    assert_eq!(individuals.len(), 4);
    assert_eq!(individuals[0].solution.routes.len(), 2);
    assert_eq!(individuals[1].solution.routes.len(), 2);
    assert_eq!(individuals[2].solution.routes.len(), 2);
    assert_eq!(individuals[3].solution.routes.len(), 1);
}

#[test]
fn can_partition_route_and_registry_scopes_without_overlap() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());
    let expected_actors =
        individual.solution.routes.iter().map(|route| route.route().actor.clone()).collect::<HashSet<_>>();

    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 4)).unwrap();
    let actual_actors = individuals
        .iter()
        .flat_map(|individual| individual.solution.routes.iter().map(|route| route.route().actor.clone()))
        .collect::<Vec<_>>();

    assert_eq!(actual_actors.len(), expected_actors.len());
    assert_eq!(actual_actors.iter().cloned().collect::<HashSet<_>>(), expected_actors);

    individuals.iter().for_each(|individual| {
        let route_actors =
            individual.solution.routes.iter().map(|route| route.route().actor.clone()).collect::<HashSet<_>>();
        let registry_actors = individual.solution.registry.resources().all().collect::<HashSet<_>>();

        assert_eq!(registry_actors, route_actors);
        assert_eq!(individual.solution.registry.resources().available().count(), 0);

        let mut registry = individual.solution.registry.deep_copy();
        individual.solution.routes.iter().for_each(|route| assert!(registry.free_route(route.deep_copy())));
        assert_eq!(registry.resources().available().count(), individual.solution.routes.len());
    });
}

#[test]
fn can_initialize_solution_state_for_each_fragment() {
    let environment = Arc::new(Environment::default());
    let (mut problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    problem.goal = Arc::new(
        TestGoalContextBuilder::empty()
            .add_feature(
                FeatureBuilder::default()
                    .with_name("route_count")
                    .with_objective(RouteCountObjective)
                    .with_state(RouteCountState)
                    .build()
                    .unwrap(),
            )
            .build(),
    );
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());

    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 4)).unwrap();

    assert!(
        individuals
            .iter()
            .all(|individual| { individual.fitness().next().unwrap() == individual.solution.routes.len() as Cost })
    );
}

#[test]
fn can_create_multiple_insertion_ctxs_with_unassigned() {
    let environment = Arc::new(Environment::default());
    let (problem, mut solution) = generate_matrix_routes_with_defaults(5, 6, false);
    solution.registry.free_actor(&solution.routes[0].actor);
    solution.unassigned.extend(solution.routes[0].tour.jobs().cloned().map(|job| (job, UnassignmentInfo::Unknown)));
    solution.routes.remove(0);
    let individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());

    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 2)).unwrap();

    assert_eq!(individuals.len(), 4);

    assert_eq!(individuals[0].solution.routes.len(), 2);
    assert_eq!(individuals[0].solution.unassigned.len(), 0);

    assert_eq!(individuals[1].solution.routes.len(), 2);
    assert_eq!(individuals[1].solution.unassigned.len(), 0);

    assert_eq!(individuals[2].solution.routes.len(), 1);
    assert_eq!(individuals[2].solution.unassigned.len(), 0);

    assert_eq!(individuals[3].solution.routes.len(), 0);
    assert_eq!(individuals[3].solution.unassigned.len(), 5);
}

#[test]
fn can_partition_locked_jobs() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(4, 3, false);
    let mut individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());
    individual.solution.locked =
        individual.solution.routes.iter().flat_map(|route| route.route().tour.jobs().take(1).cloned()).collect();

    let expected = individual.solution.locked.clone();
    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 2)).unwrap();
    let actual =
        individuals.iter().flat_map(|individual| individual.solution.locked.iter().cloned()).collect::<HashSet<_>>();

    assert_eq!(actual, expected);
    assert_eq!(individuals.len(), 2);
    assert!(individuals.iter().all(|individual| {
        individual.solution.locked.iter().all(|job| {
            individual.solution.routes.iter().any(|route| route.route().tour.jobs().any(|candidate| candidate == job))
        })
    }));
}

#[test]
fn can_partition_assigned_and_unassigned_locked_jobs() {
    let environment = Arc::new(Environment::default());
    let (problem, mut solution) = generate_matrix_routes_with_defaults(3, 2, false);
    let assigned = solution.routes[1].tour.jobs().next().unwrap().clone();
    let unassigned = solution.routes[0].tour.jobs().next().unwrap().clone();
    solution.registry.free_actor(&solution.routes[0].actor);
    solution.unassigned.extend(solution.routes[0].tour.jobs().cloned().map(|job| (job, UnassignmentInfo::Unknown)));
    solution.routes.remove(0);

    let mut individual = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment.clone());
    individual.solution.locked.extend([assigned.clone(), unassigned.clone()]);
    let individuals = create_multiple_insertion_contexts(&individual, environment, (2, 2)).unwrap();
    let empty = individuals.iter().find(|individual| individual.solution.routes.is_empty()).unwrap();
    let assigned_part = individuals
        .iter()
        .find(|individual| individual.solution.routes.iter().any(|route| route.route().tour.contains(&assigned)))
        .unwrap();

    assert_eq!(empty.solution.locked, HashSet::from([unassigned]));
    assert_eq!(assigned_part.solution.locked, HashSet::from([assigned]));
}

#[test]
fn can_perform_search() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let problem = Arc::new(problem);
    let population = Box::new(GreedyPopulation::new(problem.goal.clone(), 1, None));

    let refinement_ctx = RefinementContext::new(problem.clone(), population, TelemetryMode::None, environment.clone());
    let insertion_ctx = InsertionContext::new_from_solution(problem.clone(), (solution, None), environment.clone());
    let inner_search = create_default_heuristic_operator(problem, environment);
    let decompose_search = DecomposeSearch::new(inner_search, (2, 2), 10);

    let result = decompose_search.search(&refinement_ctx, &insertion_ctx);

    let solution = &result.solution;
    assert!(solution.ignored.is_empty());
    assert!(solution.locked.is_empty());
    assert!(solution.required.is_empty());
    assert!(!solution.routes.is_empty());
    let total_jobs =
        solution.routes.iter().flat_map(|route_ctx| route_ctx.route().tour.jobs()).collect::<HashSet<_>>().len()
            + solution.unassigned.len();
    assert_eq!(total_jobs, 35);
    assert_registry_consistency(&result);
}

#[test]
fn can_recombine_noop_fragments_without_changing_solution() {
    let environment = Arc::new(Environment::default());
    let (problem, solution) = generate_matrix_routes_with_defaults(5, 7, false);
    let problem = Arc::new(problem);
    let population = Box::new(GreedyPopulation::new(problem.goal.clone(), 1, None));
    let refinement_ctx = RefinementContext::new(problem.clone(), population, TelemetryMode::None, environment.clone());
    let insertion_ctx = InsertionContext::new_from_solution(problem, (solution, None), environment);
    let expected_actors =
        insertion_ctx.solution.routes.iter().map(|route| route.route().actor.clone()).collect::<HashSet<_>>();
    let expected_jobs = insertion_ctx
        .solution
        .routes
        .iter()
        .flat_map(|route| route.route().tour.jobs().cloned())
        .collect::<HashSet<_>>();
    let expected_fitness = insertion_ctx.fitness().collect::<Vec<_>>();

    let result = DecomposeSearch::new(Arc::new(NoopSearch), (2, 4), 1).search(&refinement_ctx, &insertion_ctx);
    let actual_actors = result.solution.routes.iter().map(|route| route.route().actor.clone()).collect::<HashSet<_>>();
    let actual_jobs =
        result.solution.routes.iter().flat_map(|route| route.route().tour.jobs().cloned()).collect::<HashSet<_>>();

    assert_eq!(actual_actors, expected_actors);
    assert_eq!(actual_jobs, expected_jobs);
    assert_eq!(result.fitness().collect::<Vec<_>>(), expected_fitness);
    assert!(result.solution.required.is_empty());
    assert!(result.solution.ignored.is_empty());
    assert!(result.solution.unassigned.is_empty());
    assert!(result.solution.routes.iter().all(|route| !route.is_stale()));
    assert_eq!(result.solution.registry.resources().available().count(), 0);
    assert_registry_consistency(&result);
    assert!(Arc::ptr_eq(&result.problem, &refinement_ctx.problem));
    assert!(Arc::ptr_eq(&result.environment, &refinement_ctx.environment));
}

#[test]
fn can_recombine_without_expanding_original_registry_scope() {
    let environment = Arc::new(Environment::default());
    let (problem, mut solution) = generate_matrix_routes_with_defaults(5, 6, false);
    let excluded_actor = solution.routes[0].actor.clone();
    solution.registry.free_actor(&excluded_actor);
    solution.unassigned.extend(solution.routes[0].tour.jobs().cloned().map(|job| (job, UnassignmentInfo::Unknown)));
    solution.routes.remove(0);

    let problem = Arc::new(problem);
    let population = Box::new(GreedyPopulation::new(problem.goal.clone(), 1, None));
    let refinement_ctx = RefinementContext::new(problem.clone(), population, TelemetryMode::None, environment.clone());
    let mut insertion_ctx = InsertionContext::new_from_solution(problem, (solution, None), environment);
    insertion_ctx.solution.registry =
        insertion_ctx.solution.registry.deep_slice(|actor| !std::ptr::eq(actor, excluded_actor.as_ref()));
    let expected_jobs = insertion_ctx
        .solution
        .routes
        .iter()
        .flat_map(|route| route.route().tour.jobs().cloned())
        .chain(insertion_ctx.solution.unassigned.keys().cloned())
        .collect::<HashSet<_>>();

    let result = DecomposeSearch::new(Arc::new(NoopSearch), (2, 4), 1).search(&refinement_ctx, &insertion_ctx);
    let actual_jobs = result
        .solution
        .routes
        .iter()
        .flat_map(|route| route.route().tour.jobs().cloned())
        .chain(result.solution.unassigned.keys().cloned())
        .collect::<HashSet<_>>();
    let mut registry = result.solution.registry.deep_copy();

    assert_eq!(actual_jobs, expected_jobs);
    assert_registry_consistency(&result);
    assert!(registry.get_route(&excluded_actor).is_none());
    assert!(registry.resources().all().all(|actor| actor != excluded_actor));
}

#[test]
fn can_process_all_population_candidates_and_report_them_consistently() {
    let environment = Arc::new(Environment::default());
    let (mut problem, solution) = generate_matrix_routes_with_defaults(2, 3, false);
    problem.goal = Arc::new(
        TestGoalContextBuilder::empty()
            .add_feature(
                FeatureBuilder::default()
                    .with_name("route_count")
                    .with_objective(RouteCountObjective)
                    .with_state(RouteCountState)
                    .build()
                    .unwrap(),
            )
            .build(),
    );
    let baseline = InsertionContext::new_from_solution(Arc::new(problem), (solution, None), environment);
    let remove_route = |mut insertion_ctx: InsertionContext| {
        let route = insertion_ctx.solution.routes.pop().unwrap();
        assert!(insertion_ctx.solution.registry.free_route(route));
        insertion_ctx.problem.goal.accept_solution_state(&mut insertion_ctx.solution);
        insertion_ctx
    };
    let first = remove_route(baseline.deep_copy());
    let second = remove_route(first.deep_copy());
    let mut population = DecomposePopulation::new(baseline.problem.goal.clone(), 1, baseline);

    assert!(population.add_all(vec![first.deep_copy(), second.deep_copy()]));
    assert_eq!(population.best_ref().fitness().next(), second.fitness().next());

    let mut population = DecomposePopulation::new(second.problem.goal.clone(), 1, second);
    assert!(!population.add(first));
    assert_eq!(population.size(), 2);
    assert_eq!(population.ranked().count(), population.size());
    assert_eq!(population.iter().count(), population.size());
}
